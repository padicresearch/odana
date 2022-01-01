use commitlog::{CommitLog, ReadLimit};
use storage::{KVStore, KVEntry};
use std::sync::{Arc, RwLock, RwLockReadGuard};
use anyhow::Result;
use codec::impl_codec;
use codec::{Encoder, Decoder};
use crate::{MorphOperation, Hash};
use serde::{Serialize, Deserialize};
use commitlog::message::{MessageBuf, MessageSet};
use crate::error::Error;

pub type LogDatabaseKV = dyn KVStore<HistoryLog> + Send + Sync;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogIndex {
    message_len: u64,
    offset: u64,
}

impl LogIndex {
    pub fn read_limit(&self) -> ReadLimit {
        fit_read_limit(self.message_len)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogData {
    op: MorphOperation,
    hash: Hash,
}

impl LogData {
    pub fn new(op: MorphOperation, hash: Hash) -> Self {
        Self {
            op,
            hash,
        }
    }
}

impl_codec!(LogData);
impl_codec!(LogIndex);

pub struct HistoryLog {
    commit_log: Arc<RwLock<CommitLog>>,
    kv: Arc<LogDatabaseKV>,
}

impl KVEntry for HistoryLog {
    type Key = u64;
    type Value = LogIndex;

    fn column() -> &'static str {
        "history"
    }
}

pub type OperationsIterator<'a> =
Box<dyn 'a + Send + Iterator<Item=Result<MorphOperation>>>;

pub type HistoryRootIterator<'a> =
Box<dyn 'a + Send + Iterator<Item=Result<Hash>>>;

pub type HistoryLogIterator<'a> =
Box<dyn 'a + Send + Iterator<Item=Result<LogData>>>;

#[inline]
fn fit_read_limit(limit: u64) -> ReadLimit {
    ReadLimit::max_bytes(limit as usize + 32)
}

impl HistoryLog {
    pub fn new(commit_log: Arc<RwLock<CommitLog>>, kv: Arc<LogDatabaseKV>) -> Result<Self> {
        Ok(Self {
            commit_log,
            kv,
        })
    }

    pub fn append(&self, data: LogData) -> Result<()> {
        let mut commit_log = self.commit_log.write().map_err(|e| Error::RWPoison)?;
        let encoded_data = data.encode()?;
        let encoded_data_size = encoded_data.len() as u64;
        let mut msg = MessageBuf::default();
        msg.push(encoded_data);
        // let mut msg = MessageBuf::from_bytes().map_err(|e|
        //     Error::CommitLogMessageError(e))?;
        let range = commit_log.append(&mut msg)?;
        let index = LogIndex {
            message_len: encoded_data_size,
            offset: range.first(),
        };
        self.kv.put(index.offset, index)
    }

    pub fn get(&self, index: u64) -> Result<Option<LogData>> {
        let commit_log = self.commit_log.read().map_err(|e| Error::RWPoison)?;
        let index = match self.kv.get(&index)? {
            None => {
                return Ok(None);
            }
            Some(index) => {
                index
            }
        };
        let message = commit_log.read(index.offset, index.read_limit())?;
        Ok(Some(LogData::decode(message.bytes())?))
    }

    pub fn get_operation(&self, index: u64) -> Result<Option<MorphOperation>> {
        let log = match self.get(index)? {
            None => {
                return Ok(None);
            }
            Some(log) => {
                log
            }
        };

        Ok(Some(log.op))
    }

    pub fn get_root_at(&self, index: u64) -> Result<Option<Hash>> {
        let log = match self.get(index)? {
            None => {
                return Ok(None);
            }
            Some(log) => {
                log
            }
        };

        Ok(Some(log.hash))
    }

    pub fn iter_operations(&self) -> Result<OperationsIterator> {
        let commit_log = self.commit_log.clone();
        let iter = self.kv.iter()?.map( move |(_, v)| {
            v.and_then(|index| {
                let commit_log = commit_log.read().map_err(|e| Error::RWPoison)?;
                commit_log.read(index.offset,index.read_limit() ).map_err(|e|e.into()).and_then(|message|{
                    LogData::decode(message.bytes()).and_then(|data| {
                        Ok(data.op)
                    })
                })
            })
        });
        Ok(Box::new(iter))
    }

    pub fn iter_history(&self) -> Result<HistoryRootIterator> {
        let commit_log = self.commit_log.clone();
        let iter = self.kv.iter()?.map( move |(_, v)| {
            v.and_then(|index| {
                let commit_log = commit_log.read().map_err(|e| Error::RWPoison)?;
                commit_log.read(index.offset,index.read_limit() ).map_err(|e|e.into()).and_then(|message|{
                    LogData::decode(message.bytes()).and_then(|data| {
                        Ok(data.hash)
                    })
                })
            })
        });
        Ok(Box::new(iter))
    }

    pub fn iter(&self) -> Result<HistoryLogIterator> {
        let commit_log = self.commit_log.clone();
        let iter = self.kv.iter()?.map( move |(_, v)| {
            v.and_then(|index| {
                let commit_log = commit_log.read().map_err(|e| Error::RWPoison)?;
                commit_log.read(index.offset, index.read_limit()).map_err(|e|e.into()).and_then(|message|{
                    LogData::decode(message.bytes()).and_then(|data| {
                        Ok(data)
                    })
                })
            })
        });
        Ok(Box::new(iter))
    }

    pub fn last_index(&self) -> u64 {
        let commit_log = match self.commit_log.read().map_err(|e| Error::RWPoison){
            Ok(commit_log) => {
                commit_log
            }
            Err(_) => {
                return Default::default()
            }
        };
        commit_log.last_offset().unwrap_or_default()
    }

    pub fn len(&self) -> u64 {
        let commit_log = match self.commit_log.read().map_err(|e| Error::RWPoison){
            Ok(commit_log) => {
                commit_log
            }
            Err(_) => {
                return Default::default()
            }
        };
        commit_log.last_offset().map(|last_offset| if last_offset > 0 { last_offset + 1 } else { last_offset }).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use crate::logdb::{HistoryLog, LogData};
    use std::sync::{Arc, RwLock};
    use storage::memstore::MemStore;
    use storage::KVEntry;
    use commitlog::{CommitLog, LogOptions};
    use tempdir::TempDir;
    use account::create_account;
    use transaction::{Transaction, make_sign_transaction, TransactionKind};
    use chrono::Utc;
    use crate::{get_operations, MorphOperation};
    use codec::Encoder;
    use std::convert::TryInto;
    use tiny_keccak::Hasher;

    fn sha3_hash(data : &[u8] ) -> [u8;32] {
        let mut out = [0;32];
        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(data);
        sha3.finalize(&mut out);
        out
    }

    fn sample_log_data() -> Vec<LogData> {
        let alice = create_account();
        let bob = create_account();
        let mut ops = Vec::new();
        let t = make_sign_transaction(&alice, Utc::now().timestamp_subsec_nanos(), TransactionKind::Transfer {
            from: alice.pub_key,
            to: bob.pub_key,
            amount: 10
        }).unwrap();
        ops.extend(get_operations(&t).into_iter());
        ops.into_iter().map(|t| {
            LogData::new(t.clone(), sha3_hash(&t.encode().unwrap()))
        }).collect()
    }


    #[test]
    fn log_test() {
        let tmp_dir = TempDir::new("history").unwrap();
        let commit_log = Arc::new(RwLock::new(CommitLog::new(LogOptions::new(tmp_dir.path())).unwrap()));
        let memstore = Arc::new(MemStore::new(vec![HistoryLog::column()]));
        let history = HistoryLog::new(commit_log, memstore).unwrap();

        for log in sample_log_data() {
            history.append( log).unwrap();
        }

        for log in history.iter().unwrap() {
            println!("{:?}", log)
        }
    }
}
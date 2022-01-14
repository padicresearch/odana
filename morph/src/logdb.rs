use std::sync::{Arc, RwLock, RwLockReadGuard};

use anyhow::Result;
use commitlog::{CommitLog, ReadError, ReadLimit};
use commitlog::message::{MessageBuf, MessageSet};
use serde::{Deserialize, Serialize};

use codec::{Decoder, Encoder};
use codec::impl_codec;
use storage::{KVEntry, KVStore};

use crate::{Hash, MorphOperation};
use crate::error::MorphError;

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
        Self { op, hash }
    }
}

impl PartialEq for LogData {
    fn eq(&self, other: &Self) -> bool {
        self.hash.eq(&other.hash)
    }
}

impl Eq for LogData {}

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
    Box<dyn 'a + Send + Iterator<Item = Result<(u64, MorphOperation)>>>;

pub type HistoryRootIterator<'a> = Box<dyn 'a + Send + Iterator<Item = Result<(u64, Hash)>>>;

pub type HistoryLogIterator<'a> = Box<dyn 'a + Send + Iterator<Item = Result<LogData>>>;

#[inline]
fn fit_read_limit(limit: u64) -> ReadLimit {
    ReadLimit::max_bytes(limit as usize + 32)
}

impl HistoryLog {
    pub fn new(commit_log: Arc<RwLock<CommitLog>>, kv: Arc<LogDatabaseKV>) -> Result<Self> {
        Ok(Self { commit_log, kv })
    }

    pub fn append(&self, data: LogData) -> Result<()> {
        let mut commit_log = self.commit_log.write().map_err(|e| MorphError::RWPoison)?;
        let encoded_data = data.encode()?;
        let encoded_data_size = encoded_data.len() as u64;
        // let mut msg = MessageBuf::from_bytes().map_err(|e|
        //     Error::CommitLogMessageError(e))?;
        let offset = commit_log.append_msg(encoded_data)?;
        let index = LogIndex {
            message_len: encoded_data_size,
            offset,
        };
        self.kv.put(index.offset, index)
    }

    pub fn get(&self, index: u64) -> Result<Option<LogData>> {
        let commit_log = self.commit_log.read().map_err(|e| MorphError::RWPoison)?;
        let index = match self.kv.get(&index)? {
            None => {
                return Ok(None);
            }
            Some(index) => index,
        };
        let msg_buf = commit_log.read(index.offset, index.read_limit())?;
        let bytes = msg_buf
            .iter()
            .next()
            .ok_or(MorphError::CommitLogReadErrorCorruptData)?;
        Ok(Some(LogData::decode(bytes.payload())?))
    }

    pub fn get_operation(&self, index: u64) -> Result<Option<MorphOperation>> {
        let log = match self.get(index)? {
            None => {
                return Ok(None);
            }
            Some(log) => log,
        };

        Ok(Some(log.op))
    }

    pub fn get_root_at(&self, index: u64) -> Result<Option<Hash>> {
        let log = match self.get(index)? {
            None => {
                return Ok(None);
            }
            Some(log) => log,
        };

        Ok(Some(log.hash))
    }

    pub fn iter_operations(&self) -> Result<OperationsIterator> {
        let commit_log = self.commit_log.clone();
        let iter = self.kv.iter()?.map(move |(_, v)| {
            v.and_then(|index| {
                let commit_log = commit_log.read().map_err(|e| MorphError::RWPoison)?;
                commit_log
                    .read(index.offset, index.read_limit())
                    .map_err(|e| e.into())
                    .and_then(|msg_buf| {
                        msg_buf
                            .iter()
                            .next()
                            .ok_or(MorphError::CommitLogReadError(ReadError::CorruptLog).into())
                            .and_then(|bytes| {
                                LogData::decode(bytes.payload())
                                    .and_then(|data| Ok((index.offset, data.op)))
                            })
                    })
            })
        });
        Ok(Box::new(iter))
    }

    pub fn iter_history(&self) -> Result<HistoryRootIterator> {
        let commit_log = self.commit_log.clone();
        let iter = self.kv.iter()?.map(move |(_, v)| {
            v.and_then(|index| {
                let commit_log = commit_log.read().map_err(|e| MorphError::RWPoison)?;
                commit_log
                    .read(index.offset, index.read_limit())
                    .map_err(|e| e.into())
                    .and_then(|msg_buf| {
                        msg_buf
                            .iter()
                            .next()
                            .ok_or(MorphError::CommitLogReadError(ReadError::CorruptLog).into())
                            .and_then(|bytes| {
                                LogData::decode(bytes.payload())
                                    .and_then(|data| Ok((index.offset, data.hash)))
                            })
                    })
            })
        });
        Ok(Box::new(iter))
    }

    pub fn iter(&self) -> Result<HistoryLogIterator> {
        let commit_log = self.commit_log.clone();
        let iter = self.kv.iter()?.map(move |(_, v)| {
            v.and_then(|index| {
                let commit_log = commit_log.read().map_err(|e| MorphError::RWPoison)?;
                commit_log
                    .read(index.offset, index.read_limit())
                    .map_err(|e| e.into())
                    .and_then(|msg_buf| {
                        msg_buf
                            .iter()
                            .next()
                            .ok_or(MorphError::CommitLogReadError(ReadError::CorruptLog).into())
                            .and_then(|bytes| {
                                LogData::decode(bytes.payload()).and_then(|data| Ok(data))
                            })
                    })
            })
        });
        Ok(Box::new(iter))
    }

    pub fn last_index(&self) -> u64 {
        let commit_log = match self.commit_log.read().map_err(|e| MorphError::RWPoison) {
            Ok(commit_log) => commit_log,
            Err(_) => {
                return Default::default();
            }
        };
        commit_log.last_offset().unwrap_or_default()
    }

    pub fn len(&self) -> u64 {
        let last_index = self.last_index();
        if last_index > 0 {
            return last_index + 1;
        }
        last_index
    }

    pub fn last_history(&self) -> Option<Hash> {
        let last_index = self.last_index();
        return match self.get_root_at(last_index) {
            Ok(root) => root,
            Err(_) => None,
        };
    }

    pub fn last_op(&self) -> Option<MorphOperation> {
        let last_index = self.last_index();
        return match self.get_operation(last_index) {
            Ok(root) => root,
            Err(_) => None,
        };
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;
    use std::sync::{Arc, RwLock};

    use chrono::Utc;
    use commitlog::{CommitLog, LogOptions};
    use tempdir::TempDir;
    use tiny_keccak::Hasher;

    use account::create_account;
    use codec::{Decoder, Encoder};
    use storage::KVEntry;
    use storage::memstore::MemStore;
    use transaction::make_sign_transaction;
    use types::tx::TransactionKind;

    use crate::{get_operations, MorphOperation};
    use crate::logdb::{HistoryLog, LogData};

    fn sha3_hash(data: &[u8]) -> [u8; 32] {
        let mut out = [0; 32];
        let mut sha3 = tiny_keccak::Sha3::v256();
        sha3.update(data);
        sha3.finalize(&mut out);
        out
    }

    fn sample_log_data() -> Vec<LogData> {
        let alice = create_account();
        let bob = create_account();
        let mut ops = Vec::new();
        let t = make_sign_transaction(
            &alice,
            Utc::now().timestamp_nanos() as u64,
            TransactionKind::Transfer {
                from: alice.pub_key,
                to: bob.pub_key,
                amount: 10,
                fee: 0,
            },
        )
        .unwrap();
        let opsz = get_operations(&t);
        //println!("{:?} ", opsz);
        ops.extend(opsz.into_iter());
        ops.into_iter()
            .map(|t| LogData::new(t.clone(), sha3_hash(&t.encode().unwrap())))
            .collect()
    }

    #[test]
    fn log_test() {
        let tmp_dir = TempDir::new("history").unwrap();
        let commit_log = Arc::new(RwLock::new(
            CommitLog::new(LogOptions::new(tmp_dir.path())).unwrap(),
        ));
        let memstore = Arc::new(MemStore::new(vec![HistoryLog::column()]));
        let history = HistoryLog::new(commit_log, memstore).unwrap();

        let sample_data = sample_log_data();
        for log in sample_data.iter() {
            history.append(log.clone()).unwrap();
        }

        let decoded_history: Vec<_> = history.iter().unwrap().map(|res| res.unwrap()).collect();
        assert_eq!(sample_data, decoded_history);
        assert_eq!(history.len() as usize, sample_data.len());
    }
}

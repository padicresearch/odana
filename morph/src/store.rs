use crate::kv::{KV, Schema};
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use types::Hash;
use codec::{Encoder, Decoder};
use codec::impl_codec;
use crate::{MorphOperation, GENESIS_ROOT};
use anyhow::Result;
use crate::error::MorphError;
use tracing::{warn, Value};
use primitive_types::H160;
use types::account::AccountState;
use rocksdb::{ColumnFamilyDescriptor, BlockBasedOptions, MergeOperands};
use rocksdb::Options;

pub fn default_db_opts() -> Options {
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.increase_parallelism(num_cpus::get() as i32);
    // opts.set_advise_random_on_open(false);
    opts.set_allow_mmap_writes(true);
    opts.set_allow_mmap_reads(true);
    opts.create_missing_column_families(true);
    opts.set_atomic_flush(true);
    // TODO: tune
    opts
}

pub fn default_table_options() -> Options {
    // default db options
    let mut db_opts = Options::default();

    // https://github.com/facebook/rocksdb/wiki/Setup-Options-and-Basic-Tuning#other-general-options
    db_opts.set_level_compaction_dynamic_level_bytes(false);
    db_opts.set_write_buffer_size(32 * 1024 * 1024);

    // block table options
    let mut table_options = BlockBasedOptions::default();
    // table_options.set_block_cache(cache);
    // table_options.set_block_size(16 * 1024);
    // table_options.set_cache_index_and_filter_blocks(true);
    // table_options.set_pin_l0_filter_and_index_blocks_in_cache(true);

    // set format_version 4 https://rocksdb.org/blog/2019/03/08/format-version-4.html
    table_options.set_format_version(4);
    // table_options.set_index_block_restart_interval(16);

    db_opts.set_block_based_table_factory(&table_options);

    db_opts
}

pub type HistoryStorageKV = dyn KV<HistoryStorage> + Send + Sync;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HistoryIKey {
    Root,
    Lookup(Hash),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HistoryIValue {
    operation: MorphOperation,
    root: Hash,
    parent_root: Hash,
    seq: u128,
}

impl_codec!(HistoryIKey);
impl_codec!(HistoryIValue);


impl Schema for HistoryStorage {
    type Key = HistoryIKey;
    type Value = HistoryIValue;

    fn column() -> &'static str {
        "history"
    }

    fn descriptor() -> ColumnFamilyDescriptor {
        ColumnFamilyDescriptor::new(Self::column(), default_table_options())
    }
}

pub struct HistoryStorage {
    mu: Mutex<()>,
    kv: Arc<HistoryStorageKV>,
}

impl HistoryStorage {
    pub fn new(kv: Arc<HistoryStorageKV>) -> Self {
        Self {
            mu: Mutex::new(()),
            kv,
        }
    }

    //TODO:  implement Rollback
    pub fn append(&self, key: Hash, value: MorphOperation) -> Result<HistoryIValue> {
        self.mu.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        anyhow::ensure!(!self.kv.contains(&HistoryIKey::Lookup(key))?, "duplicate state root");
        let root_seq = self.root()?.map(|root| root.seq).unwrap_or_default();
        let value = HistoryIValue {
            operation: value,
            root: key,
            parent_root: self.root_hash()?,
            seq: root_seq + 1,
        };
        self.kv.batch(vec![(HistoryIKey::Lookup(key), value.clone()), (HistoryIKey::Root, value.clone())])?;
        Ok(value)
    }

    pub fn get(&self, key: &HistoryIKey) -> Result<Option<HistoryIValue>> {
        self.kv.get(key)
    }

    pub fn root(&self) -> Result<Option<HistoryIValue>> {
        self.kv.get(&HistoryIKey::Root)
    }

    pub fn root_hash(&self) -> Result<Hash> {
        let root = self.kv.get(&HistoryIKey::Root)?.map(|history| history.root);
        Ok(root.unwrap_or(GENESIS_ROOT))
    }
    pub fn root_seq(&self) -> Result<u128> {
        let root = self.kv.get(&HistoryIKey::Root)?.map(|history| history.seq);
        Ok(root.unwrap_or(0))
    }
}


pub type AccountStateStorageKV = dyn KV<AccountStateStorage> + Send + Sync;

impl Schema for AccountStateStorage {
    type Key = H160;
    type Value = AccountState;

    fn column() -> &'static str {
        "account_state"
    }

    fn descriptor() -> ColumnFamilyDescriptor {
        ColumnFamilyDescriptor::new(Self::column(), default_table_options())
    }
}

#[derive(Clone)]
pub struct AccountStateStorage {
    kv: Arc<AccountStateStorageKV>,
}

impl AccountStateStorage {
    pub fn new(kv: Arc<AccountStateStorageKV>) -> Self {
        Self {
            kv
        }
    }
    pub fn put(&self, key: H160, value: AccountState) -> Result<()> {
        self.kv.put(key, value)
    }

    pub fn get(&self, key: &H160) -> Result<Option<AccountState>> {
        self.kv.get(key)
    }
}

pub type AccountMetadataStorageKV = dyn KV<AccountMetadataStorage> + Send + Sync;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccountRoots(Vec<Hash>);

impl Schema for AccountMetadataStorage {
    type Key = H160;
    type Value = AccountRoots;

    fn column() -> &'static str {
        "account_metadata"
    }

    fn descriptor() -> ColumnFamilyDescriptor {
        let mut opts = default_table_options();
        opts.set_merge_operator_associative("account_metadata_merge_operator", account_metadata_merge_operator);
        ColumnFamilyDescriptor::new(Self::column(), opts)
    }
}

pub fn account_metadata_merge_operator(
    _new_key: &[u8],
    existing_val: Option<&[u8]>,
    operands: &mut MergeOperands,
) -> Option<Vec<u8>> {
    let mut result = existing_val.map(|v| v.to_vec());
    for op in operands {
        match result {
            Some(ref mut val) => {
                val.extend_from_slice(op)
            }
            None => result = Some(op.to_vec()),
        }
    }
    result
}

impl Encoder for AccountRoots {
    fn encode(&self) -> Result<Vec<u8>> {
        let encoded: Vec<_> = self.0.iter().flatten().map(|v| *v).collect();
        Ok(encoded)
    }
}

impl Decoder for AccountRoots {
    fn decode(buf: &[u8]) -> Result<Self> {
        let chunks = buf.chunks_exact(32);
        if !chunks.remainder().is_empty() {
            return Err(MorphError::CodecErrorDecoding.into());
        }
        let mut out = Vec::new();
        for chunk in chunks {
            let mut hash = [0_u8; 32];
            hash.copy_from_slice(chunk);
            out.push(hash);
        }
        Ok(AccountRoots(out))
    }
}


#[derive(Clone)]
pub struct AccountMetadataStorage {
    kv: Arc<AccountMetadataStorageKV>,
}

impl AccountMetadataStorage {
    pub fn new(kv: Arc<AccountMetadataStorageKV>) -> Self {
        Self {
            kv
        }
    }
    pub fn put(&self, key: H160, value: AccountRoots) -> Result<()> {
        self.kv.merge(key, value)
    }

    pub fn get(&self, key: &H160) -> Result<Option<AccountRoots>> {
        self.kv.get(key)
    }
}

fn column_families() -> Vec<ColumnFamilyDescriptor> {
    vec![
        HistoryStorage::descriptor(),
        AccountStateStorage::descriptor(),
        AccountMetadataStorage::descriptor(),
        HistorySequenceStorage::descriptor()
    ]
}

pub type HistorySequenceIterator<'a> =
Box<dyn 'a + Send + Iterator<Item=(Result<u128>, Result<Hash>)>>;
pub type HistorySequenceStorageKV = dyn KV<HistorySequenceStorage> + Send + Sync;

impl Schema for HistorySequenceStorage {
    type Key = u128;
    type Value = Hash;

    fn column() -> &'static str {
        "history_sequence"
    }

    fn descriptor() -> ColumnFamilyDescriptor {
        ColumnFamilyDescriptor::new(Self::column(), default_table_options())
    }
}

#[derive(Clone)]
pub struct HistorySequenceStorage {
    kv: Arc<HistorySequenceStorageKV>,
}

impl HistorySequenceStorage {
    pub fn new(kv: Arc<HistorySequenceStorageKV>) -> Self {
        Self {
            kv
        }
    }
    pub fn put(&self, key: u128, value: Hash) -> Result<()> {
        if self.kv.contains(&key)? {
            warn!(target : "key", warning = "Ward", "already present");
            return Err(MorphError::SequenceAlreadyPresent(key).into());
        }
        self.kv.put(key, value)
    }

    pub fn get(&self, key: &u128) -> Result<Option<Hash>> {
        self.kv.get(key)
    }

    pub fn iter(&self) -> Result<HistorySequenceIterator> {
        self.kv.iter()
    }
}

#[cfg(test)]
mod test {
    use tempdir::TempDir;
    use crate::store::{default_db_opts, column_families, AccountMetadataStorage, AccountRoots, AccountStateStorage, HistoryStorage, HistoryIKey, HistoryIValue, HistorySequenceStorage};
    use std::sync::Arc;
    use account::create_account;
    use crate::MorphOperation;

    #[test]
    fn test_merge_account_meta() {
        let dir = TempDir::new("_test_merge_account_state").unwrap();
        let db = Arc::new(rocksdb::DB::open_cf_descriptors(&default_db_opts(), dir.path(), column_families()).unwrap());
        let account_meta_storage = AccountMetadataStorage::new(db);
        let alice = create_account();
        account_meta_storage.put(alice.address, AccountRoots(vec![[1; 32]])).unwrap();
        account_meta_storage.put(alice.address, AccountRoots(vec![[2; 32]])).unwrap();
        account_meta_storage.put(alice.address, AccountRoots(vec![[3; 32]])).unwrap();

        assert_eq!(account_meta_storage.get(&alice.address).unwrap().unwrap().0, vec![[1_u8; 32], [2_u8; 32], [3_u8; 32]])
    }

    #[test]
    fn test_history() {
        let alice = create_account();
        let dir = TempDir::new("_test_merge_account_state").unwrap();
        let db = Arc::new(rocksdb::DB::open_cf_descriptors(&default_db_opts(), dir.path(), column_families()).unwrap());
        let history_storage = HistoryStorage::new(db);
        history_storage.append([1; 32], MorphOperation::UpdateNonce {
            account: alice.address,
            nonce: 1,
            tx_hash: [0; 32],
        }).unwrap();
        history_storage.append([2; 32], MorphOperation::UpdateNonce {
            account: alice.address,
            nonce: 2,
            tx_hash: [0; 32],
        }).unwrap();
        history_storage.append([3; 32], MorphOperation::UpdateNonce {
            account: alice.address,
            nonce: 3,
            tx_hash: [0; 32],
        }).unwrap();
        println!("{:?}", history_storage.root_hash().unwrap())
    }

    #[test]
    fn test_multi_thread_history() {
        let alice = create_account();
        let dir = TempDir::new("_test_merge_account_state").unwrap();
        let db = Arc::new(rocksdb::DB::open_cf_descriptors(&default_db_opts(), dir.path(), column_families()).unwrap());
        let history_storage = Arc::new(HistoryStorage::new(db.clone()));
        let history_sequence = Arc::new(HistorySequenceStorage::new(db));
        let mut handles = Vec::new();
        for i in 1..=30 {
            let history_storage = history_storage.clone();
            let history_sequence = history_sequence.clone();
            let handle = std::thread::spawn(move || {
                let seq = history_storage.root_seq().unwrap();
                history_sequence.put(seq + 1, [i as u8; 32]).unwrap();
                let his = history_storage.append([i as u8; 32], MorphOperation::UpdateNonce {
                    account: alice.address,
                    nonce: i,
                    tx_hash: [i as u8; 32],
                }).unwrap();
                //history_sequence.put(his.seq, his.root).unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join();
        }

        for (seq, root) in history_sequence.iter().unwrap() {
            println!("SEQ {} ROOT {:?}", seq.unwrap(), root.unwrap());
        }
        println!("{:?}", history_storage.root_hash().unwrap())
    }
}

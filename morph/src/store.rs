use crate::kv::{KV, Schema};
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use types::Hash;
use codec::{Encoder, Decoder};
use codec::impl_codec;
use crate::{MorphOperation, GENESIS_ROOT};
use anyhow::Result;
use crate::error::MorphError;
use tracing::warn;
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

#[derive(Serialize, Deserialize, Clone)]
pub enum HistoryIKey {
    Root,
    Lookup(Hash),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HistoryIValue {
    operation: MorphOperation,
    root: Hash,
    prev_root: Hash,
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

#[derive(Clone)]
pub struct HistoryStorage {
    kv: Arc<HistoryStorageKV>,
}

impl HistoryStorage {
    pub fn set_root(&self, value: HistoryIValue) -> Result<()> {
        self.kv.put(HistoryIKey::Root, value)
    }

    pub fn put(&self, key: HistoryIKey, value: HistoryIValue) -> Result<()> {
        self.kv.put(key, value)
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
        AccountMetadataStorage::descriptor()
    ]
}

#[cfg(test)]
mod test {
    use tempdir::TempDir;
    use crate::store::{default_db_opts, column_families, AccountMetadataStorage, AccountRoots};
    use std::sync::Arc;
    use account::create_account;

    #[test]
    fn test_merge_account_state() {
        let dir = TempDir::new("_test_merge_account_state").unwrap();
        let db = Arc::new(rocksdb::DB::open_cf_descriptors(&default_db_opts(), dir.path(), column_families()).unwrap());
        let account_meta_storage = AccountMetadataStorage::new(db);
        let alice = create_account();
        account_meta_storage.put(alice.address, AccountRoots(vec![[1; 32]])).unwrap();
        account_meta_storage.put(alice.address, AccountRoots(vec![[2; 32]])).unwrap();
        account_meta_storage.put(alice.address, AccountRoots(vec![[3; 32]])).unwrap();

        println!("{:?}", account_meta_storage.get(&alice.address).unwrap())
    }
}
// pub type HistorySequenceStorageKV = dyn KV<HistorySequenceStorage> + Send + Sync;
//
// impl Schema for HistorySequenceStorage {
//     type Key = u128;
//     type Value = Hash;
//
//     fn column() -> &'static str {
//         "history_sequence"
//     }
// }
//
// #[derive(Clone)]
// pub struct HistorySequenceStorage {
//     kv: Arc<HistorySequenceStorageKV>,
// }
//
// impl HistorySequenceStorage {
//
//     pub fn put(&self,key : u128, value : Hash) -> Result<()>{
//         if self.kv.contains(&key)? {
//             warn!(sequence : key, "already present");
//             return Err(MorphError::SequenceAlreadyPresent(key).into());
//         }
//         self.kv.put(key, value)
//     }
//
//     pub fn get(&self,key : &u128) -> Result<Option<Hash>>{
//         self.kv.get(key)
//     }
//
// }
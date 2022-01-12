use crate::error::MorphError;
use crate::kv::{Schema, KV};
use crate::{MorphOperation, GENESIS_ROOT};
use anyhow::Result;
use codec::impl_codec;
use codec::{Decoder, Encoder};
use primitive_types::{H160, H256};
use rocksdb::Options;
use rocksdb::{BlockBasedOptions, ColumnFamilyDescriptor, MergeOperands};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt::{Display, Formatter};
use std::option::Option::Some;
use std::sync::{Arc, Mutex};
use tracing::{warn, Value};
use types::account::AccountState;
use types::Hash;

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

#[derive(Serialize, Deserialize, Clone)]
pub struct HistoryIValue {
    pub operation: MorphOperation,
    pub root: Hash,
    pub parent_root: Hash,
    pub seq: u128,
}

impl std::fmt::Debug for HistoryIValue {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("HistoryIValue")
            .field("operation", &self.operation)
            .field("root", &H256::from(self.root))
            .field("parent_root", &H256::from(self.parent_root))
            .field("seq", &self.seq)
            .finish()
    }
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
        anyhow::ensure!(
            !self.kv.contains(&HistoryIKey::Lookup(key))?,
            "duplicate state root"
        );
        let root_seq = self.root()?.map(|root| root.seq).unwrap_or_default();
        let value = HistoryIValue {
            operation: value,
            root: key,
            parent_root: self.root_hash()?,
            seq: root_seq + 1,
        };
        self.kv.batch(vec![
            (HistoryIKey::Lookup(key), value.clone()),
            (HistoryIKey::Root, value.clone()),
        ])?;
        Ok(value)
    }

    pub fn get(&self, key: Hash) -> Result<Option<HistoryIValue>> {
        self.kv.get(&HistoryIKey::Lookup(key))
    }

    pub fn root_history(&self) -> Result<VecDeque<HistoryIValue>> {
        let mut root = self.root()?.ok_or(anyhow::anyhow!("root not present"))?;
        let mut out = VecDeque::with_capacity(root.seq as usize);
        out.push_front(root.clone());
        while let Some(history) = self.kv.get(&HistoryIKey::Lookup(root.parent_root))? {
            out.push_front(history.clone());
            root = history;
            if root.root == GENESIS_ROOT {
                break;
            }
        }
        Ok(out)
    }

    pub fn address_history(&self, address: H160) -> Result<VecDeque<HistoryIValue>> {
        let mut root = self.root()?.ok_or(anyhow::anyhow!("root not present"))?;
        let mut out = VecDeque::with_capacity(root.seq as usize);
        if root.operation.get_address() == address {
            out.push_front(root.clone());
        }
        while let Some(history) = self.kv.get(&HistoryIKey::Lookup(root.parent_root))? {
            root = history;
            if root.operation.get_address() == address {
                out.push_front(root.clone());
            }
            if root.root == GENESIS_ROOT {
                break;
            }
        }
        Ok(out)
    }

    pub fn root_hash(&self) -> Result<Hash> {
        let root = self.kv.get(&HistoryIKey::Root)?.map(|history| history.root);
        Ok(root.unwrap_or(GENESIS_ROOT))
    }

    pub fn multi_get(&self, key: Vec<Hash>) -> Result<Vec<Option<HistoryIValue>>> {
        let keys: Vec<_> = key
            .into_iter()
            .map(|key| HistoryIKey::Lookup(key))
            .collect();
        self.kv.multi_get(keys)
    }

    pub fn root(&self) -> Result<Option<HistoryIValue>> {
        self.kv.get(&HistoryIKey::Root)
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
        Self { kv }
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
        opts.set_merge_operator_associative(
            "account_metadata_merge_operator",
            account_metadata_merge_operator,
        );
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
            Some(ref mut val) => val.extend_from_slice(op),
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
        Self { kv }
    }
    pub fn put(&self, key: H160, value: [u8; 32]) -> Result<()> {
        self.kv.merge(key, AccountRoots(vec![value]))
    }

    pub fn get(&self, key: &H160) -> Result<Option<Vec<Hash>>> {
        let roots = self.kv.get(key)?.map(|r| r.0);
        Ok(roots)
    }
}

pub(crate) fn column_families() -> Vec<ColumnFamilyDescriptor> {
    vec![
        HistoryStorage::descriptor(),
        AccountStateStorage::descriptor(),
        AccountMetadataStorage::descriptor(),
        HistorySequenceStorage::descriptor(),
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
        Self { kv }
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
    use crate::store::{
        column_families, default_db_opts, AccountMetadataStorage, AccountRoots,
        AccountStateStorage, HistoryIKey, HistoryIValue, HistorySequenceStorage, HistoryStorage,
    };
    use crate::MorphOperation;
    use account::create_account;
    use std::sync::Arc;
    use tempdir::TempDir;

    #[test]
    fn test_merge_account_meta() {
        let dir = TempDir::new("_test_merge_account_state").unwrap();
        let db = Arc::new(
            rocksdb::DB::open_cf_descriptors(&default_db_opts(), dir.path(), column_families())
                .unwrap(),
        );
        let account_meta_storage = AccountMetadataStorage::new(db);
        let alice = create_account();
        account_meta_storage.put(alice.address, [1; 32]).unwrap();
        account_meta_storage.put(alice.address, [2; 32]).unwrap();
        account_meta_storage.put(alice.address, [3; 32]).unwrap();

        assert_eq!(
            account_meta_storage.get(&alice.address).unwrap().unwrap(),
            vec![[1_u8; 32], [2_u8; 32], [3_u8; 32]]
        )
    }

    #[test]
    fn test_history() {
        let alice = create_account();
        let dir = TempDir::new("_test_merge_account_state").unwrap();
        let db = Arc::new(
            rocksdb::DB::open_cf_descriptors(&default_db_opts(), dir.path(), column_families())
                .unwrap(),
        );
        let history_storage = HistoryStorage::new(db);
        history_storage
            .append(
                [1; 32],
                MorphOperation::UpdateNonce {
                    account: alice.address,
                    nonce: 1,
                    tx_hash: [0; 32],
                },
            )
            .unwrap();
        history_storage
            .append(
                [2; 32],
                MorphOperation::UpdateNonce {
                    account: alice.address,
                    nonce: 2,
                    tx_hash: [0; 32],
                },
            )
            .unwrap();
        history_storage
            .append(
                [3; 32],
                MorphOperation::UpdateNonce {
                    account: alice.address,
                    nonce: 3,
                    tx_hash: [0; 32],
                },
            )
            .unwrap();
        println!("{:?}", history_storage.root_hash().unwrap())
    }

    #[test]
    fn test_multi_thread_history() {
        let alice = create_account();
        let bob = create_account();
        let dir = TempDir::new("_test_merge_account_state").unwrap();
        let db = Arc::new(
            rocksdb::DB::open_cf_descriptors(&default_db_opts(), dir.path(), column_families())
                .unwrap(),
        );
        let history_storage = Arc::new(HistoryStorage::new(db.clone()));
        let history_sequence = Arc::new(HistorySequenceStorage::new(db.clone()));
        let account_metadata = Arc::new(AccountMetadataStorage::new(db));

        for i in 0..=30 {
            if i % 2 == 0 {
                let his = history_storage
                    .append(
                        [i as u8; 32],
                        MorphOperation::UpdateNonce {
                            account: bob.address,
                            nonce: i + 1,
                            tx_hash: [i as u8; 32],
                        },
                    )
                    .unwrap();
                history_sequence.put(his.seq, his.root).unwrap();
                account_metadata.put(bob.address, his.root).unwrap();
            } else {
                let his = history_storage
                    .append(
                        [i as u8; 32],
                        MorphOperation::UpdateNonce {
                            account: alice.address,
                            nonce: i,
                            tx_hash: [i as u8; 32],
                        },
                    )
                    .unwrap();
                history_sequence.put(his.seq, his.root).unwrap();
                account_metadata.put(alice.address, his.root).unwrap();
            }
        }
        let mut history_sequence_iter = history_sequence.iter().unwrap();
        let iter = history_storage.root_history().unwrap();
        let mut history_storage_iter = iter.iter().map(|his| his.root);
        while let (Some((_, Ok(seq_his))), Some(his)) =
        (history_sequence_iter.next(), history_storage_iter.next())
        {
            assert_eq!(seq_his, his)
        }

        let alice_roots_c = account_metadata.get(&alice.address).unwrap().unwrap();
        let mut alice_roots_c_iter = alice_roots_c.iter();
        let alice_roots = history_storage.address_history(alice.address).unwrap();
        let mut alice_roots_iter = alice_roots.iter().map(|his| his.root);
        while let (Some(lhs), Some(rhs)) = (alice_roots_c_iter.next(), alice_roots_iter.next()) {
            assert_eq!(*lhs, rhs)
        }

        println!(
            "ALICE[{}] {:?}",
            alice.address,
            history_storage.address_history(alice.address)
        );
    }
}

use crate::codec::Codec;
use anyhow::Result;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::RwLock;

pub mod codec;
pub mod memstore;
pub mod error;
pub mod presistent_store;

pub trait KVEntry {
    type Key: Codec + Clone;
    type Value: Codec;
}

pub trait Storage<Entry>
where
    Entry: KVEntry,
{
    fn get(&self, key: &Entry::Key) -> Result<Option<Entry::Value>>;
    //fn mutate(&self, key : &Entry::Key, f : dyn Fn(&mut Entry::Value)) -> Result<Option<Entry::Value>>;
    fn put(&self, key: Entry::Key, value: Entry::Value) -> Result<()>;
    fn delete(&self, key: &Entry::Key) -> Result<()>;
    fn contains(&self, key: &Entry::Key) -> Result<bool>;
    fn iter(&self) -> Result<StorageIterator<Entry>>;
    //fn prefix<'a>(&self, key : &Entry::Key) -> StorageIterator<'a, Entry>;
}
pub type StorageIterator<'a, Entry: KVEntry> =
    Box<dyn 'a + Send + Iterator<Item = (Result<Entry::Key>, Result<Entry::Value>)>>;



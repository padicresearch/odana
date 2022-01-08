use codec::Codec;
use anyhow::Result;

pub trait KVEntry {
    type Key: Codec + Clone;
    type Value: Codec;
    fn column() -> &'static str;
}

pub trait KV<Entry>
    where
        Entry: KVEntry,
{
    fn get(&self, key: &Entry::Key) -> Result<Option<Entry::Value>>;
    fn put(&self, key: Entry::Key, value: Entry::Value) -> Result<()>;
    fn iter(&self) -> Result<StorageIterator<Entry>>;
}

pub type StorageIterator<'a, Entry: KVEntry> =
Box<dyn 'a + Send + Iterator<Item=(Result<Entry::Key>, Result<Entry::Value>)>>;



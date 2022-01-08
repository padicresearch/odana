use codec::{Codec, Encoder, Decoder};
use anyhow::Result;
use rocksdb::{DB, ColumnFamilyDescriptor};
use crate::error::MorphError;

pub trait Schema {
    type Key: Codec + Clone;
    type Value: Codec;
    fn column() -> &'static str;
    fn descriptor() -> ColumnFamilyDescriptor;
}

pub trait KV<Entry>
    where
        Entry: Schema,
{
    fn get(&self, key: &Entry::Key) -> Result<Option<Entry::Value>>;
    fn put(&self, key: Entry::Key, value: Entry::Value) -> Result<()>;
    fn merge(&self, key: Entry::Key, value: Entry::Value) -> Result<()>;
    fn contains(&self, key: &Entry::Key) -> Result<bool>;
    fn iter(&self) -> Result<SchemaIterator<Entry>>;
}

pub type SchemaIterator<'a, Entry: Schema> =
Box<dyn 'a + Send + Iterator<Item=(Result<Entry::Key>, Result<Entry::Value>)>>;


pub fn default_write_opts() -> rocksdb::WriteOptions {
    let mut opts = rocksdb::WriteOptions::default();
    opts.set_sync(true);
    opts
}

pub fn default_read_opts() -> rocksdb::ReadOptions {
    let mut opts = rocksdb::ReadOptions::default();
    opts
}


impl<S: Schema> KV<S> for DB {
    fn get(&self, key: &S::Key) -> Result<Option<S::Value>> {
        let cf = self.cf_handle(S::column()).ok_or(MorphError::ColumnFamilyMissing(S::column()))?;
        let key = key.encode()?;
        let value = self.get_cf(cf, key)?;
        match value {
            None => {
                Ok(None)
            }
            Some(value) => {
                Ok(Some(S::Value::decode(&value)?))
            }
        }
    }

    fn put(&self, key: S::Key, value: S::Value) -> Result<()> {
        let cf = self.cf_handle(S::column()).ok_or(MorphError::ColumnFamilyMissing(S::column()))?;
        let key = key.encode()?;
        let value = value.encode()?;
        self.put_cf_opt(cf, key, value, &default_write_opts()).map_err(|e| e.into())
    }

    fn merge(&self, key: S::Key, value: S::Value) -> Result<()> {
        let cf = self.cf_handle(S::column()).ok_or(MorphError::ColumnFamilyMissing(S::column()))?;
        let key = key.encode()?;
        let value = value.encode()?;
        self.merge_cf_opt(cf, key, value, &default_write_opts()).map_err(|e| e.into())
    }

    fn contains(&self, key: &S::Key) -> Result<bool> {
        let cf = self.cf_handle(S::column()).ok_or(MorphError::ColumnFamilyMissing(S::column()))?;
        let key = key.encode()?;
        let val = self.get_pinned_cf(cf, key)?;
        Ok(val.is_some())
    }

    fn iter(&self) -> Result<SchemaIterator<S>> {
        let cf = self.cf_handle(S::column()).ok_or(MorphError::ColumnFamilyMissing(S::column()))?;
        let iter = self.iterator_cf(cf, rocksdb::IteratorMode::Start);
        Ok(Box::new(iter.map(|(k, v)| {
            (S::Key::decode(&k), S::Value::decode(&v))
        })))
    }
}
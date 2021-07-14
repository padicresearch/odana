#[cfg(test)]
mod tests;

mod word;
mod simple;
mod compact;

pub use crate::word::*;
pub use crate::simple::*;

pub trait Trie<C, K, V> where V: Sized + Clone, K: Word<C>, C: Symbol {
    fn insert(&mut self, key: K, value: V);
    fn get(&self, key: K) -> Option<V>;
    fn prefix(&self, key: K) -> Option<Vec<(K, V)>>;
    fn remove(&mut self, key: K);
}

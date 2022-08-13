pub use crate::simple::*;
pub use crate::word::*;

#[cfg(test)]
mod tests;

mod compact;
mod simple;
mod word;

pub trait Trie<C, K, V>
where
    V: Sized + Clone,
    K: Word<C>,
    C: Symbol,
{
    fn insert(&mut self, key: K, value: V);
    fn get(&self, key: K) -> Option<V>;
    fn prefix(&self, key: K) -> Option<Vec<(K, V)>>;
    fn remove(&mut self, key: K);
}

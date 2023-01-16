use crate::tree::TrieDB;
use smt::SparseMerkleTree;
use std::path::PathBuf;
use std::sync::Arc;

pub struct ContextState {
    trie: Arc<TrieDB<u32, SparseMerkleTree>>,
    path: PathBuf,
    read_only: bool,
}

use std::path::PathBuf;
use std::sync::Arc;
use smt::SparseMerkleTree;
use crate::tree::Tree;

pub struct ContextState {
    trie: Arc<Tree<u32, SparseMerkleTree>>,
    path: PathBuf,
    read_only: bool,
}

use crate::tree::Tree;
use smt::SparseMerkleTree;
use std::path::PathBuf;
use std::sync::Arc;

pub struct ContextState {
    trie: Arc<Tree<u32, SparseMerkleTree>>,
    path: PathBuf,
    read_only: bool,
}

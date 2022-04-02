use std::collections::HashMap;
use std::sync::RwLock;
use dashmap::{DashMap, DashSet};
use primitive_types::H256;

pub trait DBTx {
    fn set(&self, key: Vec<u8>, value: Vec<u8>);
    fn delete(&self, key: &Vec<u8>);
}

type NodeList = Vec<Node>;
type Node = Vec<u8>;

pub struct CacheDB {
    live_cache: DashMap<H256, NodeList>,
    updated_nodes: DashMap<H256, NodeList>,
    nodes_to_revert: RwLock<Vec<Node>>,
    store: DashMap<Vec<u8>, Vec<u8>>,
}
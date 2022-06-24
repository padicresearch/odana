use std::cmp::Ordering;
use std::collections::{BTreeSet, HashSet};
use std::sync::{Arc, LockResult, RwLock};
use types::block::{BlockHeader, HeightSortedBlockHeader};
use types::Hash;

pub struct Downloader {
    queue: Arc<RwLock<BTreeSet<HeightSortedBlockHeader>>>,
    requested: Arc<RwLock<HashSet<Hash>>>,
}

impl Downloader {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Default::default()),
            requested: Arc::new(Default::default()),
        }
    }
    pub fn enqueue(&self, headers: Vec<BlockHeader>) {
        let mut queue = self.queue.clone();
        let mut queue = queue.write();
        match queue {
            Ok(mut queue) => {
                queue.extend(
                    headers
                        .into_iter()
                        .map(|header| HeightSortedBlockHeader(header)));
            }
            Err(_) => {}
        }
    }
    pub fn finish_download(&self, block_hash: &Hash) {
        let requested = self.requested.write();
        match requested {
            Ok(mut requested) => {
                requested.remove(block_hash);
            }
            _ => {}
        }
    }

    pub fn next_blocks_to_download(&self) -> Vec<Hash> {
        let mut next = Vec::with_capacity(16);


        let queue = self.queue.clone();
        let queue = queue.write();
        let requested = self.requested.write();
        match (queue, requested) {
            (Ok(mut queue), Ok(mut requested)) => {
                while let Some(header) = queue.pop_first() {
                    if next.len() >= 20 {
                        break;
                    }
                    next.push(header.hash());
                    requested.insert(header.hash());
                }
            }
            _ => {}
        }
        next
    }

    pub fn last_header_in_queue(&self) -> Option<Hash> {
        let queue = self.queue.clone();
        let queue = queue.read();
        match queue {
            Ok(queue) => {
                queue.last().map(|header| header.hash())
            }
            Err(_) => {
                None
            }
        }
    }

    pub fn is_downloading(&self) -> bool {
        let queue = self.queue.clone();
        let queue = queue.read().unwrap();
        let requested = self.requested.clone();
        let requested = requested.read().unwrap();
        return !queue.is_empty() || !requested.is_empty();
    }
}
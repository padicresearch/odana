use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::{Arc, LockResult, RwLock};
use types::block::{BlockHeader, HeightSortedBlockHeader};
use types::Hash;

pub struct Downloader {
    queue: Arc<RwLock<BTreeMap<i32, BlockHeader>>>,
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
                        .map(|header| (header.level, header)));
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
                while let Some((_, header)) = queue.pop_first() {
                    next.push(header);
                    requested.insert(header.hash());

                    if next.len() >= 20 {
                        break;
                    }
                }
            }
            _ => {}
        }

        //println!("Downloading next block {:?} - {:?}", next.first().map(|header| header.level), next.last().map(|header| header.level));
        next.into_iter().map(|header| header.hash()).collect()
    }

    pub fn last_header_in_queue(&self) -> Option<Hash> {
        let queue = self.queue.clone();
        let queue = queue.read();
        match queue {
            Ok(queue) => {
                queue.last_key_value().map(|(_, header)| header.hash())
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
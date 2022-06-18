use std::collections::{HashMap, HashSet, VecDeque};
use indexmap::IndexMap;
use types::block::BlockHeader;
use types::Hash;

pub struct DownloadManager {
    queue: Vec<Hash>,
    requested: HashSet<Hash>,
}


impl DownloadManager {
    pub fn enqueue(&mut self, queue: Vec<BlockHeader>) {
        self.queue.extend(queue.into_iter().map(|header| header.hash()))
    }
    pub fn finish_download(&mut self, block_hash: &Hash) {
        self.requested.remove(block_hash);
    }

    pub fn next_request(&mut self) -> Vec<Hash> {
        let mut next = Vec::with_capacity(16);
        while let Some(hash) = self.queue.pop() {
            if next.len() >= 16 {
                break;
            }
            next.push(hash);
            self.requested.insert(hash);
        }
        next
    }

    pub fn next_headers_to_download(&self) -> Option<Hash> {
        self.queue.last().map(|hash| *hash)
    }

    pub fn is_downloading(&self) -> bool {
        return !self.queue.is_empty() || !self.requested.is_empty();
    }
}
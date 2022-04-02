use std::sync::{Arc, RwLock};
use crate::trie_cache::CacheDB;

struct Trie {
    db: Arc<CacheDB>,
    root: Vec<u8>,
    prev_root: Vec<u8>,
    lock: RwLock<()>,
    height: i32,
    load_db_counter: i32,
    // Must be atomic
    load_cache_counter: i32,
    // Must be atomic
    counter_on: bool,
    cache_height_limit: i32,
    past_tries: Vec<Vec<u8>>,
    atomic_update: bool,
}
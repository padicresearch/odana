mod word;

use std::collections::HashMap;
use std::marker::PhantomData;
use std::borrow::BorrowMut;
use bloomfilter::Bloom;
use crate::word::{Word, Character};

#[derive(Debug, Clone)]
pub struct Trie<C, K, V> where V: Sized + Clone, K: Word<C>, C: Character {
    key: Option<K>,
    value: Option<V>,
    root: bool,
    is_word: bool,
    children: HashMap<C, Trie<C, K, V>>,
}

impl<C, K, V> Trie<C, K, V> where V: Sized + Clone, K: Word<C>, C: Character {
    pub fn new() -> Self {
        Self {
            key: None,
            value: None,
            root: true,
            is_word: false,
            children: Default::default(),
        }
    }


    pub fn insert(&mut self, key: K, value: V) {
        let mut current_tree = self;
        for (index, char) in key.chars().iter().enumerate() {
            if index == key.len() - 1 {
                let mut tree = current_tree.children.entry(char.clone()).or_insert(Trie {
                    key: None,
                    value: None,
                    root: false,
                    is_word: true,
                    children: Default::default(),
                });
                tree.is_word = true;
                tree.key = Some(key.clone());
                tree.value = Some(value.clone());
            } else {
                current_tree = current_tree.children.entry(char.clone()).or_insert(Trie {
                    key: None,
                    value: None,
                    root: false,
                    is_word: false,
                    children: Default::default(),
                });
            }
        }
    }

    pub fn get(&mut self, key: K) -> Option<V> {
        let mut current_tree = self;
        for (index, char) in key.chars().iter().enumerate() {
            current_tree = match current_tree.children.get_mut(char) {
                None => {
                    return None;
                }
                Some(item) => {
                    item
                }
            };
            if index == key.len() - 1 && current_tree.is_word {
                return current_tree.value.clone();
            } else if index == key.len() - 1 && !current_tree.is_word {
                return None;
            }
        };
        None
    }

    pub fn remove(&mut self, key: K) {
        let mut tries = vec![];
        let mut current_tree = self;
        tries.push(current_tree as *mut Trie<C, K, V>);
        for char in key.chars() {
            current_tree = match current_tree.children.get_mut(char) {
                None => {
                    return;
                }
                Some(child_tree) => {
                    child_tree
                }
            };
            tries.push(current_tree as *mut Trie<C, K, V>)
        }
        for (i, c) in key.chars().iter().rev().enumerate() {
            let index = (key.len() - 1) - i;
            let mut parent_tree = match tries.get(index) {
                None => {
                    return;
                }
                Some(tree) => {
                    unsafe {
                        if let Some(tree) = tree.as_mut() {
                            tree
                        } else {
                            return;
                        }
                    }
                }
            };
            let child_tree = parent_tree.children.get_mut(c).unwrap();
            if child_tree.is_word && i == 0 && child_tree.children.is_empty() {
                parent_tree.children.remove(c);
            } else if child_tree.is_word && i == 0 && !child_tree.children.is_empty() {
                child_tree.is_word = false;
                child_tree.key = None;
                child_tree.value = None
            } else if child_tree.is_word && i > 0 && child_tree.children.is_empty() {
                parent_tree.children.remove(c);
            }
        }
    }
}


#[cfg(test)]
mod tests;
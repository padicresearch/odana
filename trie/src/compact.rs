use crate::{Word, Symbol, Trie};
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct CompactTrie<C, K, V> where V: Sized + Clone, K: Word<C>, C: Symbol {
    key: Option<K>,
    value: Option<V>,
    root: bool,
    is_word: bool,
    children: Vec<Option<CompactTrie<C, K, V>>>,
    data: PhantomData<C>,
}

impl<C, K, V> CompactTrie<C, K, V> where V: Sized + Clone, K: Word<C>, C: Symbol {
    pub fn new() -> Self {
        Self {
            key: None,
            value: None,
            root: true,
            is_word: false,
            children: vec![None; C::max_index()],
            data: Default::default(),
        }
    }

    fn _search(&self, root: &CompactTrie<C, K, V>, values: &mut Vec<(K, V)>) {
        if root.is_word {
            values.push((root.key.clone().unwrap(), root.value.clone().unwrap()))
        }
        for child in root.children.iter() {
            match child {
                None => {}
                Some(child) => {
                    self._search(child, values)
                }
            }
        }
    }
}


impl<C, K, V> Trie<C, K, V> for CompactTrie<C, K, V> where V: Sized + Clone, K: Word<C>, C: Symbol {
    fn insert(&mut self, key: K, value: V) {
        let mut current_tree = self;

        for char in key.chars().iter() {
            if current_tree.children[char.index()].is_none() {
                current_tree.children[char.index()] = Some(CompactTrie {
                    key: None,
                    value: None,
                    root: false,
                    is_word: false,
                    children: vec![None; C::max_index()],
                    data: Default::default(),
                });
            }
            current_tree = match &mut current_tree.children[char.index()] {
                None => {
                    return;
                }
                Some(child_tree) => {
                    child_tree
                }
            };
        }
        current_tree.key = Some(key.clone());
        current_tree.value = Some(value.clone());
        current_tree.is_word = true
    }

    fn get(&self, key: K) -> Option<V> {
        let mut current_tree = self;
        for char in key.chars().iter() {
            current_tree = match &current_tree.children[char.index()] {
                None => {
                    return None;
                }
                Some(item) => {
                    item
                }
            };
        };
        if current_tree.is_word {
             current_tree.value.clone()
        } else {
             None
        }
    }

    fn prefix(&self, key: K) -> Option<Vec<(K, V)>> {
        let mut current_tree = self;
        let mut found = vec![];
        for char in key.chars().iter() {
            current_tree = match &current_tree.children[char.index()] {
                None => {
                    return None;
                }
                Some(item) => {
                    item
                }
            };
        };
        self._search(current_tree, &mut found);
        Some(found)
    }


    fn remove(&mut self, key: K) {
        let mut tries = vec![];
        let mut current_tree = self;
        tries.push(current_tree as *mut CompactTrie<C, K, V>);
        for char in key.chars() {
            current_tree = match &mut current_tree.children[char.index()] {
                None => {
                    return;
                }
                Some(child_tree) => {
                    child_tree
                }
            };
            tries.push(current_tree as *mut CompactTrie<C, K, V>)
        }
        for (i, c) in key.chars().iter().rev().enumerate() {
            let index = (key.len() - 1) - i;
            let parent_tree = unsafe {
                &mut *tries[index]
            };
            let child_tree = match &mut parent_tree.children[(c.index())] {
                None => {
                    return;
                }
                Some(tree) => {
                    tree
                }
            };
            if child_tree.is_word && i == 0 && child_tree.children.is_empty() {
                parent_tree.children.remove(c.index());
            } else if child_tree.is_word && i == 0 && !child_tree.children.is_empty() {
                child_tree.is_word = false;
                child_tree.key = None;
                child_tree.value = None
            } else if child_tree.is_word && i > 0 && child_tree.children.is_empty() {
                parent_tree.children.remove(c.index());
            }
        }
    }
}

use std::collections::BTreeMap;

use crate::{Symbol, Trie, Word};

#[derive(Debug, Clone)]
pub struct SimpleTrie<C, K, V>
where
    V: Sized + Clone,
    K: Word<C>,
    C: Symbol,
{
    key: Option<K>,
    value: Option<V>,
    root: bool,
    is_word: bool,
    children: BTreeMap<C, SimpleTrie<C, K, V>>,
}

impl<C, K, V> Default for SimpleTrie<C, K, V>
where
    V: Sized + Clone,
    K: Word<C>,
    C: Symbol,
{
    fn default() -> Self {
        Self {
            key: None,
            value: None,
            root: true,
            is_word: false,
            children: Default::default(),
        }
    }
}

impl<C, K, V> SimpleTrie<C, K, V>
where
    V: Sized + Clone,
    K: Word<C>,
    C: Symbol,
{
    pub fn new() -> Self {
        Self::default()
    }

    fn _search(&self, root: &SimpleTrie<C, K, V>, values: &mut Vec<(K, V)>) {
        if root.is_word {
            values.push((root.key.clone().unwrap(), root.value.clone().unwrap()))
        }
        for (_, child) in root.children.iter() {
            self._search(child, values)
        }
    }
}

impl<C, K, V> Trie<C, K, V> for SimpleTrie<C, K, V>
where
    V: Sized + Clone,
    K: Word<C>,
    C: Symbol,
{
    fn insert(&mut self, key: K, value: V) {
        let mut current_tree = self;
        for char in key.chars().iter() {
            current_tree = current_tree
                .children
                .entry(char.clone())
                .or_insert(SimpleTrie {
                    key: None,
                    value: None,
                    root: false,
                    is_word: false,
                    children: Default::default(),
                });
        }
        current_tree.is_word = true;
        current_tree.key = Some(key.clone());
        current_tree.value = Some(value.clone());
    }

    fn get(&self, key: K) -> Option<V> {
        let mut current_tree = self;
        for char in key.chars().iter() {
            current_tree = match current_tree.children.get(char) {
                None => {
                    return None;
                }
                Some(item) => item,
            };
        }
        if current_tree.is_word {
            current_tree.value.clone()
        } else {
            None
        }
    }

    fn prefix(&self, key: K) -> Option<Vec<(K, V)>> {
        let mut current_tree = self;
        let mut found = vec![];
        for (index, char) in key.chars().iter().enumerate() {
            current_tree = match current_tree.children.get(char) {
                None => {
                    return None;
                }
                Some(item) => item,
            };
            if index == key.len() - 1 {
                self._search(current_tree, &mut found)
            }
        }
        Some(found)
    }

    fn remove(&mut self, key: K) {
        let mut tries = vec![];
        let mut current_tree = self;
        tries.push(current_tree as *mut SimpleTrie<C, K, V>);
        for char in key.chars() {
            current_tree = match current_tree.children.get_mut(char) {
                None => {
                    return;
                }
                Some(child_tree) => child_tree,
            };
            tries.push(current_tree as *mut SimpleTrie<C, K, V>)
        }
        for (i, c) in key.chars().iter().rev().enumerate() {
            let index = (key.len() - 1) - i;
            let parent_tree = unsafe { &mut *tries[index] };
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

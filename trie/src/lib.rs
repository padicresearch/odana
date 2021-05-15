#![feature(associated_type_defaults)]
mod word;

use std::collections::HashMap;
use std::marker::PhantomData;
use std::borrow::BorrowMut;
use bloomfilter::Bloom;

#[derive(Debug, Clone)]
pub struct Trie<V> where V: Sized + Clone {
    key: Option<String>,
    value: Option<V>,
    root : bool,
    is_word: bool,
    children: HashMap<char, Trie<V>>,
}

impl<V> Trie<V> where V: Sized + Clone {
    pub fn new() -> Self {
        Self {
            key: None,
            value: None,
            root: true,
            is_word: false,
            children: Default::default(),
        }
    }


    pub fn insert<K>(&mut self, key: K, value: V) where K: AsRef<str> {
        let mut root = self;
        for (index, char) in key.as_ref().chars().enumerate() {
            if index == key.as_ref().len() - 1 {
                let mut item = root.children.entry(char).or_insert(Trie {
                    key: None,
                    value: None,
                    root: false,
                    is_word: true,
                    children: Default::default(),
                });
                item.is_word = true;
                item.key = Some(key.as_ref().into());
                item.value = Some(value.clone());
            } else {
                root = root.children.entry(char).or_insert(Trie {
                    key: None,
                    value: None,
                    root: false,
                    is_word: false,
                    children: Default::default(),
                });
            }
        }
    }

    pub fn get<K>(&mut self, key: K) -> Option<V> where K: AsRef<str> {
        let mut root = self;
        for (index,char) in key.as_ref().chars().enumerate(){
            root = match root.children.get_mut(&char) {
                None => {
                    return None;
                }
                Some(item) => {

                    item
                }
            };
            if index == key.as_ref().len() - 1 && root.is_word {
               return root.value.clone()
            }else if index == key.as_ref().len() - 1 && !root.is_word {
                return None
            }
        };
        None
    }

    pub fn remove<K>(&mut self, key: K) where K: AsRef<str> {
        let mut tries = vec![];
        let mut root = self;
        tries.push(root as *mut Trie<V>);
        for char in key.as_ref().chars(){
            root = match root.children.get_mut(&char) {
                None => {
                    return;
                }
                Some(item) => {
                    item
                }
            };
            tries.push(root as *mut Trie<V>)
        }
        for (i,c) in key.as_ref().chars().rev().enumerate(){
            let index = (key.as_ref().len() - 1) - i;
            let mut parent = match tries.get(index) {
                None => {
                    return;
                }
                Some(tree) => {
                    unsafe  {
                        if let Some(tree) = tree.as_mut() {
                            tree
                        }else {
                            return;
                        }
                    }
                }
            };
            let current = parent.children.get_mut(&c).unwrap();
            if current.is_word && i == 0 && current.children.is_empty(){
                parent.children.remove(&c);

            }else if current.is_word && i == 0 && !current.children.is_empty() {
                current.is_word = false;
                current.key = None;
                current.value = None
            }else if current.is_word && i > 0 && current.children.is_empty() {
                parent.children.remove(&c);
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::Trie;

    #[test]
    fn it_works() {
        let mut trees = Trie::new();
        trees.insert("Bob", "Hello".to_string());
        trees.insert("Boi", "World".to_string());
        trees.insert("Bo", "Rush".to_string());
        trees.insert("Bolla", "Kunta".to_string());
        trees.remove("Bo");
        println!("get {:?}", trees.get("Bo"));
    }
}

mod word;

use std::collections::HashMap;
use std::marker::PhantomData;
use std::borrow::BorrowMut;
use bloomfilter::Bloom;
use crate::word::{Word, Character};

#[derive(Debug, Clone)]
pub struct Trie<C, K, V> where V: Sized + Clone, K : Word<C>, C : Character {
    key: Option<K>,
    value: Option<V>,
    root : bool,
    is_word: bool,
    children: HashMap<C, Trie<C,K,V>>,
}

impl<C, K, V> Trie<C, K, V> where V: Sized + Clone, K : Word<C>,  C : Character{
    pub fn new() -> Self {
        Self {
            key: None,
            value: None,
            root: true,
            is_word: false,
            children: Default::default(),
        }
    }


    pub fn insert(&mut self, key: K, value: V){
        let mut root = self;
        for (index, char) in key.chars().iter().enumerate() {
            if index == key.len() - 1 {
                let mut item = root.children.entry(char.clone()).or_insert(Trie {
                    key: None,
                    value: None,
                    root: false,
                    is_word: true,
                    children: Default::default(),
                });
                item.is_word = true;
                item.key = Some(key.clone());
                item.value = Some(value.clone());
            } else {
                root = root.children.entry(char.clone()).or_insert(Trie {
                    key: None,
                    value: None,
                    root: false,
                    is_word: false,
                    children: Default::default(),
                });
            }
        }
    }

    pub fn get(&mut self, key: K) -> Option<V>{
        let mut root = self;
        for (index,char) in key.chars().iter().enumerate(){
            root = match root.children.get_mut(char) {
                None => {
                    return None;
                }
                Some(item) => {

                    item
                }
            };
            if index == key.len() - 1 && root.is_word {
               return root.value.clone()
            }else if index == key.len() - 1 && !root.is_word {
                return None
            }
        };
        None
    }

    pub fn remove(&mut self, key: K){
        let mut tries = vec![];
        let mut root = self;
        tries.push(root as *mut Trie<C,K,V>);
        for char in key.chars(){
            root = match root.children.get_mut(char) {
                None => {
                    return;
                }
                Some(item) => {
                    item
                }
            };
            tries.push(root as *mut Trie<C,K,V>)
        }
        for (i,c) in key.chars().iter().rev().enumerate(){
            let index = (key.len() - 1) - i;
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
            let current = parent.children.get_mut(c).unwrap();
            if current.is_word && i == 0 && current.children.is_empty(){
                parent.children.remove(c);

            }else if current.is_word && i == 0 && !current.children.is_empty() {
                current.is_word = false;
                current.key = None;
                current.value = None
            }else if current.is_word && i > 0 && current.children.is_empty() {
                parent.children.remove(c);
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::Trie;
    use crate::word::Alphabet;

    #[test]
    fn it_works() {
        let mut trees = Trie::new();
        trees.insert(Alphabet::from("hello".to_string()), "Hello".to_string());
        println!("get {:#?}", trees);
    }
}

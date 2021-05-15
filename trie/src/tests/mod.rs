use crate::Trie;
use crate::word::{ Word, Index, Character};
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug)]
pub struct Char {
    c: char
}

impl Char {
    pub fn new(c : char) -> Self {
        Char {
            c
        }
    }
}

impl Character for Char {}

impl PartialEq for Char {
    fn eq(&self, other: &Self) -> bool {
        self.c.eq_ignore_ascii_case(&other.c)
    }
}

impl Eq for Char {}

impl Index for Char {
    fn index(&self) -> usize {
        self.c.to_digit(10).unwrap() as usize
    }
}

impl Hash for Char {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u8(self.c as u8)
    }
}

#[derive(Clone, Debug)]
pub struct Alphabet {
    chars: Vec<Char>
}

impl From<String> for Alphabet {
    fn from(word: String) -> Self {
        let mut chars = vec![];
        for c in word.chars() {
            chars.push(Char::new(c))
        }
        Alphabet {
            chars
        }
    }
}

impl From<&str> for Alphabet {
    fn from(word: &str) -> Self {
        let mut chars = vec![];
        for c in word.chars() {
            chars.push(Char::new(c))
        }
        Alphabet {
            chars
        }
    }
}

impl Word<Char> for Alphabet {
    fn chars(&self) -> &Vec<Char> {
        &self.chars
    }

    fn len(&self) -> usize {
        self.chars.len()
    }
}

#[test]
fn it_works() {
    let mut trees = Trie::new();
    trees.insert(Alphabet::from("hello"), "Hello".to_string());
    println!("get {:#?}", trees.get("hello".into()));
}
use std::hash::{Hash, Hasher};
use std::cmp::Ordering;

pub trait Index {
    fn index(&self) -> usize;
}

pub trait Character: PartialOrd + Ord + Index + Sized + Hash + Clone {}

pub trait Word<T>: Sized + Clone where T: Character {
    fn chars(&self) -> &Vec<T>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
}

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
        self.c as usize
    }
}

impl Hash for Char {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u8(self.c as u8)
    }
}

impl PartialOrd for Char {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.c.cmp(&other.c))
    }
}

impl Ord for Char {
    fn cmp(&self, other: &Self) -> Ordering {
        self.c.cmp(&other.c)
    }
}


#[derive(Clone, Debug)]
pub struct StringWord {
    chars: Vec<Char>
}

impl From<String> for StringWord {
    fn from(word: String) -> Self {
        let mut chars = vec![];
        for c in word.chars() {
            chars.push(Char::new(c))
        }
        StringWord {
            chars
        }
    }
}

impl From<&str> for StringWord {
    fn from(word: &str) -> Self {
        let mut chars = vec![];
        for c in word.chars() {
            chars.push(Char::new(c))
        }
        StringWord {
            chars
        }
    }
}

impl Word<Char> for StringWord {
    fn chars(&self) -> &Vec<Char> {
        &self.chars
    }

    fn len(&self) -> usize {
        self.chars.len()
    }

    fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }
}

impl PartialEq for StringWord {
    fn eq(&self, other: &Self) -> bool {
        self.chars.eq(other.chars())
    }
}

impl Eq for StringWord {}
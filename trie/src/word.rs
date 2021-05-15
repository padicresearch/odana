use std::hash::{Hash, Hasher};

pub trait Index {
    fn index(&self) -> usize;
}

pub trait Character: PartialEq + Eq + Index + Sized + Hash + Clone {}

pub trait Word<T>: Sized + Clone where T: Character {
    fn chars(&self) -> &Vec<T>;
    fn len(&self) -> usize;
}
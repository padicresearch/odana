trait Index {
    fn index(&self) -> usize;
}

trait Character: PartialEq + Eq + Index + Sized {}

trait Word<T> : Sized where T: Character {
    type Characters = Vec<T>;
    fn chars(&self) -> Self::Characters;
}

struct Char {
    c: char
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
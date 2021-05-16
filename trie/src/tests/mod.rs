use crate::compact::CompactTrie;
use crate::{Trie, SimpleTrie};
use crate::word::StringWord;

#[test]
fn it_works() {
    let mut compact = CompactTrie::new(300);
    compact.insert(StringWord::from("hello"), "Hello".to_string());
    compact.insert(StringWord::from("helius"), "helius".to_string());
    compact.insert(StringWord::from("heell"), "heell".to_string());
    compact.insert(StringWord::from("home"), "home".to_string());
    compact.insert(StringWord::from("mello"), "mello".to_string());
    compact.insert(StringWord::from("maze"), "maze".to_string());
    compact.insert(StringWord::from("home"), "home".to_string());
    compact.insert(StringWord::from("have"), "home".to_string());


    let mut simple = SimpleTrie::new();
    simple.insert(StringWord::from("hello"), "Hello".to_string());
    simple.insert(StringWord::from("helius"), "helius".to_string());
    simple.insert(StringWord::from("heell"), "heell".to_string());
    simple.insert(StringWord::from("home"), "home".to_string());
    simple.insert(StringWord::from("mello"), "mello".to_string());
    simple.insert(StringWord::from("maze"), "maze".to_string());
    simple.insert(StringWord::from("home"), "home".to_string());
    simple.insert(StringWord::from("have"), "home".to_string());
    println!("Simple Prefix {:#?}", simple.prefix("h".into()));
    println!("Compact Prefix {:#?}", compact.prefix("h".into()));
    assert_eq!(simple.prefix("h".into()),compact.prefix("h".into()))

}
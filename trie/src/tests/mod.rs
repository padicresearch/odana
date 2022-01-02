use crate::compact::CompactTrie;
use crate::word::TString;
use crate::{SimpleTrie, Trie};

#[test]
fn it_works() {
    let mut compact = CompactTrie::new();
    compact.insert(TString::from("hello"), "Hello".to_string());
    compact.insert(TString::from("helius"), "helius".to_string());
    compact.insert(TString::from("heell"), "heell".to_string());
    compact.insert(TString::from("home"), "home".to_string());
    compact.insert(TString::from("mello"), "mello".to_string());
    compact.insert(TString::from("maze"), "maze".to_string());
    compact.insert(TString::from("home"), "home".to_string());
    compact.insert(TString::from("have"), "home".to_string());

    let mut simple = SimpleTrie::new();
    simple.insert(TString::from("hello"), "Hello".to_string());
    simple.insert(TString::from("helius"), "helius".to_string());
    simple.insert(TString::from("heell"), "heell".to_string());
    simple.insert(TString::from("home"), "home".to_string());
    simple.insert(TString::from("mello"), "mello".to_string());
    simple.insert(TString::from("maze"), "maze".to_string());
    simple.insert(TString::from("home"), "home".to_string());
    simple.insert(TString::from("have"), "home".to_string());
    println!("Simple Prefix {:#?}", simple.prefix("h".into()));
    println!("Compact Prefix {:#?}", compact.prefix("h".into()));
    println!("Get {:#?}", simple.get("have".into()));
    assert_eq!(simple.prefix("h".into()), compact.prefix("h".into()))
}

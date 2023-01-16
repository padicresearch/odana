#![cfg_attr(feature = "std", no_std)]

pub struct FrameworkId(&'static str);

pub trait Framework {
    fn id() -> FrameworkId();
}
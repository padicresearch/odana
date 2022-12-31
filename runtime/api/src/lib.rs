#![no_std]
use bincode::{Decode, Encode};
use core::fmt::{Debug, Formatter};
use odana_std::prelude::*;
use serde::{Deserialize, Serialize};

impl Debug for Executable {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        self.metadata.fmt(f)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Metadata {
    pub activation_height: u64,
    pub publisher: Vec<u8>,
    pub docs: Vec<u8>,
    pub genesis: Vec<u8>,
}

#[derive(Clone,Serialize, Deserialize, Encode, Decode)]
pub struct Executable {
    pub binary: Vec<u8>,
    pub metadata: Metadata,
}


/*
 * Copyright (c) 2023 Padic Research.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *    http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use anyhow::anyhow;
use bincode::{Decode, Encode};
use codec::{Decodable, Encodable};
use primitive_types::address::Address;
use primitive_types::H256;

#[derive(Encode, Decode, Clone, Debug)]
pub struct AppMetadata {
    pub binary: Vec<u8>,
    pub descriptor: Vec<u8>,
}

#[derive(Encode, Decode, Clone, Debug)]
pub struct AppStateKey(pub Address, pub H256);

impl Encodable for AppStateKey {
    fn encode(&self) -> anyhow::Result<Vec<u8>> {
        bincode::encode_to_vec(self, codec::config()).map_err(|e| anyhow!(e))
    }
}

impl Decodable for AppStateKey {
    fn decode(buf: &[u8]) -> anyhow::Result<Self> {
        bincode::decode_from_slice(buf, codec::config())
            .map(|(out, _)| out)
            .map_err(|e| anyhow!(e))
    }
}

impl Encodable for AppMetadata {
    fn encode(&self) -> anyhow::Result<Vec<u8>> {
        bincode::encode_to_vec(self, codec::config()).map_err(|e| anyhow!(e))
    }
}

impl Decodable for AppMetadata {
    fn decode(buf: &[u8]) -> anyhow::Result<Self> {
        bincode::decode_from_slice(buf, codec::config())
            .map(|(out, _)| out)
            .map_err(|e| anyhow!(e))
    }
}

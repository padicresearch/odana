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

use alloc::vec::Vec;
use primitive_types::H256;
use rune_framework::io::Hashing;

pub(crate) fn restricted_namespaces() -> Vec<H256> {
    let known = [
        "network.odax",
        "io.padic",
        "com.padicresearch",
        "com.odax",
        "io.odax",
        "com.odana",
        "io.odana",
        "org.odana",
        "uk.odana",
    ];
    known
        .into_iter()
        .map(|ns| {
            ns.split('.')
                .map(|part| Hashing::keccak256(part.as_bytes()))
                .reduce(|acc, part| Hashing::keccak256(&[acc.as_bytes(), part.as_bytes()].concat()))
                .unwrap_or_default()
        })
        .collect()
}

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

use anyhow::bail;
use primitive_types::H256;
use rune_framework::io::Hashing;
use rune_std::vec::Vec;

const MAX_PACKAGE_NAME_LEN: usize = 128;

pub(crate) fn decode_namespace(package_name: &str) -> anyhow::Result<H256> {
    let parts = parts(package_name)?;
    let organisation_id = parts
        .iter()
        .take(2)
        .map(|t| Hashing::keccak256(t.as_bytes()))
        .reduce(|acc, t| {
            Hashing::keccak256(
                [acc.to_fixed_bytes(), t.to_fixed_bytes()]
                    .concat()
                    .as_slice(),
            )
        })
        .unwrap_or_default();
    Ok(organisation_id)
}
fn parts(s: &str) -> anyhow::Result<Vec<&str>> {
    if s.len() > MAX_PACKAGE_NAME_LEN {
        bail!("max package name length excceed{MAX_PACKAGE_NAME_LEN} ")
    }

    let parts = s.split('.').collect::<Vec<_>>();

    if parts.len() < 2 || parts.len() > 8 {
        bail!("Package name must have at least 3 parts and at most 8 parts")
    }

    if !parts[0].chars().all(|t| t.is_ascii_lowercase()) {
        bail!("First part of the package name must contain only lowercase letters")
    }

    let mut is_valid = true;

    for part in parts.iter() {
        if !part
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            is_valid = false;
            break;
        }
    }

    if !is_valid {
        bail!("All parts of the package name must contain only lowercase letters, digits, or underscores")
    }
    Ok(parts)
}

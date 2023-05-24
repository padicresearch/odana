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
use crypto::keccak256;
use primitive_types::H256;

const MAX_PACKAGE_NAME_LEN: usize = 128;

#[derive(Debug, PartialEq)]
pub struct PackageName {
    pub organisation_id: H256,
    pub package_id: H256,
}

impl PackageName {
    pub fn parse(s: &str) -> anyhow::Result<PackageName> {
        let parts = Self::parts(s)?;
        let organisation_id = parts
            .iter()
            .take(2)
            .map(keccak256)
            .reduce(|acc, t| {
                keccak256(
                    [acc.to_fixed_bytes(), t.to_fixed_bytes()]
                        .concat()
                        .as_slice(),
                )
            })
            .unwrap_or_default();

        let package_id = parts
            .iter()
            .skip(2)
            .map(keccak256)
            .fold(organisation_id, |acc, t| {
                keccak256(
                    [acc.to_fixed_bytes(), t.to_fixed_bytes()]
                        .concat()
                        .as_slice(),
                )
            });

        Ok(PackageName::new(organisation_id, package_id))
    }

    fn parts(s: &str) -> anyhow::Result<Vec<&str>> {
        if s.len() > MAX_PACKAGE_NAME_LEN {
            bail!("max package name length excceed{MAX_PACKAGE_NAME_LEN} ")
        }

        let parts = s.split('.').collect::<Vec<_>>();

        if parts.len() < 3 || parts.len() > 8 {
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

    pub fn is_valid(s: &str) -> bool {
        if s.len() > MAX_PACKAGE_NAME_LEN {
            return false;
        }

        let parts = s.split('.').collect::<Vec<_>>();

        if parts.len() < 3 || parts.len() > 8 {
            return false;
        }

        if !parts[0].chars().all(|t| t.is_ascii_lowercase()) {
            return false;
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

        is_valid
    }
    pub fn new(organisation_id: H256, package_id: H256) -> Self {
        Self {
            organisation_id,
            package_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::util::PackageName;
    use crypto::keccak256;

    #[test]
    fn test_package_name_parts() {
        assert_eq!(
            PackageName::parts("com.example.subpackage").unwrap(),
            vec!["com", "example", "subpackage"]
        );
        assert_eq!(
            PackageName::parts("java.util.date").unwrap(),
            vec!["java", "util", "date"]
        );
        assert!(PackageName::parse("1231.msom.test").is_err());
        assert_eq!(
            PackageName::parts("java.1231.test").unwrap(),
            vec!["java", "1231", "test"]
        );
    }

    #[test]
    fn test_package_name_parsing() {
        assert_eq!(
            PackageName::parse("com.example.subpackage")
                .unwrap()
                .package_id,
            keccak256(
                [
                    keccak256(
                        [keccak256("com").as_bytes(), keccak256("example").as_bytes()].concat()
                    )
                    .as_bytes(),
                    keccak256("subpackage").as_bytes()
                ]
                .concat()
            )
        );

        assert_eq!(
            PackageName::parse("com.example.subpackage")
                .unwrap()
                .organisation_id,
            keccak256([keccak256("com").as_bytes(), keccak256("example").as_bytes()].concat())
        );
    }
}

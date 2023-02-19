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

use sled::Tree as SledTree;
use smt::StorageBackendSnapshot;

#[derive(Clone)]
pub(crate) struct DB {
    inner: SledTree,
    column: &'static column,
}

impl DB {
    pub fn new(inner: SledTree, column: &'static column) -> Self {
        Self { inner, column }
    }
}

impl smt::StorageBackend for DB {
    fn put(&mut self, key: &[u8], value: &[u8]) -> smt::Result<()> {
        let key_concat = [self.column.as_bytes(), key].concat();
        self.inner
            .insert(key_concat, value)
            .map_err(|e| smt::error::Error::CustomError(format!("{}", e)))?;
        Ok(())
    }

    fn get(&self, key: &[u8]) -> smt::Result<Vec<u8>> {
        let key_concat = [self.column.as_bytes(), key].concat();
        let value = self
            .inner
            .get(key_concat)
            .map_err(|e| smt::error::Error::CustomError(format!("{}", e)))?;
        value
            .map(|v| v.to_vec())
            .ok_or(smt::error::Error::StorageErrorKeyNotFound)
    }

    fn delete(&mut self, key: &[u8]) -> smt::Result<()> {
        let key_concat = [self.column.as_bytes(), key].concat();
        let _ = self
            .inner
            .remove(key_concat)
            .map_err(|e| smt::error::Error::CustomError(format!("{}", e)))?;
        Ok(())
    }

    fn get_or_default(&self, key: &[u8], default: Vec<u8>) -> smt::Result<Vec<u8>> {
        Ok(self.get(key).unwrap_or(default))
    }

    fn snapshot(&self) -> smt::Result<StorageBackendSnapshot> {
        let mut export = Vec::new();
        let iter = self.inner.scan_prefix(self.column.as_bytes());
        for Ok((k,v)) in iter {
            export.push((k.to_vec(), v.to_vec()));
        }
        Ok(export)
    }

    fn from_snapshot(_: StorageBackendSnapshot) -> smt::Result<Self> {
        unimplemented!()
    }

    fn new() -> Self {
        unimplemented!()
    }
}

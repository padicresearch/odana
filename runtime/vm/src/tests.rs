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

use crate::internal::event::Event;
use crate::internal::execution_context::ExecutionContext;
use crate::internal::log::Log;
use crate::internal::storage::Storage;
use crate::internal::syscall::Syscall;
use crate::internal::App;
use crate::{internal, WasmVM};
use anyhow::bail;
use primitive_types::{Address, H256};
use smt::SparseMerkleTree;
use std::sync::Arc;
use traits::{AppData, ChainHeadReader, StateDB, WasmVMInstance};
use types::account::AccountState;
use types::block::IndexedBlockHeader;
use types::prelude::SignedTransaction;
use types::Hash;
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Module, Store};

struct DummyState;

impl StateDB for DummyState {
    fn nonce(&self, address: &Address) -> u64 {
        todo!()
    }

    fn account_state(&self, address: &Address) -> AccountState {
        todo!()
    }

    fn balance(&self, address: &Address) -> u64 {
        todo!()
    }

    fn credit_balance(&self, address: &Address, amount: u64) -> anyhow::Result<H256> {
        todo!()
    }

    fn debit_balance(&self, address: &Address, amount: u64) -> anyhow::Result<H256> {
        todo!()
    }

    fn reset(&self, root: H256) -> anyhow::Result<()> {
        todo!()
    }

    fn apply_txs(
        &self,
        vm: Arc<dyn WasmVMInstance>,
        txs: &[SignedTransaction],
    ) -> anyhow::Result<H256> {
        todo!()
    }

    fn root(&self) -> Hash {
        todo!()
    }

    fn commit(&self) -> anyhow::Result<()> {
        todo!()
    }

    fn snapshot(&self) -> anyhow::Result<Arc<dyn StateDB>> {
        todo!()
    }

    fn state_at(&self, root: H256) -> anyhow::Result<Arc<dyn StateDB>> {
        todo!()
    }
}

struct TestComponentEnvironment {
    state: Arc<dyn StateDB>,
}

impl Storage for TestComponentEnvironment {
    fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) -> anyhow::Result<()> {
        bail!("error")
    }

    fn get(&mut self, key: Vec<u8>) -> anyhow::Result<Option<Vec<u8>>> {
        bail!("error")
    }

    fn remove(&mut self, key: Vec<u8>) -> anyhow::Result<bool> {
        bail!("error")
    }
}

impl Syscall for TestComponentEnvironment {
    fn block_hash(&mut self, level: u32) -> anyhow::Result<Vec<u8>> {
        todo!()
    }

    fn block(&mut self, block_hash: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        todo!()
    }

    fn address_from_pk(&mut self, pk: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        todo!()
    }

    fn generate_keypair(&mut self) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
        todo!()
    }

    fn generate_native_address(&mut self, seed: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        todo!()
    }

    fn sign(&mut self, sk: Vec<u8>, msg: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        todo!()
    }

    fn transfer(&mut self, to: Vec<u8>, amount: u64) -> anyhow::Result<bool> {
        todo!()
    }

    fn reserve(&mut self, amount: u64) -> anyhow::Result<bool> {
        todo!()
    }

    fn unreserve(&mut self, amount: u64) -> anyhow::Result<bool> {
        todo!()
    }
}

impl ExecutionContext for TestComponentEnvironment {
    fn value(&mut self) -> anyhow::Result<u64> {
        todo!()
    }

    fn block_level(&mut self) -> anyhow::Result<u32> {
        todo!()
    }

    fn sender(&mut self) -> anyhow::Result<Vec<u8>> {
        Ok(Address::default().to_vec())
    }

    fn network(&mut self) -> anyhow::Result<u32> {
        todo!()
    }

    fn sender_pk(&mut self) -> anyhow::Result<Vec<u8>> {
        todo!()
    }
}

impl Event for TestComponentEnvironment {
    fn emit(&mut self, event: Vec<u8>) -> anyhow::Result<()> {
        todo!()
    }
}

impl Log for TestComponentEnvironment {
    fn print(&mut self, output: Vec<char>) -> anyhow::Result<()> {
        todo!()
    }
}

#[test]
fn test_compile_wasm() {
    let bytes = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/testdata/example.component.wasm"
    ));
    let engine = Engine::new(Config::new().consume_fuel(false).wasm_component_model(true)).unwrap();
    let mut store = Store::new(
        &engine,
        TestComponentEnvironment {
            state: Arc::new(DummyState),
        },
    );

    //let module = Module::validate(&engine, bytes.as_slice()).unwrap();
    //Component::from_binary()

    let mut linker = Linker::<TestComponentEnvironment>::new(&engine);
    internal::syscall::add_to_linker(&mut linker, |env| env).unwrap();
    internal::log::add_to_linker(&mut linker, |env| env).unwrap();
    internal::execution_context::add_to_linker(&mut linker, |env| env).unwrap();
    internal::storage::add_to_linker(&mut linker, |env| env).unwrap();
    internal::event::add_to_linker(&mut linker, |env| env).unwrap();
    let component = Component::from_binary(&engine, bytes.as_slice()).unwrap();
    let instance = linker.instantiate(&mut store, &component).unwrap();
    let app = App::new(&mut store, &instance).unwrap();
    app.app().genesis(&mut store).unwrap();
}

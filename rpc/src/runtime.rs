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

use crate::rpc::runtime_api_service_server::RuntimeApiService;
use crate::rpc::{Query, QueryResponse, QueryStorage};
use primitive_types::Address;
use std::sync::Arc;
use tonic::{Code, Request, Response, Status};
use traits::{StateDB, WasmVMInstance};

pub(crate) struct RuntimeApiServiceImpl {
    state: Arc<dyn StateDB>,
    vm: Arc<dyn WasmVMInstance>,
}

impl RuntimeApiServiceImpl {
    pub(crate) fn new(state: Arc<dyn StateDB>, vm: Arc<dyn WasmVMInstance>) -> Self {
        Self { state, vm }
    }
}

#[tonic::async_trait]
impl RuntimeApiService for RuntimeApiServiceImpl {
    async fn query_runtime(
        &self,
        request: Request<Query>,
    ) -> Result<Response<QueryResponse>, Status> {
        let app_id = Address::from_slice(request.get_ref().app_id.as_slice())
            .map_err(|_e| Status::new(Code::Unknown, "failed to obtain app address"))?;
        let raw_query = request.get_ref().query.as_slice();
        self.vm
            .execute_app_query(self.state.clone(), app_id, raw_query)
            .map(|(typename, data)| Response::new(QueryResponse { typename, data }))
            .map_err(|e| Status::new(Code::Unknown, format!("failed to execute query: {}", e)))
    }

    async fn query_runtime_storage(
        &self,
        _request: Request<QueryStorage>,
    ) -> Result<Response<QueryResponse>, Status> {
        todo!()
    }
}

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
use crate::rpc::{GetDescriptorRequest, GetDescriptorResponse, QueryResponse, QueryStorage};
use std::sync::Arc;
use tonic::{Code, Request, Response, Status};
use traits::{StateDB, WasmVMInstance};
use types::prelude::ApplicationCall;

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
        request: Request<ApplicationCall>,
    ) -> Result<Response<QueryResponse>, Status> {
        let call = request.get_ref();
        self.vm
            .execute_app_query(self.state.clone(), call)
            .map(|data| Response::new(QueryResponse { data }))
            .map_err(|e| Status::new(Code::Unknown, format!("failed to execute query: {}", e)))
    }

    async fn query_runtime_storage(
        &self,
        _request: Request<QueryStorage>,
    ) -> Result<Response<QueryResponse>, Status> {
        todo!()
    }

    async fn get_descriptor(
        &self,
        request: Request<GetDescriptorRequest>,
    ) -> Result<Response<GetDescriptorResponse>, Status> {
        let app_id = request
            .get_ref()
            .app_id
            .ok_or_else(|| Status::new(Code::Unknown, "failed to obtain app address"))?;
        self.vm
            .execute_get_descriptor(self.state.clone(), app_id)
            .map(|descriptor| Response::new(GetDescriptorResponse { descriptor }))
            .map_err(|e| Status::new(Code::Unknown, e.to_string()))
    }
}

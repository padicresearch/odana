#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetAccountBalanceRequest {
    #[prost(string, tag = "1")]
    pub address: ::prost::alloc::string::String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetAccountBalanceResponse {
    #[prost(string, tag = "1")]
    pub balance: ::prost::alloc::string::String,
}

/// Generated client implementations.
pub mod account_service_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]

    use tonic::codegen::*;

    #[derive(Debug, Clone)]
    pub struct AccountServiceClient<T> {
        inner: tonic::client::Grpc<T>,
    }

    impl AccountServiceClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
            where
                D: std::convert::TryInto<tonic::transport::Endpoint>,
                D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }

    impl<T> AccountServiceClient<T>
        where
            T: tonic::client::GrpcService<tonic::body::BoxBody>,
            T::Error: Into<StdError>,
            T::ResponseBody: Body<Data=Bytes> + Send + 'static,
            <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> AccountServiceClient<InterceptedService<T, F>>
            where
                F: tonic::service::Interceptor,
                T::ResponseBody: Default,
                T: tonic::codegen::Service<
                    http::Request<tonic::body::BoxBody>,
                    Response=http::Response<
                        <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                    >,
                >,
                <T as tonic::codegen::Service<
                    http::Request<tonic::body::BoxBody>,
                >>::Error: Into<StdError> + Send + Sync,
        {
            AccountServiceClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with `gzip`.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_gzip(mut self) -> Self {
            self.inner = self.inner.send_gzip();
            self
        }
        /// Enable decompressing responses with `gzip`.
        #[must_use]
        pub fn accept_gzip(mut self) -> Self {
            self.inner = self.inner.accept_gzip();
            self
        }
        pub async fn get_account_balance(
            &mut self,
            request: impl tonic::IntoRequest<super::GetAccountBalanceRequest>,
        ) -> Result<tonic::Response<super::GetAccountBalanceResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/rpc.AccountService/GetAccountBalance",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
    }
}

/// Generated server implementations.
pub mod account_service_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]

    use tonic::codegen::*;

    ///Generated trait containing gRPC methods that should be implemented for use with AccountServiceServer.
    #[async_trait]
    pub trait AccountService: Send + Sync + 'static {
        async fn get_account_balance(
            &self,
            request: tonic::Request<super::GetAccountBalanceRequest>,
        ) -> Result<tonic::Response<super::GetAccountBalanceResponse>, tonic::Status>;
    }

    #[derive(Debug)]
    pub struct AccountServiceServer<T: AccountService> {
        inner: _Inner<T>,
        accept_compression_encodings: (),
        send_compression_encodings: (),
    }

    struct _Inner<T>(Arc<T>);

    impl<T: AccountService> AccountServiceServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
            where
                F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
    }

    impl<T, B> tonic::codegen::Service<http::Request<B>> for AccountServiceServer<T>
        where
            T: AccountService,
            B: Body + Send + 'static,
            B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/rpc.AccountService/GetAccountBalance" => {
                    #[allow(non_camel_case_types)]
                    struct GetAccountBalanceSvc<T: AccountService>(pub Arc<T>);
                    impl<
                        T: AccountService,
                    > tonic::server::UnaryService<super::GetAccountBalanceRequest>
                    for GetAccountBalanceSvc<T> {
                        type Response = super::GetAccountBalanceResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetAccountBalanceRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).get_account_balance(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetAccountBalanceSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        Ok(
                            http::Response::builder()
                                .status(200)
                                .header("grpc-status", "12")
                                .header("content-type", "application/grpc")
                                .body(empty_body())
                                .unwrap(),
                        )
                    })
                }
            }
        }
    }

    impl<T: AccountService> Clone for AccountServiceServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
            }
        }
    }

    impl<T: AccountService> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }

    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }

    impl<T: AccountService> tonic::transport::NamedService for AccountServiceServer<T> {
        const NAME: &'static str = "rpc.AccountService";
    }
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlockByHashRequest {
    #[prost(string, tag = "1")]
    pub hash: ::prost::alloc::string::String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlockByLevelRequest {
    #[prost(string, tag = "1")]
    pub hash: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlockResponse {
    #[prost(message, optional, tag = "1")]
    pub header: ::core::option::Option<super::types::BlockHeader>,
    #[prost(message, repeated, tag = "2")]
    pub txs: ::prost::alloc::vec::Vec<super::types::Transaction>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlocksRequest {
    #[prost(int32, tag = "1")]
    pub from: i32,
    #[prost(uint32, tag = "2")]
    pub count: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlocksResponse {
    #[prost(message, repeated, tag = "1")]
    pub blocks: ::prost::alloc::vec::Vec<super::types::BlockHeader>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlockNumberResponse {
    #[prost(int32, tag = "1")]
    pub level: i32,
}

/// Generated client implementations.
pub mod chain_service_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]

    use tonic::codegen::*;

    #[derive(Debug, Clone)]
    pub struct ChainServiceClient<T> {
        inner: tonic::client::Grpc<T>,
    }

    impl ChainServiceClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
            where
                D: std::convert::TryInto<tonic::transport::Endpoint>,
                D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }

    impl<T> ChainServiceClient<T>
        where
            T: tonic::client::GrpcService<tonic::body::BoxBody>,
            T::Error: Into<StdError>,
            T::ResponseBody: Body<Data=Bytes> + Send + 'static,
            <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> ChainServiceClient<InterceptedService<T, F>>
            where
                F: tonic::service::Interceptor,
                T::ResponseBody: Default,
                T: tonic::codegen::Service<
                    http::Request<tonic::body::BoxBody>,
                    Response=http::Response<
                        <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                    >,
                >,
                <T as tonic::codegen::Service<
                    http::Request<tonic::body::BoxBody>,
                >>::Error: Into<StdError> + Send + Sync,
        {
            ChainServiceClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with `gzip`.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_gzip(mut self) -> Self {
            self.inner = self.inner.send_gzip();
            self
        }
        /// Enable decompressing responses with `gzip`.
        #[must_use]
        pub fn accept_gzip(mut self) -> Self {
            self.inner = self.inner.accept_gzip();
            self
        }
        pub async fn current_head(
            &mut self,
            request: impl tonic::IntoRequest<super::super::types::Empty>,
        ) -> Result<tonic::Response<super::super::types::BlockHeader>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/rpc.ChainService/CurrentHead",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn block_level(
            &mut self,
            request: impl tonic::IntoRequest<super::super::types::Empty>,
        ) -> Result<tonic::Response<super::GetBlockNumberResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/rpc.ChainService/BlockLevel",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn get_block_by_hash(
            &mut self,
            request: impl tonic::IntoRequest<super::GetBlockByHashRequest>,
        ) -> Result<tonic::Response<super::GetBlockResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/rpc.ChainService/GetBlockByHash",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn get_block_by_level(
            &mut self,
            request: impl tonic::IntoRequest<super::GetBlockByLevelRequest>,
        ) -> Result<tonic::Response<super::GetBlockResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/rpc.ChainService/GetBlockByLevel",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn get_blocks(
            &mut self,
            request: impl tonic::IntoRequest<super::GetBlocksRequest>,
        ) -> Result<tonic::Response<super::GetBlocksResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/rpc.ChainService/GetBlocks",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
    }
}

/// Generated server implementations.
pub mod chain_service_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]

    use tonic::codegen::*;

    ///Generated trait containing gRPC methods that should be implemented for use with ChainServiceServer.
    #[async_trait]
    pub trait ChainService: Send + Sync + 'static {
        async fn current_head(
            &self,
            request: tonic::Request<super::super::types::Empty>,
        ) -> Result<tonic::Response<super::super::types::BlockHeader>, tonic::Status>;
        async fn block_level(
            &self,
            request: tonic::Request<super::super::types::Empty>,
        ) -> Result<tonic::Response<super::GetBlockNumberResponse>, tonic::Status>;
        async fn get_block_by_hash(
            &self,
            request: tonic::Request<super::GetBlockByHashRequest>,
        ) -> Result<tonic::Response<super::GetBlockResponse>, tonic::Status>;
        async fn get_block_by_level(
            &self,
            request: tonic::Request<super::GetBlockByLevelRequest>,
        ) -> Result<tonic::Response<super::GetBlockResponse>, tonic::Status>;
        async fn get_blocks(
            &self,
            request: tonic::Request<super::GetBlocksRequest>,
        ) -> Result<tonic::Response<super::GetBlocksResponse>, tonic::Status>;
    }

    #[derive(Debug)]
    pub struct ChainServiceServer<T: ChainService> {
        inner: _Inner<T>,
        accept_compression_encodings: (),
        send_compression_encodings: (),
    }

    struct _Inner<T>(Arc<T>);

    impl<T: ChainService> ChainServiceServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
            where
                F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
    }

    impl<T, B> tonic::codegen::Service<http::Request<B>> for ChainServiceServer<T>
        where
            T: ChainService,
            B: Body + Send + 'static,
            B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/rpc.ChainService/CurrentHead" => {
                    #[allow(non_camel_case_types)]
                    struct CurrentHeadSvc<T: ChainService>(pub Arc<T>);
                    impl<
                        T: ChainService,
                    > tonic::server::UnaryService<super::super::types::Empty>
                    for CurrentHeadSvc<T> {
                        type Response = super::super::types::BlockHeader;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::super::types::Empty>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).current_head(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = CurrentHeadSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/rpc.ChainService/BlockLevel" => {
                    #[allow(non_camel_case_types)]
                    struct BlockLevelSvc<T: ChainService>(pub Arc<T>);
                    impl<
                        T: ChainService,
                    > tonic::server::UnaryService<super::super::types::Empty>
                    for BlockLevelSvc<T> {
                        type Response = super::GetBlockNumberResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::super::types::Empty>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).block_level(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = BlockLevelSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/rpc.ChainService/GetBlockByHash" => {
                    #[allow(non_camel_case_types)]
                    struct GetBlockByHashSvc<T: ChainService>(pub Arc<T>);
                    impl<
                        T: ChainService,
                    > tonic::server::UnaryService<super::GetBlockByHashRequest>
                    for GetBlockByHashSvc<T> {
                        type Response = super::GetBlockResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetBlockByHashRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).get_block_by_hash(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetBlockByHashSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/rpc.ChainService/GetBlockByLevel" => {
                    #[allow(non_camel_case_types)]
                    struct GetBlockByLevelSvc<T: ChainService>(pub Arc<T>);
                    impl<
                        T: ChainService,
                    > tonic::server::UnaryService<super::GetBlockByLevelRequest>
                    for GetBlockByLevelSvc<T> {
                        type Response = super::GetBlockResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetBlockByLevelRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).get_block_by_level(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetBlockByLevelSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/rpc.ChainService/GetBlocks" => {
                    #[allow(non_camel_case_types)]
                    struct GetBlocksSvc<T: ChainService>(pub Arc<T>);
                    impl<
                        T: ChainService,
                    > tonic::server::UnaryService<super::GetBlocksRequest>
                    for GetBlocksSvc<T> {
                        type Response = super::GetBlocksResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetBlocksRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).get_blocks(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetBlocksSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        Ok(
                            http::Response::builder()
                                .status(200)
                                .header("grpc-status", "12")
                                .header("content-type", "application/grpc")
                                .body(empty_body())
                                .unwrap(),
                        )
                    })
                }
            }
        }
    }

    impl<T: ChainService> Clone for ChainServiceServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
            }
        }
    }

    impl<T: ChainService> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }

    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }

    impl<T: ChainService> tonic::transport::NamedService for ChainServiceServer<T> {
        const NAME: &'static str = "rpc.ChainService";
    }
}

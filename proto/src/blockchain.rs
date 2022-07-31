#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Empty {}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BlockHeader {
    #[prost(string, tag = "1")]
    pub parent_hash: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub merkle_root: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub state_root: ::prost::alloc::string::String,
    #[prost(string, tag = "4")]
    pub mix_nonce: ::prost::alloc::string::String,
    #[prost(string, tag = "5")]
    pub coinbase: ::prost::alloc::string::String,
    #[prost(uint32, tag = "6")]
    pub difficulty: u32,
    #[prost(uint32, tag = "7")]
    pub chain_id: u32,
    #[prost(int32, tag = "8")]
    pub level: i32,
    #[prost(uint32, tag = "9")]
    pub time: u32,
    #[prost(string, tag = "10")]
    pub nonce: ::prost::alloc::string::String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlockRequest {
    #[prost(oneof = "get_block_request::Query", tags = "1, 2")]
    pub query: ::core::option::Option<get_block_request::Query>,
}

/// Nested message and enum types in `GetBlockRequest`.
pub mod get_block_request {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Query {
        #[prost(string, tag = "1")]
        Hash(::prost::alloc::string::String),
        #[prost(int32, tag = "2")]
        Level(i32),
    }
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlockResponse {
    #[prost(message, optional, tag = "1")]
    pub header: ::core::option::Option<BlockHeader>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlocksRequest {
    #[prost(oneof = "get_blocks_request::Query", tags = "1, 2")]
    pub query: ::core::option::Option<get_blocks_request::Query>,
}

/// Nested message and enum types in `GetBlocksRequest`.
pub mod get_blocks_request {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct FromHash {
        #[prost(string, tag = "1")]
        pub from: ::prost::alloc::string::String,
        #[prost(uint32, tag = "2")]
        pub count: u32,
    }

    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct FromLevel {
        #[prost(int32, tag = "1")]
        pub from: i32,
        #[prost(uint32, tag = "2")]
        pub count: u32,
    }

    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Query {
        #[prost(message, tag = "1")]
        FromHash(FromHash),
        #[prost(message, tag = "2")]
        FromLevel(FromLevel),
    }
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlocksResponse {
    #[prost(message, repeated, tag = "1")]
    pub blocks: ::prost::alloc::vec::Vec<BlockHeader>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetHeadResponse {
    #[prost(message, optional, tag = "1")]
    pub block: ::core::option::Option<BlockHeader>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlockNumberResponse {
    #[prost(int32, tag = "1")]
    pub level: i32,
}

/// Generated client implementations.
pub mod blockchain_rpc_service_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]

    use tonic::codegen::*;

    #[derive(Debug, Clone)]
    pub struct BlockchainRpcServiceClient<T> {
        inner: tonic::client::Grpc<T>,
    }

    impl BlockchainRpcServiceClient<tonic::transport::Channel> {
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

    impl<T> BlockchainRpcServiceClient<T>
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
        ) -> BlockchainRpcServiceClient<InterceptedService<T, F>>
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
            BlockchainRpcServiceClient::new(InterceptedService::new(inner, interceptor))
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
        pub async fn get_head(
            &mut self,
            request: impl tonic::IntoRequest<super::Empty>,
        ) -> Result<tonic::Response<super::GetHeadResponse>, tonic::Status> {
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
                "/blockchain.BlockchainRPCService/GetHead",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn get_block_number(
            &mut self,
            request: impl tonic::IntoRequest<super::Empty>,
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
                "/blockchain.BlockchainRPCService/GetBlockNumber",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn get_block(
            &mut self,
            request: impl tonic::IntoRequest<super::GetBlockRequest>,
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
                "/blockchain.BlockchainRPCService/GetBlock",
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
                "/blockchain.BlockchainRPCService/GetBlocks",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
    }
}

/// Generated server implementations.
pub mod blockchain_rpc_service_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]

    use tonic::codegen::*;

    ///Generated trait containing gRPC methods that should be implemented for use with BlockchainRpcServiceServer.
    #[async_trait]
    pub trait BlockchainRpcService: Send + Sync + 'static {
        async fn get_head(
            &self,
            request: tonic::Request<super::Empty>,
        ) -> Result<tonic::Response<super::GetHeadResponse>, tonic::Status>;
        async fn get_block_number(
            &self,
            request: tonic::Request<super::Empty>,
        ) -> Result<tonic::Response<super::GetBlockNumberResponse>, tonic::Status>;
        async fn get_block(
            &self,
            request: tonic::Request<super::GetBlockRequest>,
        ) -> Result<tonic::Response<super::GetBlockResponse>, tonic::Status>;
        async fn get_blocks(
            &self,
            request: tonic::Request<super::GetBlocksRequest>,
        ) -> Result<tonic::Response<super::GetBlocksResponse>, tonic::Status>;
    }

    #[derive(Debug)]
    pub struct BlockchainRpcServiceServer<T: BlockchainRpcService> {
        inner: _Inner<T>,
        accept_compression_encodings: (),
        send_compression_encodings: (),
    }

    struct _Inner<T>(Arc<T>);

    impl<T: BlockchainRpcService> BlockchainRpcServiceServer<T> {
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

    impl<T, B> tonic::codegen::Service<http::Request<B>>
    for BlockchainRpcServiceServer<T>
        where
            T: BlockchainRpcService,
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
                "/blockchain.BlockchainRPCService/GetHead" => {
                    #[allow(non_camel_case_types)]
                    struct GetHeadSvc<T: BlockchainRpcService>(pub Arc<T>);
                    impl<
                        T: BlockchainRpcService,
                    > tonic::server::UnaryService<super::Empty> for GetHeadSvc<T> {
                        type Response = super::GetHeadResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::Empty>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).get_head(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetHeadSvc(inner);
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
                "/blockchain.BlockchainRPCService/GetBlockNumber" => {
                    #[allow(non_camel_case_types)]
                    struct GetBlockNumberSvc<T: BlockchainRpcService>(pub Arc<T>);
                    impl<
                        T: BlockchainRpcService,
                    > tonic::server::UnaryService<super::Empty>
                    for GetBlockNumberSvc<T> {
                        type Response = super::GetBlockNumberResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::Empty>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).get_block_number(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetBlockNumberSvc(inner);
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
                "/blockchain.BlockchainRPCService/GetBlock" => {
                    #[allow(non_camel_case_types)]
                    struct GetBlockSvc<T: BlockchainRpcService>(pub Arc<T>);
                    impl<
                        T: BlockchainRpcService,
                    > tonic::server::UnaryService<super::GetBlockRequest>
                    for GetBlockSvc<T> {
                        type Response = super::GetBlockResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetBlockRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).get_block(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetBlockSvc(inner);
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
                "/blockchain.BlockchainRPCService/GetBlocks" => {
                    #[allow(non_camel_case_types)]
                    struct GetBlocksSvc<T: BlockchainRpcService>(pub Arc<T>);
                    impl<
                        T: BlockchainRpcService,
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

    impl<T: BlockchainRpcService> Clone for BlockchainRpcServiceServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
            }
        }
    }

    impl<T: BlockchainRpcService> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }

    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }

    impl<T: BlockchainRpcService> tonic::transport::NamedService
    for BlockchainRpcServiceServer<T> {
        const NAME: &'static str = "blockchain.BlockchainRPCService";
    }
}

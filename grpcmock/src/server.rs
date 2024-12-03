use std::{convert::Infallible, net::SocketAddr, sync::Arc, time::Duration};

use http::{Request, Response};
use http_body_util::BodyExt;
use tokio::net::TcpStream;
use tonic::{
    body::BoxBody,
    codegen::{http, Body, BoxFuture, StdError},
    Code,
};
use tracing::debug;

use crate::{method::GrpcMethod, mock::MockSet, utils::find_available_port, Error};

const CONNECT_TIMEOUT_DURATION: Duration = Duration::from_millis(30);
const CONNECT_RETRY_SLEEP_DURATION: Duration = Duration::from_millis(30);
const CONNECT_RETRY_MAX_ATTEMPTS: i32 = 10;

/// State for a [`MockServer`].
#[derive(Debug)]
struct MockServerState {
    pub mocks: MockSet,
}

impl MockServerState {
    pub fn new(mocks: MockSet) -> Self {
        Self { mocks }
    }
}

/// A mock gRPC server.
#[derive(Clone)]
pub struct MockServer {
    name: &'static str,
    addr: SocketAddr,
    state: Arc<MockServerState>,
    inner: Arc<Option<Inner>>,
}

impl MockServer {
    /// Creates a new [`MockServer`].
    pub fn new(name: &'static str, mocks: MockSet) -> Result<Self, Error> {
        if mocks.iter().any(|(method, _)| method.service() != name) {
            return Err(Error::Invalid(format!(
                "all mocks must be for `{name}` service"
            )));
        }
        let port = find_available_port().unwrap();
        let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
        Ok(Self {
            name,
            addr,
            state: Arc::new(MockServerState::new(mocks)),
            inner: Arc::default(),
        })
    }

    /// Returns the server's service name.
    pub fn name(&self) -> &str {
        self.name
    }

    /// Returns the server's address.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    #[doc(hidden)]
    pub async fn _start(
        &mut self,
        handle: tokio::task::JoinHandle<Result<(), tonic::transport::Error>>,
    ) {
        for _ in 0..CONNECT_RETRY_MAX_ATTEMPTS {
            if tokio::time::timeout(CONNECT_TIMEOUT_DURATION, TcpStream::connect(self.addr()))
                .await
                .is_ok()
            {
                debug!("{} server listening on {}", self.name(), self.addr());
                break;
            }
            tokio::time::sleep(CONNECT_RETRY_SLEEP_DURATION).await;
        }
        self.inner = Arc::new(Some(Inner { handle }));
    }
}

impl MockServer {
    /// Handles a client request, returning a mock response.
    pub fn handle<B>(&self, req: Request<B>) -> BoxFuture<Response<BoxBody>, Infallible>
    where
        B: Body + Send + 'static,
        B::Data: Send,
        B::Error: Into<StdError> + Send + std::fmt::Debug + 'static,
    {
        let state = self.state.clone();
        let fut = async move {
            let method: GrpcMethod = req.uri().path().parse().unwrap();
            debug!(%method, "handling request");

            // Collect request body
            let body = req.into_body().collect().await.unwrap().to_bytes();

            // Match to mock and send response
            if let Some(mock) = state.mocks.find(&method, &body) {
                Ok(grpc_response(
                    mock.response.grpc_code(),
                    mock.response.body().to_boxed(),
                    mock.response.error(),
                ))
            } else {
                // Request not matched to mock, send error response
                Ok(grpc_response(
                    Code::NotFound,
                    tonic::body::empty_body(),
                    None,
                ))
            }
        };
        Box::pin(fut)
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct Inner {
    handle: tokio::task::JoinHandle<Result<(), tonic::transport::Error>>,
}

/// Builds a gRPC response.
fn grpc_response<B>(code: Code, body: B, error: Option<&str>) -> Response<B> {
    let mut builder = Response::builder()
        .status(200)
        .header("content-type", "application/grpc")
        .header("grpc-status", code as i32);
    if let Some(error) = error {
        builder = builder.header("grpc-message", error);
    }
    builder.body(body).unwrap()
}

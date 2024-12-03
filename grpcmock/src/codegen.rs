/// Generates a mock gRPC server.
#[macro_export]
macro_rules! generate_server {
    ($name:literal, $type:ident) => {
        use std::{
            ops::{Deref, DerefMut},
            task::Poll,
        };
        use tonic::codegen::{http, Body, BoxFuture, Service, StdError};

        use $crate::mock::MockSet;
        use $crate::server::MockServer;
        use $crate::Error;

        #[derive(Clone)]
        pub struct $type(MockServer);

        impl Deref for $type {
            type Target = MockServer;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl DerefMut for $type {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl<B> Service<http::Request<B>> for $type
        where
            B: Body + Send + 'static,
            B::Data: Send,
            B::Error: Into<StdError> + Send + std::fmt::Debug + 'static,
        {
            type Response = http::Response<tonic::body::BoxBody>;
            type Error = std::convert::Infallible;
            type Future = BoxFuture<Self::Response, Self::Error>;

            fn poll_ready(
                &mut self,
                _cx: &mut std::task::Context<'_>,
            ) -> Poll<std::result::Result<(), Self::Error>> {
                Poll::Ready(Ok(()))
            }
            fn call(&mut self, req: http::Request<B>) -> Self::Future {
                self.0.handle(req)
            }
        }

        impl tonic::server::NamedService for $type {
            const NAME: &'static str = $name;
        }

        impl $type {
            pub async fn start(mocks: MockSet) -> Result<Self, Error> {
                let server = MockServer::new($name, mocks)?;
                Ok(Self(server).serve().await)
            }

            async fn serve(&mut self) -> Self {
                let handle = tokio::spawn(
                    tonic::transport::Server::builder()
                        .add_service(self.clone())
                        .serve(self.addr()),
                );
                self._start(handle).await;
                self.to_owned()
            }
        }
    };
}

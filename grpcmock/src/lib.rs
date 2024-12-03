pub mod codegen;
pub mod method;
pub mod mock;
pub mod server;
pub mod utils;
pub mod prelude {
    pub use crate::generate_server;
    pub use crate::method::GrpcMethod;
    pub use crate::mock::{Mock, MockBody, MockRequest, MockResponse, MockSet};
    pub use crate::server::MockServer;
    pub use crate::utils::prost::MessageExt as _;
    pub use crate::Error;
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("invalid: {0}")]
    Invalid(String),
    #[error("yaml error: {0}")]
    YamlError(#[from] serde_yml::Error),
    #[error("json error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
}

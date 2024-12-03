use std::{
    collections::{hash_map, HashMap},
    fs::File,
    path::Path,
};

use bytes::Bytes;
use http::HeaderMap;
use http_body::Frame;
use http_body_util::{Full, StreamBody};
use prost::Message;
use serde::{de::DeserializeOwned, Deserialize};
use tonic::body::BoxBody;

use crate::{
    method::GrpcMethod,
    utils::{prost::MessageExt, tonic::CodeExt},
    Error,
};

/// A set of mocks for a service.
#[derive(Default, Debug, Clone)]
pub struct MockSet(HashMap<GrpcMethod, Vec<Mock>>);

impl MockSet {
    /// Creates a empty [`MockSet`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new [`MockSet`] with mocks.
    pub fn with_mocks(mocks: impl IntoIterator<Item = (GrpcMethod, Vec<Mock>)>) -> Self {
        Self(mocks.into_iter().collect())
    }

    /// Inserts [`Mock`]s from a mock file.
    pub fn insert_from_file<I, O>(&mut self, path: impl AsRef<Path>) -> Result<(), Error>
    where
        I: Message + DeserializeOwned,
        O: Message + DeserializeOwned,
    {
        let (method, mut mocks) = MockFile::read::<I, O>(path)?;
        match self.0.entry(method) {
            hash_map::Entry::Occupied(mut entry) => {
                entry.get_mut().append(&mut mocks);
            }
            hash_map::Entry::Vacant(entry) => {
                entry.insert(mocks);
            }
        }
        Ok(())
    }

    /// Inserts a [`Mock`].
    pub fn insert(&mut self, method: GrpcMethod, mock: Mock) {
        match self.0.entry(method) {
            hash_map::Entry::Occupied(mut entry) => {
                entry.get_mut().push(mock);
            }
            hash_map::Entry::Vacant(entry) => {
                entry.insert(vec![mock]);
            }
        }
    }

    /// Matches a [`Mock`] by method and request body.
    pub fn find(&self, method: &GrpcMethod, body: &[u8]) -> Option<&Mock> {
        self.0
            .get(method)
            .and_then(|mocks| mocks.iter().find(|&mock| mock.request.body() == body))
    }
}

impl FromIterator<(GrpcMethod, Vec<Mock>)> for MockSet {
    fn from_iter<T: IntoIterator<Item = (GrpcMethod, Vec<Mock>)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl std::ops::Deref for MockSet {
    type Target = HashMap<GrpcMethod, Vec<Mock>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A mock request and response pair.
#[derive(Debug, Clone, Deserialize)]
pub struct Mock {
    pub request: MockRequest,
    pub response: MockResponse,
}

impl Mock {
    /// Creates a new [`Mock`] from a request and response message.
    pub fn new(request: impl Message, response: impl Message) -> Self {
        let request = MockRequest::new(request.to_bytes().into());
        let response = MockResponse::new(response.to_bytes().into());
        Self { request, response }
    }

    /// Encode JSON body representation ([`JsonMockBody`]) to protobuf body ([`MockBody`]).
    fn encode_body<I, O>(&mut self) -> Result<(), Error>
    where
        I: Message + DeserializeOwned,
        O: Message + DeserializeOwned,
    {
        self.request.body = MockBody::from_json::<I>(&self.request.json_body, true)?;
        self.response.body = MockBody::from_json::<O>(&self.response.json_body, false)?;

        Ok(())
    }
}

/// A mock body in JSON format.
#[derive(Default, Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum JsonMockBody {
    #[default]
    Empty,
    Full(String),
    Stream(Vec<String>),
}

/// A mock body in protobuf bytes format.
#[derive(Default, Debug, Clone)]
pub enum MockBody {
    #[default]
    Empty,
    Full(Bytes),
    Stream(Vec<Bytes>),
}

impl MockBody {
    /// Creates a [`MockBody`] from a [`JsonMockBody`].
    pub fn from_json<T>(json_body: &JsonMockBody, flatten: bool) -> Result<Self, Error>
    where
        T: Message + DeserializeOwned,
    {
        use JsonMockBody::*;
        match json_body {
            Empty => Ok(MockBody::Empty),
            Full(value) => {
                let message = serde_json::from_str::<T>(value)?;
                Ok(MockBody::Full(message.to_bytes()))
            }
            Stream(values) => {
                let messages = values
                    .iter()
                    .map(|value| Ok(serde_json::from_str::<T>(value)?.to_bytes()))
                    .collect::<Result<Vec<_>, Error>>()?;
                if flatten {
                    // Flatten to a single byte array
                    Ok(MockBody::Full(messages.into_iter().flatten().collect()))
                } else {
                    Ok(MockBody::Stream(messages))
                }
            }
        }
    }

    /// Returns a type-erased HTTP body.
    pub fn to_boxed(&self) -> BoxBody {
        match self {
            MockBody::Empty => tonic::body::empty_body(),
            MockBody::Full(data) => tonic::body::boxed(Full::new(data.clone())),
            MockBody::Stream(data) => {
                let messages: Vec<Result<_, tonic::Status>> = data
                    .iter()
                    .map(|message| Ok(Frame::data(message.clone())))
                    .collect();
                BoxBody::new(StreamBody::new(futures::stream::iter(messages)))
            }
        }
    }
}

/// A mock request.
#[derive(Default, Debug, Clone, Deserialize)]
pub struct MockRequest {
    #[serde(default, with = "http_serde::header_map")]
    headers: HeaderMap,
    #[serde(rename = "body")]
    json_body: JsonMockBody,
    #[serde(skip)]
    body: MockBody,
}

impl MockRequest {
    pub fn new(body: MockBody) -> Self {
        Self {
            body,
            ..Default::default()
        }
    }

    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.headers = headers;
        self
    }

    pub fn with_body(mut self, body: MockBody) -> Self {
        self.body = body;
        self
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn json_body(&self) -> &JsonMockBody {
        &self.json_body
    }

    pub fn body(&self) -> &MockBody {
        &self.body
    }
}

/// A mock response.
#[derive(Default, Debug, Clone, Deserialize)]
pub struct MockResponse {
    #[serde(default, rename = "code", with = "http_serde::status_code")]
    http_code: http::StatusCode,
    #[serde(default, with = "http_serde::header_map")]
    headers: HeaderMap,
    #[serde(rename = "body", default)]
    json_body: JsonMockBody,
    #[serde(skip)]
    body: MockBody,
    error: Option<String>,
}

impl MockResponse {
    pub fn new(body: MockBody) -> Self {
        Self {
            body,
            ..Default::default()
        }
    }

    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.headers = headers;
        self
    }

    pub fn with_body(mut self, body: MockBody) -> Self {
        self.body = body;
        self
    }

    pub fn with_error(mut self, error: String) -> Self {
        self.error = Some(error);
        self
    }

    pub fn with_http_code(mut self, http_code: http::StatusCode) -> Self {
        self.http_code = http_code;
        self
    }

    pub fn http_code(&self) -> http::StatusCode {
        self.http_code
    }

    pub fn grpc_code(&self) -> tonic::Code {
        tonic::Code::from_http(self.http_code)
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn json_body(&self) -> &JsonMockBody {
        &self.json_body
    }

    pub fn body(&self) -> &MockBody {
        &self.body
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

impl PartialEq<[u8]> for MockBody {
    fn eq(&self, other: &[u8]) -> bool {
        match self {
            MockBody::Empty => other.is_empty(),
            MockBody::Full(bytes) => bytes == other,
            MockBody::Stream(data) => data.concat() == other,
        }
    }
}

impl From<Bytes> for MockBody {
    fn from(value: Bytes) -> Self {
        Self::Full(value)
    }
}

impl From<Vec<Bytes>> for MockBody {
    fn from(value: Vec<Bytes>) -> Self {
        Self::Stream(value)
    }
}

/// A YAML file defining a set of mocks for a method.
#[derive(Debug, Clone, Deserialize)]
pub struct MockFile {
    pub service: String,
    pub method: String,
    pub mocks: Vec<Mock>,
}

impl MockFile {
    pub fn read<I, O>(path: impl AsRef<Path>) -> Result<(GrpcMethod, Vec<Mock>), Error>
    where
        I: Message + DeserializeOwned,
        O: Message + DeserializeOwned,
    {
        let MockFile {
            service,
            method,
            mut mocks,
        } = serde_yml::from_reader(File::open(path)?)?;
        let method = GrpcMethod::new(service, method)?;
        for mock in mocks.iter_mut() {
            mock.encode_body::<I, O>()?;
        }
        Ok((method, mocks))
    }
}

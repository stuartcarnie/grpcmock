use serde::Deserialize;

use crate::Error;

/// A gRPC method.
#[derive(Debug, Clone, PartialEq, Hash, Eq, Deserialize)]
pub struct GrpcMethod {
    service: String,
    name: String,
}

impl GrpcMethod {
    pub fn new(service: impl Into<String>, name: impl Into<String>) -> Result<Self, Error> {
        let service = service.into();
        let name = name.into();
        let service_parts: Vec<&str> = service.split('.').collect();
        if service_parts
            .last()
            .unwrap()
            .chars()
            .nth(0)
            .is_some_and(|c| !c.is_uppercase())
        {
            return Err(Error::Invalid("service should start with uppercase".into()));
        }
        if name.chars().nth(0).is_some_and(|c| !c.is_uppercase()) {
            return Err(Error::Invalid("name should start with uppercase".into()));
        }
        Ok(Self { service, name })
    }

    /// Returns method's service name.
    pub fn service(&self) -> &str {
        &self.service
    }

    /// Returns method's unqualified name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns method's path.
    pub fn path(&self) -> String {
        format!("/{}/{}", self.service, self.name)
    }
}

impl std::fmt::Display for GrpcMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path())
    }
}

impl std::str::FromStr for GrpcMethod {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = if s.starts_with("/") {
            s.strip_prefix('/').unwrap()
        } else {
            s
        };
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            Err(Error::Invalid(
                "path should be formatted `<service>/<name>`".into(),
            ))
        } else {
            let service = parts
                .first()
                .unwrap() // len checked above
                .to_string();
            let name = parts
                .get(1)
                .unwrap() // len checked above
                .to_string();
            Self::new(service, name)
        }
    }
}

# grpcmock

grpcmock is a minimal framework for mocking gRPC services in Rust, supporting unary, client-streaming, server-streaming, and bidirectional-streaming methods.

# Table of contents
* [Features](#features)
* [Stubbing](#stubbing)
  * [In Rust](#in-rust)
  * [Mock Files](#mock-files)
* [Usage](#usage)
* [Examples](#examples)

## Features
- Mocks tonic gRPC services
- Mocks defined in Rust or YAML files using simple, intuitive spec
- Supports unary, client-streaming, server-streaming, and bidirectional-streaming methods
- Performs basic "full body" (equals) matching

## Stubbing

Reference service spec for examples below.
```proto
syntax = "proto3";

package example;

service Hello {
  rpc HelloUnary(HelloRequest) returns (HelloResponse) {}
  rpc HelloClientStreaming(stream HelloRequest) returns (HelloResponse) {}
}

message HelloRequest { string name = 1; }
message HelloResponse { string message = 1; }
```

### In Rust
Mocks can be defined in Rust using prost-generated types.

#### Using MockSet::insert()
Inserts a single `Mock` into a `MockSet`.

```rust
let mut mocks = MockSet::new();
mocks.insert(
    GrpcMethod::new("example.Hello", "HelloUnary")?, 
    Mock::new(HelloRequest {}, HelloResponse {})
);
```

#### Using MockSet::with_mocks()
Creates a new `MockSet` with a batch of mocks.

```rust
let mocks = MockSet::with_mocks(
    [
        // Mocks for HelloUnary method
        (
            GrpcMethod::new("example.Hello", "HelloUnary")?,
            vec![Mock::new(HelloRequest {}, HelloResponse {})],
        ),
        // Mocks for HelloClientStreaming method
        (
            GrpcMethod::new("example.Hello", "HelloClientStreaming")?,
            vec![Mock::new(HelloRequest {}, HelloResponse {})],
        ),
    ]
);
```

### Mock Files
Mocks can be defined in Mock Files, which are YAML-formatted specs for a single service method.

Example directory structure for the `example.Hello` service in `<project>/stubs/hello`:
```
├── hello
│   ├── method1.yaml
│   ├── method2.yaml
│   ├── method3.yaml
```

#### Spec:

```yaml
service: 'package.ServiceName' # fully-qualified gRPC service name
method: 'MethodName' # gRPC method name
mocks:
- request:
    body: '' # [''] for streaming
  response:
    code: 200 # optional, default=200
    body: '' # [''] for streaming
    error: '' # optional
```

#### Examples
1. **Client-streaming** method with success response
    ```yaml
    service: example.Hello
    method: HelloClientStreaming
    mocks:
    - request:
        body: # a list body represents a stream of messages
        - '{"name": "Dan"}' # HelloRequest
        - '{"name": "Gaurav"}'
        - '{"name": "Paul"}'
      response:
        code: 200
        body: '{"message": "Hello Dan, Gaurav, and Paul!"}' # HelloResponse
    ```
2. **Unary** method with error response
    ```yaml
    service: example.Hello
    method: HelloUnary
    mocks:
    - request:
        body: '{"name": "Error"}' # HelloRequest
      response:
        code: 400
        error: 'some error message'
    ```

#### Notes
- `service` is the fully-qualified gRPC service name (`<package>.<name>`) as defined in the proto file.
    - `name` starts with an uppercase letter, e.g. `example.Hello`.
`method` is the method name
    - Starts with an uppercase letter, e.g. `HelloUnary`
- `mocks` is a list of mocks for the method
- `request.body` / `response.body` is a JSON representation of the protobuf message
    - `string` for unary, `array<string>` for streaming
    - Currently, values must be set (even if empty) for *all* `repeated` and `map` fields
- `response.code` is a HTTP status code that is converted to an equivalent gRPC status code
- `response.error` is an optional error message for error responses

#### Using MockSet::insert_from_file<I, O>()

Generic type parameters correspond to prost-generated input and output types 
of the method defined in the mock file.

```rust
let mut mocks = MockSet::new();
mocks.insert_from_file::<HelloRequest, HelloResponse>("/path/to/file.yaml")?;
```

## Usage
1. Add `grpcmock` to `Cargo.toml`:
    ```
    [dev-dependencies]
    grpcmock = "0.1.0"
    ```

2. Add **required** type attributes to your `build.rs` configuration for generated protobuf types:
    ```rust
    tonic_build::configure()
        .type_attribute(
            ".",
            "#[derive(serde::Deserialize)] #[serde(rename_all = \"snake_case\")]",
        )
    ```
   This is to enable `JSON->T` deserialization of prost-generated (protobuf) types via serde.

3. Define stubs for your service following [Stubbing](#stubbing) guidance above.

4. In a test context, use as follows:
    ```rust

    #[cfg(test)]
    mod tests {
        use pb::{HelloRequest, HelloResponse, hello_client::HelloClient};
        use grpcmock::prelude::*;
        use tonic::transport::Channel;

        // Generate server `MockHelloServer` for the `example.Hello` service.
        generate_server!("example.Hello", MockHelloServer);

        #[tokio::test]
        async fn test_hello_with_mock_files() -> Result<(), anyhow::Error> {
            let mut mocks = MockSet::new();
            // Insert mocks from mock files
            // Generic type parameters correspond to prost-generated input and output types of the method.
            mocks.insert_from_file::<HelloRequest, HelloResponse>("stubs/hello/unary.yaml")?;
            mocks.insert_from_file::<HelloRequest, HelloResponse>("stubs/hello/client_streaming.yaml")?;

            // Start mock server
            let server = MockHelloServer::start(mocks).await?;

            // Create mock client
            let channel = Channel::from_shared(format!("http://0.0.0.0:{}", server.addr().port()))?
                .connect()
                .await?;
            let mut client = HelloClient::new(channel);

            // Send unary request
            let response = client
                .hello_unary(HelloRequest { name: "Dan".into() })
                .await;
            dbg!(response);

            // Send client-streaming request
            let request_stream = futures::stream::iter(vec![
                HelloRequest { name: "Dan".into() },
                HelloRequest { name: "Gaurav".into() },
                HelloRequest { name: "Paul".into() },
            ]);
            let response = client.hello_client_streaming(request_stream).await;
            dbg!(response);

            Ok(())
        }
    }
    ```

## Examples
See [grpcmock-test](/grpcmock-test/) crate for more examples.

mod pb {
    tonic::include_proto!("example");
}

#[cfg(test)]
mod tests {
    use super::pb::{hello_client::HelloClient, HelloRequest, HelloResponse};
    use futures::StreamExt;
    use grpcmock::prelude::*;
    use tonic::transport::Channel;

    grpcmock::generate_server!("example.Hello", MockHelloServer);

    #[tokio::test]
    async fn test_hello_with_mock_files() -> Result<(), anyhow::Error> {
        let mut mocks = MockSet::new();
        // Load mocks for HelloUnary method
        mocks.insert_from_file::<HelloRequest, HelloResponse>("stubs/hello/unary.yaml")?;
        // Load mocks for HelloClientStreaming method
        mocks
            .insert_from_file::<HelloRequest, HelloResponse>("stubs/hello/client_streaming.yaml")?;
        // Load mocks for HelloServerStreaming method
        mocks
            .insert_from_file::<HelloRequest, HelloResponse>("stubs/hello/server_streaming.yaml")?;
        // Load mocks for HelloBidiStreaming method
        mocks.insert_from_file::<HelloRequest, HelloResponse>("stubs/hello/bidi_streaming.yaml")?;

        let server = MockHelloServer::start(mocks).await?;

        let channel = Channel::from_shared(format!("http://0.0.0.0:{}", server.addr().port()))?
            .connect()
            .await?;
        let mut client = HelloClient::new(channel);

        let response = client
            .hello_unary(HelloRequest { name: "Dan".into() })
            .await;
        println!("unary response:\n{response:?}");

        let request_stream = futures::stream::iter(vec![
            HelloRequest { name: "Dan".into() },
            HelloRequest {
                name: "Gaurav".into(),
            },
            HelloRequest {
                name: "Paul".into(),
            },
        ]);
        let response = client.hello_client_streaming(request_stream).await;
        println!("client streaming response:\n{response:?}");

        let response = client
            .hello_server_streaming(HelloRequest {
                name: "Dan, Paul, Gaurav".into(),
            })
            .await;
        let mut stream = response.unwrap().into_inner();
        println!("server streaming response:");

        while let Some(result) = stream.next().await {
            println!("{result:?}");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_hello_with_invalid_mocks() {
        let mut mocks = MockSet::new();
        mocks.insert(
            GrpcMethod::new("WrongService", "Hello").unwrap(),
            Mock::unary(
                HelloRequest { name: "you".into() },
                HelloResponse {
                    message: "Hello you!".into(),
                },
            ),
        );
        assert!(MockHelloServer::start(mocks).await.is_err_and(|error| {
            error.to_string() == "invalid: all mocks must be for `example.Hello` service"
        }))
    }
}

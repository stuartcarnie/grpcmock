mod pb {
    tonic::include_proto!("tgis");
}

#[cfg(test)]
mod tests {
    use super::pb::{
        generation_service_client::GenerationServiceClient, BatchedGenerationRequest,
        BatchedGenerationResponse, GenerationRequest, GenerationResponse,
    };
    use grpcmock::prelude::*;
    use tonic::{transport::Channel, Code};

    grpcmock::generate_server!("tgis.GenerationService", MockGenerationServer);

    #[tokio::test]
    async fn test_generate_with_mock_files() -> Result<(), anyhow::Error> {
        let mut mocks = MockSet::new();
        // Load mocks for Generate method
        mocks.insert_from_file::<BatchedGenerationRequest, BatchedGenerationResponse>(
            "stubs/tgis/generate.yaml",
        )?;
        let server = MockGenerationServer::start(mocks).await?;

        let channel = Channel::from_shared(format!("http://0.0.0.0:{}", server.addr().port()))?
            .connect()
            .await?;
        let mut client = GenerationServiceClient::new(channel);

        let response = client
            .generate(BatchedGenerationRequest {
                model_id: "bloom-560m".into(),
                prefix_id: None,
                requests: vec![GenerationRequest {
                    text: "What's up?".into(),
                }],
                params: None,
            })
            .await;
        dbg!(&response);
        assert!(response.is_ok());

        let response = client
            .generate(BatchedGenerationRequest {
                model_id: "bloom-560m".into(),
                prefix_id: None,
                requests: vec![GenerationRequest {
                    text: "should not match".into(),
                }],
                params: None,
            })
            .await;
        dbg!(&response);
        assert!(response.is_err_and(|r| r.code() == Code::NotFound));

        Ok(())
    }

    #[tokio::test]
    async fn test_generate() -> Result<(), anyhow::Error> {
        let mocks = MockSet::with_mocks([(
            GrpcMethod::new("tgis.GenerationService", "Generate")?,
            vec![Mock::new(
                BatchedGenerationRequest {
                    model_id: "bloom-560m".into(),
                    prefix_id: None,
                    requests: vec![GenerationRequest {
                        text: "What's up?".into(),
                    }],
                    params: None,
                },
                BatchedGenerationResponse {
                    responses: vec![GenerationResponse {
                        input_token_count: 5,
                        generated_token_count: 12,
                        text: "Not much, you?".into(),
                        stop_reason: 1,
                        stop_sequence: "".into(),
                        seed: 0,
                        tokens: vec![],
                        input_tokens: vec![],
                    }],
                },
            )],
        )]);
        let server = MockGenerationServer::start(mocks).await?;

        let channel = Channel::from_shared(format!("http://0.0.0.0:{}", server.addr().port()))?
            .connect()
            .await?;
        let mut client = GenerationServiceClient::new(channel);

        let response = client
            .generate(BatchedGenerationRequest {
                model_id: "bloom-560m".into(),
                prefix_id: None,
                requests: vec![GenerationRequest {
                    text: "What's up?".into(),
                }],
                params: None,
            })
            .await;
        dbg!(&response);
        assert!(response.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_generate_error_message() -> Result<(), anyhow::Error> {
        let mut mocks = MockSet::new();
        mocks.insert(
            GrpcMethod::new("tgis.GenerationService", "Generate")?,
            Mock {
                request: MockRequest::new(MockBody::Full(
                    BatchedGenerationRequest {
                        model_id: "bloom-560m".into(),
                        prefix_id: None,
                        requests: vec![GenerationRequest { text: "".into() }],
                        params: None,
                    }
                    .to_bytes(),
                )),
                response: MockResponse::default()
                    .with_http_code(http::StatusCode::BAD_REQUEST)
                    .with_error("text cannot be empty".into()),
            },
        );
        mocks.insert(
            GrpcMethod::new("tgis.GenerationService", "Generate")?,
            Mock {
                request: MockRequest::new(MockBody::Full(
                    BatchedGenerationRequest {
                        model_id: "invalid_model".into(),
                        prefix_id: None,
                        requests: vec![GenerationRequest { text: ".".into() }],
                        params: None,
                    }
                    .to_bytes(),
                )),
                response: MockResponse::default()
                    .with_http_code(http::StatusCode::NOT_FOUND)
                    .with_error("model not found".into()),
            },
        );
        let server = MockGenerationServer::start(mocks).await?;

        let channel = Channel::from_shared(format!("http://0.0.0.0:{}", server.addr().port()))?
            .connect()
            .await?;
        let mut client = GenerationServiceClient::new(channel);

        let response = client
            .generate(BatchedGenerationRequest {
                model_id: "bloom-560m".into(),
                prefix_id: None,
                requests: vec![GenerationRequest { text: "".into() }],
                params: None,
            })
            .await;
        dbg!(&response);
        assert!(response.is_err_and(
            |e| e.code() == tonic::Code::Internal && e.message() == "text cannot be empty"
        ));

        let response = client
            .generate(BatchedGenerationRequest {
                model_id: "invalid_model".into(),
                prefix_id: None,
                requests: vec![GenerationRequest { text: ".".into() }],
                params: None,
            })
            .await;
        dbg!(&response);
        assert!(response
            .is_err_and(|e| e.code() == tonic::Code::NotFound && e.message() == "model not found"));

        Ok(())
    }
}

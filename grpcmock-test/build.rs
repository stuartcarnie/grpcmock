fn main() -> Result<(), Box<dyn std::error::Error>> {
    let descriptor_path =
        std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("descriptor.bin");
    tonic_build::configure()
        .build_client(true)
        .build_server(false)
        .file_descriptor_set_path(descriptor_path)
        .type_attribute(
            ".",
            "#[derive(serde::Deserialize)] #[serde(rename_all = \"snake_case\")]",
        )
        .compile_protos(
            &[
                "proto/example.proto",
                "proto/tgis.proto",
                "proto/health.proto",
            ],
            &["proto"],
        )
        .unwrap_or_else(|e| panic!("protobuf compilation failed: {}", e));
    Ok(())
}

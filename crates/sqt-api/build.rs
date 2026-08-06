fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var(
        "PROTOC",
        protoc_bin_vendored::protoc_bin_path().expect("vendored protoc"),
    );
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile_protos(
            &["../../proto/health.proto", "../../proto/agent.proto"],
            &["../../proto"],
        )?;
    Ok(())
}

fn main() {
    tonic_build::configure()
        .type_attribute("routeguide.Point", "#[derive(Hash)]")
        .build_transport(false)
        .compile_protos(&["proto/route_guide.proto"], &[""])
        .unwrap();
}

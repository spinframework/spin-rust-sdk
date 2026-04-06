fn main() {
    tonic_build::compile_protos("proto/greeter.proto").unwrap();
}

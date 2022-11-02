extern crate prost_build;

fn main() {
    prost_build::compile_protos(&["proto/valence.proto"], &["proto/"]).unwrap();
}

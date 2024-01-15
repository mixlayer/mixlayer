extern crate prost_build;

fn main() {
    let mut config = prost_build::Config::default();
    config.type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]");

    config
        .compile_protos(&["proto/valence.proto"], &["proto/"])
        .unwrap();
}

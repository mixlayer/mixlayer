pub mod protos {
    include!(concat!(env!("OUT_DIR"), "/valence.rs"));
}

mod buffer;

pub use prost;

pub use buffer::{ByteBuffer, FFIMessage};

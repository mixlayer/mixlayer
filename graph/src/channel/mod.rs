use bytes::Bytes;

use crate::Frame;

// pub mod inmemory;

pub trait OutputChannel {
    fn send(&self, data: Frame<Bytes>) -> ();
}

pub trait InputChannel {
    fn finished(&self) -> bool;
    fn recv(&self) -> Option<Frame<Bytes>>;
}

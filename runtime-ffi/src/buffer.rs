use prost::bytes::Bytes;

use crate::prost::Message;

#[repr(C)]
#[derive(Debug)]
pub struct ByteBuffer {
    pub data: *const u8,
    pub len: usize,
}

impl ByteBuffer {
    pub fn into_vec(self) -> Vec<u8> {
        unsafe { Vec::from_raw_parts(self.data as *mut u8, self.len, self.len) }
    }

    pub fn into_bytes(self) -> Bytes {
        self.into_vec().into()
    }

    pub fn from_slice(buf: &[u8]) -> Self {
        let len = buf.len();

        Self {
            data: buf.as_ptr(),
            len,
        }
    }
}

impl From<String> for ByteBuffer {
    fn from(s: String) -> Self {
        s.into_bytes().into()
    }
}

impl From<Vec<u8>> for ByteBuffer {
    fn from(mut bytes: Vec<u8>) -> Self {
        let len = bytes.len();
        let data = bytes.as_mut_ptr();

        std::mem::forget(bytes);

        Self { data, len }
    }
}

impl Into<ByteBuffer> for Bytes {
    fn into(self) -> ByteBuffer {
        //TODO remove copy
        let buf: Vec<u8> = self.into();
        buf.into()
    }
}

#[repr(transparent)]
pub struct FFIMessage<'a, T: Message>(pub &'a T);

impl<'a, T: Message> TryInto<ByteBuffer> for FFIMessage<'a, T> {
    type Error = prost::EncodeError;

    fn try_into(self) -> Result<ByteBuffer, Self::Error> {
        let mut buf = vec![];
        T::encode(self.0, &mut buf)?;
        Ok(buf.into())
    }
}

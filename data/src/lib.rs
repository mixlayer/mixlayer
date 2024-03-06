use bytes::Bytes;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{borrow::Borrow, fmt::Debug};

mod channel;

pub use channel::{InputChannel, OutputChannel};

pub trait JsonMxlData:
    Serialize + DeserializeOwned + Debug + Clone + Sized + Sync + Send + 'static
{
}

impl<T> MxlData for T
where
    T: JsonMxlData,
{
    fn from_buffer_frame(frame: Frame<Bytes>) -> Frame<Self> {
        frame.map(|s| {
            serde_json::from_slice(&s).unwrap() //FIXME from_buffer_from should return result
        })
    }

    fn into_buffer_frame(self) -> Result<Frame<Bytes>, ()> {
        Ok(Frame::Data(serde_json::to_vec(&self).unwrap().into()))
    }
}

pub trait MxlData: Debug + Clone + Sized + Sync + Send + 'static {
    fn from_buffer_frame(frame: Frame<Bytes>) -> Frame<Self>;
    fn into_buffer_frame(self) -> Result<Frame<Bytes>, ()>;
}

impl MxlData for String {
    fn from_buffer_frame(frame: Frame<Bytes>) -> Frame<Self> {
        frame.map(|d| String::from_utf8(d.into()).unwrap()) //FIXME
    }

    fn into_buffer_frame(self) -> Result<Frame<Bytes>, ()> {
        Ok(Frame::Data(self.into_bytes().into()))
    }
}

//FIXME in this impl, Some("") will be serialized as None, probably need a prefix byte
impl<T> MxlData for Option<T>
where
    T: MxlData,
{
    fn from_buffer_frame(frame: Frame<Bytes>) -> Frame<Self> {
        match frame {
            Frame::Data(mut d) => {
                use bytes::Buf;

                let some_none = d.get_u8();

                if some_none == 0 {
                    Frame::Data(None)
                } else {
                    T::from_buffer_frame(Frame::Data(d)).map(|v| Some(v))
                }
            }
            Frame::End => Frame::End,
            Frame::Error => Frame::Error,
        }
    }

    fn into_buffer_frame(self) -> Result<Frame<Bytes>, ()> {
        use bytes::BufMut;
        let mut buf = vec![];

        match self {
            Some(v) => {
                buf.put_u8(1);

                if let Ok(Frame::Data(element_bytes)) = T::into_buffer_frame(v) {
                    buf.put(element_bytes);
                } else {
                    panic!("element Option<T> serialize fialed")
                }
            }
            None => {
                buf.put_u8(0);
            }
        }

        Ok(Frame::Data(buf.into()))
    }
}

impl MxlData for () {
    fn from_buffer_frame(_frame: Frame<Bytes>) -> Frame<Self> {
        Frame::End
    }

    fn into_buffer_frame(self) -> Result<Frame<Bytes>, ()> {
        Ok(Frame::End)
    }
}

impl MxlData for u32 {
    fn from_buffer_frame(frame: Frame<Bytes>) -> Frame<Self> {
        use bytes::Buf;

        frame.map(|mut d| d.get_u32())
    }

    fn into_buffer_frame(self) -> Result<Frame<Bytes>, ()> {
        use bytes::BufMut;

        let mut buf = vec![];
        buf.put_u32(self);

        Ok(Frame::Data(buf.into()))
    }
}

impl<V: MxlData> MxlData for Vec<V> {
    fn from_buffer_frame(frame: Frame<Bytes>) -> Frame<Self> {
        use bytes::Buf;

        frame.map(|mut buf| {
            let mut out = Vec::new();

            // any empty Vec will be 0 bytes
            if buf.remaining() > 0 {
                loop {
                    let element_len = buf.get_u32() as usize;
                    let element_bytes = buf.split_to(element_len);
                    let element = V::from_buffer_frame(Frame::Data(element_bytes));

                    if let Frame::Data(v) = element {
                        out.push(v);
                    } else {
                        panic!("frame expected when reading to vec")
                    }

                    if buf.remaining() < 4 {
                        break;
                    }
                }
            }

            out
        })
    }

    fn into_buffer_frame(self) -> Result<Frame<Bytes>, ()> {
        use bytes::BufMut;
        let mut out: Vec<u8> = Vec::new();

        for v in self.into_iter() {
            if let Ok(Frame::Data(element_bytes)) = V::into_buffer_frame(v) {
                let element_len = element_bytes.len();
                out.put_u32(element_len.try_into().expect("element size exceeded limit"));
                out.put(element_bytes);
            } else {
                panic!("element serialize failed");
            }
        }

        Ok(Frame::Data(out.into()))
    }
}

#[derive(Clone, Debug)]
pub struct KV<K: MxlData + Debug, V: MxlData + Debug>(pub K, pub V);

impl<K: MxlData + Debug, V: MxlData + Debug> KV<K, V> {
    pub fn into_parts(self) -> (K, V) {
        (self.0, self.1)
    }

    pub fn key(&self) -> &K {
        &self.0
    }

    pub fn value(&self) -> &V {
        &self.1
    }
}

//TODO remove panics
impl<K: MxlData, V: MxlData> MxlData for KV<K, V> {
    fn from_buffer_frame(frame: Frame<Bytes>) -> Frame<Self> {
        frame.map(|mut bs| {
            use bytes::Buf;

            if bs.len() < 8 {
                panic!("kv length is too short: {}", bs.len());
            }

            let key_len = bs.get_u32() as usize;
            let val_len = bs.get_u32() as usize;

            let key_bytes = bs.split_to(key_len); //bs.slice(key_start_ofs..key_end_ofs);
            let val_bytes = bs.split_to(val_len); //bs.slice(val_start_ofs..val_end_ofs);

            let key_frame = K::from_buffer_frame(Frame::Data(key_bytes));
            let val_frame = V::from_buffer_frame(Frame::Data(val_bytes));

            match (key_frame, val_frame) {
                (Frame::Data(key), Frame::Data(value)) => KV(key, value),
                other => panic!("unexpected value when deserializng KV: {:?}", other),
            }
        })
    }

    fn into_buffer_frame(self) -> Result<Frame<Bytes>, ()> {
        let key_bytes = self.0.into_buffer_frame();
        let val_bytes = self.1.into_buffer_frame();

        match (key_bytes, val_bytes) {
            (Ok(Frame::Data(mut key_bytes)), Ok(Frame::Data(mut val_bytes))) => {
                use bytes::BufMut;

                //TODO may want to use i32 instead because it's friendlier to other languages which may not support unsigned
                let key_len: u32 = key_bytes.len().try_into().expect("key size exceeds limit");
                let val_len: u32 = val_bytes
                    .len()
                    .try_into()
                    .expect("value size exceeds limit");

                // println!("ibf key_len: {}, val_len: {}", key_len, val_len);

                let total_len = key_bytes.len() + val_bytes.len() + 8;

                let mut out = Vec::with_capacity(total_len);

                out.put_u32(key_len);
                out.put_u32(val_len);

                out.put(&mut key_bytes);
                out.put(&mut val_bytes);

                Ok(Frame::Data(out.into()))
            }
            other => panic!("unexpected value when serializing kv: {:?}", other),
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Frame, MxlData, KV};

    #[test]
    fn kv_serialize() {
        let sample = KV("A".to_owned(), "ab".to_owned());
        let data = sample.into_buffer_frame().unwrap();

        if let Frame::Data(mut data) = data {
            assert_eq!(data.len(), 8 + 3);

            use bytes::Buf;

            let key_len = data.get_u32() as usize;
            let val_len = data.get_u32() as usize;

            assert_eq!(key_len, 1);
            assert_eq!(val_len, 2);

            // let mut key_vec = Vec::with_capacity(key_len as usize);
            let key_bytes = data.split_to(key_len);
            let val_bytes = data.split_to(val_len);

            let key_str = String::from_utf8(key_bytes.to_vec()).unwrap();
            let val_str = String::from_utf8(val_bytes.to_vec()).unwrap();

            assert_eq!("A", key_str);
            assert_eq!("ab", val_str);
        } else {
            panic!("frame was not data")
        }
    }

    #[test]
    fn kv_serialize_nested() {
        let sample = KV("A".to_owned(), KV("AA".to_owned(), "aa".to_owned()));
        let data = sample.into_buffer_frame().unwrap();

        if let Frame::Data(data) = data {
            assert_eq!(data.len(), 16 + 1 + 4);
        } else {
            panic!("frame was not data")
        }
    }
}

#[derive(Debug, Clone)]
pub enum Frame<T> {
    Data(T),
    End,
    Error, //TODO error details
}

impl<T> Frame<T> {
    pub fn map<U, F: Fn(T) -> U>(self, f: F) -> Frame<U> {
        match self {
            Frame::Data(d) => Frame::Data(f(d)),
            Frame::Error => Frame::Error,
            Frame::End => Frame::End,
        }
    }

    pub fn flat_map<U, F: Fn(T) -> Frame<U>>(self, f: F) -> Frame<U> {
        match self {
            Frame::Data(d) => f(d),
            Frame::Error => Frame::Error,
            Frame::End => Frame::End,
        }
    }
}

impl Frame<Bytes> {
    //TODO embed some kind of versioning info or use proto?
    //TODO should probably put type, len information at end to avoid a copy
    pub fn into_bytes(self) -> Bytes {
        use bytes::BufMut;
        let mut out_buf: Vec<u8> = Vec::new();

        match self {
            Frame::Data(b) => {
                out_buf.put_u8(0);
                out_buf.put_u32(b.len() as u32);

                out_buf.put(b);
            }
            Frame::End => {
                out_buf.put_u8(1);
            }
            Frame::Error => {
                out_buf.put_u8(2);
            }
        };

        out_buf.into()
    }

    //TODO result
    pub fn from_bytes(mut b: Bytes) -> Frame<Bytes> {
        use bytes::Buf;
        let ordinal = b.get_u8();

        match ordinal {
            0 => {
                let len = b.get_u32() as usize;
                let buf = b.split_to(len);
                Frame::Data(buf)
            }
            1 => Frame::End,
            2 => Frame::Error,
            _ => panic!("invalid frame ordinal in bytes"),
        }
    }
}

/// Newtype that wraps a JSON value guaranteed to be an object
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct JsonObject(serde_json::Map<String, serde_json::Value>);
impl JsonMxlData for JsonObject {}
impl JsonObject {
    pub fn as_map(&self) -> &serde_json::Map<String, serde_json::Value> {
        &self.0
    }

    pub fn as_map_mut(&mut self) -> &mut serde_json::Map<String, serde_json::Value> {
        &mut self.0
    }

    pub fn into_map(self) -> serde_json::Map<String, serde_json::Value> {
        self.0
    }

    pub fn into_value(self) -> serde_json::Value {
        serde_json::Value::Object(self.into_map())
    }
}

impl<'a, Q> std::ops::Index<&'a Q> for JsonObject
where
    String: Borrow<Q>,
    Q: ?Sized + Ord + Eq + std::hash::Hash,
{
    type Output = serde_json::Value;

    fn index(&self, index: &Q) -> &serde_json::Value {
        self.0.index(index)
    }
}

impl<'a, Q> std::ops::IndexMut<&'a Q> for JsonObject
where
    String: Borrow<Q>,
    Q: ?Sized + Ord + Eq + std::hash::Hash,
{
    fn index_mut(&mut self, index: &'a Q) -> &mut Self::Output {
        self.0.index_mut(index)
    }
}

impl TryFrom<serde_json::Value> for JsonObject {
    type Error = anyhow::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        match value {
            serde_json::Value::Object(map) => Ok(JsonObject(map)),
            _ => Err(anyhow::anyhow!("not a json object")),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct JsonValue(serde_json::Value);
impl JsonMxlData for JsonValue {}
impl JsonValue {
    pub fn as_value(&self) -> &serde_json::Value {
        &self.0
    }

    pub fn as_value_mut(&mut self) -> &mut serde_json::Value {
        &mut self.0
    }
}

impl From<serde_json::Value> for JsonValue {
    fn from(value: serde_json::Value) -> Self {
        JsonValue(value)
    }
}

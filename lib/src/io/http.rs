use anyhow::Result;
use valence_runtime_ffi::ByteBuffer;

use valence_runtime_ffi::protos::{HttpMethodProto, HttpRequestProto, HttpResponseProto};

extern "C" {
    // fn _valence_http_client_handle(name_buf: *const ByteBuffer) -> i32;
    fn _valence_http_request(handle: i32, request: *const ByteBuffer) -> *mut ByteBuffer;
}

#[derive(Clone)]
pub struct VHttpClient {
    handle: i32,
}

impl VHttpClient {
    pub fn new() -> Self {
        //TODO introduce handles that are connected to different HTTP sandbox configs
        VHttpClient { handle: 0 }
    }

    // pub fn with_name(name: &str) -> Result<Client> {
    //     let handle = unsafe {
    //         let buf = ByteBuffer::from_slice(name.as_bytes());
    //         _valence_http_client_handle(&buf)
    //     };

    //     if handle == 0 {
    //         return Err(anyhow::Error::msg("invalid http client"));
    //     };

    //     Ok(Client { handle })
    // }

    pub fn send(&mut self, request: http::Request<Vec<u8>>) -> Result<http::Response<Vec<u8>>> {
        use valence_runtime_ffi::prost::Message;

        let (parts, body) = request.into_parts();
        let parts_proto = HttpRequestProto {
            method: match parts.method {
                http::Method::GET => HttpMethodProto::HttpMethodGet as i32,
                http::Method::POST => HttpMethodProto::HttpMethodPost as i32,
                http::Method::PUT => HttpMethodProto::HttpMethodPut as i32,
                http::Method::PATCH => HttpMethodProto::HttpMethodPatch as i32,
                _ => Err(anyhow::Error::msg("invalid http method"))?,
            },
            url: parts.uri.to_string(),
            headers: vec![], //FIXME header support
            body,
        };

        let mut buf = vec![];
        parts_proto.encode(&mut buf)?;

        let request_buf = ByteBuffer::from_slice(&buf);
        let buf_ptr =
            unsafe { Box::from_raw(_valence_http_request(self.handle, &request_buf)).into_bytes() };

        let resp_msg = HttpResponseProto::decode(buf_ptr)?;

        //TODO headers, version
        let resp = http::response::Builder::new()
            .status(resp_msg.status_code as u16)
            .body(resp_msg.body)?;

        Ok(resp)
    }
}

use anyhow::Result;
use http::HeaderMap;
use mixlayer_runtime_ffi::ByteBuffer;

use mixlayer_runtime_ffi::protos::{
    HttpHeaderProto, HttpMethodProto, HttpRequestProto, HttpResponseProto,
};

extern "C" {
    // fn _valence_http_client_handle(name_buf: *const ByteBuffer) -> i32;
    fn _valence_http_request(handle: i32, request: *const ByteBuffer) -> *mut ByteBuffer;
}

#[derive(Debug, Clone)]
pub struct VHttpClient {
    default_headers: HeaderMap,
    handle: i32,
}

impl VHttpClient {
    pub fn new(default_headers: Option<HeaderMap>) -> Self {
        //TODO introduce handles that are connected to different HTTP sandbox configs
        VHttpClient {
            default_headers: default_headers.unwrap_or_default(),
            handle: 0,
        }
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

    //TODO make url arg generic over TryInto<Uri>
    pub fn get(&mut self, url: &str) -> Result<http::Response<Vec<u8>>> {
        let uri: http::Uri = url.try_into()?;
        let http_request = http::Request::builder()
            .uri(uri)
            .method(http::Method::GET)
            .body(vec![])?;

        let resp = self.send(http_request)?;

        Ok(resp)
    }

    //TODO make url arg generic over TryInto<Uri>
    pub fn post(&mut self, url: &str) -> Result<http::Response<Vec<u8>>> {
        let uri: http::Uri = url.try_into()?;
        let http_request = http::Request::builder()
            .uri(uri)
            .method(http::Method::GET)
            .body(vec![])?;

        let resp = self.send(http_request)?;

        Ok(resp)
    }

    pub fn send(&mut self, request: http::Request<Vec<u8>>) -> Result<http::Response<Vec<u8>>> {
        use mixlayer_runtime_ffi::prost::Message;

        let headers: Vec<HttpHeaderProto> = self
            .default_headers
            .iter()
            .chain(request.headers())
            .map(|(hn, hv)| HttpHeaderProto {
                header_name: hn.to_string(),
                header_value: hv.to_str().unwrap().to_owned(),
            })
            .collect();

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
            headers,
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

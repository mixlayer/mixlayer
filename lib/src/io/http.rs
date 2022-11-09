use valence_runtime_ffi::ByteBuffer;

extern "C" {
    fn _valence_http_request(request_proto: *const ByteBuffer) -> u32;
}

pub struct VHttpRequest {}

pub struct VHttpResponse {}

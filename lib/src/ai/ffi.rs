use valence_runtime_ffi::prost::Message;
use valence_runtime_ffi::protos::BatchChatCompletionResponse;
use valence_runtime_ffi::protos::ChatCompletionResponse;
use valence_runtime_ffi::protos::CreateEmbeddingRequest;
use valence_runtime_ffi::protos::CreateEmbeddingResponse;

use crate::ByteBuffer;
use crate::Result;

extern "C" {
    // creates a new embedding from the request
    // proto: CreateEmbeddingRequest -> CreateEmbeddingRespones
    fn _embedding_request(request: *const ByteBuffer) -> *mut ByteBuffer;

    // proto: ChatCompletionRequest -> ChatCompletionResponse
    fn _chat_completion_request(request: *const ByteBuffer) -> *mut ByteBuffer;

    // proto: BatchChatCompletionRequest -> BatchChatCompletionResponse
    fn _batch_chat_completion_request(request: *const ByteBuffer) -> *mut ByteBuffer;
}

pub fn embedding_request(request: CreateEmbeddingRequest) -> Result<CreateEmbeddingResponse> {
    let request_bytes: ByteBuffer = request.encode_to_vec().into();
    let response_bytes: Box<ByteBuffer> =
        unsafe { Box::from_raw(_embedding_request(&request_bytes)) };

    let response_bytes = response_bytes.into_bytes();

    Ok(CreateEmbeddingResponse::decode(response_bytes)?)
}

pub fn chat_completion_request(
    req: valence_runtime_ffi::protos::ChatCompletionRequest,
) -> Result<String> {
    let request_bytes: ByteBuffer = req.encode_to_vec().into();
    let response_bytes: Box<ByteBuffer> =
        unsafe { Box::from_raw(_chat_completion_request(&request_bytes)) };

    let response_bytes = response_bytes.into_bytes();

    Ok(ChatCompletionResponse::decode(response_bytes)?.message)
}

pub fn batch_chat_completion_request(
    req: valence_runtime_ffi::protos::BatchChatCompletionRequest,
) -> Result<Vec<String>> {
    let request_bytes: ByteBuffer = req.encode_to_vec().into();
    let response_bytes: Box<ByteBuffer> =
        unsafe { Box::from_raw(_batch_chat_completion_request(&request_bytes)) };

    let response_bytes = response_bytes.into_bytes();

    let messages = BatchChatCompletionResponse::decode(response_bytes)?
        .responses
        .into_iter()
        .map(|r| r.message)
        .collect();

    Ok(messages)
}

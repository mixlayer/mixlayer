use valence_runtime_ffi::protos::{
    ChatCompletionModelProto, ChatCompletionRequest, CreateEmbeddingRequest, EmbeddingModelProto,
};

use super::{ChatCompletionModel, EmbeddingModel};
use crate::Result;

pub struct OpenAIAda002;

// TODO: currently relying on FFI calls to the host to do the embedding but
// we should be able to do this in wasm through HTTP requests. Easier now beacuse
// we can use async-openai library in host
impl EmbeddingModel for OpenAIAda002 {
    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let resp = super::ffi::embedding_request(CreateEmbeddingRequest {
            input: text.to_owned(),
            model: EmbeddingModelProto::OpenAiAda002 as i32,
        })?;

        Ok(resp.embedding)
    }

    fn num_dims(&self) -> u32 {
        1536
    }

    fn token_limit(&self) -> u32 {
        8191
    }
}

pub struct Gpt4Turbo;

impl ChatCompletionModel for Gpt4Turbo {
    fn complete(&self, messages: &str) -> Result<String> {
        Ok(super::ffi::chat_completion_request(
            ChatCompletionRequest {
                prompt: messages.to_owned(),
                model: ChatCompletionModelProto::OpenAiGpt4Turbo as i32,
            },
        )?)
    }

    fn token_limit(&self) -> u32 {
        128000
    }
}

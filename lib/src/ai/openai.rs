use mixlayer_runtime_ffi::protos::{
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

pub trait FFIChatCompletionModel {
    fn token_limit(&self) -> u32;
    fn ffi_model(&self) -> ChatCompletionModelProto;
}

impl FFIChatCompletionModel for Gpt4Turbo {
    fn token_limit(&self) -> u32 {
        128000
    }

    fn ffi_model(&self) -> ChatCompletionModelProto {
        ChatCompletionModelProto::OpenAiGpt4Turbo
    }
}

impl FFIChatCompletionModel for Gpt4 {
    fn token_limit(&self) -> u32 {
        128000
    }

    fn ffi_model(&self) -> ChatCompletionModelProto {
        ChatCompletionModelProto::OpenAiGpt4
    }
}

impl FFIChatCompletionModel for Gpt35Turbo {
    fn token_limit(&self) -> u32 {
        16385
    }

    fn ffi_model(&self) -> ChatCompletionModelProto {
        ChatCompletionModelProto::OpenAiGpt35Turbo
    }
}

pub struct Gpt4Turbo;
pub struct Gpt4;

pub struct Gpt35Turbo;

impl<T> ChatCompletionModel for T
where
    T: FFIChatCompletionModel,
{
    fn complete(&self, messages: &str) -> Result<String> {
        Ok(super::ffi::chat_completion_request(
            ChatCompletionRequest {
                prompt: messages.to_owned(),
                model: self.ffi_model() as i32,
            },
        )?)
    }

    fn token_limit(&self) -> u32 {
        128000
    }
}

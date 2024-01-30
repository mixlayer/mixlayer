use crate::Result;

mod ffi;
mod openai;

pub trait EmbeddingModel {
    // embed the given text into a vector
    fn embed(&self, text: &str) -> Result<Vec<f32>>;

    // number of dimensions in the output vector
    fn num_dims(&self) -> u32;

    // maximum number of input tokens this model can accept
    fn token_limit(&self) -> u32;
}

pub trait ChatCompletionModel {
    fn complete(&self, messages: &str) -> Result<String>;
    fn token_limit(&self) -> u32;
}

pub use openai::FFIChatCompletionModel;
pub use openai::Gpt4;
pub use openai::Gpt4Turbo;
pub use openai::OpenAIAda002;

pub use ffi::batch_chat_completion_request;

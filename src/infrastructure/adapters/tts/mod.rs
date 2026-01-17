//! TTS Adapter - HTTP TTS 客户端实现

mod fake_tts_client;
mod http_tts_client;

pub use fake_tts_client::{FakeTtsClient, FakeTtsClientConfig};
pub use http_tts_client::*;

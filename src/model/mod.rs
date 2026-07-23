pub mod adapter;
pub mod external;
#[cfg(feature = "live-model")]
pub mod live;

pub use adapter::{FixtureModelAdapter, ModelAdapter, ModelError, ModelResponse};
pub use external::{ArgvExternalGenerator, GeneratorJailOpts, DEFAULT_STAGING_OUTPUT};
#[cfg(feature = "live-model")]
pub use live::{
    build_generation_prompt, LiveModelConfig, OpenAiCompatibleAdapter, DEFAULT_CHAT_PATH,
};

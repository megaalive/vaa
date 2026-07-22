pub mod adapter;
pub mod external;

pub use adapter::{FixtureModelAdapter, ModelAdapter, ModelError, ModelResponse};
pub use external::{ArgvExternalGenerator, DEFAULT_STAGING_OUTPUT};

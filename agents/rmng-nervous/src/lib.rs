pub mod connector;
pub mod mock;
pub mod ollama;

pub use connector::{ConnectorError, NervousConnector};
pub use ollama::OllamaAdapter;

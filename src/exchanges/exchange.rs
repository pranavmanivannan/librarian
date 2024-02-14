use crate::{buffer::Buffer, error::SymbolError, listeners::listener::Listener};
use async_trait::async_trait;
use tokio::task::JoinHandle;

/// The `Exchange` trait is used to instantiate both a Listener and Buffer for each exchange which both run in separate
/// `tokio::task`.
#[async_trait]
pub trait Exchange: Sized {
    /// The associated listener for each exchange.
    type Listener: Listener;

    /// This function creates a `TaskSet` which contains the `JoinHandle` for both the listener and buffer per exchange.
    async fn build(exchange_name: &str) -> TaskSet {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let listener = Self::Listener::listen(sender).await;
        let buffer = Buffer::create_task(exchange_name, 5000, receiver);

        return TaskSet::Default(listener, buffer);
    }
}

/// The `TaskSet` enum is used to hold variants of multiple `JoinHandle` tuples.
pub enum TaskSet {
    /// The default variant is used to hold a `JoinHandle` for both the listener and buffer. This is what most exchanges
    /// return as they do not override the default `build` function.
    Default(JoinHandle<Result<(), tungstenite::Error>>, JoinHandle<()>),
    /// The extended variant is used to hold a `JoinHandle` for the listener, buffer, and an additional `JoinHandle` in
    /// case the `build` function requires additional tasks to be created.
    Extended(
        JoinHandle<Result<(), tungstenite::Error>>,
        JoinHandle<()>,
        JoinHandle<Result<(), SymbolError>>,
    ),
}

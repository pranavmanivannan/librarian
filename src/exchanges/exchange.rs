use crate::{buffer::Buffer, error::SymbolError, listeners::listener::Listener};
use async_trait::async_trait;
use tokio::task::JoinHandle;

#[async_trait]
pub trait Exchange: Sized {
    type Listener: Listener;

    async fn build(
        exchange_name: &str,
    ) -> TaskSet {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let listener = Self::Listener::listen(sender).await;
        let buffer = Buffer::create_task(exchange_name, 1000, receiver);

        return TaskSet::Default(listener, buffer)
    }
}


pub enum TaskSet {
    Default(JoinHandle<Result<(), tungstenite::Error>>, JoinHandle<()>),
    Extended(JoinHandle<Result<(), tungstenite::Error>>, JoinHandle<()>, JoinHandle<Result<(), SymbolError>>),
}
use crate::{buffer::Buffer, listeners::listener::Listener};
use async_trait::async_trait;
use tokio::task::JoinHandle;

#[async_trait]
pub trait Exchange: Sized {
    type Listener: Listener;

    async fn build(
        exchange_name: &str,
    ) -> (JoinHandle<Result<(), tungstenite::Error>>, JoinHandle<()>) {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let listener = Self::Listener::listen(sender).await;
        let buffer = Buffer::create_task(exchange_name, 100, receiver);

        return (listener, buffer);
    }
}

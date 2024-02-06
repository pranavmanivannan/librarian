use crate::{buffer::Buffer, listeners::listener::Listener};
use async_trait::async_trait;
use tokio::task::JoinHandle;

#[async_trait]
pub trait Exchange: Sized {
    type Listener: Listener;

    fn get_socket_url(self) -> String;

    async fn build(self) -> (JoinHandle<()>, JoinHandle<Result<(), tungstenite::Error>>) {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let websocket_url = self.get_socket_url();
        let listener = Self::Listener::listen(&websocket_url, sender).await;
        let buffer = Buffer::create_task("ByBit", 500, receiver);

        return (buffer, listener);
    }
}

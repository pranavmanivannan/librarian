use tokio::sync::mpsc::UnboundedReceiver;

use crate::{buffer::Buffer, data_packet::{self, DataPacket}};

pub async fn storage_loop(mut buffer: Buffer, mut receiver: UnboundedReceiver<DataPacket>) {
    loop {
        if let Some(data_packet) = receiver.recv().await {
            let _ = buffer.ingest(data_packet).await;
        }
    }
}
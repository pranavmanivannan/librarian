use std::sync::Arc;

use tokio::sync::mpsc::UnboundedReceiver;
use tokio_util::sync::CancellationToken;

use crate::{
    buffer::Buffer,
    data_packet::DataPacket,
    stats::{Metric, MetricManager},
};

/// Continuously polls a receiver for DataPackets. If there is a DataPacket, it will send
/// it to the buffer.
///
/// Arguments
/// * `buffer` - A Buffer struct used to hold data before sending it to InfluxDB.
/// * `receiver` - An UnboundedReceiver of the type DataPacket. The corresponding UnboundedSender is in a Listener.
pub async fn storage_loop(
    mut buffer: Buffer,
    mut receiver: UnboundedReceiver<DataPacket>,
    cancellation_token: CancellationToken,
) {
    loop {
        // if let Some(data_packet) = receiver.recv().await {
        //     let _ = buffer.ingest(data_packet).await;
        // }
        // if cancellation_token.is_cancelled() {
        //     let _ = buffer.shutdown().await;
        //     println!("Buffers flushed!");
        //     break;
        // }
        tokio::select! {
            msg = receiver.recv() => match msg {
                Some(msg) => {let _ = buffer.ingest(msg).await;},
                None => break,
            },
            () = cancellation_token.cancelled() => {
                let _ = buffer.shutdown().await;
                println!("Buffers flushed!");
                break;
            }
        }
    }
}

pub async fn stats_loop(metrics: Arc<MetricManager>) {
    let time = 30;
    loop {
        metrics.throughput.log();
        metrics.parsetime.log();
        metrics.packetsize.log();
        tokio::time::sleep(tokio::time::Duration::from_secs(time)).await;
    }
}

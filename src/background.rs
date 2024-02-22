use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::{buffer::Buffer, data_packet::DataPacket, stats::Counter};

/// Continuously polls a receiver for DataPackets. If there is a DataPacket, it will send
/// it to the buffer.
///
/// Arguments
/// * `buffer` - A Buffer struct used to hold data before sending it to InfluxDB.
/// * `receiver` - An UnboundedReceiver of the type DataPacket. The corresponding UnboundedSender is in a Listener.
pub async fn storage_loop(mut buffer: Buffer, mut receiver: UnboundedReceiver<DataPacket>) {
    loop {
        if let Some(data_packet) = receiver.recv().await {
            let _ = buffer.ingest(data_packet).await;
        }
    }
}

/// Runs a background task that gets the number of messages ingested into a buffer every 10 seconds to calculate
/// throughput and logs it to a file.
///
/// Arguments
/// * `counter` - A reference to an Arc<Counter> used to keep track of the number of messages ingested.
pub async fn stats_loop(counter: &Arc<Counter>) {
    loop {
        let count: f64 = counter.get_value() as f64;
        counter.reset();
        log::info!(
            "Messages ingested per second (throughput): {:?}",
            count / 10.0
        );
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}

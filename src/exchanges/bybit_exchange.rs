use std::sync::Arc;

use super::exchange::{Exchange, TaskSet};
use crate::{buffer::Buffer, listeners::{bybit_listener::ByBitListener, bybit_snap_listener::ByBitSnapshotListener, listener::Listener}, stats::MetricManager};
use async_trait::async_trait;

pub struct ByBitExchange {}

#[async_trait]
impl Exchange for ByBitExchange {
    type Listener = ByBitListener;

    /// An overriden `build` function that returns a `TaskSet` of the `Extended` variant.
    ///
    /// # Arguments
    /// * `exchange_name` - A string slice that holds the name of the exchange. This is used when creating the buffer
    /// and should refer to the first half of the bucket name on InfluxDB.
    ///
    /// # Returns
    /// A `TaskSet` containing a `JoinHandle` for the listener, buffer, and an additional `JoinHandle` for the HTTP
    /// listener. The HTTP listener is used to retrieve orderbook snapshots as Binance does not send them through the
    /// websocket stream.
    async fn build(exchange_name: &str, metric_manager: Arc<MetricManager>) -> TaskSet {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let listener = ByBitListener::listen(sender.clone(), metric_manager.clone()).await;
        let buffer = Buffer::create_task(exchange_name, 500, receiver, metric_manager.clone());

        let snapshot_listener = tokio::spawn(async move {
            loop{
                let snap_listener = ByBitSnapshotListener::listen(sender.clone(), metric_manager.clone()).await;
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                drop(snap_listener);
            }
        });

        TaskSet::Extended(listener, buffer, snapshot_listener)
    }
}

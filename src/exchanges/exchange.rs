use std::sync::Arc;

use crate::{
    buffer::Buffer, error::SymbolError, listeners::listener::Listener, stats::MetricManager,
};
use async_trait::async_trait;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// The `Exchange` trait is used to instantiate both a Listener and Buffer for each exchange which both run in separate
/// `tokio::task`.
#[async_trait]
pub trait Exchange: Sized {
    /// The associated listener for each exchange.
    type Listener: Listener;

    /// This function creates a `TaskSet` which contains the `JoinHandle` for both the listener and buffer per exchange.
    ///
    /// # Arguments
    /// * `exchange_name` - A string slice that holds the name of the exchange. This is used when creating the buffer
    /// and should refer to the first half of the bucket name on InfluxDB. For example, "ByBit" or "Binance" would be
    /// appropriate exchange names which correspond to the InfluxDB bucket names "ByBit-Incremental" and
    /// "Binance-Snapshot".
    ///
    /// # Returns
    /// A `TaskSet` enum which holds multiple `JoinHandle` tuples.
    async fn build(
        exchange_name: &str,
        metric_manager: Arc<MetricManager>,
        cancellation_token: CancellationToken,
    ) -> TaskSet {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let listener = Self::Listener::listen(sender, metric_manager.clone()).await;
        let buffer = Buffer::create_task(
            exchange_name,
            5000,
            receiver,
            metric_manager.clone(),
            cancellation_token,
        );

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

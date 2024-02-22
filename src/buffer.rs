use crate::{background::storage_loop, data_packet, error::DBError, stats::COUNTER};
use data_packet::DataPacket;
use dotenv::dotenv;
use reqwest::{self};
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use std::env;
use tokio::{sync::mpsc::UnboundedReceiver, task::JoinHandle};

const ORGANIZATION: &str = "Quant Dev";

/// A struct for making a buffer
pub struct Buffer {
    client: reqwest_middleware::ClientWithMiddleware,
    snapshots: Vec<String>,
    incrementals: Vec<String>,
    bucket: String,
    capacity: usize,
}

/// An implementation of the Buffer struct which allows Buffers
impl Buffer {
    /// Creates a new buffer with a reqwest client to push to InfluxDB.
    ///
    /// # Arguments
    /// * `bucket_name` - The name of the bucket on InfluxDB.
    /// * `capacity` - The capacity of the buffer before it pushes to InfluxDB.
    ///
    /// # Returns
    /// A `Buffer` struct to be used with a corresponding listener.
    pub fn new(bucket_name: &str, capacity: usize) -> Buffer {
        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(5);
        let retry_middleware = RetryTransientMiddleware::new_with_policy(retry_policy);
        let reqwest_client = ClientBuilder::new(reqwest::Client::new())
            .with(retry_middleware)
            .build();

        if capacity == 0 {
            panic!("Buffer capacity must be greater than 0");
        }

        Buffer {
            client: reqwest_client,
            snapshots: Vec::with_capacity(capacity),
            incrementals: Vec::with_capacity(capacity),
            bucket: bucket_name.to_string(),
            capacity,
        }
    }

    /// A function that creates a new buffer and then creates a tokio::task using that buffer,
    ///
    /// # Arguments
    /// * `bucket_name` - The name of the bucket on InfluxDB.
    /// * `capacity` - The capacity of the buffer before it pushes to InfluxDB.
    /// * `receiver` - An `UnboundedReceiver` that receives the type `DataPacket`.
    ///
    /// # Returns
    /// A JoinHandle to use.
    pub fn create_task(
        bucket_name: &str,
        capacity: usize,
        receiver: UnboundedReceiver<DataPacket>,
    ) -> JoinHandle<()> {
        let buffer = Buffer::new(bucket_name, capacity);
        tokio::spawn(storage_loop(buffer, receiver))
    }

    /// A separate function that sorts datapackets by type and pushes it to buffer.
    ///
    /// # Arguments
    /// * `data_packet` - A DataPacket received from a listener.
    ///
    /// # Returns
    /// A Result with an empty Ok or a DBError if the DataPacket couldn't be pushed.
    pub async fn ingest(&mut self, data_packet: DataPacket) -> Result<(), DBError> {
        match &data_packet {
            DataPacket::MI(msg) => {
                let asks = serde_json::to_string(&msg.asks).map_err(DBError::JsonError)?;
                let bids = serde_json::to_string(&msg.bids).map_err(DBError::JsonError)?;
                let message = format!(
                    "{} asks={:?},bids={:?},cur_seq={},prev_seq={} {}",
                    msg.symbol_pair, asks, bids, msg.cur_seq, msg.prev_seq, msg.timestamp
                );
                self.incrementals.push(message);
                COUNTER.increment();
                if self.incrementals.len() >= self.capacity {
                    self.push_to_influx(DataType::MI).await?;
                    self.incrementals.clear();
                }
                Ok(())
            }
            DataPacket::ST(msg) => {
                let asks = serde_json::to_string(&msg.asks).map_err(DBError::JsonError)?;
                let bids = serde_json::to_string(&msg.bids).map_err(DBError::JsonError)?;
                let message = format!(
                    "{} asks={:?},bids={:?},cur_seq={},prev_seq={} {}",
                    msg.symbol_pair, asks, bids, msg.cur_seq, msg.prev_seq, msg.timestamp
                );
                self.snapshots.push(message);
                COUNTER.increment();
                if self.snapshots.len() >= self.capacity {
                    self.push_to_influx(DataType::ST).await?;
                    self.snapshots.clear();
                }
                Ok(())
            }
            DataPacket::Ping(_) => Ok(()),
        }
    }

    /// Pushes the data in a buffer to an InfluxDB bucket.
    ///
    /// # Arguments
    /// * `data_type` - The type of data to push to InfluxDB.
    ///
    /// # Returns
    /// A Result containing an empty Ok if pushing to InfluxDB was successful, else a DBError.
    async fn push_to_influx(&self, data_type: DataType) -> Result<(), DBError> {
        dotenv().ok();
        let storage = match data_type {
            DataType::MI => &self.incrementals,
            DataType::ST => &self.snapshots,
        };
        let data = storage.join("\n");
        let bucket_name = match data_type {
            DataType::MI => format!("{}-{}", &self.bucket, "Incremental"),
            DataType::ST => format!("{}-{}", &self.bucket, "Snapshot"),
        };

        let url = format!(
            "https://us-east-1-1.aws.cloud2.influxdata.com/api/v2/write?org={}&bucket={}&precision=ms",
            ORGANIZATION, bucket_name,
        );

        let api_token = env::var("API_TOKEN").expect("API_TOKEN must be set");

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Token {}", api_token))
            .header("Content-Type", "text/plain; charset=utf-8")
            .header("Accept", "application/json")
            .body(data)
            .send()
            .await;

        match response {
            Ok(res) => {
                if res.status().is_success() {
                    log::info!(
                        "Uploaded bucket: {:?}, Status code: {:?}",
                        bucket_name,
                        res.status()
                    );
                    Ok(())
                } else {
                    log::error!("Upload Error: HTTP {}", res.status());
                    Err(DBError::HttpError(res.status()))
                }
            }
            Err(e) => {
                log::error!("Reqwest Error: {:?}", e);
                Err(DBError::ReqwestMiddlewareError(e))
            }
        }
    }
}

/// The `DataType` enum allows for functions to differentiate between which type of data is being referred to when
/// pushing to a buffer or InfluxDB.
pub enum DataType {
    MI,
    ST,
}

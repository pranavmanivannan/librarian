use crate::{data_packet, error::DBError};
use data_packet::DataPacket;
use dotenv::dotenv;
use reqwest::{self};
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use std::env;

const ORGANIZATION: &str = "Quant Dev";

/// A struct for making a buffer
pub struct Buffer {
    client: reqwest_middleware::ClientWithMiddleware,
    storage: Vec<String>,
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
            storage: Vec::with_capacity(capacity),
            bucket: bucket_name.to_string(),
            capacity,
        }
    }

    /// A separate function that sorts datapackets and pushes it to buffer.
    ///
    /// # Arguments
    /// * `data_packet` - A DataPacket received from a listener.
    ///
    /// # Returns
    /// A Result with an empty Ok or a DBError if the DataPacket couldn't be pushed.
    pub async fn ingest(&mut self, data_packet: DataPacket) -> Result<(), DBError> {
        let message = match &data_packet {
            DataPacket::MI(msg) => {
                let asks = serde_json::to_string(&msg.asks)?;
                let bids = serde_json::to_string(&msg.bids)?;
                format!(
                    "{}, asks={}, bids={}, cur_seq={}, prev_seq={}, timestamp={}",
                    msg.symbol_pair, asks, bids, msg.cur_seq, msg.prev_seq, msg.timestamp
                )
            }
            _ => "".to_string(),
        };

        self.push_and_flush(message).await
    }
    /// Pushes data from a buffer to an InfluxDB bucket and clears the buffer afterwards.
    ///
    /// # Arguments
    /// * `message` - A string of the datapacket formatted to fit InfluxDB.
    ///
    /// # Returns
    /// A Result that is either empty or a DBError if the message couldn't be pushed to a buffer.
    pub async fn push_and_flush(&mut self, message: String) -> Result<(), DBError> {
        self.storage.push(message);

        if self.storage.len() == self.capacity {
            self.push_to_influx().await?;
            self.storage.clear();
        }

        Ok(())
    }

    /// Pushes the data in a buffer to an InfluxDB bucket.
    async fn push_to_influx(&self) -> Result<(), DBError> {
        dotenv().ok();
        let data = self.storage.join("\n");
        let bucket_name = &self.bucket;

        let url = format!(
            "https://us-east-1-1.aws.cloud2.influxdata.com/api/v2/write?org={}&bucket={}",
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
                    println!(
                        "Uploaded bucket: {:?}, Status code: {:?}",
                        &self.bucket,
                        res.status()
                    );
                    let data = res.text().await?;
                    println!("Uploaded Data:\n{}", data);
                    Ok(())
                } else {
                    eprintln!("Upload Error: HTTP {}", res.status());
                    Err(DBError::HttpError(res.status()))
                }
            }
            Err(error) => {
                eprintln!("Reqwest Error: {:?}", error);
                Err(DBError::ReqwestError(error))
            }
        }
    }
}

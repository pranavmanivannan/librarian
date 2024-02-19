use super::exchange::Exchange;
use crate::listeners::bybit_listener::ByBitListener;
use async_trait::async_trait;

pub struct ByBitExchange {}

#[async_trait]
impl Exchange for ByBitExchange {
    type Listener = ByBitListener;
}

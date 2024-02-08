use async_trait::async_trait;

use crate::listeners::bybit_listener::ByBitListener;

use super::exchange::Exchange;

pub struct ByBitExchange {}

#[async_trait]
impl Exchange for ByBitExchange {
    type Listener = ByBitListener;
}

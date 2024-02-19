use super::exchange::Exchange;
use crate::listeners::huobi_listener::HuobiListener;
use async_trait::async_trait;

pub struct HuobiExchange {}

#[async_trait]
impl Exchange for HuobiExchange {
    type Listener = HuobiListener;
}

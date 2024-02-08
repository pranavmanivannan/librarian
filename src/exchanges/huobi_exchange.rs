use async_trait::async_trait;

use crate::listeners::huobi_listener::HuobiListener;

use super::exchange::Exchange;

pub struct HuobiExchange {}

#[async_trait]
impl Exchange for HuobiExchange {
    type Listener = HuobiListener;
}

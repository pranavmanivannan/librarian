use crate::listeners::huobi_listener::HuobiListener;

use super::exchange::Exchange;

pub struct HuobiExchange {}

impl Exchange for HuobiExchange {
    type Listener = HuobiListener;
}

use crate::listeners::bybit_listener::ByBitListener;

use super::exchange::Exchange;

pub struct ByBitExchange {}

impl Exchange for ByBitExchange {
    type Listener = ByBitListener;
}

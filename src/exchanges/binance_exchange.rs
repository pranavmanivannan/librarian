use crate::listeners::binance_listener::BinanceListener;

use super::exchange::Exchange;

pub struct BinanceExchange {}

impl Exchange for BinanceExchange {
    type Listener = BinanceListener;
}

use crate::listeners::binance_listener::BinanceListener;

use super::exchange::Exchange;

const BINANCE_WS: &str = "wss://fstream.binance.com";

pub struct BinanceExchange {
    websocket_url: String,
}

impl BinanceExchange {
    pub fn new() -> Self {
        BinanceExchange {
            websocket_url: BINANCE_WS.to_string(),
        }
    }
}

impl Exchange for BinanceExchange {
    type Listener = BinanceListener;

    fn get_socket_url(self) -> String {
        self.websocket_url
    }
}

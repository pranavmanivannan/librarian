use crate::listeners::bybit_listener::ByBitListener;

use super::exchange::Exchange;

/// The websocket url used to connect to ByBit's perpetuals and futures market data.
const BYBIT_WS: &str = "wss://stream.bybit.com/v5/public/linear";

pub struct ByBitExchange {
    websocket_url: String,
}

impl ByBitExchange {
    pub fn new() -> Self {
        ByBitExchange {
            websocket_url: BYBIT_WS.to_string(),
        }
    }
}

impl Exchange for ByBitExchange {
    type Listener = ByBitListener;

    fn get_socket_url(self) -> String {
        self.websocket_url
    }
}

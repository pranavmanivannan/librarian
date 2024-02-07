use crate::listeners::huobi_listener::HuobiListener;

use super::exchange::Exchange;

/// The websocket url used to connect to Huobi
const HUOBI_WS: &str = "wss://api.hbdm.vn/linear-swap-ws";

pub struct HuobiExchange {
    websocket_url: String,
}

impl HuobiExchange {
    pub fn new() -> Self {
        HuobiExchange {
            websocket_url: HUOBI_WS.to_string(),
        }
    }
}

impl Exchange for HuobiExchange {
    type Listener = HuobiListener;

    fn get_socket_url(self) -> String {
        self.websocket_url
    }
}

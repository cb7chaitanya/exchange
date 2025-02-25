use serde::{Deserialize, Serialize};
use crate::redis::redis_manager::OrderSide;
use crate::trade::orderbook::Order;
use crate::trade::orderbook::Fill;


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MessageFromApi {
    #[serde(rename = "CREATE_ORDER")]
    CreateOrder {
        market: String,
        price: String,
        quantity: String,
        side: OrderSide,
        user_id: String,
    },
    
    #[serde(rename = "CANCEL_ORDER")]
    CancelOrder {
        order_id: String,
        market: String,
        user_id: String,
    },
    
    #[serde(rename = "ON_RAMP")]
    OnRamp {
        amount: String,
        user_id: String,
        txn_id: String,
    },
    
    #[serde(rename = "GET_DEPTH")]
    GetDepth {
        market: String,
        user_id: String,
    },
    
    #[serde(rename = "GET_OPEN_ORDERS")]
    GetOpenOrders {
        user_id: String,
        market: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum MessageToApi {
    #[serde(rename = "DEPTH")]
    Depth {
        bids: Vec<(String, String)>,
        asks: Vec<(String, String)>,
    },

    #[serde(rename = "ORDER_PLACED")]
    OrderPlaced {
        order_id: String,
        executed_qty: f64,
        fills: Vec<Fill>,
    },

    #[serde(rename = "ORDER_CANCELLED")]
    OrderCancelled {
        order_id: String,
        executed_qty: f64,
        remaining_qty: f64,
    },

    #[serde(rename = "OPEN_ORDERS")]
    OpenOrders {
        orders: Vec<Order>,
    },
}

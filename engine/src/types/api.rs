use serde::{Deserialize, Serialize};
use crate::redis::redis_manager::OrderSide;
use crate::trade::orderbook::Order;
use crate::trade::orderbook::Fill;


#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum MessageFromApi {
    #[serde(rename = "CREATE_ORDER")]
    CreateOrder {
        data: CreateOrderData,
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
        data: GetDepthData,
    },
    
    #[serde(rename = "GET_OPEN_ORDERS")]
    GetOpenOrders {
        data: GetOpenOrdersData,
    },
}

#[derive(Debug, Deserialize)]
pub struct GetDepthData {
    pub market: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOpenOrdersData {
    pub user_id: String,
    pub market: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessageToApi {
    #[serde(rename = "DEPTH")]
    Depth {
        payload: DepthPayload,
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
        payload: Vec<Order>,
    },

    #[serde(rename = "ERROR")]
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthPayload {
    pub market: String,
    pub bids: Vec<(String, String)>,
    pub asks: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrderData {
    pub market: String,
    pub price: String,
    pub quantity: String,
    pub side: OrderSide,
}

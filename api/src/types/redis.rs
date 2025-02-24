use serde::{Serialize, Deserialize};

pub const CREATE_ORDER: &str = "CREATE_ORDER";
pub const CANCEL_ORDER: &str = "CANCEL_ORDER";
pub const ON_RAMP: &str = "ON_RAMP";
pub const GET_OPEN_ORDERS: &str = "GET_OPEN_ORDERS";
pub const GET_DEPTH: &str = "GET_DEPTH";

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessageFromOrderbook {
    #[serde(rename = "DEPTH")]
    Depth {
        payload: DepthPayload,
    },
    #[serde(rename = "ORDER_PLACED")]
    OrderPlaced {
        payload: OrderPlacedPayload,
    },
    #[serde(rename = "ORDER_CANCELLED")]
    OrderCancelled {
        payload: OrderCancelledPayload,
    },
    #[serde(rename = "OPEN_ORDERS")]
    OpenOrders {
        payload: Vec<OpenOrder>,
    },
}

#[derive(Serialize, Deserialize)]
pub struct DepthPayload {
    pub market: String,
    pub bids: Vec<[String; 2]>,
    pub asks: Vec<[String; 2]>,
}

#[derive(Serialize, Deserialize)]
pub struct Fill {
    pub price: String,
    pub qty: f64,
    pub trade_id: i64,
}

#[derive(Serialize, Deserialize)]
pub struct OrderPlacedPayload {
    pub order_id: String,
    pub executed_qty: f64,
    pub fills: Vec<Fill>,
}

#[derive(Serialize, Deserialize)]
pub struct OrderCancelledPayload {
    pub order_id: String,
    pub executed_qty: f64,
    pub remaining_qty: f64,
}

#[derive(Serialize, Deserialize)]
pub struct OpenOrder {
    pub order_id: String,
    pub executed_qty: f64,
    pub price: String,
    pub quantity: String,
    #[serde(rename = "side")]
    pub side: OrderSide,
    pub user_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessageToEngine {
    #[serde(rename = "CREATE_ORDER")]
    CreateOrder {
        data: CreateOrderData,
    },
    #[serde(rename = "CANCEL_ORDER")]
    CancelOrder {
        data: CancelOrderData,
    },
    #[serde(rename = "ON_RAMP")]
    OnRamp {
        data: OnRampData,
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

#[derive(Serialize, Deserialize)]
pub struct CreateOrderData {
    pub market: String,
    pub price: String,
    pub quantity: String,
    #[serde(rename = "side")]
    pub side: OrderSide,
    pub user_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct CancelOrderData {
    pub order_id: String,
    pub market: String,
}

#[derive(Serialize, Deserialize)]
pub struct OnRampData {
    pub amount: String,
    pub user_id: String,
    pub txn_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct GetDepthData {
    pub market: String,
}

#[derive(Serialize, Deserialize)]
pub struct GetOpenOrdersData {
    pub user_id: String,
    pub market: String,
}


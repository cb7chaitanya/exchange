use serde::{Deserialize, Serialize};
use redis::{Client, RedisResult, Commands};
use std::sync::Mutex;
use once_cell::sync::Lazy;
use serde_json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DbMessage {
    TradeAdded {
        id: String,
        is_buyer_maker: bool,
        price: String,
        quantity: String,
        quote_quantity: String,
        timestamp: i64,
        market: String,
    },
    OrderUpdate {
        order_id: String,
        executed_qty: f64,
        market: Option<String>,
        price: Option<String>,
        quantity: Option<String>,
        side: Option<OrderSide>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderSide {
    Buy,
    Sell,
}

static INSTANCE: Lazy<Mutex<RedisManager>> = Lazy::new(|| {
    Mutex::new(RedisManager::new())
});

pub struct RedisManager {
    redis_client: Client,
}

impl RedisManager {
    pub fn new() -> Self {
        let redis_client = Client::open("redis://localhost:6379").unwrap();
        Self {
            redis_client,
        }
    }

    pub fn get_instance() -> &'static Mutex<RedisManager> {
        &INSTANCE
    }

    pub fn push_message(&self, message: DbMessage) -> RedisResult<()> {
        let message_json = serde_json::to_string(&message).unwrap();
        let mut conn = self.redis_client.get_connection()?;
        conn.lpush("db_processor", message_json)
    }

    pub fn publish_message(&self, message: DbMessage) -> RedisResult<()> {
        let message_json = serde_json::to_string(&message).unwrap();
        let mut conn = self.redis_client.get_connection()?;
        conn.publish("db_processor", message_json)
    }

    pub fn send_to_api(&self, message: DbMessage) -> RedisResult<()> {
        let message_json = serde_json::to_string(&message).unwrap();
        let mut conn = self.redis_client.get_connection()?;
        conn.publish("api_processor", message_json)
    }
}

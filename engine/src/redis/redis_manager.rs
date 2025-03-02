use serde::{Deserialize, Serialize};
use redis::{Client, RedisResult, Commands};
use std::sync::Mutex;
use once_cell::sync::Lazy;
use serde_json;
use crate::types::ws::WsMessage;
use crate::types::api::MessageToApi;
use std::env;
use dotenv::dotenv;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DbMessage {
    TradeAdded(TradeMessage),
    OrderUpdate(OrderMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct TradeMessage {
    pub id: String,
    pub is_buyer_maker: bool,
    pub price: String,
    pub quantity: String,
    pub quote_quantity: String,
    pub timestamp: i64,
    pub market: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderMessage {
    pub order_id: String,
    pub executed_qty: f64,
    pub market: Option<String>,
    pub price: Option<String>,
    pub quantity: Option<String>,
    pub side: Option<OrderSide>,
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
        dotenv().ok(); // Load .env file if it exists
        
        let redis_url = env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string());
        
        let redis_client = Client::open(redis_url.as_str())
            .expect("Failed to create Redis client");
            
        println!("Connected to Redis at {}", redis_url);
        
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

    pub fn publish_message(&self, channel: &str, message: WsMessage) -> RedisResult<()> {
        let message_json = serde_json::to_string(&message).unwrap();
        let mut conn = self.redis_client.get_connection()?;
        conn.publish(channel, message_json)
    }

    pub fn send_to_api(&self, client_id: &str, message: MessageToApi) -> RedisResult<()> {
        let message_json = serde_json::to_string(&message).unwrap();
        let mut conn = self.redis_client.get_connection()?;
        conn.publish(client_id, message_json)
    }

    pub fn pop_message(&self) -> redis::RedisResult<Option<String>> {
        let mut conn = self.redis_client.get_connection()?;
        redis::cmd("RPOP").arg("messages").query(&mut conn)
    }
}

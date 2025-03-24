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
use log::info;

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
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Buy,
    Sell,
}

static INSTANCE: Lazy<Mutex<RedisManager>> = Lazy::new(|| {
    info!("Creating new RedisManager instance");
    Mutex::new(RedisManager::new())
});

#[derive(Debug)]
pub struct RedisManager {
    redis_client: Client,
}

impl RedisManager {
    pub fn new() -> Self {
        info!("Initializing new RedisManager");
        dotenv().ok();
        
        let redis_url = env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string());
        
        info!("Connecting to Redis at {}", redis_url);
        let redis_client = Client::open(redis_url.as_str())
            .expect("Failed to create Redis client");
            
        info!("Successfully created Redis client");
        
        Self {
            redis_client,
        }
    }

    pub fn get_instance() -> &'static Mutex<RedisManager> {
        info!("Getting Redis instance");
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
        info!("Attempting to send message to API for client: {}", client_id);
        let message_json = serde_json::to_string(&message).unwrap();
        info!("Serialized message: {}", message_json);
        let mut conn = match self.redis_client.get_connection() {
            Ok(conn) => {
                info!("Got Redis connection successfully");
                conn
            },
            Err(e) => {
                info!("Failed to get Redis connection: {:?}", e);
                return Err(e);
            }
        };
        match conn.publish(client_id, message_json) {
            Ok(result) => {
                info!("Successfully published to Redis, result: {:?}", result);
                Ok(result)
            },
            Err(e) => {
                info!("Failed to publish to Redis: {:?}", e);
                Err(e)
            }
        }
    }

    pub fn pop_message(&self) -> redis::RedisResult<Option<String>> {
        info!("Popping message from Redis queue 'messages'");
        let mut conn = self.redis_client.get_connection()?;
        info!("Connected to Redis");
        
        match redis::cmd("BRPOP").arg("messages").arg(1).query::<Option<(String, String)>>(&mut conn) {
            Ok(Some((_, message))) => {
                info!("Received message from Redis: {:?}", message);
                Ok(Some(message))
            }
            Ok(None) => {
                info!("No message available");
                Ok(None)
            }
            Err(e) => {
                info!("Error popping message: {:?}", e);
                Err(e)
            }
        }
    }
}

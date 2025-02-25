use std::sync::{Arc, Mutex};
use tokio;
use crate::redis::redis_manager::RedisManager;
use crate::trade::engine::Engine;
use crate::types::api::MessageFromApi;
use serde_json;

mod types;
mod redis;
mod trade;

#[tokio::main]
async fn main() {
    let engine = Arc::new(Mutex::new(Engine::new()));
    let redis_client = RedisManager::get_instance();
    println!("connected to redis");

    loop {
        if let Ok(response) = redis_client.lock().unwrap().pop_message() {
            if let Some(message_str) = response {
                if let Ok(message) = serde_json::from_str::<MessageFromApi>(&message_str) {
                    let mut engine = engine.lock().unwrap();
                    engine.process(message, "default_user"); // TODO: Get user_id from message
                }
            }
        }
    }
}

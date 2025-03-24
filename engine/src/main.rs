use std::sync::{Arc, Mutex};
use tokio;
use crate::redis::redis_manager::RedisManager;
use crate::trade::engine::Engine;
use crate::types::api::{MessageFromApi, MessageToApi};
use serde_json;
use log::info;
use env_logger;
use dotenv::dotenv;
use serde::Deserialize;

mod types;
mod redis;
mod trade;

#[derive(Debug, Deserialize)]
struct MessageWrapper {
    client_id: String,
    user_id: String,
    message: MessageFromApi,
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init();
    info!("Starting engine...");
    
    let engine = Arc::new(Mutex::new(Engine::new()));
    
    info!("Engine initialized, waiting for messages...");

    loop {
        // Get Redis instance and release lock after getting message
        let response = {
            match RedisManager::get_instance().lock() {
                Ok(redis) => redis.pop_message(),
                Err(e) => {
                    info!("Failed to get Redis lock: {:?}", e);
                    continue;
                }
            }
        };

        if let Ok(response) = response {
            info!("Received response from Redis: {:?}", response);
            if let Some(message_str) = response {
                match serde_json::from_str::<MessageWrapper>(&message_str) {
                    Ok(wrapper) => {
                        info!("Processing message: {:?}", wrapper.message);
                        let mut engine = engine.lock().unwrap();
                        engine.process(wrapper.message, wrapper.client_id, wrapper.user_id);
                    }
                    Err(e) => {
                        info!("Failed to parse message: {:?}", e);
                        if let Ok(raw) = serde_json::from_str::<serde_json::Value>(&message_str) {
                            if let Some(client_id) = raw.get("client_id").and_then(|v| v.as_str()) {
                                if let Ok(redis) = RedisManager::get_instance().lock() {
                                    if let Err(e) = redis.send_to_api(
                                        client_id,
                                        MessageToApi::Error {
                                            message: e.to_string(),
                                        }
                                    ) {
                                        info!("Failed to send error message: {:?}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

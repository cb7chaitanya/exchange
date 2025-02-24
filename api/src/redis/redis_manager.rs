use redis::{Client, RedisResult, Commands};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use crate::types::redis::{MessageFromOrderbook, MessageToEngine};
use serde::Serialize;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

#[derive(Serialize)]
struct MessageWithId {
    client_id: String,
    message: MessageToEngine,
}

static INSTANCE: Lazy<Mutex<RedisManager>> = Lazy::new(|| {
    Mutex::new(RedisManager::new())
});

pub struct RedisManager {
    client: Client,
    publisher: Client,
}

impl RedisManager {
    fn new() -> Self {
        let client = Client::open("redis://localhost:6379").unwrap();
        let publisher = Client::open("redis://localhost:6379").unwrap();
        RedisManager { client, publisher }
    }

    pub fn get_instance() -> &'static Mutex<RedisManager> {
        &INSTANCE
    }

    fn get_random_client_id(&self) -> String {
        thread_rng()
            .sample_iter(&Alphanumeric)
            .take(26) 
            .map(char::from)
            .collect()
    }

    pub async fn send_and_await(&self, message: MessageToEngine) -> RedisResult<MessageFromOrderbook> {
        let mut conn = self.client.get_connection()?;
        let mut pub_conn = self.publisher.get_connection()?;
        
        let client_id = self.get_random_client_id();
        
        let message_with_id = MessageWithId {
            client_id: client_id.clone(),
            message,
        };

        let mut pubsub = conn.as_pubsub();
        pubsub.subscribe(&client_id)?;
        
        let _: () = pub_conn.lpush("messages", serde_json::to_string(&message_with_id).expect("Failed to serialize message"))?;
        
        let msg = pubsub.get_message()?;
        let payload: String = msg.get_payload()?;
        
        pubsub.unsubscribe(&client_id)?;
        
        Ok(serde_json::from_str(&payload).expect("Failed to deserialize message"))
    }
}

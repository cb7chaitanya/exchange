use redis::{Client, RedisResult, Commands};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use crate::types::redis::{MessageFromOrderbook, MessageToEngine};
use serde::Serialize;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use log::info;

#[derive(Serialize, Debug)]
struct MessageWithId<'a> {
    client_id: &'a String,
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
        info!("Sending message to engine: {:?}", message);
        let mut conn = self.client.get_connection()?;
        let mut pub_conn = self.publisher.get_connection()?;
        info!("Connected to Redis");
        let client_id = self.get_random_client_id();
        info!("Client ID: {:?}", client_id);

        let mut pubsub = conn.as_pubsub();
        info!("Created PubSub");
        pubsub.subscribe(&client_id)?;
        info!("Subscribed to channel: {}", client_id);

        let message_with_id = MessageWithId {
            client_id: &client_id,
            message,
        };
        info!("Message with ID: {:?}", message_with_id);
        let _: () = pub_conn.lpush("messages", serde_json::to_string(&message_with_id).expect("Failed to serialize message"))?;
        info!("Pushed message to Redis");
        info!("Waiting for response on channel: {}", client_id);
        let msg = pubsub.get_message()?;
        info!("Received message from Redis: {:?}", msg);
        let payload: String = msg.get_payload()?;
        info!("Received message from Redis: {:?}", payload);
        pubsub.unsubscribe(&client_id)?;
        info!("Unsubscribed from client ID: {:?}", client_id);
        Ok(serde_json::from_str(&payload).expect("Failed to deserialize message"))
    }
}

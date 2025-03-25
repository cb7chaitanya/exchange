use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;
use redis::Client;
use serde_json::Value;
use log::info;

pub struct SubscriptionManager {
    subscriptions: HashMap<String, Vec<String>>,         // userId -> [subscriptions]
    reverse_subscriptions: HashMap<String, Vec<String>>, // subscription -> [userIds]
    redis_client: Client,
}

static INSTANCE: Lazy<Arc<Mutex<SubscriptionManager>>> = Lazy::new(|| {
    Arc::new(Mutex::new(SubscriptionManager::new()))
});

impl SubscriptionManager {
    fn new() -> Self {
        let redis_url = std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string());
            
        let redis_client = Client::open(redis_url)
            .expect("Failed to create Redis client");

        Self {
            subscriptions: HashMap::new(),
            reverse_subscriptions: HashMap::new(),
            redis_client,
        }
    }

    pub fn get_instance() -> Arc<Mutex<SubscriptionManager>> {
        Arc::clone(&INSTANCE)
    }

    pub fn subscribe(&mut self, user_id: &str, subscription: String) {
        info!("Subscribing to {}", subscription);
        // Check if already subscribed
        if let Some(subs) = self.subscriptions.get(user_id) {
            if subs.contains(&subscription) {
                return;
            }
        }

        // Update user's subscriptions
        self.subscriptions
            .entry(user_id.to_string())
            .or_insert_with(Vec::new)
            .push(subscription.clone());

        // Update reverse subscriptions
        self.reverse_subscriptions
            .entry(subscription.clone())
            .or_insert_with(Vec::new)
            .push(user_id.to_string());

        // Subscribe to Redis channel if this is the first subscriber
        if self.reverse_subscriptions.get(&subscription).map_or(0, |v| v.len()) == 1 {
            let mut conn = self.redis_client.get_connection()
                .expect("Failed to get Redis connection");
            
            let _ = redis::cmd("SUBSCRIBE")
                .arg(&subscription)
                .query::<()>(&mut conn);
        }

        info!("Subscribed to {}", subscription);
    }

    pub fn unsubscribe(&mut self, user_id: &str, subscription: &str) {
        // Remove from user's subscriptions
        if let Some(subs) = self.subscriptions.get_mut(user_id) {
            subs.retain(|s| s != subscription);
        }

        // Remove from reverse subscriptions
        if let Some(users) = self.reverse_subscriptions.get_mut(subscription) {
            users.retain(|u| u != user_id);

            // If no users left, unsubscribe from Redis
            if users.is_empty() {
                self.reverse_subscriptions.remove(subscription);
                let mut conn = self.redis_client.get_connection()
                    .expect("Failed to get Redis connection");
                
                let _ = redis::cmd("UNSUBSCRIBE")
                    .arg(subscription)
                    .query::<()>(&mut conn);
            }
        }
    }

    pub fn user_left(&mut self, user_id: &str) {
        println!("user left {}", user_id);
        if let Some(subs) = self.subscriptions.get(user_id).cloned() {
            for subscription in subs {
                self.unsubscribe(user_id, &subscription);
            }
        }
        self.subscriptions.remove(user_id);
    }

    pub fn get_subscriptions(&self, user_id: &str) -> Vec<String> {
        self.subscriptions
            .get(user_id)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn handle_redis_message(&self, channel: &str, message: String) {
        if let Ok(parsed_message) = serde_json::from_str::<Value>(&message) {
            if let Some(users) = self.reverse_subscriptions.get(channel) {
                for user_id in users {
                    // TODO: Emit to user through UserManager
                    println!("Emitting to user {}: {}", user_id, message);
                }
            }
        }
    }
}
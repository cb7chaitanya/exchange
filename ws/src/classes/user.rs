use tokio::sync::mpsc::{Sender, Receiver};
use warp::ws::{Message, WebSocket};
use serde_json::{json, Value};
use crate::classes::subscription_manager::SubscriptionManager;

pub struct User {
    pub id: String,
    pub ws: Sender<Message>,
    subscriptions: Vec<String>,
}

impl User {
    pub fn new(id: String, ws: Sender<Message>) -> Self {
        Self {
            id,
            ws,
            subscriptions: Vec::new(),
        }
    }

    pub fn subscribe(&mut self, subscription: String) {
        self.subscriptions.push(subscription);
    }

    pub fn unsubscribe(&mut self, subscription: &str) {
        self.subscriptions.retain(|s| s != subscription);
    }

    pub async fn emit(&self, message: Value) {
        if let Ok(message_str) = serde_json::to_string(&message) {
            if let Err(e) = self.ws.send(Message::text(message_str)).await {
                println!("Error sending message: {}", e);
            }
        }
    }

    pub async fn handle_message(&mut self, message: String) {
        if let Ok(parsed) = serde_json::from_str::<Value>(&message) {
            match parsed["method"].as_str() {
                Some("SUBSCRIBE") => {
                    if let Some(params) = parsed["params"].as_array() {
                        for param in params {
                            if let Some(subscription) = param.as_str() {
                                if let Ok(mut manager) = SubscriptionManager::get_instance().lock() {
                                    manager.subscribe(&self.id, subscription.to_string());
                                }
                            }
                        }
                    }
                }
                Some("UNSUBSCRIBE") => {
                    if let Some(params) = parsed["params"].as_array() {
                        for param in params {
                            if let Some(subscription) = param.as_str() {
                                if let Ok(mut manager) = SubscriptionManager::get_instance().lock() {
                                    manager.unsubscribe(&self.id, subscription);
                                }
                            }
                        }
                    }
                }
                _ => println!("Unknown method"),
            }
        }
    }
}
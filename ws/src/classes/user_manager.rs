use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc::channel;
use once_cell::sync::Lazy;
use warp::ws::{Message, WebSocket};
use rand::{thread_rng, Rng, distributions::Alphanumeric};
use crate::classes::{user::User, subscription_manager::SubscriptionManager};

pub struct UserManager {
    users: HashMap<String, User>,
}

static INSTANCE: Lazy<Arc<Mutex<UserManager>>> = Lazy::new(|| {
    Arc::new(Mutex::new(UserManager::new()))
});

impl UserManager {
    fn new() -> Self {
        Self {
            users: HashMap::new(),
        }
    }

    pub fn get_instance() -> Arc<Mutex<UserManager>> {
        Arc::clone(&INSTANCE)
    }

    pub async fn add_user(&mut self, ws: WebSocket) -> String {
        let id = self.get_random_id();
        let (tx, _rx) = channel(32);
        let user = User::new(id.clone(), tx);
        self.users.insert(id.clone(), user);
        
        // Handle WebSocket closure
        let id_clone = id.clone();
        tokio::spawn(async move {
            if let Ok(mut manager) = SubscriptionManager::get_instance().lock() {
                manager.user_left(&id_clone);
            }
        });

        id
    }

    pub fn get_user(&self, id: &str) -> Option<&User> {
        self.users.get(id)
    }

    fn get_random_id(&self) -> String {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect()
    }

    pub fn remove_user(&mut self, id: &str) {
        self.users.remove(id);
        if let Ok(mut manager) = SubscriptionManager::get_instance().lock() {
            manager.user_left(id);
        }
    }
}
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc::channel;
use once_cell::sync::Lazy;
use warp::ws::WebSocket;
use futures_util::{StreamExt, SinkExt};
use rand::{Rng, distributions::Alphanumeric};
use crate::classes::{user::User, subscription_manager::SubscriptionManager};
use log::info;

pub struct UserManager {
    users: Mutex<HashMap<String, User>>,
}

static INSTANCE: Lazy<Arc<Mutex<UserManager>>> = Lazy::new(|| {
    Arc::new(Mutex::new(UserManager::new()))
});

impl UserManager {
    fn new() -> Self {
        Self {
            users: Mutex::new(HashMap::new()),
        }
    }

    pub fn get_instance() -> Arc<Mutex<UserManager>> {
        Arc::clone(&INSTANCE)
    }

    pub async fn add_user(&mut self, ws: WebSocket) -> String {
        let id = self.get_random_id();
        let (tx, mut rx) = channel(32);
        let user = User::new(id.clone(), tx);
        self.users.lock().await.insert(id.clone(), user);
        
        let (mut ws_tx, mut ws_rx) = ws.split();
        
        // Handle incoming messages
        let id_clone = id.clone();
        tokio::spawn(async move {
            while let Some(result) = ws_rx.next().await {
                match result {
                    Ok(msg) => {
                        if let Ok(text) = msg.to_str() {
                            if let Some(user) = UserManager::get_instance()
                                .lock()
                                .await
                                .users
                                .lock()
                                .await
                                .get_mut(&id_clone) 
                            {
                                user.handle_message(text.to_string()).await;
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Handle outgoing messages
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if let Err(_) = ws_tx.send(message).await {
                    break;
                }
            }
        });

        id
    }

    pub async fn get_user<'a>(&'a self, id: &str) -> Option<tokio::sync::MappedMutexGuard<'a, User>> {
        let users = self.users.lock().await;
        if let Some(user) = users.get(id) {
            Some(tokio::sync::MutexGuard::map(users, |users| users.get_mut(id).unwrap()))
        } else {
            None
        }
    }

    fn get_random_id(&self) -> String {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect()
    }

    pub async fn remove_user(&mut self, id: &str) {
        info!("Removing user: {}", id);
        if let Some(user) = self.users.lock().await.remove(id) {
            // First unsubscribe from all channels
            if let Ok(mut sub_manager) = SubscriptionManager::get_instance().lock() {
                for channel in user.get_subscriptions() {
                    sub_manager.unsubscribe(id, &channel);
                }
            }
            
            // Then notify about user leaving
            if let Ok(mut sub_manager) = SubscriptionManager::get_instance().lock() {
                sub_manager.user_left(id);
            }
            
            info!("User {} removed successfully", id);
        }
    }
}
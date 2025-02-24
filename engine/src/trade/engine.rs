use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::redis::redis_manager::OrderSide;
use crate::trade::orderbook::Orderbook;
pub const BASE_CURRENCY: &str = "INR";


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    price: f64,
    quantity: f64,
    order_id: String,
    filled: f64,
    side: OrderSide,
    user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBalance {
    available: f64,
    locked: f64,
}

pub struct Engine {
    pub markets: Vec<Orderbook>,
    balances: HashMap<String, HashMap<String, UserBalance>>,
}

impl Engine {
    pub fn new() -> Self {
        let markets = vec![Orderbook::new("TATA".to_string())];

        let mut engine = Self {
            markets,
            balances: HashMap::new(),
        };
        
        engine.set_base_balances();
        engine
    }

    fn set_base_balances(&mut self) {
        for user_id in ["1", "2", "5"].iter() {
            let mut user_balance = HashMap::new();
            user_balance.insert(BASE_CURRENCY.to_string(), UserBalance {
                available: 10_000_000.0,
                locked: 0.0,
            });
            user_balance.insert("TATA".to_string(), UserBalance {
                available: 10_000_000.0,
                locked: 0.0,
            });
            self.balances.insert(user_id.to_string(), user_balance);
        }
    }
}


use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::trade::orderbook::{Orderbook, Order, Fill};
use std::fs;
use std::env;
use serde_json;
use crate::types::api::{MessageFromApi, MessageToApi};
use crate::redis::redis_manager::RedisManager;
use crate::redis::redis_manager::{DbMessage, OrderMessage, TradeMessage, OrderSide};
use rand::{thread_rng, Rng, distributions::Alphanumeric};
use crate::types::ws::{WsMessage, WsMessageData, TradeData, DepthData};

pub const BASE_CURRENCY: &str = "INR";


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBalance {
    available: f64,
    locked: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Engine {
    pub orderbooks: Vec<Orderbook>,
    balances: HashMap<String, HashMap<String, UserBalance>>,
}

impl Engine {
    pub fn new() -> Self {
        let mut snapshot: Option<String> = None;
        
        if env::var("WITH_SNAPSHOT").is_ok() {
            match fs::read_to_string("./snapshot.json") {
                Ok(contents) => snapshot = Some(contents),
                Err(_) => println!("No snapshot found"),
            }
        }

        let mut engine = if let Some(snapshot_str) = snapshot {
            match serde_json::from_str::<serde_json::Value>(&snapshot_str) {
                Ok(snapshot_data) => {
                    let orderbooks = snapshot_data["orderbooks"]
                        .as_array()
                        .unwrap_or(&Vec::new())
                        .iter()
                        .map(|o| Orderbook::new(
                            o["baseAsset"].as_str().unwrap_or("TATA").to_string(),
                        ))
                        .collect();

                    Self {
                        orderbooks,
                        balances: HashMap::new(),
                    }
                }
                Err(_) => Self {
                    orderbooks: vec![Orderbook::new("TATA".to_string())],
                    balances: HashMap::new(),
                }
            }
        } else {
            Self {
                orderbooks: vec![Orderbook::new("TATA".to_string())],
                balances: HashMap::new(),
            }
        };

        engine.set_base_balances();
        engine
    }

    #[allow(dead_code)]
    pub fn save_snapshot(&self) {
        let snapshot = serde_json::json!({
            "orderbooks": self.orderbooks.iter().map(|o| o.get_snapshot()).collect::<Vec<_>>(),
            "balances": self.balances.clone(),
        });
        
        fs::write("./snapshot.json", serde_json::to_string_pretty(&snapshot).unwrap()).unwrap();
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

    pub fn process(&mut self, message: MessageFromApi) {
        match message {
            MessageFromApi::CreateOrder { market, price, quantity, side, user_id } => {
                match self.create_order(&market, &price, &quantity, side, &user_id) {
                    Ok((executed_qty, fills, order_id)) => {
                        RedisManager::get_instance().lock().unwrap().send_to_api(
                            &user_id,
                            MessageToApi::OrderPlaced {
                                order_id,
                                executed_qty,
                                fills,
                            }
                        ).unwrap();
                    }
                    Err(e) => {
                        println!("{}", e);
                        RedisManager::get_instance().lock().unwrap().send_to_api(
                            &user_id,
                            MessageToApi::OrderCancelled {
                                order_id: String::new(),
                                executed_qty: 0.0,
                                remaining_qty: 0.0,
                            }
                        ).unwrap();
                    }
                }
            }

            MessageFromApi::CancelOrder { order_id, market, .. } => {
                if let Some(orderbook) = self.orderbooks.iter_mut().find(|o| o.ticker() == market) {
                    match orderbook.get_open_orders(&order_id).first() {
                        Some(order) => {
                            let quote_asset = market.split('_').nth(1).unwrap_or(BASE_CURRENCY);
                            if order.side == OrderSide::Buy {
                                if let Ok(price) = orderbook.cancel_bid(&order_id) {
                                    let left_quantity = (order.quantity - order.filled) * order.price;
                                    if let Some(balance) = self.balances.get_mut(&order.user_id) {
                                        if let Some(asset_balance) = balance.get_mut(BASE_CURRENCY) {
                                            asset_balance.available += left_quantity;
                                            asset_balance.locked -= left_quantity;
                                        }
                                    }
                                    self.send_updated_depth_at(price, &market);
                                }
                            } else {
                                if let Ok(price) = orderbook.cancel_ask(&order_id) {
                                    let left_quantity = order.quantity - order.filled;
                                    if let Some(balance) = self.balances.get_mut(&order.user_id) {
                                        if let Some(asset_balance) = balance.get_mut(quote_asset) {
                                            asset_balance.available += left_quantity;
                                            asset_balance.locked -= left_quantity;
                                        }
                                    }
                                    self.send_updated_depth_at(price, &market);
                                }
                            }
                        }
                        None => println!("No order found"),
                    }
                } else {
                    println!("No orderbook found");
                }
            }

            MessageFromApi::GetOpenOrders { market, user_id } => {
                if let Some(orderbook) = self.orderbooks.iter().find(|o| o.ticker() == market) {
                    let orders = orderbook.get_open_orders(&user_id);
                    RedisManager::get_instance().lock().unwrap().send_to_api(
                        &user_id,
                        MessageToApi::OpenOrders { orders }
                    ).unwrap();
                }
            }

            MessageFromApi::OnRamp { amount, user_id, .. } => {
                let amount = amount.parse::<f64>().unwrap_or(0.0);
                self.on_ramp(&user_id, amount);
            }

            MessageFromApi::GetDepth { market, user_id } => {
                if let Some(orderbook) = self.orderbooks.iter().find(|o| o.ticker() == market) {
                    let depth = orderbook.get_depth();
                    RedisManager::get_instance().lock().unwrap().send_to_api(
                        &user_id,
                        MessageToApi::Depth {
                            bids: depth.bids,
                            asks: depth.asks,
                        }
                    ).unwrap();
                } else {
                    RedisManager::get_instance().lock().unwrap().send_to_api(
                        &user_id,
                        MessageToApi::Depth {
                            bids: Vec::new(),
                            asks: Vec::new(),
                        }
                    ).unwrap();
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn add_orderbook(&mut self, orderbook: Orderbook) {
        self.orderbooks.push(orderbook);
    }

    pub fn create_order(
        &mut self,
        market: &str,
        price: &str,
        quantity: &str,
        side: OrderSide,
        user_id: &str,
    ) -> Result<(f64, Vec<Fill>, String), String> {
        // First validate and parse inputs
        let price_val = price.parse::<f64>().map_err(|_| "Invalid price")?;
        let quantity_val = quantity.parse::<f64>().map_err(|_| "Invalid quantity")?;
        let base_asset = market.split('_').next().unwrap_or("TATA");
        let quote_asset = market.split('_').nth(1).unwrap_or(BASE_CURRENCY);

        self.check_and_lock_funds(
            base_asset,
            quote_asset,
            &side,
            user_id,
            price,
            quantity,
        )?;

        // Generate order ID
        let order_id: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(26) 
            .map(char::from)
            .collect();

        // Create and process order
        let mut order = Order {
            price: price_val,
            quantity: quantity_val,
            order_id: order_id.clone(),
            filled: 0.0,
            side: side.clone(),
            user_id: user_id.to_string(),
        };

        let orderbook = self.orderbooks
            .iter_mut()
            .find(|o| o.ticker() == market)
            .ok_or("No orderbook found")?;

        let (fills, executed_qty) = orderbook.add_order(&mut order)?;

        self.update_balance(user_id, base_asset, quote_asset, &side, &fills, executed_qty)?;
        self.create_db_trades(&fills, market, user_id);
        self.update_db_orders(&order, executed_qty, &fills, market);
        self.publish_ws_depth_updates(&fills, price.to_string(), &side, market);
        self.publish_ws_trades(&fills, user_id, market);

        Ok((executed_qty, fills, order_id))
    }

    fn check_and_lock_funds(
        &mut self,
        base_asset: &str,
        quote_asset: &str,
        side: &OrderSide,
        user_id: &str,
        price: &str,
        quantity: &str,
    ) -> Result<(), String> {
        let price = price.parse::<f64>().map_err(|_| "Invalid price")?;
        let quantity = quantity.parse::<f64>().map_err(|_| "Invalid quantity")?;

        let balances = self.balances.get_mut(user_id).ok_or("User not found")?;

        match side {
            OrderSide::Buy => {
                let asset_balance = balances.get_mut(quote_asset).ok_or("Asset not found")?;
                let required_amount = price * quantity;
                if asset_balance.available < required_amount {
                    return Err("Insufficient funds".to_string());
                }
                asset_balance.available -= required_amount;
                asset_balance.locked += required_amount;
            }
            OrderSide::Sell => {
                let asset_balance = balances.get_mut(base_asset).ok_or("Asset not found")?;
                if asset_balance.available < quantity {
                    return Err("Insufficient funds".to_string());
                }
                asset_balance.available -= quantity;
                asset_balance.locked += quantity;
            }
        }
        Ok(())
    }

    fn update_db_orders(&mut self, order: &Order, executed_qty: f64, fills: &Vec<Fill>, market: &str) {
        let conn = RedisManager::get_instance().lock().unwrap();
        let message = DbMessage::OrderUpdate(OrderMessage {
            order_id: order.order_id.clone(),
            executed_qty,
            market: Some(market.to_string()),
            price: Some(order.price.to_string()),
            quantity: Some(order.quantity.to_string()),
            side: Some(order.side.clone()),
        });

        if let Err(e) = conn.push_message(message) {
            println!("Failed to push order update: {}", e);
        }

        for fill in fills {
            if let Err(e) = conn.push_message(DbMessage::OrderUpdate(OrderMessage {
                order_id: fill.marker_order_id.clone(),
                executed_qty: fill.qty,
                market: None,
                price: None,
                quantity: None,
                side: None,
            })) {
                println!("Failed to push order update: {}", e);
            }
        }
    }

    fn create_db_trades(&mut self, fills: &Vec<Fill>, market: &str, user_id: &str) {
        for fill in fills {
            let conn = RedisManager::get_instance().lock().unwrap();
            let quote_qty = fill.qty * fill.price;
            let message = DbMessage::TradeAdded(TradeMessage {
                id: fill.trade_id.to_string(),
                is_buyer_maker: fill.other_user_id == user_id,
                price: fill.price.to_string(),
                quantity: fill.qty.to_string(),
                quote_quantity: quote_qty.to_string(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64,
                market: market.to_string(),
            });

            if let Err(e) = conn.push_message(message) {
                println!("Failed to push trade update: {}", e);
            }
        }
    }

    fn publish_ws_trades(&mut self, fills: &Vec<Fill>, user_id: &str, market: &str) {
        let conn = RedisManager::get_instance().lock().unwrap();
        for fill in fills {
            let message = WsMessage {
                stream: format!("trade@{}", market),
                data: WsMessageData::Trade(TradeData {
                    e: "trade".to_string(),
                    t: fill.trade_id,
                    m: fill.other_user_id == user_id,
                    p: fill.price.to_string(),
                    q: fill.qty.to_string(),
                    s: market.to_string(),
                }),
            };

            if let Err(e) = conn.publish_message(&format!("trade@{}", market), message) {
                println!("Failed to publish trade update: {}", e);
            }
        }
    }

    fn publish_ws_depth_updates(&mut self, fills: &Vec<Fill>, price: String, side: &OrderSide, market: &str) {
        let orderbook = match self.orderbooks.iter().find(|o| o.ticker() == market) {
            Some(ob) => ob,
            None => return,
        };

        let depth = orderbook.get_depth();
        let fill_prices: Vec<String> = fills.iter().map(|f| f.price.to_string()).collect();

        let message = match side {
            OrderSide::Buy => {
                let updated_asks: Vec<[String; 2]> = depth.asks
                    .into_iter()
                    .filter(|(p, _)| fill_prices.contains(p))
                    .map(|(p, q)| [p, q])
                    .collect();

                let updated_bid: Vec<[String; 2]> = depth.bids
                    .into_iter()
                    .filter(|(p, _)| p == &price)
                    .map(|(p, q)| [p, q])
                    .take(1)
                    .collect();

                WsMessage {
                    stream: format!("depth@{}", market),
                    data: WsMessageData::Depth(DepthData {
                        e: "depth".to_string(),
                        a: Some(updated_asks),
                        b: Some(updated_bid),
                    }),
                }
            },
            OrderSide::Sell => {
                let updated_bids: Vec<[String; 2]> = depth.bids
                    .into_iter()
                    .filter(|(p, _)| fill_prices.contains(p))
                    .map(|(p, q)| [p, q])
                    .collect();

                let updated_ask: Vec<[String; 2]> = depth.asks
                    .into_iter()
                    .filter(|(p, _)| p == &price)
                    .map(|(p, q)| [p, q])
                    .take(1)
                    .collect();

                WsMessage {
                    stream: format!("depth@{}", market),
                    data: WsMessageData::Depth(DepthData {
                        e: "depth".to_string(),
                        a: Some(updated_ask),
                        b: Some(updated_bids),
                    }),
                }
            }
        };

        println!("publish ws depth updates");
        let conn = RedisManager::get_instance().lock().unwrap();
        if let Err(e) = conn.publish_message(&format!("depth@{}", market), message) {
            println!("Failed to publish depth update: {}", e);
        }
    }
    
    #[allow(dead_code)]
    fn send_updated_depth_at(&mut self, price: f64, market: &str) {
        let orderbook = match self.orderbooks.iter().find(|o| o.ticker() == market) {
            Some(ob) => ob,
            None => return,
        };

        let price_str = price.to_string();
        let depth = orderbook.get_depth();
        let updated_bids: Vec<[String; 2]> = depth.bids
            .iter()
            .filter(|(p, _)| *p == price_str)
            .map(|(p, q)| [p.clone(), q.clone()])
            .collect();
        let updated_asks: Vec<[String; 2]> = depth.asks
            .iter()
            .filter(|(p, _)| *p == price_str)
            .map(|(p, q)| [p.clone(), q.clone()])
            .collect();

        let message = WsMessage {
            stream: format!("depth@{}", market),
            data: WsMessageData::Depth(DepthData { 
                e: "depth".to_string(), 
                a: Some(updated_asks), 
                b: Some(updated_bids) 
            }),
        };

        let conn = RedisManager::get_instance().lock().unwrap();
        if let Err(e) = conn.publish_message(&format!("depth@{}", market), message) {
            println!("Failed to publish depth update: {}", e);
        }
    }

    fn update_balance(
        &mut self,
        user_id: &str,
        base_asset: &str,
        quote_asset: &str,
        side: &OrderSide,
        fills: &Vec<Fill>,
        _executed_qty: f64,
    ) -> Result<(), String> {
        match side {
            OrderSide::Buy => {
                for fill in fills {
                    // Update quote asset balance for other user
                    if let Some(other_balance) = self.balances.get_mut(&fill.other_user_id) {
                        if let Some(quote_balance) = other_balance.get_mut(quote_asset) {
                            quote_balance.available += fill.qty * fill.price;
                        }
                    }

                    // Update quote asset balance for buyer
                    if let Some(user_balance) = self.balances.get_mut(user_id) {
                        if let Some(quote_balance) = user_balance.get_mut(quote_asset) {
                            quote_balance.locked -= fill.qty * fill.price;
                        }
                    }

                    // Update base asset balance for other user
                    if let Some(other_balance) = self.balances.get_mut(&fill.other_user_id) {
                        if let Some(base_balance) = other_balance.get_mut(base_asset) {
                            base_balance.locked -= fill.qty;
                        }
                    }

                    // Update base asset balance for buyer
                    if let Some(user_balance) = self.balances.get_mut(user_id) {
                        if let Some(base_balance) = user_balance.get_mut(base_asset) {
                            base_balance.available += fill.qty;
                        }
                    }
                }
            },
            OrderSide::Sell => {
                for fill in fills {
                    // Update quote asset balance for other user
                    if let Some(other_balance) = self.balances.get_mut(&fill.other_user_id) {
                        if let Some(quote_balance) = other_balance.get_mut(quote_asset) {
                            quote_balance.locked -= fill.qty * fill.price;
                        }
                    }

                    // Update quote asset balance for seller
                    if let Some(user_balance) = self.balances.get_mut(user_id) {
                        if let Some(quote_balance) = user_balance.get_mut(quote_asset) {
                            quote_balance.available += fill.qty * fill.price;
                        }
                    }

                    // Update base asset balance for other user
                    if let Some(other_balance) = self.balances.get_mut(&fill.other_user_id) {
                        if let Some(base_balance) = other_balance.get_mut(base_asset) {
                            base_balance.available += fill.qty;
                        }
                    }

                    // Update base asset balance for seller
                    if let Some(user_balance) = self.balances.get_mut(user_id) {
                        if let Some(base_balance) = user_balance.get_mut(base_asset) {
                            base_balance.locked -= fill.qty;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn on_ramp(&mut self, user_id: &str, amount: f64) {
        if let Some(user_balance) = self.balances.get_mut(user_id) {
            if let Some(base_balance) = user_balance.get_mut(BASE_CURRENCY) {
                base_balance.available += amount;
            } else {
                user_balance.insert(BASE_CURRENCY.to_string(), UserBalance {
                    available: amount,
                    locked: 0.0,
                });
            }
        } else {
            let mut new_balance = HashMap::new();
            new_balance.insert(BASE_CURRENCY.to_string(), UserBalance {
                available: amount,
                locked: 0.0,
            });
            self.balances.insert(user_id.to_string(), new_balance);
        }
    }
}


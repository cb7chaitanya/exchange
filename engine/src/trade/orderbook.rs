use serde::{Deserialize, Serialize};
use crate::redis::redis_manager::OrderSide;
use log::info;
use std::collections::{BTreeMap, HashMap};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Price(f64);

impl Eq for Price {}

impl PartialOrd for Price {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for Price {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    pub qty: f64,
    pub price: f64,
    pub trade_id: i64,
    pub marker_order_id: String,
    pub other_user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Orderbook {
    market: String,
    pub bids: BTreeMap<Price, Vec<Order>>,
    pub asks: BTreeMap<Price, Vec<Order>>,
    last_trade_id: i64,
    current_price: f64,
    orders: HashMap<String, Order>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub price: f64,
    pub quantity: f64,
    pub order_id: String,
    pub filled: f64,
    pub side: OrderSide,
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderbookSnapshot {
    pub bids: Vec<(String, String)>,  // (price, quantity)
    pub asks: Vec<(String, String)>,  // (price, quantity)
}

impl Orderbook {
    pub fn new(market: String) -> Self {
        info!("Creating new orderbook for market: {}", market);
        Self {
            market,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            orders: HashMap::new(),
            last_trade_id: 0,
            current_price: 0.0,
        }
    }

    pub fn ticker(&self) -> &str {
        &self.market  // Return reference to market string
    }

    #[allow(dead_code)]
    pub fn get_snapshot(&self) -> Orderbook {
        let snapshot = Orderbook {
            market: self.market.clone(),
            bids: self.bids.clone(),
            asks: self.asks.clone(),
            orders: self.orders.clone(),
            last_trade_id: self.last_trade_id,
            current_price: self.current_price,
        };

        snapshot
    }

    pub fn add_order(&mut self, order: &mut Order) -> Result<(Vec<Fill>, f64), String> {
        if order.side == OrderSide::Buy {
            let (fills, executed_qty) = self.match_bid(order).expect("Error matching bid");
            order.filled = executed_qty;
            if executed_qty == order.quantity {
                Ok((fills, executed_qty))
            } else {
                self.bids.insert(Price(order.price), vec![order.clone()]);
                Ok((fills, executed_qty))
            }
        } else {
            let (fills, executed_qty) = self.match_ask(order).expect("Error matching ask");
            order.filled = executed_qty;
            if executed_qty == order.quantity {
                Ok((fills, executed_qty))
            } else {
                self.asks.insert(Price(order.price), vec![order.clone()]);
                Ok((fills, executed_qty))
            }
        }
    }

    pub fn get_depth(&self) -> OrderbookSnapshot {
        let mut bids: Vec<(String, String)> = Vec::new();
        let mut asks: Vec<(String, String)> = Vec::new();
        info!("Getting depth for market: {:?}", self.market);
        // Aggregate bids at same price level
        for (price, orders) in &self.bids {
            let remaining = orders.iter().map(|o| o.quantity - o.filled).sum::<f64>();
            if remaining > 0.0 {
                bids.push((price.0.to_string(), remaining.to_string()));
            }
        }
        info!("Bids: {:?}", bids);
        // Aggregate asks at same price level
        for (price, orders) in &self.asks {
            let remaining = orders.iter().map(|o| o.quantity - o.filled).sum::<f64>();
            if remaining > 0.0 {
                asks.push((price.0.to_string(), remaining.to_string()));
            }
        }
        info!("Asks: {:?}", asks);
        // Sort bids in descending order (highest price first)
        bids.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        // Sort asks in ascending order (lowest price first)
        asks.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        info!("Sorted bids: {:?}", bids);
        info!("Sorted asks: {:?}", asks); 
        OrderbookSnapshot { bids, asks }
    }

    pub fn get_open_orders(&self, user_id: &str) -> Vec<Order> {
        let mut orders = Vec::new();
        orders.extend(
            self.bids.values()
                .flat_map(|bids| bids.iter().filter(|o| o.user_id == user_id && o.filled < o.quantity))
                .cloned(),
        );
        orders.extend(
            self.asks.values()
                .flat_map(|asks| asks.iter().filter(|o| o.user_id == user_id && o.filled < o.quantity))
                .cloned(),
        );
        orders
    }

    pub fn match_ask(&mut self, order: &Order) -> Result<(Vec<Fill>, f64), String> {
        let mut fills = Vec::new();
        let mut executed_qty = 0.0;

        for (price, bids) in self.bids.iter_mut() {
            if price.0 >= order.price && executed_qty < order.quantity {
                let amount_remaining = f64::min(
                    order.quantity - executed_qty,
                    bids.iter().map(|b| b.quantity - b.filled).sum::<f64>()
                );
                
                executed_qty += amount_remaining;
                for bid in bids.iter_mut() {
                    if bid.user_id != order.user_id {
                        bid.filled += amount_remaining;
                        fills.push(Fill {
                            price: price.0,
                            qty: amount_remaining,
                            trade_id: {
                                self.last_trade_id += 1;
                                self.last_trade_id
                            },
                            other_user_id: bid.user_id.clone(),
                            marker_order_id: bid.order_id.clone(),
                        });
                    }
                }
            }
        }
        self.bids.retain(|&price, bids| {
            bids.retain(|bid| bid.filled < bid.quantity);
            !bids.is_empty()
        });
        
        Ok((fills, executed_qty))
    }

    pub fn match_bid(&mut self, order: &Order) -> Result<(Vec<Fill>, f64), String> {
        let mut fills = Vec::new();
        let mut executed_qty = 0.0;

        // Collect asks to remove to avoid borrow checker issues
        let mut asks_to_remove = Vec::new();

        for (price, asks) in self.asks.iter_mut() {
            if price.0 <= order.price && executed_qty < order.quantity {
                let available_qty = asks.iter().map(|a| a.quantity - a.filled).sum::<f64>();
                let filled_qty = f64::min(order.quantity - executed_qty, available_qty);
                
                if filled_qty > 0.0 {
                    executed_qty += filled_qty;
                    for ask in asks.iter_mut() {
                        if ask.user_id != order.user_id {
                            ask.filled += filled_qty;
                            fills.push(Fill {
                                price: price.0,
                                qty: filled_qty,
                                trade_id: {
                                    self.last_trade_id += 1;
                                    self.last_trade_id
                                },
                                other_user_id: ask.user_id.clone(),
                                marker_order_id: ask.order_id.clone(),
                            });
                        }
                    }

                    // If all orders at this price level are filled, mark for removal
                    if asks.iter().all(|ask| ask.filled >= ask.quantity) {
                        asks_to_remove.push(*price);
                    }
                }
            }
        }

        // Remove filled price levels
        for price in asks_to_remove {
            self.asks.remove(&price);
        }

        Ok((fills, executed_qty))
    }

    pub fn cancel_bid(&mut self, order_id: &str) -> Result<f64, String> {
        let price = self.bids.iter()
            .find_map(|(price, bids)| bids.iter().position(|bid| bid.order_id == order_id).map(|_| price.0))
            .ok_or("Order not found")?;
        
        self.bids.entry(Price(price)).and_modify(|bids| {
            bids.retain(|bid| bid.order_id != order_id);
        });
        Ok(price)                            
    }

    pub fn cancel_ask(&mut self, order_id: &str) -> Result<f64, String> {
        let price = self.asks.iter()
            .find_map(|(price, asks)| asks.iter().position(|ask| ask.order_id == order_id).map(|_| price.0))
            .ok_or("Order not found")?;
        
        self.asks.entry(Price(price)).and_modify(|asks| {
            asks.retain(|ask| ask.order_id != order_id);
        });
        Ok(price)                           
    }
}

use serde::{Deserialize, Serialize};
use crate::redis::redis_manager::OrderSide;
use log::info;
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
    base_asset: String,
    pub bids: Vec<Order>,
    pub asks: Vec<Order>,
    last_trade_id: i64,
    current_price: f64,
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
    pub fn new(base_asset: String) -> Self {
        Self {
            base_asset,
            bids: Vec::new(),
            asks: Vec::new(),
            last_trade_id: 0,
            current_price: 0.0,
        }
    }

    pub fn ticker(&self) -> String {
        format!("{}_INR", self.base_asset)
    }

    #[allow(dead_code)]
    pub fn get_snapshot(&self) -> Orderbook {
        let snapshot = Orderbook {
            base_asset: self.base_asset.clone(),
            bids: self.bids.clone(),
            asks: self.asks.clone(),
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
                self.bids.push(order.clone());
                Ok((fills, executed_qty))
            }
        } else {
            let (fills, executed_qty) = self.match_ask(order).expect("Error matching ask");
            order.filled = executed_qty;
            if executed_qty == order.quantity {
                Ok((fills, executed_qty))
            } else {
                self.asks.push(order.clone());
                Ok((fills, executed_qty))
            }
        }
    }

    pub fn get_depth(&self) -> OrderbookSnapshot {
        let mut bids: Vec<(String, String)> = Vec::new();
        let mut asks: Vec<(String, String)> = Vec::new();
        info!("Getting depth for market: {:?}", self.base_asset);
        // Aggregate bids at same price level
        for bid in &self.bids {
            let price = bid.price.to_string();
            let remaining = bid.quantity - bid.filled;
            if remaining > 0.0 {
                if let Some(existing) = bids.iter_mut().find(|(p, _)| p == &price) {
                    existing.1 = (existing.1.parse::<f64>().unwrap() + remaining).to_string();
                } else {
                    bids.push((price, remaining.to_string()));
                }
            }
        }
        info!("Bids: {:?}", bids);
        // Aggregate asks at same price level
        for ask in &self.asks {
            let price = ask.price.to_string();
            let remaining = ask.quantity - ask.filled;
            if remaining > 0.0 {
                if let Some(existing) = asks.iter_mut().find(|(p, _)| p == &price) {
                    existing.1 = (existing.1.parse::<f64>().unwrap() + remaining).to_string();
                } else {
                    asks.push((price, remaining.to_string()));
                }
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
            self.bids
                .iter()
                .filter(|o| o.user_id == user_id && o.filled < o.quantity)
                .cloned(),
        );
        orders.extend(
            self.asks
                .iter()
                .filter(|o| o.user_id == user_id && o.filled < o.quantity)
                .cloned(),
        );
        orders
    }

    pub fn match_ask(&mut self, order: &Order) -> Result<(Vec<Fill>, f64), String> {
        let mut fills = Vec::new();
        let mut executed_qty = 0.0;

        for bid in self.bids.iter_mut() {
            if bid.price >= order.price && executed_qty < order.quantity && bid.user_id != order.user_id {
                let amount_remaining = f64::min(
                    order.quantity - executed_qty,
                    bid.quantity - bid.filled
                );
                
                executed_qty += amount_remaining;
                bid.filled += amount_remaining;
                
                fills.push(Fill {
                    price: bid.price,
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
        self.bids.retain(|bid| bid.filled < bid.quantity);
        
        Ok((fills, executed_qty))
    }

    pub fn match_bid(&mut self, order: &Order) -> Result<(Vec<Fill>, f64), String> {
        let mut fills = Vec::new();
        let mut executed_qty = 0.0;

        for ask in self.asks.iter_mut() {
            if ask.price <= order.price && executed_qty < order.quantity && ask.user_id != order.user_id {
                let filled_qty = f64::min(order.quantity - executed_qty, ask.quantity - ask.filled);
                executed_qty += filled_qty;
                ask.filled += filled_qty;

                fills.push(Fill {
                    price: ask.price,
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

        self.asks.retain(|ask| ask.filled < ask.quantity);

        Ok((fills, executed_qty))
    }

    pub fn cancel_bid(&mut self, order_id: &str) -> Result<f64, String> {
        let index = self.bids
            .iter()
            .position(|bid| bid.order_id == order_id)
            .ok_or("Order not found")?;
        
        let price = self.bids[index].price;  
        self.bids.remove(index);             
        Ok(price)                            
    }

    pub fn cancel_ask(&mut self, order_id: &str) -> Result<f64, String> {
        let index = self.asks
            .iter()
            .position(|ask| ask.order_id == order_id)
            .ok_or("Order not found")?;
        
        let price = self.asks[index].price;  
        self.asks.remove(index);             
        Ok(price)                           
    }
}

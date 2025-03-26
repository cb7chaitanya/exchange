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
                self.bids.entry(Price(order.price))
                    .and_modify(|bids| bids.push(order.clone()))
                    .or_insert_with(|| vec![order.clone()]);
                Ok((fills, executed_qty))
            }
        } else {
            let (fills, executed_qty) = self.match_ask(order).expect("Error matching ask");
            order.filled = executed_qty;
            if executed_qty == order.quantity {
                Ok((fills, executed_qty))
            } else {
                self.asks.entry(Price(order.price))
                    .and_modify(|asks| asks.push(order.clone()))
                    .or_insert_with(|| vec![order.clone()]);
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
        
        // Create a list of price levels to process, sorted by price (highest first)
        let mut price_levels: Vec<Price> = self.bids.keys().cloned().collect();
        price_levels.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap()); // Sort in descending order
        
        for price in price_levels {
            if price.0 >= order.price && executed_qty < order.quantity {
                if let Some(bids) = self.bids.get_mut(&price) {
                    let remaining_to_fill = order.quantity - executed_qty;
                    let mut filled_at_this_level = 0.0;
                    
                    // Process each bid at this price level
                    for bid in bids.iter_mut() {
                        if bid.user_id != order.user_id {
                            let bid_remaining = bid.quantity - bid.filled;
                            if bid_remaining > 0.0 {
                                let fill_qty = f64::min(remaining_to_fill - filled_at_this_level, bid_remaining);
                                
                                if fill_qty > 0.0 {
                                    bid.filled += fill_qty;
                                    filled_at_this_level += fill_qty;
                                    
                                    fills.push(Fill {
                                        price: price.0,
                                        qty: fill_qty,
                                        trade_id: {
                                            self.last_trade_id += 1;
                                            self.last_trade_id
                                        },
                                        other_user_id: bid.user_id.clone(),
                                        marker_order_id: bid.order_id.clone(),
                                    });
                                    
                                    if filled_at_this_level >= remaining_to_fill {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    
                    executed_qty += filled_at_this_level;
                }
            }
        }
        
        // Clean up filled orders
        self.bids.retain(|_price, bids| {
            bids.retain(|bid| bid.filled < bid.quantity);
            !bids.is_empty()
        });
        
        Ok((fills, executed_qty))
    }

    pub fn match_bid(&mut self, order: &Order) -> Result<(Vec<Fill>, f64), String> {
        let mut fills = Vec::new();
        let mut executed_qty = 0.0;

        // Create a list of price levels to process, sorted by price (lowest first)
        let mut price_levels: Vec<Price> = self.asks.keys().cloned().collect();
        price_levels.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap()); // Sort in ascending order
        
        for price in price_levels {
            if price.0 <= order.price && executed_qty < order.quantity {
                if let Some(asks) = self.asks.get_mut(&price) {
                    let remaining_to_fill = order.quantity - executed_qty;
                    let mut filled_at_this_level = 0.0;
                    
                    // Process each ask at this price level
                    for ask in asks.iter_mut() {
                        if ask.user_id != order.user_id {
                            let ask_remaining = ask.quantity - ask.filled;
                            if ask_remaining > 0.0 {
                                let fill_qty = f64::min(remaining_to_fill - filled_at_this_level, ask_remaining);
                                
                                if fill_qty > 0.0 {
                                    ask.filled += fill_qty;
                                    filled_at_this_level += fill_qty;
                                    
                                    fills.push(Fill {
                                        price: price.0,
                                        qty: fill_qty,
                                        trade_id: {
                                            self.last_trade_id += 1;
                                            self.last_trade_id
                                        },
                                        other_user_id: ask.user_id.clone(),
                                        marker_order_id: ask.order_id.clone(),
                                    });
                                    
                                    if filled_at_this_level >= remaining_to_fill {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    
                    executed_qty += filled_at_this_level;
                }
            }
        }
        
        // Clean up filled orders
        self.asks.retain(|_price, asks| {
            asks.retain(|ask| ask.filled < ask.quantity);
            !asks.is_empty()
        });
        
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trade::orderbook::{OrderSide, Order};
    use rand::thread_rng;
    use rand::distributions::{Alphanumeric, DistString};

    fn generate_order_id() -> String {
        Alphanumeric.sample_string(&mut thread_rng(), 24)
    }

    #[test]
    fn test_add_bid_order() {
        let mut orderbook = Orderbook::new("TEST_MARKET".to_string());
        let mut order = Order {
            order_id: generate_order_id(),
            user_id: "user1".to_string(),
            price: 100.0,
            quantity: 10.0,
            filled: 0.0,
            side: OrderSide::Buy,
        };

        let (fills, executed_qty) = orderbook.add_order(&mut order).unwrap();
        assert_eq!(fills.len(), 0);
        assert_eq!(executed_qty, 0.0);
        assert_eq!(orderbook.bids.len(), 1);
        assert_eq!(orderbook.asks.len(), 0);
    }

    #[test]
    fn test_add_ask_order() {
        let mut orderbook = Orderbook::new("TEST_MARKET".to_string());
        let mut order = Order {
            order_id: generate_order_id(),
            user_id: "user1".to_string(),
            price: 100.0,
            quantity: 10.0,
            filled: 0.0,
            side: OrderSide::Sell,
        };

        let (fills, executed_qty) = orderbook.add_order(&mut order).unwrap();
        assert_eq!(fills.len(), 0);
        assert_eq!(executed_qty, 0.0);
        assert_eq!(orderbook.bids.len(), 0);
        assert_eq!(orderbook.asks.len(), 1);
    }

    #[test]
    fn test_match_orders_different_users() {
        let mut orderbook = Orderbook::new("TEST_MARKET".to_string());
        
        // Add a buy order
        let mut buy_order = Order {
            order_id: generate_order_id(),
            user_id: "user1".to_string(),
            price: 100.0,
            quantity: 5.0,
            filled: 0.0,
            side: OrderSide::Buy,
        };
        
        let (fills, executed_qty) = orderbook.add_order(&mut buy_order).unwrap();
        assert_eq!(fills.len(), 0);
        assert_eq!(executed_qty, 0.0);
        
        // Add a matching sell order from a different user
        let mut sell_order = Order {
            order_id: generate_order_id(),
            user_id: "user2".to_string(),
            price: 100.0,
            quantity: 5.0,
            filled: 0.0,
            side: OrderSide::Sell,
        };
        
        let (fills, executed_qty) = orderbook.add_order(&mut sell_order).unwrap();
        assert_eq!(fills.len(), 1);
        assert_eq!(executed_qty, 5.0);
        assert_eq!(orderbook.bids.len(), 0); // Buy order should be fully matched and removed
        assert_eq!(orderbook.asks.len(), 0); // Sell order should be fully matched and not added
    }

    #[test]
    fn test_match_orders_same_user() {
        let mut orderbook = Orderbook::new("TEST_MARKET".to_string());
        
        // Add a buy order
        let mut buy_order = Order {
            order_id: generate_order_id(),
            user_id: "user1".to_string(),
            price: 100.0,
            quantity: 5.0,
            filled: 0.0,
            side: OrderSide::Buy,
        };
        
        let (fills, executed_qty) = orderbook.add_order(&mut buy_order).unwrap();
        assert_eq!(fills.len(), 0);
        assert_eq!(executed_qty, 0.0);
        
        // Add a matching sell order from the same user
        let mut sell_order = Order {
            order_id: generate_order_id(),
            user_id: "user1".to_string(),
            price: 100.0,
            quantity: 5.0,
            filled: 0.0,
            side: OrderSide::Sell,
        };
        
        let (fills, executed_qty) = orderbook.add_order(&mut sell_order).unwrap();
        assert_eq!(fills.len(), 0); // No fills because same user
        assert_eq!(executed_qty, 0.0);
        assert_eq!(orderbook.bids.len(), 1); // Buy order should remain
        assert_eq!(orderbook.asks.len(), 1); // Sell order should be added
    }

    #[test]
    fn test_partial_match() {
        let mut orderbook = Orderbook::new("TEST_MARKET".to_string());
        
        // Add a buy order
        let mut buy_order = Order {
            order_id: generate_order_id(),
            user_id: "user1".to_string(),
            price: 100.0,
            quantity: 10.0,
            filled: 0.0,
            side: OrderSide::Buy,
        };
        
        let (fills, executed_qty) = orderbook.add_order(&mut buy_order).unwrap();
        assert_eq!(fills.len(), 0);
        assert_eq!(executed_qty, 0.0);
        
        // Add a smaller matching sell order
        let mut sell_order = Order {
            order_id: generate_order_id(),
            user_id: "user2".to_string(),
            price: 100.0,
            quantity: 5.0,
            filled: 0.0,
            side: OrderSide::Sell,
        };
        
        let (fills, executed_qty) = orderbook.add_order(&mut sell_order).unwrap();
        assert_eq!(fills.len(), 1);
        assert_eq!(executed_qty, 5.0);
        assert_eq!(orderbook.bids.len(), 1); // Buy order should remain with reduced quantity
        assert_eq!(orderbook.asks.len(), 0); // Sell order should be fully matched and not added
        
        // Check remaining buy order quantity
        let remaining_qty = orderbook.bids.values().next().unwrap()[0].quantity - orderbook.bids.values().next().unwrap()[0].filled;
        assert_eq!(remaining_qty, 5.0);
    }

    #[test]
    fn test_price_priority() {
        let mut orderbook = Orderbook::new("TEST_MARKET".to_string());
        
        // Add buy orders at different prices
        let mut buy_order1 = Order {
            order_id: generate_order_id(),
            user_id: "user1".to_string(),
            price: 100.0,
            quantity: 5.0,
            filled: 0.0,
            side: OrderSide::Buy,
        };
        
        let mut buy_order2 = Order {
            order_id: generate_order_id(),
            user_id: "user2".to_string(),
            price: 102.0,
            quantity: 5.0,
            filled: 0.0,
            side: OrderSide::Buy,
        };
        
        orderbook.add_order(&mut buy_order1).unwrap();
        orderbook.add_order(&mut buy_order2).unwrap();
        
        // Add a matching sell order
        let mut sell_order = Order {
            order_id: generate_order_id(),
            user_id: "user3".to_string(),
            price: 99.0,
            quantity: 5.0,
            filled: 0.0,
            side: OrderSide::Sell,
        };
        
        let (fills, executed_qty) = orderbook.add_order(&mut sell_order).unwrap();
        assert_eq!(fills.len(), 1);
        assert_eq!(executed_qty, 5.0);
        assert_eq!(fills[0].price, 102.0); // Should match with the higher priced buy order
        assert_eq!(orderbook.bids.len(), 1); // Only the lower priced buy order should remain
    }
}

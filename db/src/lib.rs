use diesel::pg::PgConnection;
use diesel::r2d2::{self, ConnectionManager};
use schema::{trades, orders};
use std::env;
use redis::Client;
use serde::{Deserialize, Serialize};
use tokio;
use serde_json;
use validator::Validate;
use diesel::prelude::*;
use crate::models::{Trade, Order};
use chrono::{TimeZone, Utc};
use log::info;

pub mod schema;
pub mod models;

pub type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DbMessage {
    TradeAdded(TradeMessage),
    OrderUpdate(OrderMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct TradeMessage {
    #[validate(length(min = 1))]
    pub id: String,
    pub is_buyer_maker: bool,
    pub price: String,
    pub quantity: String,
    pub quote_quantity: String,
    pub timestamp: i64,
    pub market: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct OrderMessage {
    #[validate(length(min = 1))]
    pub order_id: String,
    pub executed_qty: f64,
    pub market: Option<String>,
    pub price: Option<String>,
    pub quantity: Option<String>,
    pub side: Option<String>,
}

pub fn establish_connection_pool() -> DbPool {
    match dotenvy::dotenv() {
        Ok(_) => println!("Loaded .env file"),
        Err(e) => println!("Could not load .env file: {}", e),
    }

    let database_url = match env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(e) => panic!("DATABASE_URL not found in environment: {}", e),
    };

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool")
}

pub async fn start_db_processor(pool: DbPool) {
    let redis_url = env::var("REDIS_3_URL")
        .unwrap_or_else(|_| "redis://localhost:6381".to_string());

    info!("Connecting to DB Redis at {:?}", redis_url);
    
    let client = Client::open(redis_url.as_str())
        .expect("Failed to create Redis client");
    let mut conn = client.get_connection()
        .expect("Failed to connect to Redis");

    println!("DB processor started");
    
    loop {
        // Try to get message from Redis
        let result: Option<String> = redis::cmd("BRPOP")
            .arg("db_processor")
            .arg(1)
            .query(&mut conn)
            .unwrap_or(None);

        if let Some(message_str) = result {
            match serde_json::from_str::<DbMessage>(&message_str) {
                Ok(message) => {
                    info!("Processing message: {:?}", message);
                    match process_message(message, &pool) {
                        Ok(_) => println!("Successfully processed message"),
                        Err(e) => println!("Error processing message: {}", e),
                    }
                }
                Err(e) => println!("Error parsing message: {}", e),
            }
        }

        // Small delay to prevent CPU spinning
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

fn process_message(message: DbMessage, pool: &DbPool) -> Result<(), diesel::result::Error> {
    let conn = &mut pool.get().unwrap();
    
    match message {
        DbMessage::TradeAdded(trade_message) => {
            println!("Processing trade: {}", trade_message.id);
            if let Err(e) = trade_message.validate() {
                println!("Trade validation failed for ID {}: {:?}", trade_message.id, e);
                return Ok(());
            }

            let trade = Trade {
                id: uuid::Uuid::parse_str(&trade_message.id).expect("Failed to parse into uuid"),
                is_buyer_maker: trade_message.is_buyer_maker,
                price: trade_message.price,
                quantity: trade_message.quantity,
                quote_quantity: trade_message.quote_quantity,
                timestamp: Utc.timestamp_opt(trade_message.timestamp, 0)
                    .unwrap()
                    .naive_utc(),
                market: trade_message.market,
            };

            diesel::insert_into(trades::table)
                .values(&trade)
                .execute(conn)?;
        }
        
        DbMessage::OrderUpdate(order_message) => {
            println!("Processing order update: {}", order_message.order_id);
            if let Err(e) = order_message.validate() {
                println!("Order validation failed for ID {}: {:?}", order_message.order_id, e);
                return Ok(());
            }
            
            let order = Order {
                id: uuid::Uuid::parse_str(&order_message.order_id).expect("Failed to parse into uuid"),
                executed_qty: order_message.executed_qty.to_string().parse().unwrap(),
                market: order_message.market.unwrap_or_default(),
                price: order_message.price.unwrap_or_default(),
                quantity: order_message.quantity.unwrap_or_default(),
                side: order_message.side.unwrap_or_default(),
                created_at: Utc::now().naive_utc(),
            };

            diesel::insert_into(orders::table)
                .values(&order)
                .execute(conn)?;
        }
    }
    
    Ok(())
}
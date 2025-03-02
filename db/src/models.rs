use diesel::prelude::*;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::NaiveDateTime;
use bigdecimal::BigDecimal;

#[derive(Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::users)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
} 
#[derive(Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::trades)]
pub struct Trade {
    pub id: Uuid,
    pub is_buyer_maker: bool,
    pub price: String,
    pub quantity: String,
    pub quote_quantity: String,
    pub timestamp: NaiveDateTime,
    pub market: String
}
#[derive(Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::orders)]
pub struct Order {
    pub id: Uuid,
    pub executed_qty: BigDecimal,
    pub market: String,
    pub price: String,
    pub quantity: String,
    pub side: String,
    pub created_at: NaiveDateTime
}
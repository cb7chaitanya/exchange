use actix_web::{web, dev::ServiceRequest};
use jsonwebtoken::{decode, DecodingKey, Validation};
use db::{DbPool, models::User};
use diesel::prelude::*;
use db::schema::users::dsl::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::env;

pub fn get_jwt_secret() -> String {
    env::var("JWT_SECRET").expect("JWT_SECRET must be set in environment")
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  
    pub exp: usize,  
}

pub fn get_user_from_jwt(req: &ServiceRequest, pool: &web::Data<DbPool>) -> Option<User> {
    
    let cookie = req.cookie("token")?;
    let token = cookie.value();

    let decoding_key = DecodingKey::from_secret(get_jwt_secret().as_bytes());
    let validation = Validation::default();

    let token_data = decode::<Claims>(token, &decoding_key, &validation).ok()?;
    let user_id = Uuid::parse_str(&token_data.claims.sub).ok()?;

    let conn = &mut pool.get().expect("Failed to get DB connection");
    match users.filter(id.eq(user_id)).first::<User>(conn) {
        Ok(user) => Some(user),
        Err(_) => None,
    }
}

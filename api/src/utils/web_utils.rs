use actix_web::{web, dev::ServiceRequest};
use jsonwebtoken::{decode, DecodingKey, Validation};
use db::{DbPool, models::User};
use diesel::prelude::*;
use db::schema::users::dsl::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::env;
use log::info;

pub fn get_jwt_secret() -> String {
    env::var("JWT_SECRET").expect("JWT_SECRET must be set in environment")
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  
    pub exp: usize,  
}

pub fn get_user_from_jwt(req: &ServiceRequest, pool: &web::Data<DbPool>) -> Option<User> {
    info!("Getting user from JWT");
    
    // Log all cookies
    if let Ok(cookies) = req.cookies() {
        info!("All cookies in request:");
        for cookie in cookies.iter() {
            info!("  {} = {}", cookie.name(), cookie.value());
        }
    } else {
        info!("No cookies found in request");
    }
    
    let cookie = req.cookie("token")?;
    let token = cookie.value();
    info!("Token: {}", token);
    let decoding_key = DecodingKey::from_secret(get_jwt_secret().as_bytes());
    let validation = Validation::default();
    info!("Validation: {:?}", validation);
    let token_data = decode::<Claims>(token, &decoding_key, &validation).ok()?;
    info!("Token data: {:?}", token_data);
    let user_id = Uuid::parse_str(&token_data.claims.sub).ok()?;
    info!("User ID: {}", user_id);
    let conn = &mut pool.get().expect("Failed to get DB connection");
    match users.filter(id.eq(user_id)).first::<User>(conn) {
        Ok(user) => {
            info!("User found: {}", user.id);
            Some(user)
        }
        Err(_) => {
            info!("User not found");
            None
        }
    }
}

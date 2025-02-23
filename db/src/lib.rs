use diesel::pg::PgConnection;
use diesel::r2d2::{self, ConnectionManager};
use std::env;

pub mod schema;
pub mod models;

pub type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

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
use db::{self, establish_connection_pool, start_db_processor};
use dotenv::dotenv;
use log::info;
use env_logger;

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init();
    
    info!("Starting DB processor...");

    let pool = establish_connection_pool();
    info!("Successfully connected to database");
    start_db_processor(pool).await;
}
use actix_web::{web, App, HttpServer, middleware};
use std::io;
use routes::auth::config as auth_config;
use db::establish_connection_pool;

mod routes;
mod types;

#[actix_web::main]
async fn main() -> io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    
    // Create a shared database connection
    let pool = web::Data::new(establish_connection_pool());

    log::info!("Starting server at http://localhost:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(pool.clone())
            .wrap(middleware::Logger::default())
            .wrap(middleware::DefaultHeaders::new().add(("X-Version", "1.0")))
            .service(
                web::scope("/api/v1")
                    .configure(auth_config)
            )
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

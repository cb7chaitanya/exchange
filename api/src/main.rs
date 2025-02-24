use actix_web::{web, App, HttpServer, middleware, http::header};
use actix_cors::Cors;
use std::io;
use routes::auth::config as auth_config;
use routes::order::config as order_config;
use routes::depth::config as depth_config;
use db::establish_connection_pool;

mod routes;
mod types;
mod redis;
mod middlewares;
mod utils;
#[actix_web::main]
async fn main() -> io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    
    // Create a shared database connection
    let pool = web::Data::new(establish_connection_pool());

    log::info!("Starting server at http://localhost:8080");

    HttpServer::new(move || {
        // Configure CORS
        let cors = Cors::default()
            .allowed_origin("http://localhost:3000")  // Your frontend URL
            .allowed_methods(vec!["GET", "POST"])
            .allowed_headers(vec![
                header::AUTHORIZATION,
                header::ACCEPT,
                header::CONTENT_TYPE,
            ])
            .supports_credentials()
            .max_age(86400);

        App::new()
            .app_data(pool.clone())
            .wrap(cors) 
            .wrap(middleware::Logger::default())
            .wrap(middleware::DefaultHeaders::new().add(("X-Version", "1.0")))
            .service(
                web::scope("/api/v1")
                    .configure(auth_config)
                    .configure(order_config)
                    .configure(depth_config)
            )
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

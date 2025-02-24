use actix_web::web;
use crate::redis::redis_manager::RedisManager;
use crate::types::redis::{MessageToEngine, GetDepthData};
use actix_web::{Responder, HttpResponse};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/depth")
            .route("/{market}", web::get().to(get_depth))
    );
}

pub async fn get_depth(
    market: String,
) -> impl Responder {
    let redis_manager = RedisManager::get_instance().lock().unwrap();

    let message = MessageToEngine::GetDepth {
        data: GetDepthData {
            market,
        },
    };

    match redis_manager.send_and_await(message).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}
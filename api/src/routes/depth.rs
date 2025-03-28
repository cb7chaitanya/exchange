use actix_web::web;
use crate::redis::redis_manager::RedisManager;
use crate::types::redis::{MessageToEngine, GetDepthData};
use actix_web::{Responder, HttpResponse};
use log::info;
use serde::Deserialize;
use crate::middlewares::auth::AuthService;

#[derive(Deserialize)]
pub struct MarketPath {
    market: String,
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/depth")
            .wrap(AuthService::new())
            .route("/{market}", web::get().to(get_depth))
    );
}

pub async fn get_depth(
    path: web::Path<MarketPath>,
    user_id: web::ReqData<String>,
) -> impl Responder {
    let market = path.into_inner().market;
    info!("Getting depth for market: {:?}", market);
    let redis_manager = RedisManager::get_instance().lock().unwrap();

    let message = MessageToEngine::GetDepth {
        data: GetDepthData {
            market
        },
    };

    match redis_manager.send_and_await(message, user_id.into_inner()).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}
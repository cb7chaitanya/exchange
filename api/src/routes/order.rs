use actix_web::{web, Responder, HttpResponse};
use crate::redis::redis_manager::RedisManager;
use crate::types::redis::{MessageToEngine, GetOpenOrdersData};
use crate::middlewares::auth::AuthService;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/orders")
            .wrap(AuthService::new())
            .route("/open", web::get().to(get_open_orders))
    );
}

#[derive(serde::Deserialize)]
pub struct OpenOrdersQuery {
    market: String,
}

pub async fn get_open_orders(
    user_id: web::ReqData<String>,
    query: web::Query<OpenOrdersQuery>,
) -> impl Responder {
    let redis_manager = RedisManager::get_instance().lock().unwrap();
    
    let message = MessageToEngine::GetOpenOrders {
        data: GetOpenOrdersData {
            user_id: user_id.into_inner(),
            market: query.market.clone(),
        }
    };

    match redis_manager.send_and_await(message).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}
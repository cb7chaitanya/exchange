use actix_web::{web, Responder, HttpResponse};
use crate::redis::redis_manager::RedisManager;
use crate::types::redis::{MessageToEngine, GetOpenOrdersData, CancelOrderData, CreateOrderData};
use crate::middlewares::auth::AuthService;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/orders")
            .wrap(AuthService::new())
            .route("/open", web::get().to(get_open_orders))
            .route("/{order_id}", web::delete().to(cancel_order))
            .route("/", web::post().to(create_order))
    );
}

#[derive(serde::Deserialize)]
pub struct OpenOrdersQuery {
    market: String,
}

pub async fn get_open_orders(
    user_id: String,
    query: web::Query<OpenOrdersQuery>,
) -> impl Responder {
    let redis_manager = RedisManager::get_instance().lock().unwrap();
    
    let message = MessageToEngine::GetOpenOrders {
        data: GetOpenOrdersData {
            user_id,
            market: query.market.clone(),
        }
    };

    match redis_manager.send_and_await(message).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

pub async fn cancel_order(
    query: web::Query<CancelOrderData>,
) -> impl Responder {
    let redis_manager = RedisManager::get_instance().lock().unwrap();

    let message = MessageToEngine::CancelOrder {
        data: CancelOrderData{
            order_id: query.order_id.clone(),
            market: query.market.clone(),
        },
    };

    match redis_manager.send_and_await(message).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

pub async fn create_order(
    user_id: String,
    query: web::Query<CreateOrderData>,
) -> impl Responder {
    let redis_manager = RedisManager::get_instance().lock().unwrap();

    let message = MessageToEngine::CreateOrder {
        data: CreateOrderData{
            user_id,
            market: query.market.clone(),
            price: query.price.clone(),
            quantity: query.quantity.clone(),
            side: query.side.clone(),
        },
    };

    match redis_manager.send_and_await(message).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

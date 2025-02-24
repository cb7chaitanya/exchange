use actix_web::body::EitherBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use std::future::{ready, Ready};
use actix_web::{dev, dev::Service, dev::Transform, Error, HttpResponse, web::Data, HttpMessage};
use futures_util::future::LocalBoxFuture;
use db::DbPool;
use log::info;
use crate::utils::web_utils::get_user_from_jwt;

pub struct AuthMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for AuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;
    dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let path = req.path().to_string();
        info!("Requested path: {}", path);

        let pool = req.app_data::<Data<DbPool>>().expect("DB Pool not found");

        let public_routes = vec!["/login", "/register", "/public"];

        if public_routes.iter().any(|route| path.starts_with(route)) {
            let res = self.service.call(req);
            Box::pin(async move { res.await.map(ServiceResponse::map_into_left_body) })
        } else {
            if let Some(user) = get_user_from_jwt(&req, pool) {
                req.extensions_mut().insert(user.id.to_string());
                let res = self.service.call(req);
                Box::pin(async move { res.await.map(ServiceResponse::map_into_left_body) })
            } else {
                let (request, _pl) = req.into_parts();
                let response = HttpResponse::Unauthorized()
                    .json(serde_json::json!({ "error": "Unauthorized" }))
                    .map_into_right_body();

                Box::pin(async { Ok(ServiceResponse::new(request, response)) })
            }
        }
    }
}

#[derive(Clone)]
pub struct AuthService;

impl AuthService {
    pub fn new() -> Self {
        AuthService
    }
}

impl<S, B> Transform<S, ServiceRequest> for AuthService
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddleware { service }))
    }
}

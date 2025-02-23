use actix_web::{web, HttpResponse, Responder, cookie::Cookie};
use crate::types::auth_types::{LoginForm, SignupForm};
use validator::Validate;
use db::{DbPool, models::User};
use diesel::prelude::*;
use db::schema::users::dsl::*;
use serde_json::json;
use bcrypt::{verify, DEFAULT_COST, hash};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,  // user id
    exp: usize,   // expiration time
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .route("/login", web::post().to(login))
            .route("/signup", web::post().to(signup))
            .route("/logout", web::post().to(logout))
    );
}

async fn login(
    pool: web::Data<DbPool>,
    form: web::Json<LoginForm>
) -> impl Responder {
    if let Err(e) = form.validate() {
        return HttpResponse::BadRequest().json(e);
    }

    let conn = &mut pool.get().expect("couldn't get db connection from pool");
    
    match users.filter(email.eq(&form.email)).first::<User>(conn) {
        Ok(user) => {
            match verify(&form.password, &user.password_hash) {
                Ok(true) => {
                    // Create JWT token
                    let claims = Claims {
                        sub: user.id.to_string(),
                        exp: (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize,
                    };

                    let token = encode(
                        &Header::default(),
                        &claims,
                        &EncodingKey::from_secret("your-secret-key".as_ref())
                    ).unwrap();

                    HttpResponse::Ok()
                        .cookie(
                            Cookie::build("token", token)
                                .http_only(true)
                                .secure(true)
                                .finish()
                        )
                        .json(json!({ "message": "Login successful" }))
                },
                _ => {
                    HttpResponse::Unauthorized()
                        .json(json!({ "error": "Invalid credentials" }))
                }
            }
        },
        Err(diesel::result::Error::NotFound) => {
            HttpResponse::NotFound()
                .json(json!({ "error": "User not found" }))
        },
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

async fn signup(
    pool: web::Data<DbPool>,
    form: web::Json<SignupForm>
) -> impl Responder {
    if let Err(e) = form.validate() {
        return HttpResponse::BadRequest().json(e);
    }

    let conn = &mut pool.get().expect("couldn't get db connection from pool");

    match users.filter(email.eq(&form.email)).first::<User>(conn) {
        Ok(_) => {
            return HttpResponse::BadRequest()
                .json(json!({ "error": "User already exists" }));
        },
        Err(diesel::result::Error::NotFound) => (), // User doesn't exist, continue
        Err(_) => return HttpResponse::InternalServerError().finish()
    };

    let hashed_password = hash(&form.password, DEFAULT_COST)
        .map_err(|_| HttpResponse::InternalServerError().finish())
        .expect("Failed to hash password");

    let new_user = User {
        id: Uuid::new_v4(),
        email: form.email.clone(),
        password_hash: hashed_password,
        username: form.username.clone(),
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
    };

    diesel::insert_into(users)
        .values(&new_user)
        .execute(conn)
        .map_err(|_| HttpResponse::InternalServerError().finish())
        .expect("Failed to insert user");

    let token = encode(
        &Header::default(),
        &Claims { sub: new_user.id.to_string(), exp: (Utc::now() + chrono::Duration::hours(24)).timestamp() as usize },
        &EncodingKey::from_secret("your-secret-key".as_ref())
    ).unwrap();

    HttpResponse::Ok()
        .cookie(
            Cookie::build("token", token)
                .http_only(true)
                .secure(true)
                .finish()
        )
        .json(json!({ "message": "Signup successful" }))
}

async fn logout() -> impl Responder {
    HttpResponse::Ok()
        .cookie(
            Cookie::build("token", "")
                .http_only(true)
                .secure(true)
                .path("/")
                .expires(actix_web::cookie::time::OffsetDateTime::now_utc())  // Expire immediately
                .finish()
        )
        .json(json!({ "message": "Logout successful" }))
}

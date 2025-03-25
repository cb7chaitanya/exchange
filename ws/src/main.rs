use std::net::SocketAddr;
use warp::Filter;
use crate::classes::user_manager::UserManager;
use dotenv::dotenv;
use std::sync::Arc;
use tokio::sync::Mutex;
use log::info;
mod classes;

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init();

    let port = std::env::var("WS_PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse::<u16>()
        .expect("Invalid port");
    
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let user_manager = UserManager::get_instance();

    // WebSocket route
    let ws_route = warp::ws()
        .and(warp::addr::remote())
        .and(warp::any().map(move || user_manager.clone()))
        .map(|ws: warp::ws::Ws, _addr, manager: Arc<Mutex<UserManager>>| {
            ws.on_upgrade(move |websocket| async move {
                let mut manager = manager.lock().await;
                let user_id = manager.add_user(websocket).await;
                info!("New connection: {}", user_id);
            })
        });

    info!("WebSocket server starting on port {}", port);
    warp::serve(ws_route).run(addr).await;
}

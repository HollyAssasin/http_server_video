mod state;
mod utils;
mod stream_handlers;
mod test_handlers;

use axum::{Router};
use state::SharedState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let shared_vector = SharedState::default(); // Global state
    let app = Router::new()
        .merge(stream_handlers::get_stream_routes()) // Routes for streaming functionality
        .merge(test_handlers::get_test_routes()) // Routes for testing
        .with_state(shared_vector);

    const LISTEN_ADDR: &str = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(LISTEN_ADDR).await.unwrap();
    tracing::info!("listening on {}", LISTEN_ADDR);
    axum::serve(listener, app).await.unwrap();
}

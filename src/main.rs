use axum::{routing::{get, post}, Router};
use favorite_api::handlers;
use favorite_api::state::new_state;

#[tokio::main]
async fn main() {
    let state = new_state();

    let app = Router::new()
        .route("/favorite-groups", post(handlers::create_group))
        .route("/favorite-groups", get(handlers::list_groups))
        .route("/favorite-items/move", post(handlers::move_item))
        .route("/favorite-items/batch-move", post(handlers::batch_move_items))
        .route("/favorite-groups/{group_id}/items", get(handlers::list_items_by_group))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on http://0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}

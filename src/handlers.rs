use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use uuid::Uuid;
use chrono::Utc;

use crate::models::*;
use crate::state::SharedState;

pub async fn create_group(
    State(state): State<SharedState>,
    Json(req): Json<CreateGroupRequest>,
) -> Result<Json<ApiResponse<FavoriteGroup>>, (StatusCode, Json<ApiResponse<()>>)> {
    if req.name.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(400, "分组名称不能为空")),
        ));
    }

    let mut store = state.write().await;

    let group = FavoriteGroup {
        id: Uuid::new_v4(),
        name: req.name.trim().to_string(),
        user_id: req.user_id,
        sort_order: req.sort_order.unwrap_or(0),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    store.groups.insert(group.id, group.clone());

    Ok(Json(ApiResponse::success(group)))
}

pub async fn list_groups(
    State(state): State<SharedState>,
) -> Json<ApiResponse<Vec<FavoriteGroup>>> {
    let store = state.read().await;
    let mut groups: Vec<FavoriteGroup> = store
        .groups
        .values()
        .cloned()
        .collect();
    groups.sort_by(|a, b| a.sort_order.cmp(&b.sort_order));
    Json(ApiResponse::success(groups))
}

pub async fn move_item(
    State(state): State<SharedState>,
    Json(req): Json<MoveItemRequest>,
) -> Result<Json<ApiResponse<FavoriteItem>>, (StatusCode, Json<ApiResponse<()>>)> {
    let mut store = state.write().await;

    if !store.groups.contains_key(&req.target_group_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(404, "目标分组不存在")),
        ));
    }

    let item = match store.items.get_mut(&req.item_id) {
        Some(i) => i,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(404, "收藏商品不存在")),
            ));
        }
    };

    if item.user_id != req.user_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error(403, "无权操作此商品")),
        ));
    }

    item.group_id = req.target_group_id;

    let updated = item.clone();
    Ok(Json(ApiResponse::success(updated)))
}

pub async fn batch_move_items(
    State(state): State<SharedState>,
    Json(req): Json<BatchMoveRequest>,
) -> Result<Json<ApiResponse<Vec<FavoriteItem>>>, (StatusCode, Json<ApiResponse<()>>)> {
    let mut store = state.write().await;

    if !store.groups.contains_key(&req.target_group_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(404, "目标分组不存在")),
        ));
    }

    let mut updated_items = Vec::new();

    for item_id in &req.item_ids {
        if let Some(item) = store.items.get_mut(item_id) {
            if item.user_id != req.user_id {
                continue;
            }
            item.group_id = req.target_group_id;
            updated_items.push(item.clone());
        }
    }

    Ok(Json(ApiResponse::success(updated_items)))
}

pub async fn list_items_by_group(
    State(state): State<SharedState>,
    axum::extract::Path(group_id): axum::extract::Path<Uuid>,
) -> Json<ApiResponse<Vec<FavoriteItem>>> {
    let store = state.read().await;
    let items: Vec<FavoriteItem> = store
        .items
        .values()
        .filter(|i| i.group_id == group_id)
        .cloned()
        .collect();
    Json(ApiResponse::success(items))
}

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
        is_default: false,
        sort_order: req.sort_order.unwrap_or(0),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    store.groups.insert(group.id, group.clone());

    Ok(Json(ApiResponse::success(group)))
}

fn ensure_default_group(store: &mut crate::state::AppStore, user_id: Uuid) -> Uuid {
    if let Some(existing) = store
        .groups
        .values()
        .find(|g| g.user_id == user_id && g.is_default)
    {
        return existing.id;
    }

    let default_group = FavoriteGroup {
        id: Uuid::new_v4(),
        name: "未分类".to_string(),
        user_id,
        is_default: true,
        sort_order: i32::MIN,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let id = default_group.id;
    store.groups.insert(id, default_group);
    id
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

pub async fn delete_group(
    State(state): State<SharedState>,
    axum::extract::Path(group_id): axum::extract::Path<Uuid>,
    Json(req): Json<DeleteGroupRequest>,
) -> Result<Json<ApiResponse<DeleteGroupResult>>, (StatusCode, Json<ApiResponse<()>>)> {
    let mut store = state.write().await;

    let group = match store.groups.get(&group_id) {
        Some(g) => g.clone(),
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(404, "分组不存在")),
            ));
        }
    };

    if group.user_id != req.user_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error(403, "无权操作此分组")),
        ));
    }

    if group.is_default {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(400, "默认分组不能删除")),
        ));
    }

    let clear_items = req.clear_items.unwrap_or(false);

    let mut migrated_count = 0usize;
    let mut cleared_count = 0usize;

    if clear_items {
        let item_ids: Vec<Uuid> = store
            .items
            .values()
            .filter(|i| i.group_id == group_id && i.user_id == req.user_id)
            .map(|i| i.id)
            .collect();
        cleared_count = item_ids.len();
        for id in item_ids {
            store.items.remove(&id);
        }
    } else {
        let target_group_id = match req.target_group_id {
            Some(tid) => {
                let target = store.groups.get(&tid);
                match target {
                    Some(tg) if tg.user_id == req.user_id => tid,
                    _ => {
                        return Err((
                            StatusCode::BAD_REQUEST,
                            Json(ApiResponse::error(400, "目标分组无效或不属于当前用户")),
                        ));
                    }
                }
            }
            None => ensure_default_group(&mut store, req.user_id),
        };

        if target_group_id == group_id {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(400, "目标分组不能是被删除的分组")),
            ));
        }

        let item_ids: Vec<Uuid> = store
            .items
            .values()
            .filter(|i| i.group_id == group_id && i.user_id == req.user_id)
            .map(|i| i.id)
            .collect();
        migrated_count = item_ids.len();
        for id in item_ids {
            if let Some(item) = store.items.get_mut(&id) {
                item.group_id = target_group_id;
            }
        }
    }

    store.groups.remove(&group_id);

    Ok(Json(ApiResponse::success(DeleteGroupResult {
        migrated_count,
        cleared_count,
    })))
}

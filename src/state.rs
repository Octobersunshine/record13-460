use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use uuid::Uuid;

use crate::models::{FavoriteGroup, FavoriteItem};

#[derive(Debug, Clone, Default)]
pub struct AppStore {
    pub groups: HashMap<Uuid, FavoriteGroup>,
    pub items: HashMap<Uuid, FavoriteItem>,
}

pub type SharedState = Arc<RwLock<AppStore>>;

pub fn new_state() -> SharedState {
    Arc::new(RwLock::new(AppStore::default()))
}

use tauri::State;

use crate::commands::blocking;
use crate::models::{AppResult, Tag, TagWithCount};
use crate::repositories::tag_repo;
use crate::state::AppState;

#[tauri::command]
pub async fn list_tags(state: State<'_, AppState>) -> AppResult<Vec<TagWithCount>> {
    let db = state.db();
    blocking(move || db.with_conn(|conn| tag_repo::list_with_counts(conn))).await
}

#[tauri::command]
pub async fn create_tag(
    state: State<'_, AppState>,
    name: String,
    color: Option<String>,
) -> AppResult<Tag> {
    let db = state.db();
    blocking(move || db.with_conn(|conn| tag_repo::create(conn, &name, color))).await
}

#[tauri::command]
pub async fn rename_tag(state: State<'_, AppState>, id: String, name: String) -> AppResult<Tag> {
    let db = state.db();
    blocking(move || db.with_conn(|conn| tag_repo::rename(conn, &id, &name))).await
}

#[tauri::command]
pub async fn delete_tag(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let db = state.db();
    blocking(move || db.with_conn(|conn| tag_repo::delete(conn, &id))).await
}

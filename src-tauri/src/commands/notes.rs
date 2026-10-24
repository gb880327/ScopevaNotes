use tauri::State;

use crate::commands::blocking;
use crate::models::{AppResult, NoteDetail, NoteSummaryPage};
use crate::repositories::note_repo::{self, Cursor, ListQuery};
use crate::state::AppState;

#[tauri::command]
pub async fn list_notes(
    state: State<'_, AppState>,
    query: Option<String>,
    tag_ids: Option<Vec<String>>,
    favorite: Option<bool>,
    limit: Option<i64>,
    cursor: Option<String>,
) -> AppResult<NoteSummaryPage> {
    let db = state.db();
    let query = ListQuery {
        search: query,
        tag_ids: tag_ids.unwrap_or_default(),
        favorites_only: favorite.unwrap_or(false),
        limit,
        cursor: cursor.as_deref().map(Cursor::decode).transpose()?,
    };
    blocking(move || db.with_conn(|conn| note_repo::list(conn, &query))).await
}

#[tauri::command]
pub async fn get_note(state: State<'_, AppState>, id: String) -> AppResult<NoteDetail> {
    let db = state.db();
    blocking(move || db.with_conn(|conn| note_repo::get(conn, &id))).await
}

#[tauri::command]
pub async fn create_note(
    state: State<'_, AppState>,
    title: Option<String>,
    content: Option<String>,
    tag_ids: Option<Vec<String>>,
) -> AppResult<NoteDetail> {
    let db = state.db();
    blocking(move || {
        db.with_conn(|conn| {
            note_repo::create(
                conn,
                title.as_deref().unwrap_or_default(),
                content.as_deref().unwrap_or_default(),
                &tag_ids.unwrap_or_default(),
            )
        })
    })
    .await
}

#[tauri::command]
pub async fn update_note(
    state: State<'_, AppState>,
    id: String,
    title: String,
    content: String,
    tag_ids: Option<Vec<String>>,
    expected_updated_at: i64,
) -> AppResult<NoteDetail> {
    let db = state.db();
    blocking(move || {
        db.with_conn(|conn| {
            note_repo::update(
                conn,
                &id,
                &title,
                &content,
                &tag_ids.unwrap_or_default(),
                expected_updated_at,
            )
        })
    })
    .await
}

#[tauri::command]
pub async fn set_note_favorite(
    state: State<'_, AppState>,
    id: String,
    value: bool,
) -> AppResult<()> {
    let db = state.db();
    blocking(move || db.with_conn(|conn| note_repo::set_favorite(conn, &id, value))).await
}

#[tauri::command]
pub async fn soft_delete_note(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let db = state.db();
    blocking(move || db.with_conn(|conn| note_repo::soft_delete(conn, &id))).await
}

#[tauri::command]
pub async fn restore_note(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let db = state.db();
    blocking(move || db.with_conn(|conn| note_repo::restore(conn, &id))).await
}

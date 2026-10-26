use tauri::State;

use crate::commands::blocking;
use crate::models::{AppResult, QuickTab};
use crate::repositories::quick_tab_repo;
use crate::state::AppState;

/// Every command here touches only `quick_tabs`. Nothing in this module may
/// reach for `note_repo` — the isolation in design D1 is enforced by review as
/// well as by schema, and `tests/quick_tabs.rs` asserts it.
#[tauri::command]
pub async fn list_quick_tabs(state: State<'_, AppState>) -> AppResult<Vec<QuickTab>> {
    let db = state.db();
    blocking(move || db.with_conn(|conn| quick_tab_repo::list(conn))).await
}

#[tauri::command]
pub async fn create_quick_tab(
    state: State<'_, AppState>,
    title: Option<String>,
    color: Option<String>,
) -> AppResult<QuickTab> {
    let db = state.db();
    let title = title.unwrap_or_default();
    let color = color.filter(|value| !value.trim().is_empty());
    blocking(move || db.with_conn(|conn| quick_tab_repo::create(conn, &title, color.as_deref())))
        .await
}

#[tauri::command]
pub async fn update_quick_tab(
    state: State<'_, AppState>,
    id: String,
    title: Option<String>,
    color: Option<String>,
    content: Option<String>,
) -> AppResult<QuickTab> {
    let db = state.db();
    let color = color.filter(|value| !value.trim().is_empty());
    blocking(move || {
        db.with_conn(|conn| {
            quick_tab_repo::update(
                conn,
                &id,
                title.as_deref(),
                color.as_deref(),
                content.as_deref(),
            )
        })
    })
    .await
}

#[tauri::command]
pub async fn delete_quick_tab(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let db = state.db();
    blocking(move || db.with_conn(|conn| quick_tab_repo::delete(conn, &id))).await
}

#[tauri::command]
pub async fn duplicate_quick_tab(state: State<'_, AppState>, id: String) -> AppResult<QuickTab> {
    let db = state.db();
    blocking(move || db.with_conn(|conn| quick_tab_repo::duplicate(conn, &id))).await
}

/// The cap, for the UI to disable `+` before the user hits the error. Still
/// enforced on the write path (design D8).
#[tauri::command]
pub async fn quick_tab_limits() -> AppResult<QuickTabLimits> {
    Ok(QuickTabLimits {
        max_tabs: quick_tab_repo::MAX_TABS,
        colors: quick_tab_repo::COLORS
            .iter()
            .map(|color| color.to_string())
            .collect(),
    })
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickTabLimits {
    pub max_tabs: i64,
    pub colors: Vec<String>,
}

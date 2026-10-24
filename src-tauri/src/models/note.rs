use serde::{Deserialize, Serialize};

use super::Tag;

/// List payload: never carries full Markdown (see plan §4.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteSummary {
    pub id: String,
    pub title: String,
    pub excerpt: String,
    pub updated_at: i64,
    pub is_favorite: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteSummaryPage {
    pub items: Vec<NoteSummary>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteDetail {
    pub id: String,
    pub title: String,
    pub content: String,
    pub is_favorite: bool,
    pub deleted_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub tags: Vec<Tag>,
}

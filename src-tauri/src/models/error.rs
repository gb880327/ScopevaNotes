use serde::{Serialize, Serializer};

/// Every Tauri command returns `Result<T, AppError>`; the TS layer normalises
/// the rejection payload into the same `kind` / `message` shape.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("tauri error: {0}")]
    Tauri(#[from] tauri::Error),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("invalid input: {0}")]
    Invalid(String),
    /// The database file is held by another process (cloud sync, a second
    /// copy, a backup tool). Kept separate from `Database` so the UI can tell
    /// the user to retry instead of showing a raw SQLite string.
    #[error("database busy: {0}")]
    Busy(String),
    #[error("{0}")]
    Other(String),
}

impl AppError {
    /// SQLite reports lock contention as a generic database error; promote it
    /// so callers can treat it as retryable (spec: library / 云同步与锁冲突).
    pub fn classify(self) -> Self {
        if let Self::Database(err) = &self {
            let text = err.to_string().to_lowercase();
            if text.contains("locked") || text.contains("busy") {
                return Self::Busy(text);
            }
        }
        self
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::Database(_) => "database",
            Self::Io(_) => "io",
            Self::Json(_) => "json",
            Self::Tauri(_) => "tauri",
            Self::NotFound(_) => "not_found",
            Self::Conflict(_) => "conflict",
            Self::Invalid(_) => "invalid",
            Self::Busy(_) => "busy",
            Self::Other(_) => "other",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("kind", self.kind())?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}

pub type AppResult<T> = Result<T, AppError>;

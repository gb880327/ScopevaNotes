export type AppErrorKind =
  | 'database'
  | 'io'
  | 'json'
  | 'tauri'
  | 'not_found'
  | 'conflict'
  | 'invalid'
  | 'busy'
  | 'other'

/** Shape produced by `models/error.rs` and by the `commands.ts` normaliser. */
export interface AppError {
  kind: AppErrorKind
  message: string
}

export interface Tag {
  id: string
  name: string
  color: string | null
}

export interface TagWithCount extends Tag {
  noteCount: number
}

export interface NoteSummary {
  id: string
  title: string
  excerpt: string
  updatedAt: number
  isFavorite: boolean
  tags: string[]
  /** Set only for trash rows: when the note was deleted, not when it was edited. */
  deletedAt: number | null
}

export interface NoteSummaryPage {
  items: NoteSummary[]
  nextCursor: string | null
}

export interface NoteDetail {
  id: string
  title: string
  content: string
  isFavorite: boolean
  deletedAt: number | null
  createdAt: number
  updatedAt: number
  tags: Tag[]
}

/** Fixed colour keys for 速记 dots; the CSS mapping lives in `QuickDot.vue`. */
export type QuickTabColor =
  | 'yellow'
  | 'orange'
  | 'red'
  | 'purple'
  | 'blue'
  | 'cyan'
  | 'green'

/** Mirrors `models/quick_tab.rs`. Isolated from notes by design (spec: 隔离). */
export interface QuickTab {
  id: string
  title: string
  color: string
  content: string
  sortOrder: number
  createdAt: number
  updatedAt: number
}

/** Mirrors `commands/quick_tabs.rs::QuickTabLimits`. */
export interface QuickTabLimits {
  maxTabs: number
  colors: string[]
}

export type LibraryNoticeKind = 'fallback' | 'cloud-sync'

/** Mirrors `library_service::LibraryNotice`. */
export interface LibraryNotice {
  kind: LibraryNoticeKind
  message: string
  path: string | null
}

export interface LibraryInfo {
  path: string
  schemaVersion: number
  isDefault: boolean
  noteCount: number
  fileSizeBytes: number
  notice: LibraryNotice | null
}

export interface ListNotesArgs {
  query?: string
  tagIds?: string[]
  favorite?: boolean
  limit?: number
  cursor?: string
}

/** Mirrors `import_service::ImportFailure`. */
export interface ImportFailure {
  path: string
  reason: string
}

/** Mirrors `import_service::ImportPreview` (dry run, nothing written). */
export interface ImportPreview {
  root: string
  totalFiles: number
  toImport: number
  skippedExisting: number
  hiddenSkipped: number
  localFileRefs: number
  unreadable: ImportFailure[]
}

/** Mirrors `import_service::ImportOptions`. */
export interface ImportOptions {
  batchSize?: number
  parentDirTag?: boolean
}

/** Mirrors `import_service::ImportProgress` (`ui://import-progress`). */
export interface ImportProgress {
  done: number
  total: number
  imported: number
}

/** Mirrors `import_service::ImportReport`. */
export interface ImportReport {
  imported: number
  skippedExisting: number
  failed: ImportFailure[]
  hiddenSkipped: number
  decodedAsGb18030: number
  localFileReferences: number
  cancelled: boolean
  elapsedMs: number
  batchTag: string
  totalFiles: number
}

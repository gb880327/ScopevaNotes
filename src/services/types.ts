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

import { invoke, isTauri } from '@tauri-apps/api/core'
import type {
  AppError,
  AppErrorKind,
  LibraryInfo,
  ListNotesArgs,
  NoteDetail,
  NoteSummary,
  NoteSummaryPage,
  Tag,
  TagWithCount,
} from './types'

export class CommandError extends Error {
  readonly kind: AppErrorKind

  constructor(kind: AppErrorKind, message: string) {
    super(message)
    this.name = 'CommandError'
    this.kind = kind
  }

  static from(payload: unknown): CommandError {
    if (payload && typeof payload === 'object' && 'kind' in payload && 'message' in payload) {
      const error = payload as AppError
      return new CommandError(error.kind, error.message)
    }
    if (payload instanceof Error) {
      return new CommandError('other', payload.message)
    }
    return new CommandError('other', String(payload))
  }
}

export const isDesktop = isTauri()

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isDesktop) {
    throw new CommandError('other', '需要在 Tauri 应用内运行（pnpm tauri dev）')
  }
  try {
    return await invoke<T>(command, args)
  } catch (payload) {
    throw CommandError.from(payload)
  }
}

export const api = {
  listNotes: (args: ListNotesArgs = {}) => call<NoteSummaryPage>('list_notes', { ...args }),
  listDeletedNotes: (limit?: number) =>
    call<NoteSummary[]>('list_deleted_notes', { limit }),
  getNote: (id: string) => call<NoteDetail>('get_note', { id }),
  createNote: (args: { title?: string; content?: string; tagIds?: string[] } = {}) =>
    call<NoteDetail>('create_note', args),
  updateNote: (args: {
    id: string
    title: string
    content: string
    tagIds?: string[]
    expectedUpdatedAt: number
    force?: boolean
  }) => call<NoteDetail>('update_note', args),
  setNoteFavorite: (id: string, value: boolean) =>
    call<void>('set_note_favorite', { id, value }),
  softDeleteNote: (id: string) => call<void>('soft_delete_note', { id }),
  restoreNote: (id: string) => call<void>('restore_note', { id }),

  listTags: () => call<TagWithCount[]>('list_tags'),
  createTag: (name: string, color?: string | null) =>
    call<Tag>('create_tag', { name, color: color ?? null }),
  renameTag: (id: string, name: string) => call<Tag>('rename_tag', { id, name }),
  deleteTag: (id: string) => call<void>('delete_tag', { id }),

  getLibraryInfo: () => call<LibraryInfo>('get_library_info'),
  createLibrary: (path: string) => call<LibraryInfo>('create_library', { path }),
  openLibrary: (path: string) => call<LibraryInfo>('open_library', { path }),
  moveLibrary: (path: string) => call<LibraryInfo>('move_library', { path }),
  backupLibrary: (destination: string) =>
    call<number>('backup_library', { destination }),

  toggleMainWindow: () => call<boolean>('toggle_main_window'),

  getGlobalShortcut: () => call<string>('get_global_shortcut'),
  setGlobalShortcut: (accelerator: string) =>
    call<string>('set_global_shortcut', { accelerator }),

  /** Ask for the quit confirmation dialog; the app exits only if confirmed. */
  requestQuit: () => call<void>('request_quit'),
}

export function errorMessage(err: unknown): string {
  if (err instanceof CommandError) return err.message
  if (err instanceof Error) return err.message
  return String(err)
}

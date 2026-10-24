import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, CommandError, errorMessage } from '@/services/commands'
import type { NoteDetail, NoteSummary, TagWithCount } from '@/services/types'
import { createAutosaveQueue } from '@/composables/useAutosave'

export type SaveStatus = 'idle' | 'saving' | 'saved' | 'error' | 'conflict'

function summarize(note: NoteDetail): NoteSummary {
  return {
    id: note.id,
    title: note.title,
    excerpt: note.content.split(/\s+/).join(' ').slice(0, 160),
    updatedAt: note.updatedAt,
    isFavorite: note.isFavorite,
    tags: note.tags.map((tag) => tag.name),
  }
}

export const useNotesStore = defineStore('notes', () => {
  const items = ref<NoteSummary[]>([])
  const selected = ref<NoteDetail | null>(null)
  const tags = ref<TagWithCount[]>([])
  const query = ref('')
  const activeTagIds = ref<string[]>([])
  const favoritesOnly = ref(false)
  const loading = ref(false)
  const error = ref<string | null>(null)
  const saveStatus = ref<SaveStatus>('idle')
  const nextCursor = ref<string | null>(null)
  const deleted = ref<NoteSummary[]>([])
  const trashOpen = ref(false)
  /** Non-null while the on-disk row is newer than what the editor holds. */
  const conflictNoteId = ref<string | null>(null)

  // Bumped on every local keystroke; a save response may only overwrite the
  // buffer when it still matches (plan §5 optimistic concurrency).
  let editVersion = 0

  const queue = createAutosaveQueue(saveNow, 600)

  async function refresh() {
    loading.value = true
    error.value = null
    try {
      const page = await api.listNotes({
        query: query.value || undefined,
        tagIds: activeTagIds.value.length ? activeTagIds.value : undefined,
        favorite: favoritesOnly.value || undefined,
      })
      items.value = page.items
      nextCursor.value = page.nextCursor
    } catch (err) {
      error.value = errorMessage(err)
    } finally {
      loading.value = false
    }
  }

  async function loadMore() {
    if (!nextCursor.value) return
    try {
      const page = await api.listNotes({
        query: query.value || undefined,
        tagIds: activeTagIds.value.length ? activeTagIds.value : undefined,
        favorite: favoritesOnly.value || undefined,
        cursor: nextCursor.value,
      })
      items.value = [...items.value, ...page.items]
      nextCursor.value = page.nextCursor
    } catch (err) {
      error.value = errorMessage(err)
    }
  }

  async function refreshTags() {
    try {
      tags.value = await api.listTags()
    } catch (err) {
      error.value = errorMessage(err)
    }
  }

  async function selectNote(id: string) {
    await queue.flush()
    conflictNoteId.value = null
    error.value = null
    try {
      selected.value = await api.getNote(id)
      saveStatus.value = 'idle'
    } catch (err) {
      error.value = errorMessage(err)
    }
  }

  async function openTrash() {
    trashOpen.value = true
    error.value = null
    try {
      deleted.value = await api.listDeletedNotes()
    } catch (err) {
      error.value = errorMessage(err)
    }
  }

  function closeTrash() {
    trashOpen.value = false
    deleted.value = []
  }

  async function restoreNote(id: string) {
    try {
      await api.restoreNote(id)
      deleted.value = deleted.value.filter((item) => item.id !== id)
      await refresh()
      await refreshTags()
    } catch (err) {
      error.value = errorMessage(err)
    }
  }

  async function createNote() {
    await queue.flush()
    try {
      const note = await api.createNote({})
      items.value = [summarize(note), ...items.value]
      selected.value = note
      saveStatus.value = 'idle'
    } catch (err) {
      error.value = errorMessage(err)
    }
  }

  function editTitle(value: string) {
    if (!selected.value) return
    selected.value.title = value
    markDirty()
  }

  function editContent(value: string) {
    if (!selected.value) return
    selected.value.content = value
    markDirty()
  }

  function markDirty() {
    editVersion += 1
    saveStatus.value = 'idle'
    queue.schedule()
  }

  async function saveNow() {
    const note = selected.value
    if (!note) return
    const version = editVersion
    saveStatus.value = 'saving'
    try {
      const saved = await api.updateNote({
        id: note.id,
        title: note.title,
        content: note.content,
        tagIds: note.tags.map((tag) => tag.id),
        expectedUpdatedAt: note.updatedAt,
      })
      if (selected.value?.id === saved.id) {
        selected.value.updatedAt = saved.updatedAt
        if (editVersion === version) {
          selected.value.title = saved.title
          selected.value.content = saved.content
        }
      }
      const index = items.value.findIndex((item) => item.id === saved.id)
      if (index >= 0) items.value[index] = summarize(saved)
      saveStatus.value = 'saved'
      error.value = null
    } catch (err) {
      if (err instanceof CommandError && err.kind === 'conflict') {
        // Keep the editor buffer; the user picks reload or overwrite.
        saveStatus.value = 'conflict'
        conflictNoteId.value = note.id
        error.value = null
        return
      }
      saveStatus.value = 'error'
      error.value = errorMessage(err)
      throw err
    }
  }

  async function resolveConflictReload() {
    const id = conflictNoteId.value
    if (!id) return
    queue.discard()
    try {
      selected.value = await api.getNote(id)
      conflictNoteId.value = null
      saveStatus.value = 'idle'
      await refresh()
    } catch (err) {
      error.value = errorMessage(err)
    }
  }

  async function resolveConflictOverwrite() {
    const note = selected.value
    if (!note) return
    queue.discard()
    try {
      const saved = await api.updateNote({
        id: note.id,
        title: note.title,
        content: note.content,
        tagIds: note.tags.map((tag) => tag.id),
        expectedUpdatedAt: note.updatedAt,
        force: true,
      })
      selected.value = saved
      conflictNoteId.value = null
      saveStatus.value = 'saved'
      const index = items.value.findIndex((item) => item.id === saved.id)
      if (index >= 0) items.value[index] = summarize(saved)
    } catch (err) {
      error.value = errorMessage(err)
    }
  }

  async function toggleFavorite() {
    if (!selected.value) return
    const next = !selected.value.isFavorite
    try {
      await api.setNoteFavorite(selected.value.id, next)
      selected.value.isFavorite = next
      const index = items.value.findIndex((item) => item.id === selected.value?.id)
      if (index >= 0) items.value[index] = { ...items.value[index], isFavorite: next }
    } catch (err) {
      error.value = errorMessage(err)
    }
  }

  async function deleteSelected() {
    if (!selected.value) return
    const id = selected.value.id
    await queue.flush()
    try {
      await api.softDeleteNote(id)
      selected.value = null
      items.value = items.value.filter((item) => item.id !== id)
    } catch (err) {
      error.value = errorMessage(err)
    }
  }

  async function createTag(name: string) {
    try {
      await api.createTag(name)
      await refreshTags()
    } catch (err) {
      error.value = errorMessage(err)
    }
  }

  function toggleTagFilter(id: string) {
    activeTagIds.value = activeTagIds.value.includes(id)
      ? activeTagIds.value.filter((tagId) => tagId !== id)
      : [...activeTagIds.value, id]
    void refresh()
  }

  function clearTagFilters() {
    activeTagIds.value = []
    void refresh()
  }

  function clearFilters() {
    activeTagIds.value = []
    favoritesOnly.value = false
    query.value = ''
    void refresh()
  }

  function flush() {
    return queue.flush()
  }

  return {
    items,
    selected,
    tags,
    query,
    activeTagIds,
    favoritesOnly,
    loading,
    error,
    saveStatus,
    nextCursor,
    deleted,
    trashOpen,
    conflictNoteId,
    refresh,
    openTrash,
    resolveConflictReload,
    resolveConflictOverwrite,
    closeTrash,
    restoreNote,
    loadMore,
    refreshTags,
    selectNote,
    createNote,
    editTitle,
    editContent,
    toggleFavorite,
    deleteSelected,
    createTag,
    toggleTagFilter,
    clearTagFilters,
    clearFilters,
    flush,
  }
})

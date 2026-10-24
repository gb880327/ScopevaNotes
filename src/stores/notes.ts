import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, CommandError, errorMessage } from '@/services/commands'
import type { NoteDetail, NoteSummary, Tag, TagWithCount } from '@/services/types'
import { createAutosaveQueue } from '@/composables/useAutosave'

export type SaveStatus = 'idle' | 'saving' | 'saved' | 'error' | 'conflict'

/**
 * Thrown by `createAndAttachTag` when the tag row was created but hanging it on
 * the note failed. The tag exists in the sidebar, so the caller must say so
 * instead of pretending nothing happened.
 */
export class TagAttachError extends Error {
  readonly tagCreated = true

  constructor(message: string) {
    super(message)
    this.name = 'TagAttachError'
  }
}

function summarize(note: NoteDetail): NoteSummary {
  return {
    id: note.id,
    title: note.title,
    excerpt: note.content.split(/\s+/).join(' ').slice(0, 160),
    updatedAt: note.updatedAt,
    isFavorite: note.isFavorite,
    tags: note.tags.map((tag) => tag.name),
    deletedAt: note.deletedAt,
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
  /** Whole-trash size, not `deleted.length`: the listing is capped by `limit`. */
  const deletedCount = ref(0)
  const trashOpen = ref(false)
  /** Non-null while the on-disk row is newer than what the editor holds. */
  const conflictNoteId = ref<string | null>(null)

  // Bumped on every local keystroke; a save response may only overwrite the
  // buffer when it still matches (plan §5 optimistic concurrency).
  let editVersion = 0

  // Bumped on every list request; only the newest response may replace `items`.
  // Without this a slow query for "sq" could land after the query for "sql".
  let listSeq = 0

  const queue = createAutosaveQueue(saveNow, 600)

  async function refresh() {
    const seq = ++listSeq
    loading.value = true
    error.value = null
    try {
      const page = await api.listNotes({
        query: query.value || undefined,
        tagIds: activeTagIds.value.length ? activeTagIds.value : undefined,
        favorite: favoritesOnly.value || undefined,
      })
      if (seq !== listSeq) return
      items.value = page.items
      nextCursor.value = page.nextCursor
    } catch (err) {
      if (seq !== listSeq) return
      error.value = errorMessage(err)
    } finally {
      if (seq === listSeq) loading.value = false
    }
  }

  async function loadMore() {
    if (!nextCursor.value) return
    const seq = listSeq
    try {
      const page = await api.listNotes({
        query: query.value || undefined,
        tagIds: activeTagIds.value.length ? activeTagIds.value : undefined,
        favorite: favoritesOnly.value || undefined,
        cursor: nextCursor.value,
      })
      // A newer query started while this page was in flight: drop it.
      if (seq !== listSeq) return
      items.value = [...items.value, ...page.items]
      nextCursor.value = page.nextCursor
    } catch (err) {
      if (seq !== listSeq) return
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

  async function refreshTrash() {
    const [items, count] = await Promise.all([
      api.listDeletedNotes(),
      api.countDeletedNotes(),
    ])
    deleted.value = items
    deletedCount.value = count
  }

  async function openTrash() {
    trashOpen.value = true
    error.value = null
    try {
      await refreshTrash()
    } catch (err) {
      error.value = errorMessage(err)
    }
  }

  function closeTrash() {
    trashOpen.value = false
    deleted.value = []
    deletedCount.value = 0
  }

  /**
   * Permanent deletes are the only path that destroys data, so they never touch
   * `selected` optimistically: the command runs first and the trash is re-read
   * afterwards. A failure leaves both lists as they were and surfaces `error`.
   */
  async function purgeNote(id: string) {
    error.value = null
    try {
      await api.purgeNote(id)
      await refreshTrash()
      await refresh()
      await refreshTags()
      if (selected.value?.id === id) selected.value = null
    } catch (err) {
      error.value = errorMessage(err)
      throw err
    }
  }

  async function emptyTrash() {
    error.value = null
    try {
      await api.emptyTrash()
      await refreshTrash()
      await refresh()
      await refreshTags()
    } catch (err) {
      error.value = errorMessage(err)
      throw err
    }
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

  async function createTag(name: string): Promise<Tag> {
    try {
      const tag = await api.createTag(name)
      await refreshTags()
      return tag
    } catch (err) {
      error.value = errorMessage(err)
      throw err
    }
  }

  /** Rename the tag entity itself; notes keep their association. */
  async function renameTag(id: string, name: string): Promise<void> {
    try {
      const renamed = await api.renameTag(id, name)
      const entry = tags.value.find((tag) => tag.id === id)
      if (entry) entry.name = renamed.name
      const onNote = selected.value?.tags.find((tag) => tag.id === id)
      if (onNote) onNote.name = renamed.name
      await refreshTags()
      // Tag names are part of the search projection, so hits may change too.
      await refresh()
    } catch (err) {
      error.value = errorMessage(err)
      throw err
    }
  }

  async function setTagColor(id: string, color: string | null): Promise<void> {
    try {
      const updated = await api.setTagColor(id, color)
      const entry = tags.value.find((tag) => tag.id === id)
      if (entry) entry.color = updated.color
      const onNote = selected.value?.tags.find((tag) => tag.id === id)
      if (onNote) onNote.color = updated.color
      await refreshTags()
    } catch (err) {
      error.value = errorMessage(err)
      throw err
    }
  }

  /** Delete the entity. `note_tags` cascades; no note is ever deleted. */
  async function deleteTag(id: string): Promise<void> {
    try {
      await api.deleteTag(id)
      tags.value = tags.value.filter((tag) => tag.id !== id)
      activeTagIds.value = activeTagIds.value.filter((tagId) => tagId !== id)
      if (selected.value) {
        selected.value.tags = selected.value.tags.filter((tag) => tag.id !== id)
      }
      await refreshTags()
      await refresh()
    } catch (err) {
      error.value = errorMessage(err)
      throw err
    }
  }

  /**
   * Apply a tag change to the open note and persist it immediately.
   *
   * Tag edits reuse the normal `update_note` path (one write path, one index
   * rebuild) instead of dedicated add/remove commands. The debounce is dropped
   * so the caller can await the write and surface a failure right away.
   */
  async function commitNoteTags(mutate: (note: NoteDetail) => void): Promise<void> {
    const note = selected.value
    if (!note) return
    mutate(note)
    editVersion += 1
    saveStatus.value = 'idle'
    queue.discard()
    await saveNow()
  }

  async function attachTagObject(tag: Tag): Promise<void> {
    const note = selected.value
    if (!note || note.tags.some((current) => current.id === tag.id)) return
    await commitNoteTags((current) => {
      current.tags = [...current.tags, { id: tag.id, name: tag.name, color: tag.color }]
    })
    await refreshTags()
  }

  async function attachTag(tagId: string): Promise<void> {
    const tag = tags.value.find((entry) => entry.id === tagId)
    if (!tag) return
    await attachTagObject(tag)
  }

  async function detachTag(tagId: string): Promise<void> {
    if (!selected.value) return
    await commitNoteTags((note) => {
      note.tags = note.tags.filter((tag) => tag.id !== tagId)
    })
    await refreshTags()
  }

  /** Create-if-absent, then hang it on the open note. */
  async function createAndAttachTag(name: string): Promise<void> {
    const created = await createTag(name)
    try {
      await attachTagObject(created)
    } catch (err) {
      throw new TagAttachError(
        `${errorMessage(err)}（标签「${created.name}」已创建，但没有挂到这条笔记上）`,
      )
    }
  }

  // Single-select by design: the sidebar shows one tag filter at a time, so picking
  // another tag replaces the current one instead of narrowing further. Clicking the
  // active tag again clears the filter. The command still accepts a list; this is a
  // UI affordance, not a transport limit.
  function toggleTagFilter(id: string) {
    activeTagIds.value = activeTagIds.value.includes(id) ? [] : [id]
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
    deletedCount,
    trashOpen,
    conflictNoteId,
    refresh,
    openTrash,
    resolveConflictReload,
    resolveConflictOverwrite,
    closeTrash,
    restoreNote,
    purgeNote,
    emptyTrash,
    loadMore,
    refreshTags,
    selectNote,
    createNote,
    editTitle,
    editContent,
    toggleFavorite,
    deleteSelected,
    createTag,
    renameTag,
    setTagColor,
    deleteTag,
    attachTag,
    detachTag,
    createAndAttachTag,
    toggleTagFilter,
    clearTagFilters,
    clearFilters,
    flush,
  }
})

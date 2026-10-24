<script setup lang="ts">
import { computed, ref } from 'vue'
import { useNotesStore } from '@/stores/notes'
import { useUiStore } from '@/stores/ui'
import { copyText } from '@/services/clipboard'
import { errorMessage } from '@/services/commands'
import { formatRelativeTime } from '@/services/format'

const notes = useNotesStore()
const ui = useUiStore()
const copied = ref(false)
const tagPickerOpen = ref(false)
const newTagName = ref('')
const tagError = ref<string | null>(null)

const note = computed(() => notes.selected)
const isEditing = computed(() => ui.viewMode === 'edit')

/** Tags the open note does not carry yet; the picker offers these. */
const attachable = computed(() => {
  const attached = new Set(note.value?.tags.map((tag) => tag.id))
  return notes.tags.filter((tag) => !attached.has(tag.id))
})

async function attach(tagId: string) {
  tagError.value = null
  try {
    await notes.attachTag(tagId)
    tagPickerOpen.value = false
  } catch (err) {
    tagError.value = errorMessage(err)
  }
}

async function detach(tagId: string) {
  tagError.value = null
  try {
    await notes.detachTag(tagId)
  } catch (err) {
    tagError.value = errorMessage(err)
  }
}

async function createTag() {
  const value = newTagName.value.trim()
  if (!value) return
  tagError.value = null
  try {
    await notes.createAndAttachTag(value)
    newTagName.value = ''
    tagPickerOpen.value = false
  } catch (err) {
    tagError.value = errorMessage(err)
  }
}

async function copyAll() {
  if (!note.value) return
  await copyText(note.value.content)
  copied.value = true
  setTimeout(() => (copied.value = false), 1200)
}

async function finishEditing() {
  await notes.flush()
  ui.viewMode = 'read'
}
</script>

<template>
  <header v-if="note" class="note-header">
    <div class="row">
      <input
        v-if="isEditing"
        class="title-input"
        :value="note.title"
        placeholder="无标题"
        @input="notes.editTitle(($event.target as HTMLInputElement).value)"
      />
      <h1 v-else class="title">{{ note.title || '无标题' }}</h1>

      <div class="actions">
        <button
          type="button"
          class="icon"
          :title="note.isFavorite ? '取消收藏' : '收藏'"
          @click="notes.toggleFavorite()"
        >
          {{ note.isFavorite ? '★' : '☆' }}
        </button>
        <button type="button" class="ghost" @click="copyAll">
          {{ copied ? '已复制' : '复制全文' }}
        </button>
        <button type="button" class="ghost" @click="isEditing ? finishEditing() : (ui.viewMode = 'edit')">
          {{ isEditing ? '完成' : '编辑' }}
        </button>
        <button type="button" class="icon danger" title="移到回收站" @click="notes.deleteSelected()">
          🗑
        </button>
      </div>
    </div>

    <div class="meta">
      <span v-for="tag in note.tags" :key="tag.id" class="chip">
        <span class="tag">{{ tag.name }}</span>
        <button
          type="button"
          class="chip-x"
          :aria-label="`移除标签 ${tag.name}`"
          @click="detach(tag.id)"
        >
          ×
        </button>
      </span>
      <button
        type="button"
        class="chip-add"
        aria-label="添加标签"
        @click="tagPickerOpen = !tagPickerOpen"
      >
        ＋
      </button>

      <span class="time">{{ formatRelativeTime(note.updatedAt) }}</span>
    </div>

    <div v-if="tagPickerOpen" class="tag-picker">
      <button
        v-for="tag in attachable"
        :key="tag.id"
        type="button"
        class="option"
        @click="attach(tag.id)"
      >
        <span class="dot" :style="{ background: tag.color ?? 'var(--accent)' }" />
        {{ tag.name }}
      </button>
      <p v-if="!attachable.length" class="hint">没有其它标签</p>
      <input
        v-model="newTagName"
        class="new-tag"
        placeholder="新标签，回车创建"
        @keydown.enter.prevent="createTag"
      />
    </div>

    <p v-if="tagError" class="tag-error" role="alert">{{ tagError }}</p>
  </header>
</template>

<style scoped>
.note-header {
  padding: 14px 24px 10px;
  border-bottom: 1px solid var(--border);
}

.row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.title {
  flex: 1;
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  line-height: 1.3;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.title-input {
  flex: 1;
  border: 1px solid transparent;
  background: transparent;
  font-size: 18px;
  font-weight: 600;
  padding: 2px 4px;
  border-radius: 5px;
}

.title-input:focus {
  outline: none;
  border-color: var(--border-strong);
  background: var(--bg);
}

.actions {
  display: flex;
  align-items: center;
  gap: 6px;
}

.ghost,
.icon {
  border: 1px solid var(--border);
  background: var(--bg-elevated);
  border-radius: 6px;
  padding: 4px 9px;
  cursor: pointer;
  font-size: 12px;
  color: var(--text-muted);
}

.icon {
  padding: 4px 7px;
}

.ghost:hover,
.icon:hover {
  border-color: var(--border-strong);
  color: var(--text);
}

.danger:hover {
  color: var(--danger);
  border-color: var(--danger);
}

.meta {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 8px;
  font-size: 11px;
  color: var(--text-faint);
  overflow-x: auto;
  scrollbar-width: none;
}

.meta::-webkit-scrollbar {
  display: none;
}

.chip {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  background: var(--bg-input);
  border-radius: 999px;
  padding: 1px 4px 1px 8px;
  flex-shrink: 0;
}

.tag {
  color: var(--text-muted);
  white-space: nowrap;
}

.chip-x,
.chip-add {
  border: 0;
  background: none;
  color: var(--text-faint);
  cursor: pointer;
  font-size: 11px;
  line-height: 1;
  padding: 1px 3px;
  border-radius: 999px;
}

.chip-x:hover,
.chip-add:hover {
  color: var(--text);
  background: var(--hover);
}

.chip-add {
  border: 1px dashed var(--border-strong);
  border-radius: 999px;
  padding: 1px 7px;
  flex-shrink: 0;
}

/* In flow under the chip row: `.meta` scrolls horizontally, so an absolute
   child would be clipped to the one-line height. */
.tag-picker {
  margin-top: 6px;
  max-width: 320px;
  max-height: 220px;
  overflow-y: auto;
  padding: 6px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-elevated);
}

.option {
  display: flex;
  align-items: center;
  gap: 7px;
  width: 100%;
  border: 0;
  background: none;
  border-radius: 5px;
  padding: 4px 6px;
  cursor: pointer;
  font-size: 12px;
  color: var(--text);
  text-align: left;
}

.option:hover {
  background: var(--hover);
}

.dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
}

.hint {
  margin: 4px 6px;
  font-size: 11px;
  color: var(--text-faint);
}

.new-tag {
  width: 100%;
  margin-top: 4px;
  padding: 4px 8px;
  border: 1px solid var(--border-strong);
  border-radius: 6px;
  background: var(--bg);
  font-size: 12px;
  outline: none;
}

.new-tag:focus {
  border-color: var(--accent);
}

.tag-error {
  margin: 6px 0 0;
  font-size: 11px;
  color: var(--danger);
}

.time {
  margin-left: auto;
}
</style>

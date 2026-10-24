<script setup lang="ts">
import { computed, ref } from 'vue'
import { useNotesStore } from '@/stores/notes'
import { useUiStore } from '@/stores/ui'
import { copyText } from '@/services/clipboard'
import { formatRelativeTime } from '@/services/format'

const notes = useNotesStore()
const ui = useUiStore()
const copied = ref(false)

const note = computed(() => notes.selected)
const isEditing = computed(() => ui.viewMode === 'edit')

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
      <span v-for="tag in note.tags" :key="tag.id" class="tag">{{ tag.name }}</span>
      <span class="time">{{ formatRelativeTime(note.updatedAt) }}</span>
    </div>
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
}

.tag {
  background: var(--bg-input);
  border-radius: 999px;
  padding: 1px 8px;
  color: var(--text-muted);
}

.time {
  margin-left: auto;
}
</style>

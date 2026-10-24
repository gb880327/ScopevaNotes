<script setup lang="ts">
import { useNotesStore } from '@/stores/notes'
import { formatRelativeTime } from '@/services/format'

const notes = useNotesStore()
</script>

<template>
  <div class="trash">
    <div class="head">
      <span>回收站</span>
      <button type="button" class="link" @click="notes.closeTrash()">返回</button>
    </div>

    <p v-if="!notes.deleted.length" class="hint">回收站是空的</p>

    <div v-for="item in notes.deleted" :key="item.id" class="item">
      <div class="body">
        <span class="title">{{ item.title || '无标题' }}</span>
        <span class="meta">删除于 {{ formatRelativeTime(item.updatedAt) }}</span>
      </div>
      <button type="button" class="restore" @click="notes.restoreNote(item.id)">恢复</button>
    </div>
  </div>
</template>

<style scoped>
.trash {
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow-y: auto;
  padding: 4px 8px 12px;
}

.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 6px 8px;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-muted);
}

.link {
  border: 0;
  background: none;
  padding: 0;
  font-size: 11px;
  font-weight: 500;
  color: var(--accent-text);
  cursor: pointer;
}

.hint {
  margin: 8px 6px;
  font-size: 12px;
  color: var(--text-faint);
}

.item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 8px;
  border-radius: 6px;
}

.item:hover {
  background: var(--bg-hover);
}

.body {
  display: flex;
  flex-direction: column;
  min-width: 0;
  flex: 1;
}

.title {
  font-size: 12.5px;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.meta {
  font-size: 10.5px;
  color: var(--text-faint);
}

.restore {
  border: 1px solid var(--border);
  background: var(--bg-elevated);
  border-radius: 5px;
  padding: 3px 8px;
  font-size: 11px;
  color: var(--text-muted);
  cursor: pointer;
}

.restore:hover {
  color: var(--text);
  border-color: var(--border-strong);
}
</style>

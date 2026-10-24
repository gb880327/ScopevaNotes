<script setup lang="ts">
import { useNotesStore } from '@/stores/notes'
import { formatRelativeTime } from '@/services/format'

const notes = useNotesStore()
</script>

<template>
  <div class="list" role="listbox" aria-label="笔记列表">
    <p v-if="notes.loading && !notes.items.length" class="hint">载入中…</p>
    <p v-else-if="!notes.items.length" class="hint">
      没有匹配的笔记
      <button type="button" class="link" @click="notes.clearFilters()">清除筛选</button>
    </p>

    <button
      v-for="item in notes.items"
      :key="item.id"
      type="button"
      role="option"
      :aria-selected="notes.selected?.id === item.id"
      class="item"
      :class="{ active: notes.selected?.id === item.id }"
      @click="notes.selectNote(item.id)"
    >
      <span class="line">
        <span class="title">{{ item.title || '无标题' }}</span>
        <span class="time">{{ formatRelativeTime(item.updatedAt) }}</span>
      </span>
      <span class="excerpt">{{ item.excerpt || '（空笔记）' }}</span>
      <span v-if="item.tags.length" class="tags">
        <span v-for="tag in item.tags" :key="tag" class="tag">{{ tag }}</span>
      </span>
    </button>

    <button
      v-if="notes.nextCursor"
      type="button"
      class="link more"
      @click="notes.loadMore()"
    >
      加载更多
    </button>
  </div>
</template>

<style scoped>
.list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 4px 8px;
}

.item {
  display: block;
  width: 100%;
  text-align: left;
  border: 0;
  background: transparent;
  border-radius: 7px;
  padding: 8px 10px;
  cursor: pointer;
}

.item:hover {
  background: var(--hover);
}

.item.active {
  background: var(--selected);
}

.line {
  display: flex;
  align-items: baseline;
  gap: 8px;
}

.title {
  flex: 1;
  font-weight: 500;
  font-size: 13px;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.time {
  font-size: 10.5px;
  color: var(--text-faint);
  flex-shrink: 0;
}

.excerpt {
  display: block;
  margin-top: 2px;
  font-size: 11.5px;
  color: var(--text-muted);
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.tags {
  display: flex;
  gap: 4px;
  margin-top: 5px;
  flex-wrap: wrap;
}

.tag {
  font-size: 10px;
  color: var(--text-muted);
  background: var(--bg-input);
  border-radius: 999px;
  padding: 1px 7px;
}

.hint {
  padding: 16px 10px;
  font-size: 12px;
  color: var(--text-faint);
}

.link {
  border: 0;
  background: none;
  color: var(--accent-text);
  cursor: pointer;
  padding: 0 2px;
  font-size: inherit;
}

.more {
  width: 100%;
  padding: 8px;
}
</style>

<script setup lang="ts">
import { useNotesStore } from '@/stores/notes'
import SearchInput from '@/features/search/SearchInput.vue'
import NoteList from '@/features/notes/NoteList.vue'
import TagList from '@/features/tags/TagList.vue'

const notes = useNotesStore()
</script>

<template>
  <aside class="sidebar">
    <header class="head">
      <span class="brand">Notes</span>
      <button type="button" class="new" title="新建笔记 (⌘N)" @click="notes.createNote()">＋</button>
    </header>

    <SearchInput />

    <div class="chips">
      <button
        type="button"
        class="chip"
        :class="{ active: !notes.favoritesOnly && !notes.activeTagIds.length }"
        @click="notes.clearFilters()"
      >
        全部
      </button>
      <button
        type="button"
        class="chip"
        :class="{ active: notes.favoritesOnly }"
        @click="notes.favoritesOnly = !notes.favoritesOnly; notes.refresh()"
      >
        收藏
      </button>
    </div>

    <NoteList />
    <TagList />
  </aside>
</template>

<style scoped>
.sidebar {
  display: flex;
  flex-direction: column;
  min-height: 0;
  height: 100%;
  background: var(--bg-sidebar);
  border-right: 1px solid var(--border);
}

.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px 10px;
}

.brand {
  font-weight: 600;
  font-size: 13px;
}

.new {
  border: 1px solid var(--border);
  background: var(--bg-elevated);
  border-radius: 6px;
  width: 24px;
  height: 22px;
  cursor: pointer;
  color: var(--text-muted);
  line-height: 1;
}

.new:hover {
  color: var(--text);
  border-color: var(--border-strong);
}

.chips {
  display: flex;
  gap: 6px;
  padding: 0 12px 6px;
}

.chip {
  border: 1px solid transparent;
  background: transparent;
  border-radius: 999px;
  padding: 2px 9px;
  font-size: 11px;
  color: var(--text-muted);
  cursor: pointer;
}

.chip:hover {
  background: var(--hover);
}

.chip.active {
  background: var(--bg-elevated);
  border-color: var(--border);
  color: var(--text);
}
</style>

<script setup lang="ts">
import { computed } from 'vue'
import { useNotesStore } from '@/stores/notes'
import { useUiStore } from '@/stores/ui'
import NoteHeader from '@/features/notes/NoteHeader.vue'
import NotePreview from '@/features/notes/NotePreview.vue'
import NoteEditor from '@/features/notes/NoteEditor.vue'
import { formatRelativeTime } from '@/services/format'

const notes = useNotesStore()
const ui = useUiStore()

const content = computed({
  get: () => notes.selected?.content ?? '',
  set: (value: string) => notes.editContent(value),
})

const statusText = computed(() => {
  switch (notes.saveStatus) {
    case 'saving':
      return '保存中…'
    case 'saved':
      return '已保存'
    case 'conflict':
      return '未保存 · 版本冲突'
    case 'error':
      return '未保存 · 将重试'
    default:
      return ''
  }
})
</script>

<template>
  <main class="content">
    <p v-if="notes.error" class="banner">
      {{ notes.error }}
      <button type="button" @click="notes.error = null">×</button>
    </p>

    <p v-if="notes.conflictNoteId" class="banner conflict">
      这条笔记在别处被修改了，本地改动尚未保存。
      <button type="button" class="action" @click="notes.resolveConflictReload()">
        重新载入
      </button>
      <button type="button" class="action" @click="notes.resolveConflictOverwrite()">
        保留本地并覆盖
      </button>
    </p>

    <template v-if="notes.selected">
      <NoteHeader />
      <section class="scroller" :class="{ editing: ui.viewMode === 'edit' }">
        <NoteEditor v-if="ui.viewMode === 'edit'" v-model="content" />
        <NotePreview v-else :content="notes.selected.content" />
      </section>
      <footer>
        <span>{{ formatRelativeTime(notes.selected.updatedAt) }}更新</span>
        <span v-if="statusText" class="status" :class="notes.saveStatus">{{ statusText }}</span>
      </footer>
    </template>

    <div v-else class="empty">
      <p class="title">还没有选中笔记</p>
      <p class="hint">从左侧选择一条，或新建第一条笔记。</p>
      <button type="button" class="primary" @click="notes.createNote()">新建笔记</button>
    </div>
  </main>
</template>

<style scoped>
.content {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  min-height: 0;
  background: var(--bg);
}

.banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin: 10px 16px 0;
  padding: 7px 10px;
  border: 1px solid var(--danger);
  border-radius: 7px;
  color: var(--danger);
  font-size: 12px;
}

.banner button {
  border: 0;
  background: none;
  color: inherit;
  cursor: pointer;
}

.scroller {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 20px 28px 40px;
}

.scroller.editing {
  padding: 8px 16px 0;
  display: flex;
  flex-direction: column;
}

footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
  padding: 6px 20px;
  border-top: 1px solid var(--border);
  font-size: 10.5px;
  color: var(--text-faint);
}

.status.saving {
  color: var(--text-muted);
}

.status.error,
.status.conflict {
  color: var(--danger);
}
.banner.conflict {
  border-color: var(--accent);
  color: var(--text);
  justify-content: flex-start;
  flex-wrap: wrap;
}
.banner .action {
  padding: 2px 8px;
  border: 1px solid var(--border);
  border-radius: 5px;
  font-size: 11.5px;
  color: var(--text-muted);
}
.banner .action:hover {
  color: var(--text);
  border-color: var(--border-strong);
}

.empty {
  margin: auto;
  text-align: center;
  color: var(--text-muted);
}

.empty .title {
  margin: 0 0 6px;
  color: var(--text);
  font-size: 14px;
  font-weight: 500;
}

.empty .hint {
  margin: 0 0 14px;
  font-size: 12px;
  color: var(--text-faint);
}

.primary {
  border: 0;
  border-radius: 6px;
  padding: 6px 14px;
  background: var(--accent);
  color: #fff;
  font-size: 12px;
  cursor: pointer;
}
</style>

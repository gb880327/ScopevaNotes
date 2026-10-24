<script setup lang="ts">
import { onUnmounted, ref, watch } from 'vue'
import { useNotesStore } from '@/stores/notes'
import { formatRelativeTime } from '@/services/format'

const notes = useNotesStore()
/** Row waiting for the "this is permanent" confirmation. */
const purgingId = ref<string | null>(null)
const clearing = ref(false)

function askPurge(id: string) {
  clearing.value = false
  purgingId.value = purgingId.value === id ? null : id
}

async function confirmPurge(id: string) {
  try {
    await notes.purgeNote(id)
  } finally {
    purgingId.value = null
  }
}

async function confirmClear() {
  try {
    await notes.emptyTrash()
  } finally {
    clearing.value = false
  }
}

function onKeydown(event: KeyboardEvent) {
  if (event.key !== 'Escape') return
  purgingId.value = null
  clearing.value = false
}

watch([purgingId, clearing], ([purging, clear]) => {
  const active = Boolean(purging || clear)
  if (active) document.addEventListener('keydown', onKeydown)
  else document.removeEventListener('keydown', onKeydown)
})

onUnmounted(() => document.removeEventListener('keydown', onKeydown))
</script>

<template>
  <div class="trash">
    <div class="head">
      <span>回收站</span>
      <button type="button" class="link" @click="notes.closeTrash()">返回</button>
    </div>

    <div v-if="notes.deletedCount" class="toolbar">
      <span class="count">共 {{ notes.deletedCount }} 条</span>
      <button
        v-if="!clearing"
        type="button"
        class="link danger"
        data-testid="empty-trash"
        @click="((clearing = true), (purgingId = null))"
      >
        清空垃圾箱（共 {{ notes.deletedCount }} 条）
      </button>
      <span v-else class="confirm" data-testid="clear-confirm">
        <span class="warn">将永久删除 {{ notes.deletedCount }} 条笔记，不可恢复</span>
        <button
          type="button"
          class="danger"
          data-testid="confirm-empty-trash"
          @click="confirmClear"
        >
          永久删除
        </button>
        <button type="button" class="link" data-testid="cancel-empty-trash" @click="clearing = false">
          取消
        </button>
      </span>
    </div>

    <p v-if="!notes.deleted.length" class="hint">
      垃圾箱是空的。删除的笔记会先来到这里，直到你永久删除它们。
    </p>

    <div v-for="item in notes.deleted" :key="item.id" class="item">
      <div class="body">
        <span class="title">{{ item.title || '无标题' }}</span>
        <span class="meta">
          删除于 {{ formatRelativeTime(item.deletedAt ?? item.updatedAt) }}
        </span>
      </div>
      <template v-if="purgingId === item.id">
        <span class="confirm" data-testid="purge-confirm">
          <span class="warn">永久删除后不可恢复</span>
          <button
            type="button"
            class="danger"
            data-testid="confirm-purge"
            @click="confirmPurge(item.id)"
          >
            永久删除
          </button>
          <button
            type="button"
            class="link"
            data-testid="cancel-purge"
            @click="purgingId = null"
          >
            取消
          </button>
        </span>
      </template>
      <template v-else>
        <button type="button" class="restore" @click="notes.restoreNote(item.id)">恢复</button>
        <button
          type="button"
          class="purge"
          :data-testid="`purge-${item.id}`"
          :aria-label="`永久删除 ${item.title || '无标题'}`"
          @click="askPurge(item.id)"
        >
          永久删除
        </button>
      </template>
    </div>
  </div>
</template>

<style scoped>
.trash {
  flex: 1;
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

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  flex-wrap: wrap;
  padding: 0 6px 6px;
  font-size: 11px;
  color: var(--text-faint);
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

.link.danger {
  color: var(--danger);
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
  flex-wrap: wrap;
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

.restore,
.purge {
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

.purge {
  color: var(--danger);
}

.purge:hover {
  border-color: var(--danger);
}

/* Confirmation is in flow: the panel sits on its own line and never gets
   clipped by the scrolling container. */
.confirm {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  flex-basis: 100%;
  font-size: 11px;
}

.warn {
  color: var(--danger);
  font-weight: 500;
}

.confirm .danger {
  border: 1px solid var(--danger);
  background: var(--bg-elevated);
  border-radius: 5px;
  padding: 3px 8px;
  font-size: 11px;
  color: var(--danger);
  cursor: pointer;
}

.confirm .link {
  color: var(--text-muted);
}
</style>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog'
import { api, errorMessage, isDesktop } from '@/services/commands'
import type { LibraryInfo } from '@/services/types'
import { formatBytes } from '@/services/format'
import { useNotesStore } from '@/stores/notes'

const notes = useNotesStore()
const info = ref<LibraryInfo | null>(null)
const message = ref<string | null>(null)
const busy = ref(false)

async function refresh() {
  if (!isDesktop) return
  try {
    info.value = await api.getLibraryInfo()
  } catch (err) {
    message.value = errorMessage(err)
  }
}

async function openExisting() {
  const path = await openDialog({
    multiple: false,
    filters: [{ name: 'SQLite', extensions: ['db', 'sqlite', 'sqlite3'] }],
  })
  if (typeof path !== 'string') return
  await run(() => api.openLibrary(path), '已打开笔记库')
}

async function createNew() {
  const path = await saveDialog({
    defaultPath: 'notes.db',
    filters: [{ name: 'SQLite', extensions: ['db'] }],
  })
  if (!path) return
  await run(() => api.createLibrary(path), '已创建笔记库')
}

async function backup() {
  const path = await saveDialog({
    defaultPath: 'notes-backup.db',
    filters: [{ name: 'SQLite', extensions: ['db'] }],
  })
  if (!path) return
  await run(async () => void (await api.backupLibrary(path)), '备份完成')
}

async function run(action: () => Promise<unknown>, success: string) {
  busy.value = true
  message.value = null
  try {
    await notes.flush()
    await action()
    await notes.refresh()
    await notes.refreshTags()
    message.value = success
    await refresh()
  } catch (err) {
    message.value = errorMessage(err)
  } finally {
    busy.value = false
  }
}

onMounted(refresh)
</script>

<template>
  <section class="library">
    <h3>笔记库</h3>

    <p class="path">
      <span class="label">当前路径</span>
      <code>{{ info?.path ?? '—' }}</code>
    </p>
    <p class="path">
      <span class="label">Schema</span>
      <code>v{{ info?.schemaVersion ?? '—' }}{{ info?.isDefault ? ' · 默认位置' : '' }}</code>
    </p>
    <p class="path">
      <span class="label">占用</span>
      <code>
        {{ info ? `${info.noteCount} 条笔记 · ${formatBytes(info.fileSizeBytes)}` : '—' }}
      </code>
    </p>

    <div class="actions">
      <button type="button" :disabled="busy" @click="openExisting">打开现有库…</button>
      <button type="button" :disabled="busy" @click="createNew">新建库…</button>
      <button type="button" :disabled="busy" @click="backup">备份…</button>
    </div>

    <p v-if="message" class="message">{{ message }}</p>
    <p class="hint">切换笔记库前会先保存当前草稿；失败时不会新建空库。</p>
  </section>
</template>

<style scoped>
.library h3 {
  margin: 0 0 12px;
  font-size: 12px;
  font-weight: 600;
}

.path {
  display: flex;
  gap: 10px;
  margin: 0 0 8px;
  font-size: 11.5px;
}

.label {
  width: 76px;
  flex-shrink: 0;
  color: var(--text-muted);
}

code {
  font-family: 'SF Mono', ui-monospace, Menlo, monospace;
  font-size: 11px;
  word-break: break-all;
  color: var(--text-muted);
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 12px;
}

button {
  border: 1px solid var(--border);
  background: var(--bg-elevated);
  border-radius: 6px;
  padding: 4px 10px;
  font-size: 11.5px;
  cursor: pointer;
}

button:hover:not(:disabled) {
  border-color: var(--border-strong);
}

button:disabled {
  opacity: 0.5;
  cursor: default;
}

.message {
  margin: 10px 0 0;
  font-size: 11.5px;
  color: var(--accent);
}

.hint {
  margin: 10px 0 0;
  font-size: 11px;
  color: var(--text-faint);
}
</style>

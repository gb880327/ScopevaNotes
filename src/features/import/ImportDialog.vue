<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { api, errorMessage, isDesktop } from '@/services/commands'
import { formatDuration } from '@/services/format'
import type { ImportPreview, ImportProgress, ImportReport } from '@/services/types'
import { useNotesStore } from '@/stores/notes'
import { useUiStore } from '@/stores/ui'

type Stage = 'pick' | 'progress' | 'report'

const ui = useUiStore()
const notes = useNotesStore()

const stage = ref<Stage>('pick')
const root = ref<string | null>(null)
const preview = ref<ImportPreview | null>(null)
const progress = ref<ImportProgress | null>(null)
const report = ref<ImportReport | null>(null)
const parentDirTag = ref(false)
const busy = ref(false)
const cancelling = ref(false)
const error = ref<string | null>(null)
const closeHint = ref(false)
const undoMessage = ref<string | null>(null)

let unlisten: UnlistenFn | null = null

const directoryName = computed(() => {
  if (!root.value) return ''
  const parts = root.value.split('/').filter(Boolean)
  return parts[parts.length - 1] ?? root.value
})

const percent = computed(() => {
  const value = progress.value
  if (!value || value.total === 0) return 0
  return Math.min(100, Math.round((value.done / value.total) * 100))
})

const failedCount = computed(() => report.value?.failed.length ?? 0)

function reset() {
  stage.value = 'pick'
  root.value = null
  preview.value = null
  progress.value = null
  report.value = null
  parentDirTag.value = false
  busy.value = false
  cancelling.value = false
  error.value = null
  closeHint.value = false
  undoMessage.value = null
}

watch(
  () => ui.importOpen,
  (open) => {
    if (open) reset()
  },
)

async function chooseDirectory() {
  const selected = await openDialog({
    directory: true,
    multiple: false,
    title: '选择包含 Markdown 文件的目录',
  })
  if (typeof selected !== 'string') return
  await scan(selected)
}

async function scan(path: string) {
  busy.value = true
  error.value = null
  preview.value = null
  try {
    preview.value = await api.importScan(path)
    root.value = path
  } catch (err) {
    error.value = errorMessage(err)
  } finally {
    busy.value = false
  }
}

async function start() {
  if (!root.value) return
  stage.value = 'progress'
  error.value = null
  progress.value = null
  cancelling.value = false
  busy.value = true
  try {
    report.value = await api.importNotes(root.value, { parentDirTag: parentDirTag.value })
    stage.value = 'report'
    await notes.refresh()
    await notes.refreshTags()
  } catch (err) {
    // A hard failure (e.g. the library is locked by another process) is shown
    // as an error; batches already committed are kept and a retry skips them.
    error.value = errorMessage(err)
    stage.value = 'pick'
  } finally {
    busy.value = false
  }
}

async function cancel() {
  if (cancelling.value) return
  cancelling.value = true
  try {
    await api.cancelImport()
  } catch (err) {
    error.value = errorMessage(err)
    cancelling.value = false
  }
}

async function undoBatch() {
  if (!report.value) return
  busy.value = true
  try {
    const removed = await api.deleteImportBatch(report.value.batchTag)
    undoMessage.value = `已把 ${removed} 条笔记移入回收站，可在回收站中恢复。`
    await notes.refresh()
    await notes.refreshTags()
  } catch (err) {
    undoMessage.value = `删除失败：${errorMessage(err)}`
  } finally {
    busy.value = false
  }
}

function close() {
  if (stage.value === 'progress' && !report.value) {
    closeHint.value = true
    return
  }
  ui.importOpen = false
  reset()
}

onMounted(async () => {
  if (!isDesktop) return
  unlisten = await listen<ImportProgress>('ui://import-progress', (event) => {
    progress.value = event.payload
  })
})

onBeforeUnmount(() => {
  unlisten?.()
})
</script>

<template>
  <div v-if="ui.importOpen" class="overlay" @click.self="close">
    <div class="dialog" role="dialog" aria-modal="true" aria-label="导入 Markdown">
      <header>
        <h2>导入 Markdown</h2>
        <button
          v-if="stage !== 'progress' || report"
          type="button"
          class="close"
          aria-label="关闭"
          @click="close"
        >
          ×
        </button>
      </header>

      <!-- Stage 1: pick + dry-run preview -->
      <div v-if="stage === 'pick'" class="body">
        <p class="hint">
          把另一个应用（例如 Obsidian）目录中的 Markdown 文件一次性导入为笔记。只读取文件，不会修改原目录。
        </p>

        <div class="field">
          <span class="label">目录</span>
          <span class="path">{{ root ?? '尚未选择' }}</span>
          <button type="button" class="secondary" :disabled="busy" @click="chooseDirectory">
            选择目录…
          </button>
        </div>

        <div v-if="preview" class="preview">
          <div class="row"><span>待导入</span><strong>{{ preview.toImport }}</strong></div>
          <div class="row">
            <span>跳过（已导入）</span><strong>{{ preview.skippedExisting }}</strong>
          </div>
          <div v-if="preview.hiddenSkipped > 0" class="row">
            <span>跳过的隐藏条目</span><strong>{{ preview.hiddenSkipped }}</strong>
          </div>
          <div v-if="preview.localFileRefs > 0" class="row warn">
            <span>引用了本地文件（不迁移）</span><strong>{{ preview.localFileRefs }}</strong>
          </div>
          <div v-if="preview.unreadable.length > 0" class="row warn">
            <span>无法读取</span><strong>{{ preview.unreadable.length }}</strong>
          </div>
          <p v-if="preview.localFileRefs > 0" class="note">
            这些笔记的正文仍会逐字保留图片链接，但图片文件本身不会被复制，请自行保留原目录。
          </p>
        </div>

        <label class="check">
          <input v-model="parentDirTag" type="checkbox" />
          <span>用父目录名作为标签</span>
        </label>

        <p v-if="error" class="error">{{ error }}</p>

        <footer>
          <button type="button" class="secondary" @click="close">取消</button>
          <button
            type="button"
            class="primary"
            :disabled="busy || !preview || preview.toImport === 0"
            @click="start"
          >
            开始导入
          </button>
        </footer>
      </div>

      <!-- Stage 2: progress -->
      <div v-else-if="stage === 'progress'" class="body">
        <p class="hint">正在导入「{{ directoryName }}」，期间会自动保存编辑，请保持应用开启。</p>
        <div class="meter" role="progressbar" :aria-valuenow="percent" aria-valuemin="0" aria-valuemax="100">
          <div class="fill" :style="{ width: `${percent}%` }" />
        </div>
        <p class="count">
          已导入 {{ progress?.imported ?? 0 }} / {{ progress?.total ?? preview?.toImport ?? 0 }}
        </p>
        <p v-if="closeHint" class="error">导入进行中无法关闭，请等待完成或点击「取消导入」。</p>
        <p v-if="error" class="error">{{ error }}</p>
        <footer>
          <button
            type="button"
            class="secondary"
            :disabled="cancelling"
            @click="cancel"
          >
            {{ cancelling ? '正在取消…' : '取消导入' }}
          </button>
        </footer>
      </div>

      <!-- Stage 3: report -->
      <div v-else class="body">
        <p class="hint">
          {{ report?.cancelled ? '导入已取消，已提交的批次保留在笔记库中。' : '导入完成。' }}
        </p>

        <div v-if="report" class="report">
          <div class="row"><span>成功导入</span><strong>{{ report.imported }}</strong></div>
          <div class="row"><span>跳过（已导入）</span><strong>{{ report.skippedExisting }}</strong></div>
          <div class="row" :class="{ warn: failedCount > 0 }">
            <span>失败</span><strong data-testid="failed-count">{{ failedCount }}</strong>
          </div>
          <div v-if="report.hiddenSkipped > 0" class="row">
            <span>跳过的隐藏条目</span><strong>{{ report.hiddenSkipped }}</strong>
          </div>
          <div v-if="report.decodedAsGb18030 > 0" class="row">
            <span>按 GB18030 解码</span><strong>{{ report.decodedAsGb18030 }}</strong>
          </div>
          <div v-if="report.localFileReferences > 0" class="row warn">
            <span>引用了本地文件（未迁移）</span><strong>{{ report.localFileReferences }}</strong>
          </div>
          <div class="row"><span>耗时</span><strong>{{ formatDuration(report.elapsedMs) }}</strong></div>
          <div class="row subtle"><span>本批标签</span><strong>{{ report.batchTag }}</strong></div>
        </div>

        <!-- An empty failure list is not rendered at all (spec: report). -->
        <section v-if="report && report.failed.length > 0" class="failures">
          <h3>失败的文件</h3>
          <ul>
            <li v-for="failure in report.failed" :key="failure.path">
              <span class="failure-path">{{ failure.path }}</span>
              <span class="failure-reason">{{ failure.reason }}</span>
            </li>
          </ul>
        </section>

        <p v-if="report && report.localFileReferences > 0" class="note">
          这些笔记引用的图片等文件仍在原目录中，本版不迁移附件，请自行保留。
        </p>
        <p v-if="undoMessage" class="note">{{ undoMessage }}</p>

        <footer>
          <button
            v-if="report && report.imported > 0 && !undoMessage"
            type="button"
            class="danger"
            :disabled="busy"
            @click="undoBatch"
          >
            删除本批
          </button>
          <button type="button" class="primary" @click="close">关闭</button>
        </footer>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.25);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 60;
}

.dialog {
  width: 560px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: 12px;
  box-shadow: var(--shadow);
}

header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 18px;
  border-bottom: 1px solid var(--border);
}

h2 {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
}

.close {
  border: 0;
  background: none;
  font-size: 16px;
  color: var(--text-muted);
  cursor: pointer;
}

.body {
  padding: 16px 18px 18px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.hint {
  margin: 0;
  font-size: 12px;
  line-height: 1.5;
  color: var(--text-muted);
}

.field {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 12px;
}

.label {
  color: var(--text-muted);
  flex: none;
}

.path {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  padding: 5px 8px;
  border-radius: 6px;
  background: var(--bg-input);
}

.preview,
.report {
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 4px 12px;
}

.row {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
  padding: 5px 0;
  font-size: 12px;
  border-bottom: 1px solid var(--border);
}

.row:last-child {
  border-bottom: 0;
}

.row span {
  color: var(--text-muted);
}

.row.warn strong {
  color: var(--danger);
}

.row.subtle {
  font-size: 11px;
}

.check {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
}

.note {
  margin: 0;
  font-size: 11px;
  line-height: 1.5;
  color: var(--text-muted);
}

.error {
  margin: 0;
  font-size: 12px;
  color: var(--danger);
}

.meter {
  height: 6px;
  border-radius: 3px;
  background: var(--bg-input);
  overflow: hidden;
}

.fill {
  height: 100%;
  background: var(--accent);
  transition: width 120ms linear;
}

.count {
  margin: 0;
  font-size: 12px;
  color: var(--text-muted);
}

.failures h3 {
  margin: 4px 0 6px;
  font-size: 12px;
  font-weight: 600;
}

.failures ul {
  margin: 0;
  padding: 0;
  list-style: none;
  max-height: 160px;
  overflow-y: auto;
  border: 1px solid var(--border);
  border-radius: 8px;
}

.failures li {
  padding: 6px 10px;
  font-size: 11px;
  border-bottom: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.failures li:last-child {
  border-bottom: 0;
}

.failure-path {
  word-break: break-all;
}

.failure-reason {
  color: var(--danger);
}

footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding-top: 4px;
}

button {
  font: inherit;
  font-size: 12px;
  padding: 6px 14px;
  border-radius: 7px;
  border: 1px solid var(--border-strong);
  background: var(--bg-elevated);
  color: var(--text);
  cursor: pointer;
}

button:disabled {
  opacity: 0.5;
  cursor: default;
}

button.primary {
  background: var(--accent-text);
  border-color: var(--accent-text);
  color: var(--accent-contrast);
}

button.danger {
  color: var(--danger);
  border-color: var(--border-strong);
}
</style>

<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue'
import { useNotesStore } from '@/stores/notes'
import TagEditor from './TagEditor.vue'

const notes = useNotesStore()
const adding = ref(false)
const renamingId = ref<string | null>(null)
const deletingId = ref<string | null>(null)
const paletteId = ref<string | null>(null)

const total = computed(() => notes.tags.reduce((sum, tag) => sum + tag.noteCount, 0))

/** Fixed palette: enough to tell tags apart without a colour-picker dependency. */
const PALETTE = [
  { value: '#0a84ff', label: '蓝色' },
  { value: '#30d158', label: '绿色' },
  { value: '#ff9f0a', label: '橙色' },
  { value: '#bf5af2', label: '紫色' },
  { value: '#ff375f', label: '粉色' },
  { value: '#64d2ff', label: '青色' },
  { value: '#ffd60a', label: '黄色' },
  { value: '#8e8e93', label: '灰色' },
]

function startRename(id: string) {
  paletteId.value = null
  deletingId.value = null
  renamingId.value = id
}

function startDelete(id: string) {
  paletteId.value = null
  renamingId.value = null
  deletingId.value = id
}

async function confirmDelete(id: string) {
  try {
    await notes.deleteTag(id)
  } finally {
    deletingId.value = null
  }
}

function togglePalette(id: string) {
  renamingId.value = null
  deletingId.value = null
  paletteId.value = paletteId.value === id ? null : id
}

function chooseColor(id: string, color: string) {
  paletteId.value = null
  void notes.setTagColor(id, color)
}

/** Esc backs out of the inline delete confirmation (the row is not focusable). */
function onEscape(event: KeyboardEvent) {
  if (event.key === 'Escape') deletingId.value = null
}

watch(deletingId, (value) => {
  if (value) document.addEventListener('keydown', onEscape)
  else document.removeEventListener('keydown', onEscape)
})

onUnmounted(() => document.removeEventListener('keydown', onEscape))
</script>

<template>
  <section class="tags">
    <header>
      <span class="label">标签</span>
      <button
        type="button"
        class="add"
        title="新建标签"
        aria-label="新建标签"
        @click="adding = !adding"
      >
        ＋
      </button>
    </header>

    <TagEditor v-if="adding" @done="adding = false" />

    <div class="scroll">
      <button
        type="button"
        class="tag all"
        :class="{ active: !notes.activeTagIds.length }"
        @click="notes.clearTagFilters()"
      >
        <span class="name">全部标签</span>
        <span class="count">{{ total }}</span>
      </button>

      <p v-if="!notes.tags.length" class="empty">还没有标签</p>

      <div
        v-for="tag in notes.tags"
        :key="tag.id"
        class="row"
        :class="{ active: notes.activeTagIds.includes(tag.id) }"
      >
        <TagEditor
          v-if="renamingId === tag.id"
          inline
          :value="tag.name"
          :tag-id="tag.id"
          @done="renamingId = null"
        />

        <template v-else-if="deletingId === tag.id">
          <span class="confirm">将影响 {{ tag.noteCount }} 条笔记</span>
          <span class="actions pinned">
            <button
              type="button"
              class="danger"
              data-testid="confirm-delete-tag"
              @click="confirmDelete(tag.id)"
            >
              删除
            </button>
            <button type="button" data-testid="cancel-delete-tag" @click="deletingId = null">
              取消
            </button>
          </span>
        </template>

        <template v-else>
          <button
            type="button"
            class="dot-button"
            :aria-label="`改色：${tag.name}`"
            data-testid="tag-color"
            @click.stop="togglePalette(tag.id)"
          >
            <span class="dot" :style="{ background: tag.color ?? 'var(--accent)' }" />
          </button>

          <button
            type="button"
            class="tag"
            :class="{ active: notes.activeTagIds.includes(tag.id) }"
            @click="notes.toggleTagFilter(tag.id)"
          >
            <span class="name">{{ tag.name }}</span>
            <span class="count">{{ tag.noteCount }}</span>
          </button>

          <span class="actions">
            <button
              type="button"
              aria-label="重命名标签"
              data-testid="rename-tag"
              @click="startRename(tag.id)"
            >
              ✎
            </button>
            <button
              type="button"
              aria-label="删除标签"
              data-testid="delete-tag"
              @click="startDelete(tag.id)"
            >
              ✕
            </button>
          </span>

          <div v-if="paletteId === tag.id" class="palette">
            <button
              v-for="color in PALETTE"
              :key="color.value"
              type="button"
              class="swatch"
              :class="{ picked: tag.color === color.value }"
              :style="{ background: color.value }"
              :aria-label="color.label"
              :data-testid="`swatch-${color.value}`"
              @click.stop="chooseColor(tag.id, color.value)"
            />
          </div>
        </template>
      </div>
    </div>
  </section>
</template>

<style scoped>
.tags {
  /* Fixed share of the sidebar: the note list and the trash have different
     content heights, and a content-sized panel would slide around when the user
     switches between them. */
  flex: 0 0 auto;
  height: 38%;
  min-height: 104px;
  max-height: 320px;
  border-top: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  min-width: 0;
}

header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 14px 4px;
}

.label {
  font-size: 10.5px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-faint);
}

.add {
  border: 0;
  background: none;
  color: var(--text-faint);
  cursor: pointer;
  font-size: 13px;
}

.add:hover {
  color: var(--text);
}

.scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 0 8px 8px;
}

.empty {
  margin: 2px 6px 0;
  font-size: 11px;
  color: var(--text-faint);
}

/* One tag: colour dot + name area + hover actions. */
.row {
  position: relative;
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  border-radius: 6px;
  padding: 0 6px;
  min-height: 22px;
}

.row:hover {
  background: var(--hover);
}

.row.active {
  background: var(--selected);
}

.dot-button {
  display: flex;
  align-items: center;
  border: 0;
  background: none;
  padding: 0 7px 0 0;
  cursor: pointer;
}

.dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
  box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.08);
}

.tag {
  display: flex;
  align-items: center;
  gap: 7px;
  flex: 1;
  min-width: 0;
  border: 0;
  background: transparent;
  border-radius: 6px;
  padding: 4px 0;
  cursor: pointer;
  font-size: 12px;
  text-align: left;
}

.tag.all {
  width: 100%;
  flex: none;
  padding: 4px 6px;
  justify-content: space-between;
}

.tag.all.active {
  background: var(--selected);
}

.name {
  flex: 1;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.count {
  color: var(--text-faint);
  font-size: 10.5px;
}

/* Revealed on hover, pinned for the inline confirmation. */
.actions {
  display: flex;
  align-items: center;
  gap: 2px;
  opacity: 0;
  transition: opacity 80ms linear;
}

.row:hover .actions,
.row:focus-within .actions,
.actions.pinned {
  opacity: 1;
}

.row:hover .count {
  opacity: 0;
}

.actions button {
  border: 0;
  background: none;
  color: var(--text-faint);
  cursor: pointer;
  font-size: 11px;
  padding: 2px 4px;
  border-radius: 4px;
}

.actions button:hover {
  color: var(--text);
  background: var(--bg-input);
}

.actions .danger {
  color: var(--danger);
}

.confirm {
  flex: 1;
  font-size: 11px;
  color: var(--text-muted);
  padding: 4px 0;
}

/* In flow, so a popover near the bottom of the list is never clipped. */
.palette {
  flex-basis: 100%;
  display: grid;
  grid-template-columns: repeat(8, 16px);
  gap: 5px;
  margin: 0 0 6px 14px;
  padding: 6px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-elevated);
}

.swatch {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  border: 1px solid rgba(0, 0, 0, 0.12);
  cursor: pointer;
  padding: 0;
}

.swatch.picked {
  outline: 2px solid var(--text);
  outline-offset: 1px;
}
</style>

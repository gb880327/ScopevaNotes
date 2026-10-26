<script setup lang="ts">
/**
 * The 更多 menu for one 速记 tab. Rendered **in the document flow** below the
 * header on purpose: the panel root scrolls, and an absolutely positioned menu
 * inside a scrolling container gets clipped (paid for twice already).
 */
import { nextTick, ref } from 'vue'
import { QUICK_TAB_COLOR_KEYS, colorSpec } from './quickTabColors'
import type { QuickTab } from '@/services/types'

const props = defineProps<{ tab: QuickTab; atLimit: boolean }>()

const emit = defineEmits<{
  close: []
  rename: [title: string]
  setColor: [color: string]
  duplicate: []
  remove: []
}>()

const draftTitle = ref(props.tab.title)
const confirming = ref(false)
const titleInput = ref<HTMLInputElement | null>(null)

async function focusTitle() {
  await nextTick()
  titleInput.value?.focus()
  titleInput.value?.select()
}

function commitTitle() {
  const next = draftTitle.value.trim()
  if (next === props.tab.title.trim()) return
  emit('rename', next)
}

function onKeydown(event: KeyboardEvent) {
  if (event.isComposing) return
  if (event.key === 'Escape') {
    event.stopPropagation()
    emit('close')
  }
}

defineExpose({ focusTitle })
</script>

<template>
  <div class="menu" data-testid="quick-tab-menu" @keydown="onKeydown">
    <div class="row">
      <label class="label" :for="`rename-${tab.id}`">标题</label>
      <input
        :id="`rename-${tab.id}`"
        ref="titleInput"
        v-model="draftTitle"
        class="title-input"
        data-testid="quick-tab-title-input"
        type="text"
        placeholder="未命名"
        maxlength="40"
        @blur="commitTitle"
        @keydown.enter.prevent="commitTitle"
      />
    </div>

    <div class="row">
      <span class="label">颜色</span>
      <div class="swatches">
        <button
          v-for="key in QUICK_TAB_COLOR_KEYS"
          :key="key"
          class="swatch"
          :class="{ current: key === tab.color }"
          :style="{ background: colorSpec(key).dot }"
          :data-testid="`quick-tab-color-${key}`"
          :aria-label="`换成${colorSpec(key).name}色`"
          :aria-pressed="key === tab.color"
          type="button"
          @click="emit('setColor', key)"
        />
      </div>
    </div>

    <div class="row actions">
      <button
        class="action"
        data-testid="quick-tab-duplicate"
        type="button"
        :disabled="atLimit"
        :title="atLimit ? '速记已达上限' : '复制这条速记'"
        @click="emit('duplicate')"
      >
        复制
      </button>
      <template v-if="confirming">
        <span class="confirm-text">删除后不可恢复</span>
        <button
          class="action danger"
          data-testid="quick-tab-confirm-delete"
          type="button"
          @click="emit('remove')"
        >
          删除
        </button>
        <button
          class="action"
          data-testid="quick-tab-cancel-delete"
          type="button"
          @click="confirming = false"
        >
          取消
        </button>
      </template>
      <button
        v-else
        class="action danger"
        data-testid="quick-tab-delete"
        type="button"
        @click="confirming = true"
      >
        删除…
      </button>
    </div>
  </div>
</template>

<style scoped>
.menu {
  display: flex;
  flex-direction: column;
  gap: 8px;
  width: 100%;
  padding: 10px 12px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-sidebar);
}

.row {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.label {
  flex: 0 0 auto;
  color: var(--text-muted);
  font-size: 12px;
}

.title-input {
  flex: 1;
  min-width: 0;
  padding: 4px 8px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  background: var(--bg-elevated);
  outline: none;
}

.title-input:focus {
  border-color: var(--accent);
}

.swatches {
  display: flex;
  gap: 6px;
}

.swatch {
  width: 18px;
  height: 18px;
  padding: 0;
  border: none;
  border-radius: 50%;
  cursor: pointer;
  box-shadow: inset 0 0 0 1px var(--dot-ring);
}

.swatch.current {
  box-shadow:
    0 0 0 2px var(--bg-sidebar),
    0 0 0 3px var(--text);
}

.actions {
  flex-wrap: wrap;
}

.action {
  padding: 3px 10px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  background: var(--bg-elevated);
  cursor: pointer;
}

.action:disabled {
  opacity: 0.45;
  cursor: default;
}

.action.danger {
  color: var(--danger);
  border-color: var(--danger);
}

.confirm-text {
  color: var(--danger);
  font-size: 12px;
}
</style>

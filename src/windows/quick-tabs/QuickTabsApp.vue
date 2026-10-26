<script setup lang="ts">
/**
 * 速记 panel root.
 *
 * Deliberately dumb about the note world: it talks to `stores/quickTabs` only.
 * No editor, no preview, no markdown — the bundle boundary is a product
 * guarantee (design D14), not a preference.
 */
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useQuickTabsStore } from '@/stores/quickTabs'
import { useSettingsStore } from '@/stores/settings'
import { useTheme } from '@/composables/useTheme'
import { isDesktop } from '@/services/commands'
import { dotColor, dotLabel } from './quickTabColors'
import QuickTabMenu from './QuickTabMenu.vue'

const store = useQuickTabsStore()
const settings = useSettingsStore()

const disposeTheme = useTheme()
const menuFor = ref<string | null>(null)
const composing = ref(false)
const editor = ref<HTMLTextAreaElement | null>(null)
const menu = ref<InstanceType<typeof QuickTabMenu> | null>(null)

let unlisten: UnlistenFn[] = []
/** Rust's `Focused(false)` handler hides the window; this is the save half. */
const onWindowBlur = () => void store.flush()

const statusText = computed(() => {
  switch (store.saveState) {
    case 'saving':
      return '正在保存…'
    case 'saved':
      return '已保存'
    case 'error':
      return store.lastError ? `保存失败：${store.lastError}` : '保存失败'
    default:
      return ''
  }
})

const statusClass = computed(() => `status ${store.saveState}`)
const characters = computed(() => store.activeTab?.content.length ?? 0)

async function select(id: string) {
  menuFor.value = null
  await store.setActive(id)
  await focusEditor()
}

async function focusEditor() {
  await nextTick()
  editor.value?.focus()
}

async function add() {
  const created = await store.create()
  if (created) {
    menuFor.value = null
    await focusEditor()
  }
}

async function openMenu() {
  if (!store.activeTab) return
  menuFor.value = menuFor.value === store.activeTab.id ? null : store.activeTab.id
  if (menuFor.value) {
    await nextTick()
    await menu.value?.focusTitle()
  }
}

async function rename(title: string) {
  if (!store.activeTab) return
  await store.rename(store.activeTab.id, title)
}

async function duplicate() {
  await store.duplicate(store.activeTabId ?? '')
  menuFor.value = null
  await focusEditor()
}

async function remove() {
  if (!store.activeTab) return
  await store.remove(store.activeTab.id)
  menuFor.value = null
  await focusEditor()
}

function onInput(event: Event) {
  store.updateContent((event.target as HTMLTextAreaElement).value)
}

/**
 * Esc closes, everything else is left to the platform: ⌘A/⌘C/⌘V and the arrow
 * keys must behave like a plain textarea, and an IME candidate window owns its
 * own keys while composing.
 */
function onKeydown(event: KeyboardEvent) {
  if (event.key !== 'Escape') return
  if (event.isComposing || composing.value) return
  event.preventDefault()
  menuFor.value = null
  void store.hide()
}

function onCompositionStart() {
  composing.value = true
}

function onCompositionEnd() {
  composing.value = false
}

onMounted(async () => {
  await settings.hydrate()
  window.addEventListener('blur', onWindowBlur)
  await store.init()
  await focusEditor()

  if (isDesktop) {
    unlisten = await Promise.all([
      // Quit: save, never prompt — the main window owns the confirmation (D12).
      listen('ui://quit-requested', () => void store.flush()),
      // Library switched: reload and drop the previous library's drafts (D13).
      listen('ui://library-changed', () => void store.reload()),
    ])
  }
})

onBeforeUnmount(() => {
  window.removeEventListener('blur', onWindowBlur)
  for (const stop of unlisten) stop()
  disposeTheme()
  void store.flush()
})
</script>

<template>
  <div class="panel">
    <header class="bar">
      <div class="dots" role="tablist" aria-label="速记">
        <button
          v-for="tab in store.tabs"
          :key="tab.id"
          class="dot"
          :class="{ active: tab.id === store.activeTabId }"
          role="tab"
          type="button"
          :aria-selected="tab.id === store.activeTabId"
          :aria-label="dotLabel(tab.color, tab.title)"
          :data-testid="`quick-tab-dot-${tab.id}`"
          @click="select(tab.id)"
        >
          <span class="dot-core" :style="{ background: dotColor(tab.color) }" />
        </button>
        <button
          class="add"
          type="button"
          data-testid="quick-tab-add"
          :disabled="store.atLimit"
          :title="store.atLimit ? `速记最多 ${store.maxTabs} 个` : '新建速记'"
          :aria-label="store.atLimit ? `速记最多 ${store.maxTabs} 个` : '新建速记'"
          @click="add"
        >
          +
        </button>
      </div>
      <button
        class="more"
        type="button"
        data-testid="quick-tab-more"
        :disabled="!store.activeTab"
        @click="openMenu"
      >
        更多
      </button>
    </header>

    <QuickTabMenu
      v-if="menuFor && store.activeTab"
      ref="menu"
      :tab="store.activeTab"
      :at-limit="store.atLimit"
      @close="menuFor = null"
      @rename="rename"
      @set-color="(color: string) => store.setColor(store.activeTabId ?? '', color)"
      @duplicate="duplicate"
      @remove="remove"
    />

    <p v-if="store.atLimit" class="limit" data-testid="quick-tab-limit">
      速记最多 {{ store.maxTabs }} 个
    </p>

    <textarea
      ref="editor"
      class="editor"
      data-testid="quick-tab-editor"
      :value="store.activeTab?.content ?? ''"
      :disabled="!store.activeTab"
      :placeholder="store.activeTab ? '随手记点什么…' : '点 + 新建一条速记'"
      spellcheck="false"
      @input="onInput"
      @keydown="onKeydown"
      @compositionstart="onCompositionStart"
      @compositionend="onCompositionEnd"
    />

    <footer :class="statusClass" data-testid="quick-tab-status">
      <span class="state">{{ statusText }}</span>
      <span class="count">{{ characters }} 字符</span>
    </footer>
  </div>
</template>

<style scoped>
/* The window root element is outside this component; app.css only sizes #app. */
:global(#quick-tabs) {
  height: 100%;
}

.panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg);
  border: 1px solid var(--border-strong);
  border-radius: 10px;
  overflow: hidden;
}

/* Tab bar: one 56px row, two groups pushed apart, everything centred.
   Sizes are a contract (design D17): 36px hit area per tab, the active tab and
   the "+" button grow to 40px, "更多" is 40px tall. */
.bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  height: 56px;
  padding: 0 10px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-sidebar);
}

.dots {
  display: flex;
  flex: 1 1 auto;
  align-items: center;
  gap: 10px;
  min-width: 0;
  overflow-x: auto;
  overflow-y: hidden;
  scrollbar-width: none;
}

.dots::-webkit-scrollbar {
  display: none;
}

.dot {
  display: grid;
  place-items: center;
  flex: 0 0 auto;
  width: 36px;
  height: 36px;
  padding: 0;
  border: none;
  border-radius: 50%;
  background: transparent;
  cursor: pointer;
  transition:
    width 120ms ease,
    height 120ms ease,
    background-color 120ms ease;
}

.dot:hover {
  background: var(--hover);
}

.dot-core {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  /* Light fills need a boundary to be visible at all (see --dot-ring). */
  box-shadow: inset 0 0 0 1px var(--dot-ring);
  transition:
    width 120ms ease,
    height 120ms ease;
}

/* Selection must not depend on colour alone (spec: 可访问性): the active tab
   grows and its dot gets a tight ring that reads at 40px. */
.dot.active {
  width: 40px;
  height: 40px;
}

.dot.active .dot-core {
  width: 16px;
  height: 16px;
  box-shadow:
    inset 0 0 0 1px var(--dot-ring),
    0 0 0 2px var(--bg-sidebar),
    0 0 0 3px var(--text);
}

.dot:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
}

.add,
.more {
  display: grid;
  place-items: center;
  flex: 0 0 auto;
  height: 40px;
  padding: 0 12px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  transition:
    background-color 120ms ease,
    color 120ms ease;
}

.add {
  width: 40px;
  padding: 0;
  font-size: 18px;
  line-height: 1;
}

.add:hover:not(:disabled),
.more:hover:not(:disabled) {
  background: var(--hover);
  color: var(--text);
}

.add:focus-visible,
.more:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
}

.add:disabled,
.more:disabled {
  opacity: 0.45;
  cursor: default;
}

.limit {
  margin: 0;
  padding: 4px 12px;
  color: var(--text-muted);
  font-size: 12px;
  background: var(--bg-sidebar);
  border-bottom: 1px solid var(--border);
}

.editor {
  flex: 1;
  min-height: 0;
  padding: 12px;
  border: none;
  background: var(--bg);
  color: var(--text);
  font-size: var(--editor-font-size);
  line-height: 1.6;
  resize: none;
  outline: none;
}

.status {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  padding: 5px 12px;
  border-top: 1px solid var(--border);
  background: var(--bg-sidebar);
  color: var(--text-muted);
  font-size: 12px;
}

.status.error .state {
  color: var(--danger);
}
</style>

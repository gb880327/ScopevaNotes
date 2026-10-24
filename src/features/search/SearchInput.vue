<script setup lang="ts">
import { ref, watch } from 'vue'
import { useNotesStore } from '@/stores/notes'
import { useUiStore } from '@/stores/ui'

const notes = useNotesStore()
const ui = useUiStore()
const input = ref<HTMLInputElement | null>(null)
const value = ref(notes.query)
let timer: ReturnType<typeof setTimeout> | null = null

function onInput(next: string) {
  value.value = next
  if (timer) clearTimeout(timer)
  timer = setTimeout(() => {
    notes.query = next
    void notes.refresh()
  }, 150)
}

watch(
  () => ui.searchFocusToken,
  () => input.value?.focus(),
)
</script>

<template>
  <div class="search">
    <span class="icon">⌕</span>
    <input
      ref="input"
      :value="value"
      type="search"
      placeholder="搜索标题、正文、标签"
      spellcheck="false"
      @input="onInput(($event.target as HTMLInputElement).value)"
    />
  </div>
</template>

<style scoped>
.search {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 0 12px 8px;
  padding: 5px 9px;
  background: var(--bg-input);
  border: 1px solid transparent;
  border-radius: 7px;
}

.search:focus-within {
  border-color: var(--accent);
  background: var(--bg-elevated);
}

.icon {
  color: var(--text-faint);
  font-size: 13px;
}

input {
  flex: 1;
  border: 0;
  background: transparent;
  outline: none;
  font-size: 12.5px;
}

input::-webkit-search-cancel-button {
  display: none;
}
</style>

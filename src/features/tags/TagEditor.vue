<script setup lang="ts">
import { ref } from 'vue'
import { useNotesStore } from '@/stores/notes'

const emit = defineEmits<{ done: [] }>()
const notes = useNotesStore()
const name = ref('')

async function submit() {
  const value = name.value.trim()
  if (!value) {
    emit('done')
    return
  }
  await notes.createTag(value)
  name.value = ''
  emit('done')
}
</script>

<template>
  <div class="tag-editor">
    <input
      v-model="name"
      placeholder="标签名称"
      autofocus
      @keydown.enter.prevent="submit"
      @keydown.esc.prevent="emit('done')"
    />
  </div>
</template>

<style scoped>
.tag-editor {
  padding: 0 14px 8px;
}

input {
  width: 100%;
  padding: 4px 8px;
  border: 1px solid var(--border-strong);
  border-radius: 6px;
  background: var(--bg);
  font-size: 12px;
  outline: none;
}

input:focus {
  border-color: var(--accent);
}
</style>

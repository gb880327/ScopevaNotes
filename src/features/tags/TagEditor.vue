<script setup lang="ts">
import { nextTick, onMounted, ref } from 'vue'
import { useNotesStore } from '@/stores/notes'
import { errorMessage } from '@/services/commands'

const props = defineProps<{
  /** Prefilled name (rename mode). */
  value?: string
  /** Present in rename mode, absent when creating a new tag. */
  tagId?: string
  /** Set when the input replaces a sidebar row instead of sitting above the list. */
  inline?: boolean
  placeholder?: string
}>()

const emit = defineEmits<{ done: [] }>()

const notes = useNotesStore()
const name = ref(props.value ?? '')
const errorText = ref<string | null>(null)
const input = ref<HTMLInputElement | null>(null)

onMounted(async () => {
  await nextTick()
  input.value?.focus()
  input.value?.select()
})

function cancel() {
  emit('done')
}

async function submit() {
  const value = name.value.trim()
  if (!value) {
    // Renaming to nothing is a mistake worth showing; an empty "new tag" is a cancel.
    if (props.tagId) errorText.value = '标签名称不能为空'
    else cancel()
    return
  }
  if (props.tagId && value === props.value) {
    cancel()
    return
  }
  errorText.value = null
  try {
    if (props.tagId) await notes.renameTag(props.tagId, value)
    else await notes.createTag(value)
    emit('done')
  } catch (err) {
    errorText.value = errorMessage(err)
  }
}
</script>

<template>
  <div class="tag-editor" :class="{ inline }">
    <input
      ref="input"
      v-model="name"
      :placeholder="placeholder ?? '标签名称'"
      :aria-invalid="errorText !== null"
      @keydown.enter.prevent="submit"
      @keydown.esc.prevent="cancel"
      @blur="cancel"
    />
    <p v-if="errorText" class="error" role="alert">{{ errorText }}</p>
  </div>
</template>

<style scoped>
.tag-editor {
  padding: 0 14px 8px;
}

.tag-editor.inline {
  padding: 0;
  flex: 1;
  min-width: 0;
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

.error {
  margin: 3px 0 0;
  font-size: 10.5px;
  color: var(--danger);
}
</style>

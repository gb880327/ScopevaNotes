<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { EditorView, basicSetup } from 'codemirror'
import { markdown } from '@codemirror/lang-markdown'
import { Compartment, EditorState } from '@codemirror/state'
import { useSettingsStore } from '@/stores/settings'
import { editorChrome, editorSyntaxTheme } from './editorTheme'

const props = defineProps<{ modelValue: string }>()
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()

const settings = useSettingsStore()
const host = ref<HTMLDivElement | null>(null)
const syntaxCompartment = new Compartment()
let view: EditorView | null = null

onMounted(() => {
  view = new EditorView({
    parent: host.value as HTMLElement,
    state: EditorState.create({
      doc: props.modelValue,
      extensions: [
        basicSetup,
        markdown(),
        EditorView.lineWrapping,
        editorChrome,
        syntaxCompartment.of(editorSyntaxTheme(settings.resolvedTheme)),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) emit('update:modelValue', update.state.doc.toString())
        }),
      ],
    }),
  })
})

// The chrome uses design tokens, so only the syntax colours need reconfiguring.
watch(
  () => settings.resolvedTheme,
  (next) => {
    view?.dispatch({ effects: syntaxCompartment.reconfigure(editorSyntaxTheme(next)) })
  },
)

watch(
  () => props.modelValue,
  (next) => {
    if (!view) return
    const current = view.state.doc.toString()
    if (current === next) return
    view.dispatch({ changes: { from: 0, to: current.length, insert: next } })
  },
)

onBeforeUnmount(() => {
  view?.destroy()
  view = null
})
</script>

<template>
  <div ref="host" class="editor" />
</template>

<style scoped>
.editor {
  height: 100%;
  color: var(--text);
}

.editor :deep(.cm-scroller) {
  overflow: auto;
}
</style>

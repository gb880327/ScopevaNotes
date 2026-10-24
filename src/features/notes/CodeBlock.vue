<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { copyText } from '@/services/clipboard'
import { useSettingsStore } from '@/stores/settings'
import { highlightToTokens, tokenStyle, type Token } from './highlight'

const props = defineProps<{ code: string; lang: string }>()

const settings = useSettingsStore()
const lines = ref<Token[][] | null>(null)
const copyState = ref<'idle' | 'copied' | 'failed'>('idle')

const NEWLINE: Token = { content: '\n' }

/** Tokens flattened with explicit newlines so the template stays a flat loop. */
const tokens = computed<Token[] | null>(() => {
  const all = lines.value
  if (!all) return null
  const flat: Token[] = []
  all.forEach((line, index) => {
    flat.push(...line)
    if (index < all.length - 1) flat.push(NEWLINE)
  })
  return flat
})

const copyLabel = computed(() => {
  if (copyState.value === 'copied') return '已复制'
  if (copyState.value === 'failed') return '复制失败'
  return '复制代码'
})

let request = 0

watch(
  [() => props.code, () => props.lang, () => settings.resolvedTheme],
  async ([code, lang, theme]) => {
    const id = (request += 1)
    const result = await highlightToTokens(code, lang, theme)
    // A newer edit or theme switch may have landed while highlighting.
    if (id === request) lines.value = result
  },
  { immediate: true },
)

async function copy() {
  try {
    await copyText(props.code)
    copyState.value = 'copied'
  } catch (err) {
    console.error('[copy] failed', err)
    copyState.value = 'failed'
  }
  setTimeout(() => (copyState.value = 'idle'), 1200)
}
</script>

<template>
  <figure class="code-block">
    <figcaption>
      <span class="lang">{{ props.lang || 'text' }}</span>
      <button type="button" class="copy" @click="copy">{{ copyLabel }}</button>
    </figcaption>
    <pre><code><span v-for="(token, index) in tokens ?? [{ content: props.code }]" :key="index" :style="tokenStyle(token)">{{ token.content }}</span></code></pre>
  </figure>
</template>

<style scoped>
.code-block {
  margin: 1em 0;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--code-bg);
  overflow: hidden;
}

figcaption {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  border-bottom: 1px solid var(--border);
  font-size: 11px;
  color: var(--text-muted);
  text-transform: lowercase;
}

.copy {
  border: 1px solid var(--border);
  background: var(--bg-elevated);
  border-radius: 5px;
  padding: 2px 8px;
  cursor: pointer;
  font-size: 11px;
}

.copy:hover {
  border-color: var(--border-strong);
}

pre {
  margin: 0;
  padding: 12px 14px;
  overflow: auto;
}

code {
  font-family: 'SF Mono', ui-monospace, Menlo, monospace;
  font-size: 12.5px;
  line-height: 1.6;
  white-space: pre;
}
</style>

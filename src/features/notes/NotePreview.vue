<script setup lang="ts">
import { computed } from 'vue'
import { openUrl } from '@tauri-apps/plugin-opener'
import { markdown } from './markdown'
import { splitFences } from './fences'
import CodeBlock from './CodeBlock.vue'
import { isDesktop } from '@/services/commands'

const props = defineProps<{ content: string }>()

const segments = computed(() =>
  splitFences(props.content).map((segment) =>
    segment.type === 'markdown'
      ? { ...segment, html: markdown.render(segment.value) }
      : segment,
  ),
)

async function onClick(event: MouseEvent) {
  const anchor = (event.target as HTMLElement | null)?.closest('a')
  if (!anchor) return
  const href = anchor.getAttribute('href') ?? ''
  if (!/^https?:/i.test(href)) return
  event.preventDefault()
  if (isDesktop) {
    await openUrl(href)
  } else {
    window.open(href, '_blank', 'noopener')
  }
}
</script>

<template>
  <article class="markdown-body" @click="onClick">
    <template v-for="(segment, index) in segments" :key="index">
      <CodeBlock
        v-if="segment.type === 'code'"
        :code="segment.code"
        :lang="segment.lang"
      />
      <!-- eslint-disable-next-line vue/no-v-html -- markdown-it runs with html:false -->
      <div v-else v-html="segment.html" />
    </template>
  </article>
</template>

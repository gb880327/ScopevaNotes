import MarkdownIt from 'markdown-it'

/** Raw HTML is disabled: untrusted Markdown must never inject markup. */
export const markdown = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: false,
  typographer: false,
})

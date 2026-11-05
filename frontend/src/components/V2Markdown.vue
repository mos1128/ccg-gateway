<template>
  <div class="v2-md markdown-body" @error.capture="handleImageError" v-html="rendered"></div>
</template>

<script setup lang="ts">
import { convertFileSrc } from '@tauri-apps/api/core'
import MarkdownIt from 'markdown-it'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

const props = defineProps<{ content: string }>()
const { t } = useI18n()

const md = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: true
})

const defaultImage = md.renderer.rules.image
md.renderer.rules.image = (tokens, idx, options, env, self) => {
  const token = tokens[idx]
  const src = token.attrGet('src')
  if (src && isLocalImagePath(src)) {
    token.attrSet('src', convertFileSrc(src.replace(/\\/g, '/')))
  }
  token.attrSet('loading', 'lazy')
  return defaultImage
    ? defaultImage(tokens, idx, options, env, self)
    : self.renderToken(tokens, idx, options)
}

const defaultLinkOpen = md.renderer.rules.link_open
md.renderer.rules.link_open = (tokens, idx, options, env, self) => {
  tokens[idx].attrSet('target', '_blank')
  tokens[idx].attrSet('rel', 'noopener noreferrer')
  return defaultLinkOpen
    ? defaultLinkOpen(tokens, idx, options, env, self)
    : self.renderToken(tokens, idx, options)
}

function isLocalImagePath(src: string): boolean {
  return /^(?:[A-Za-z]:[\\/]|\/|~[\\/])/.test(src)
    && /\.(png|jpe?g|gif|webp|bmp|svg)(?:[?#].*)?$/i.test(src)
}

function handleImageError(event: Event) {
  if (!(event.target instanceof HTMLImageElement)) return
  const fallback = document.createElement('span')
  fallback.className = 'v2-md-image-error'
  fallback.textContent = t('sessions.imageLoadFailed')
  event.target.replaceWith(fallback)
}

const rendered = computed(() => md.render(props.content || ''))
</script>

<style scoped>
.v2-md {
  font-size: inherit;
  line-height: 1.7;
  white-space: normal;
  word-break: break-word;
}
.v2-md :deep(p) {
  margin: 0 0 10px;
}
.v2-md :deep(p:last-child) {
  margin-bottom: 0;
}
.v2-md :deep(h1),
.v2-md :deep(h2),
.v2-md :deep(h3),
.v2-md :deep(h4) {
  margin: 16px 0 8px;
  color: var(--v2-text);
  line-height: 1.35;
}
.v2-md :deep(h1:first-child),
.v2-md :deep(h2:first-child),
.v2-md :deep(h3:first-child),
.v2-md :deep(h4:first-child) {
  margin-top: 0;
}
.v2-md :deep(h1) {
  font-size: 20px;
}
.v2-md :deep(h2) {
  font-size: 17px;
}
.v2-md :deep(h3),
.v2-md :deep(h4) {
  font-size: 15px;
}
.v2-md :deep(ul),
.v2-md :deep(ol) {
  margin: 0 0 10px;
  padding-left: 22px;
}
.v2-md :deep(li + li) {
  margin-top: 4px;
}
.v2-md :deep(a) {
  color: var(--v2-accent);
  text-decoration: none;
}
.v2-md :deep(a:hover) {
  text-decoration: underline;
}
.v2-md :deep(code) {
  padding: 2px 5px;
  border-radius: 4px;
  background: var(--v2-surface-3);
  font-family: var(--font-mono);
  font-size: 0.92em;
}
.v2-md :deep(pre) {
  margin: 10px 0;
  padding: 10px 12px;
  border-radius: 6px;
  background: var(--v2-surface-3);
  overflow-x: auto;
}
.v2-md :deep(pre code) {
  padding: 0;
  background: transparent;
  font-size: 0.9em;
}
.v2-md :deep(blockquote) {
  margin: 10px 0;
  padding-left: 12px;
  border-left: 2px solid var(--v2-surface-3);
  color: var(--v2-text-2);
}
.v2-md :deep(table) {
  width: 100%;
  margin: 10px 0;
  border-collapse: collapse;
}
.v2-md :deep(th),
.v2-md :deep(td) {
  padding: 6px 8px;
  border: 1px solid var(--v2-surface-3);
  text-align: left;
}
.v2-md :deep(img) {
  display: block;
  max-width: 100%;
  max-height: 480px;
  margin: 8px 0;
  border-radius: 8px;
  object-fit: contain;
}
.v2-md :deep(.v2-md-image-error) {
  display: inline-block;
  margin: 8px 0;
  padding: 8px 10px;
  border: 1px dashed var(--v2-surface-3);
  border-radius: 6px;
  color: var(--v2-text-3);
  font-size: var(--v2-fs-xs);
}
</style>

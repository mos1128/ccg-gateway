<template>
  <div class="v2-code-editor">
    <div ref="gutterRef" class="v2-code-editor-gutter" :style="{ width: gutterWidth }">
      <pre class="v2-code-editor-gutter-content">{{ gutterText }}</pre>
    </div>
    <textarea
      ref="textareaRef"
      class="v2-code-editor-textarea"
      :value="modelValue"
      :placeholder="placeholder"
      :disabled="disabled"
      :readonly="readonly"
      :rows="rows"
      @input="handleInput"
      @scroll="handleScroll"
      @keydown="handleKeyDown"
      @blur="emit('blur', $event)"
      @focus="emit('focus', $event)"
    ></textarea>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted } from 'vue'

const props = withDefaults(defineProps<{
  modelValue?: string
  placeholder?: string
  disabled?: boolean
  readonly?: boolean
  autoPair?: boolean
  rows?: number | string
}>(), {
  modelValue: '',
  placeholder: '',
  disabled: false,
  readonly: false,
  autoPair: true
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
  (e: 'blur', event: FocusEvent): void
  (e: 'focus', event: FocusEvent): void
}>()

const gutterRef = ref<HTMLDivElement | null>(null)
const textareaRef = ref<HTMLTextAreaElement | null>(null)

// Calculate line count to determine gutter content and width
const lineCount = computed(() => {
  const val = props.modelValue || ''
  return val.split('\n').length || 1
})

const gutterWidth = computed(() => {
  const digits = String(lineCount.value).length
  // 36px base for <= 2 digits, then 8px per extra digit
  return `${Math.max(36, 12 + digits * 8)}px`
})

const gutterText = computed(() => {
  let text = ''
  const count = lineCount.value
  for (let i = 1; i <= count; i++) {
    text += i + '\n'
  }
  return text
})

const handleInput = (e: Event) => {
  const target = e.target as HTMLTextAreaElement
  emit('update:modelValue', target.value)
}

const handleScroll = () => {
  if (gutterRef.value && textareaRef.value) {
    gutterRef.value.scrollTop = textareaRef.value.scrollTop
  }
}

// Sync scroll when modelValue changes (e.g. content height changes)
watch(() => props.modelValue, () => {
  nextTick(handleScroll)
})

onMounted(() => {
  handleScroll()
})

const insertText = (text: string, selectionOffsetStart: number, selectionOffsetEnd: number) => {
  const textarea = textareaRef.value
  if (!textarea) return

  textarea.focus()
  const start = textarea.selectionStart
  const end = textarea.selectionEnd

  let success = false
  try {
    // Try using execCommand first to preserve undo/redo history
    success = document.execCommand('insertText', false, text)
  } catch (err) {
    success = false
  }

  if (!success) {
    // Fallback if execCommand is not supported
    const value = textarea.value
    const newValue = value.substring(0, start) + text + value.substring(end)
    textarea.value = newValue
    emit('update:modelValue', newValue)
  }

  // Restore cursor selection
  nextTick(() => {
    textarea.selectionStart = start + selectionOffsetStart
    textarea.selectionEnd = start + selectionOffsetStart + selectionOffsetEnd
  })
}

const handleKeyDown = (e: KeyboardEvent) => {
  const textarea = textareaRef.value
  if (!textarea) return

  // 1. Tab Key: Insert 2 spaces
  if (e.key === 'Tab') {
    e.preventDefault()
    insertText('  ', 2, 0)
    return
  }

  // 2. Brackets / Quotes Auto Pairing
  if (props.autoPair) {
    const pairs: Record<string, string> = {
      '{': '}',
      '[': ']',
      '(': ')',
      '"': '"',
      "'": "'",
      '`': '`'
    }

    if (pairs[e.key] !== undefined) {
      e.preventDefault()
      const char = e.key
      const closingChar = pairs[char]
      const start = textarea.selectionStart
      const end = textarea.selectionEnd
      const value = textarea.value

      if (start === end) {
        insertText(char + closingChar, 1, 0)
      } else {
        const selection = value.substring(start, end)
        insertText(char + selection + closingChar, 1, selection.length)
      }
      return
    }

    // Jump over closing bracket/quote if typed manually
    const closingChars = new Set(['}', ']', ')', '"', "'", '`'])
    if (closingChars.has(e.key)) {
      const start = textarea.selectionStart
      const end = textarea.selectionEnd
      const value = textarea.value
      if (start === end && value[start] === e.key) {
        e.preventDefault()
        textarea.selectionStart = textarea.selectionEnd = start + 1
        return
      }
    }

    // Backspace: Delete matched empty pairs together
    if (e.key === 'Backspace') {
      const start = textarea.selectionStart
      const end = textarea.selectionEnd
      const value = textarea.value
      if (start === end && start > 0) {
        const charBefore = value[start - 1]
        const charAfter = value[start]
        if (pairs[charBefore] === charAfter) {
          e.preventDefault()
          const newValue = value.substring(0, start - 1) + value.substring(start + 1)
          textarea.value = newValue
          emit('update:modelValue', newValue)
          nextTick(() => {
            textarea.selectionStart = textarea.selectionEnd = start - 1
          })
          return
        }
      }
    }
  }
}
</script>

<style scoped>
.v2-code-editor {
  display: flex;
  width: 100%;
  height: 100%;
  min-height: 0;
  background: transparent;
  border-radius: inherit;
  font-size: 13px;
  line-height: 20px;
  font-family: var(--font-mono), monospace;
  box-sizing: border-box;
}

.v2-code-editor-gutter {
  flex-shrink: 0;
  background: var(--v2-bg-base);
  overflow: hidden;
  user-select: none;
  box-sizing: border-box;
}

.v2-code-editor-gutter-content {
  margin: 0;
  padding: 12px 8px;
  box-sizing: border-box;
  text-align: right;
  color: var(--v2-text-3);
  font-size: inherit;
  line-height: inherit;
  font-family: inherit;
  white-space: pre;
  opacity: 0.65;
}

.v2-code-editor-textarea {
  flex: 1;
  margin: 0;
  padding: 12px 14px;
  box-sizing: border-box;
  background: var(--v2-surface);
  border: none;
  outline: none;
  color: var(--v2-text);
  font-size: inherit;
  line-height: inherit;
  font-family: inherit;
  resize: none;
  white-space: pre;
  overflow: auto;
  tab-size: 2;
}

/* Custom Scrollbar Styles for the Textarea */
.v2-code-editor-textarea::-webkit-scrollbar {
  width: 10px;
  height: 10px;
}
.v2-code-editor-textarea::-webkit-scrollbar-track {
  background: transparent;
}
.v2-code-editor-textarea::-webkit-scrollbar-thumb {
  background: color-mix(in srgb, var(--v2-surface-3) 70%, transparent);
  border-radius: 5px;
  border: 2.5px solid var(--v2-surface);
  background-clip: padding-box;
}
.v2-code-editor-textarea::-webkit-scrollbar-thumb:hover {
  background: var(--v2-text-3);
  border: 2.5px solid var(--v2-surface);
  background-clip: padding-box;
}
</style>

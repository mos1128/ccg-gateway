<template>
  <AppModal v-model="visible" title="添加插件市场" width="500px" @confirm="$emit('confirm')">
    <div class="form-group">
      <label class="c-label">市场源地址 <span class="required">*</span></label>
      <input
        type="text"
        :value="url"
        class="b-input"
        placeholder="支持 URL 地址、GitHub owner/repo、本地路径"
        @input="handleInput"
      >
    </div>
  </AppModal>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import AppModal from '@/components/AppModal.vue'

const props = defineProps<{
  modelValue: boolean
  url: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  'update:url': [value: string]
  confirm: []
}>()

const visible = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value)
})

function handleInput(event: Event) {
  emit('update:url', (event.target as HTMLInputElement).value)
}
</script>

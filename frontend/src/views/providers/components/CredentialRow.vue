<template>
  <div class="pt-row" :class="{ off: !credential.is_written }">
    <div class="pt-drag" :aria-label="t('providers.row.dragAria')">
      <svg width="10" height="16" viewBox="0 0 10 16" fill="currentColor"><circle cx="3" cy="3" r="1.5"/><circle cx="3" cy="8" r="1.5"/><circle cx="3" cy="13" r="1.5"/><circle cx="7" cy="3" r="1.5"/><circle cx="7" cy="8" r="1.5"/><circle cx="7" cy="13" r="1.5"/></svg>
    </div>
    <div class="pt-cols m-cred">
      <div class="pt-switch">
        <el-switch :model-value="credential.is_written" :loading="writeLoading" @change="onWriteSwitchChange" />
      </div>
      <div class="pt-name">
        <OverflowText :text="credential.name" />
      </div>
      <div class="pt-cell mono" :class="{ muted: !credential.display_info }"><OverflowText :text="credential.display_info || '—'" /></div>
      <div><span class="v2-pill dot" :class="credential.is_written ? 'v2-pill-success' : 'v2-pill-neutral'">{{ credential.is_written ? t('providers.credential.written') : t('providers.credential.notWritten') }}</span></div>
      <div class="pt-acts">
        <el-tooltip :content="t('common.edit')" placement="top" effect="light" :show-after="250">
          <button class="pt-act" @click="emit('edit', credential)"><svg width="17" height="17"><use href="#v2i-edit"/></svg></button>
        </el-tooltip>
        <el-tooltip :content="t('common.delete')" placement="top" effect="light" :show-after="250">
          <button class="pt-act danger" @click="emit('delete', credential)"><svg width="17" height="17"><use href="#v2i-trash"/></svg></button>
        </el-tooltip>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import OverflowText from '@/components/OverflowText.vue'
import { useI18n } from 'vue-i18n'
import type { OfficialCredential } from '@/types/models'

const { t } = useI18n()
const props = defineProps<{
  credential: OfficialCredential
  writeLoading?: boolean
}>()

const emit = defineEmits<{
  write: [credential: OfficialCredential]
  edit: [credential: OfficialCredential]
  delete: [credential: OfficialCredential]
}>()

function onWriteSwitchChange(value: string | number | boolean) {
  if (props.writeLoading || props.credential.is_written || value !== true) return
  emitWrite()
}
function emitWrite() {
  if (props.writeLoading) return
  emit('write', props.credential)
}
</script>

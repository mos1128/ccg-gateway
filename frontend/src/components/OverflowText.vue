<template>
  <el-tooltip effect="light" placement="top" :content="String(text)" :disabled="!overflow" :show-after="300" :enterable="true" popper-class="v2-of-pop">
    <span ref="el" class="of-text">{{ text }}</span>
  </el-tooltip>
</template>

<script setup lang="ts">
const props = defineProps<{ text: string | number }>()

const el = ref<HTMLElement>()
const overflow = ref(false)
let ro: ResizeObserver | null = null

function check() {
  const e = el.value
  if (e) overflow.value = e.scrollWidth > e.clientWidth + 1
}

onMounted(() => {
  check()
  ro = new ResizeObserver(check)
  if (el.value) ro.observe(el.value)
})
watch(() => props.text, () => nextTick(check))
onBeforeUnmount(() => ro?.disconnect())
</script>

<style scoped>
.of-text { display: block; max-width: 100%; min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
</style>

<style>
/* 溢出提示的 popper：限宽并允许长路径/URL 折行 */
.v2-of-pop.el-popper {
  max-width: 480px;
  overflow-wrap: anywhere;
}
</style>

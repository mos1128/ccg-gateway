<template>
  <div ref="root" class="v2-tabs" :class="{ 'is-overflowing': isOverflowing }">
    <button
      v-if="isOverflowing"
      class="v2-tabs-arrow v2-tabs-arrow-left"
      type="button"
      aria-label="向左滚动标签"
      :disabled="!canScrollLeft"
      @click="scrollTabs(-1)"
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="m15 18-6-6 6-6" />
      </svg>
    </button>

    <div ref="scroller" class="v2-tabs-track" @scroll="updateState">
      <slot />
    </div>

    <button
      v-if="isOverflowing"
      class="v2-tabs-arrow v2-tabs-arrow-right"
      type="button"
      aria-label="向右滚动标签"
      :disabled="!canScrollRight"
      @click="scrollTabs(1)"
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="m9 18 6-6-6-6" />
      </svg>
    </button>
  </div>
</template>

<script setup lang="ts">
const scroller = ref<HTMLElement | null>(null)
const root = ref<HTMLElement | null>(null)
const isOverflowing = ref(false)
const canScrollLeft = ref(false)
const canScrollRight = ref(false)

let resizeObserver: ResizeObserver | null = null
let mutationObserver: MutationObserver | null = null

function updateState() {
  const element = scroller.value
  const container = root.value
  if (!element || !container) return

  const overflowing = element.scrollWidth > container.clientWidth + 1

  if (isOverflowing.value !== overflowing) {
    isOverflowing.value = overflowing
    if (!overflowing) element.scrollLeft = 0
    nextTick(updateState)
  }

  const maxScrollLeft = Math.max(0, element.scrollWidth - element.clientWidth)
  canScrollLeft.value = overflowing && element.scrollLeft > 1
  canScrollRight.value = overflowing && element.scrollLeft < maxScrollLeft - 1
}

function scrollTabs(direction: -1 | 1) {
  const element = scroller.value
  if (!element) return
  element.scrollBy({ left: direction * Math.max(160, element.clientWidth * 0.6), behavior: 'smooth' })
}

function scrollActiveTabIntoView() {
  const element = scroller.value
  const activeTab = element?.querySelector<HTMLElement>('.v2-tab.active')
  if (!element || !activeTab) return

  const visibleLeft = element.scrollLeft
  const visibleRight = element.scrollLeft + element.clientWidth
  const tabLeft = activeTab.offsetLeft
  const tabRight = tabLeft + activeTab.offsetWidth

  if (tabLeft < visibleLeft) {
    element.scrollTo({ left: tabLeft, behavior: 'smooth' })
  } else if (tabRight > visibleRight) {
    element.scrollTo({ left: tabRight - element.clientWidth, behavior: 'smooth' })
  }
}

onMounted(() => {
  const element = scroller.value
  if (!element) return

  resizeObserver = new ResizeObserver(updateState)
  resizeObserver.observe(root.value!)

  mutationObserver = new MutationObserver(() => {
    nextTick(() => {
      updateState()
      scrollActiveTabIntoView()
    })
  })
  mutationObserver.observe(element, {
    childList: true,
    subtree: true,
    characterData: true,
    attributes: true,
    attributeFilter: ['class'],
  })

  nextTick(() => {
    updateState()
    scrollActiveTabIntoView()
  })
})

onBeforeUnmount(() => {
  resizeObserver?.disconnect()
  mutationObserver?.disconnect()
})
</script>

<style scoped>
.v2-tabs {
  position: relative;
  display: block;
  min-width: 0;
}
.v2-tabs.is-overflowing {
  display: grid;
  grid-template-columns: 28px minmax(0, 1fr) 28px;
}
.v2-tabs-track {
  box-sizing: border-box;
  display: flex;
  width: 100%;
  min-width: 0;
  overflow-x: auto;
  overflow-y: hidden;
  scrollbar-width: none;
}
.v2-tabs-track::-webkit-scrollbar {
  display: none;
}
.v2-tabs-track :deep(.v2-tab) {
  flex: 0 0 auto;
  white-space: nowrap;
}
.v2-tabs-arrow {
  display: inline-flex;
  width: 28px;
  min-height: 36px;
  align-items: center;
  justify-content: center;
  padding: 0;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: var(--v2-text-2);
  cursor: pointer;
  transition: color 0.15s, background-color 0.15s, opacity 0.15s;
}
.v2-tabs-arrow:hover:not(:disabled) {
  background: var(--v2-surface-2);
  color: var(--v2-text);
}
.v2-tabs-arrow:disabled {
  color: var(--v2-text-3);
  cursor: default;
  opacity: 0.35;
}
</style>

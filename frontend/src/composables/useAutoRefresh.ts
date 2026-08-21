import type { Ref } from 'vue'

export interface AutoRefreshOptions {
  intervalMs: number
  immediate?: boolean
  onError?: (error: unknown) => void
  paused?: Ref<boolean>
}

export type AutoRefreshTrigger = 'initial' | 'interval' | 'activation' | 'manual'

export function useAutoRefresh(
  refreshFn: (trigger: AutoRefreshTrigger) => Promise<unknown>,
  options: AutoRefreshOptions
) {
  let timer: number | null = null
  let inflight: Promise<unknown> | null = null

  function clearTimer() {
    if (timer === null) return
    window.clearTimeout(timer)
    timer = null
  }

  function scheduleNext() {
    clearTimer()
    if (options.paused?.value) return
    timer = window.setTimeout(() => {
      timer = null
      void refresh('interval')
    }, options.intervalMs)
  }

  async function refresh(trigger: AutoRefreshTrigger = 'manual', force = false) {
    if (trigger !== 'manual' && options.paused?.value) return
    if (!force && document.visibilityState !== 'visible') return
    if (inflight) return inflight
    clearTimer()
    inflight = refreshFn(trigger).catch((e) => {
      options.onError?.(e)
      return undefined
    })
    try {
      await inflight
    } finally {
      inflight = null
      scheduleNext()
    }
  }

  function handleFocus() {
    void refresh('activation', true)
  }

  function handleVisibilityChange() {
    if (document.visibilityState === 'visible') void refresh('activation', true)
  }

  onMounted(() => {
    window.addEventListener('focus', handleFocus)
    document.addEventListener('visibilitychange', handleVisibilityChange)
    if (options.immediate) void refresh('initial', true)
    else scheduleNext()
  })

  onUnmounted(() => {
    clearTimer()
    window.removeEventListener('focus', handleFocus)
    document.removeEventListener('visibilitychange', handleVisibilityChange)
  })

  return { refresh }
}

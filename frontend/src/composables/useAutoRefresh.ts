export interface AutoRefreshOptions {
  intervalMs: number
  immediate?: boolean
  onError?: (error: unknown) => void
}

export function useAutoRefresh(
  refreshFn: () => Promise<unknown>,
  options: AutoRefreshOptions
) {
  let timer: number | null = null
  let inflight: Promise<unknown> | null = null

  async function refresh(force = false) {
    if (!force && document.visibilityState !== 'visible') return
    if (inflight) return inflight
    inflight = refreshFn().catch((e) => {
      options.onError?.(e)
      return undefined
    })
    try {
      await inflight
    } finally {
      inflight = null
    }
  }

  function handleFocus() {
    void refresh(true)
  }

  function handleVisibilityChange() {
    if (document.visibilityState === 'visible') void refresh(true)
  }

  onMounted(() => {
    window.addEventListener('focus', handleFocus)
    document.addEventListener('visibilitychange', handleVisibilityChange)
    timer = window.setInterval(() => { void refresh() }, options.intervalMs)
    if (options.immediate) void refresh(true)
  })

  onUnmounted(() => {
    if (timer !== null) {
      window.clearInterval(timer)
      timer = null
    }
    window.removeEventListener('focus', handleFocus)
    document.removeEventListener('visibilitychange', handleVisibilityChange)
  })

  return { refresh }
}

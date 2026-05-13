const fs = require('fs');
const file = 'frontend/src/views/dashboard/index.vue';
let content = fs.readFileSync(file, 'utf-8');

// 1. Replace the template area (bottom charts)
content = content.replace(
/<!-- 底部图表与明细 -->[\s\S]*?<\/template>/,
`<!-- 底部图表与明细 -->
      <div style="display: flex; gap: 24px; flex-direction: column;">
        <div class="b-card responsive-bottom-card" style="width: 100%; margin-bottom: 0; padding: 24px;">
          <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; flex-wrap: wrap; gap: 16px;">
            <div class="b-card-title" style="margin-bottom: 0;">统计总览</div>
            <div style="display: flex; gap: 16px; align-items: center; flex-wrap: wrap;">
              <div class="b-segmented">
                <div class="b-seg-btn" :class="{ active: metricMode === 'requests' }" @click="metricMode = 'requests'">请求数</div>
                <div class="b-seg-btn" :class="{ active: metricMode === 'tokens' }" @click="metricMode = 'tokens'">Token</div>
              </div>
              <div class="b-segmented">
                <div class="b-seg-btn" :class="{ active: dimMode === 'provider' }" @click="dimMode = 'provider'">服务商</div>
                <div class="b-seg-btn" :class="{ active: dimMode === 'model' }" @click="dimMode = 'model'">模型</div>
              </div>
            </div>
          </div>
          <div style="height: 360px; width: 100%;">
            <v-chart class="chart" :option="chartOption" autoresize />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>`
);

// 2. Add back UI State
content = content.replace(
  /\/\/ UI State removed/,
  `// UI State\nconst metricMode = ref<'requests' | 'tokens'>('requests')\nconst dimMode = ref<'provider' | 'model'>('provider')`
);

// 3. Re-write Chart Logic
content = content.replace(
/\/\/ === Chart Logic ===[\s\S]*?onMounted\(\(\) => \{/,
`// === Chart Logic ===
const PALETTE = ['#5470c6', '#91cc75', '#fac858', '#ee6666', '#73c0de', '#3ba272', '#fc8452', '#9a60b4', '#ea7ccc', '#00b4d8', '#f472b6', '#fbbf24']
const BAR_RADIUS = 4

const chartOption = computed(() => {
  const dates: string[] = []
  for (let i = 6; i >= 0; i--) {
    const d = new Date()
    d.setDate(d.getDate() - i)
    dates.push(formatLocalDate(d))
  }

  const isTokens = metricMode.value === 'tokens'
  const groupKey = dimMode.value === 'provider' ? 'provider_name' : 'model_id'

  // Get unique groups and total for sorting
  const groupTotals = new Map<string, number>()
  advancedStats.value.forEach(s => {
    const val = isTokens ? s.total_tokens : s.total_success
    groupTotals.set(s[groupKey], (groupTotals.get(s[groupKey]) || 0) + val)
  })
  const groupArray = Array.from(groupTotals.entries())
    .sort((a, b) => b[1] - a[1])
    .map(([name]) => name)

  const seriesData: any[] = groupArray.map((gName, idx) => {
    const color = PALETTE[idx % PALETTE.length]

    if (isTokens) {
      const data = dates.map(d => {
        let sum = 0, input = 0, output = 0, cache = 0
        advancedStats.value.forEach(s => {
          if (s.date === d && s[groupKey] === gName) {
            sum += s.total_tokens
            input += s.total_input_tokens || 0
            output += s.total_output_tokens || 0
            cache += (s.total_cache_read_tokens || 0) + (s.total_cache_creation_tokens || 0)
          }
        })
        return { value: sum, input, output, cache, name: gName }
      })
      return { name: gName, type: 'bar', stack: 'total', barWidth: '60%', itemStyle: { color }, data }
    } else {
      const data = dates.map(d => {
        let sum = 0
        advancedStats.value.forEach(s => {
          if (s.date === d && s[groupKey] === gName) {
            sum += s.total_success
          }
        })
        return sum
      })
      return {
        name: gName, type: 'line', smooth: true, showSymbol: false, itemStyle: { color },
        areaStyle: {
          color: { type: 'linear', x: 0, y: 0, x2: 0, y2: 1, colorStops: [{ offset: 0, color }, { offset: 1, color: 'transparent' }] },
          opacity: 0.2
        },
        data
      }
    }
  })

  // Apply border radius for bar chart
  if (isTokens) {
    const topSeriesByDate = dates.map((_, dateIdx) => {
      for (let seriesIdx = seriesData.length - 1; seriesIdx >= 0; seriesIdx--) {
        if (seriesData[seriesIdx].data[dateIdx].value > 0) return seriesIdx
      }
      return -1
    })
    seriesData.forEach((series, seriesIdx) => {
      series.data = series.data.map((item: any, dateIdx: number) => {
        if (item.value <= 0 || topSeriesByDate[dateIdx] !== seriesIdx) return item
        return { ...item, itemStyle: { borderRadius: [BAR_RADIUS, BAR_RADIUS, 0, 0] } }
      })
    })
  }

  return {
    tooltip: {
      trigger: 'axis',
      axisPointer: { type: isTokens ? 'shadow' : 'line' },
      backgroundColor: 'rgba(255, 255, 255, 0.95)',
      borderColor: '#e2e8f0',
      textStyle: { color: '#0f172a' },
      formatter: (params: any[]) => {
        if (!params.length) return ''
        const date = params[0].name
        let html = \`<div style="font-weight: 600; margin-bottom: 8px;">\${date}</div>\`

        if (isTokens) {
          params.forEach(p => {
            if (p.value > 0) {
              const d = p.data
              html += \`<div style="margin-bottom: 6px;">
                <div style="display: flex; align-items: center; gap: 6px; font-weight: 600;">
                  <span style="display:inline-block;width:10px;height:10px;border-radius:50%;background-color:\${p.color};"></span>
                  \${d.name} (总计: \${d.value})
                </div>
                <div style="padding-left: 16px; color: #64748b; font-size: 13px;">
                  <div>- 输入: \${d.input}</div>
                  <div>- 输出: \${d.output}</div>
                  <div>- 缓存: \${d.cache}</div>
                </div>
              </div>\`
            }
          })
        } else {
          params.forEach(p => {
            if (p.value > 0) {
              html += \`<div style="margin-bottom: 4px; display: flex; align-items: center; gap: 6px;">
                <span style="display:inline-block;width:10px;height:10px;border-radius:50%;background-color:\${p.color};"></span>
                <span style="font-weight: 500;">\${p.seriesName}:</span>
                <span>\${p.value}</span>
              </div>\`
            }
          })
        }
        return html
      }
    },
    legend: { bottom: 0, left: 'center', type: 'scroll', icon: 'circle', textStyle: { color: '#64748b' } },
    grid: { top: 20, right: '3%', bottom: 40, left: '3%', containLabel: true },
    xAxis: { type: 'category', data: dates, boundaryGap: isTokens, axisLine: { lineStyle: { color: '#e2e8f0' } }, axisLabel: { color: '#64748b' } },
    yAxis: {
      type: 'value',
      splitNumber: 4,
      splitLine: { lineStyle: { type: 'dashed', color: '#f1f5f9' } },
      axisLabel: {
        color: '#64748b',
        formatter: (value: number) => {
          if (value >= 1000000) return (value / 1000000).toFixed(1) + 'M'
          if (value >= 1000) return (value / 1000).toFixed(1) + 'K'
          return value
        }
      }
    },
    series: seriesData
  }
})

onMounted(() => {`
);

fs.writeFileSync(file, content);
console.log('Done modifying dashboard.');
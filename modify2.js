const fs = require('fs');
const file = 'frontend/src/views/dashboard/index.vue';
let content = fs.readFileSync(file, 'utf-8');

// 1. Fix Height
content = content.replace(/height: 360px; width: 100%;/g, 'height: 260px; width: 100%;');

// 2. Fix K/M formatting in tooltip
content = content.replace(
/\/\/ === Chart Logic ===[\s\S]*?const chartOption = computed/,
`// === Chart Logic ===
const PALETTE = ['#5470c6', '#91cc75', '#fac858', '#ee6666', '#73c0de', '#3ba272', '#fc8452', '#9a60b4', '#ea7ccc', '#00b4d8', '#f472b6', '#fbbf24']
const BAR_RADIUS = 4

function formatTokenValue(value: number): string {
  if (value >= 1000000) return (value / 1000000).toFixed(1) + 'M'
  if (value >= 1000) return (value / 1000).toFixed(1) + 'K'
  return value.toString()
}

const chartOption = computed`
);

content = content.replace(
  /<div>- 输入: \\\$\{d\.input\}<\/div>\s*<div>- 输出: \\\$\{d\.output\}<\/div>\s*<div>- 缓存: \\\$\{d\.cache\}<\/div>/,
  `<div>- 输入: \${formatTokenValue(d.input)}</div>
                  <div>- 输出: \${formatTokenValue(d.output)}</div>
                  <div>- 缓存: \${formatTokenValue(d.cache)}</div>`
);

content = content.replace(
  /\\\$\{d\.name\} \(总计: \\\$\{d\.value\}\)/,
  `\${d.name} (总计: \${formatTokenValue(d.value)})`
);

fs.writeFileSync(file, content);
console.log('done');
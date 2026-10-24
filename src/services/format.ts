const MINUTE = 60_000
const HOUR = 60 * MINUTE
const DAY = 24 * HOUR

export function formatRelativeTime(timestamp: number, now = Date.now()): string {
  const delta = Math.max(0, now - timestamp)
  if (delta < MINUTE) return '刚刚'
  if (delta < HOUR) return `${Math.floor(delta / MINUTE)} 分钟前`
  if (delta < DAY) return `${Math.floor(delta / HOUR)} 小时前`
  if (delta < 7 * DAY) return `${Math.floor(delta / DAY)} 天前`

  const date = new Date(timestamp)
  const sameYear = date.getFullYear() === new Date(now).getFullYear()
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  return sameYear ? `${month}-${day}` : `${date.getFullYear()}-${month}-${day}`
}

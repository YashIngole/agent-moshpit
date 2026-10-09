// Small time formatters. All take milliseconds.

/** "4:12" for a countdown. Never negative. */
export function countdown(ms: number): string {
  const total = Math.max(0, Math.ceil(ms / 1000))
  const minutes = Math.floor(total / 60)
  const seconds = total % 60
  return `${minutes}:${String(seconds).padStart(2, '0')}`
}

/** How long something took, from seconds: "0.4 sec", "12 sec", "1 min 42 sec", "2 hr 5 min". */
export function took(secs: number): string {
  if (!Number.isFinite(secs) || secs < 0) return ''
  // Only a short wait is worth a tenth of a second.
  if (secs < 9.95 && !Number.isInteger(secs)) return `${secs.toFixed(1)} sec`
  const whole = Math.round(secs)
  if (whole < 60) return `${whole} sec`
  const minutes = Math.floor(whole / 60)
  if (minutes < 60) return whole % 60 ? `${minutes} min ${whole % 60} sec` : `${minutes} min`
  const hours = Math.floor(minutes / 60)
  return minutes % 60 ? `${hours} hr ${minutes % 60} min` : `${hours} hr`
}

/** "just now", "40 sec", "12 min", "3 hr", "2 days". */
export function elapsed(ms: number): string {
  const seconds = Math.max(0, Math.floor(ms / 1000))
  if (seconds < 10) return 'just now'
  if (seconds < 60) return `${seconds} sec`
  const minutes = Math.floor(seconds / 60)
  if (minutes < 60) return `${minutes} min`
  const hours = Math.floor(minutes / 60)
  if (hours < 48) return `${hours} hr`
  return `${Math.floor(hours / 24)} days`
}

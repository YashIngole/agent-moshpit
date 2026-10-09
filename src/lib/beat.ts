// The office's one clock for movement.
//
// People do not use CSS animations for their loops. A running CSS animation
// makes the browser restyle every frame, sixty times a second, for as long as
// it exists. Instead this sets three small counters on <html> about three
// times a second, and the stylesheet picks a pose from them. Between beats
// the page is idle, and while the window is hidden there are no beats at all.

const BEAT_MS = 320
const BLINK_EVERY_MS = 4300
const BLINK_MS = 130

let timer: ReturnType<typeof setInterval> | undefined
let blinkTimer: ReturnType<typeof setInterval> | undefined
let beat = 0

function tick() {
  beat = (beat + 1) % 12
  const root = document.documentElement
  root.dataset.b2 = String(beat % 2)
  root.dataset.b3 = String(beat % 3)
  root.dataset.b4 = String(beat % 4)
}

function blink() {
  const root = document.documentElement
  root.dataset.blink = ''
  setTimeout(() => delete root.dataset.blink, BLINK_MS)
}

function wanted(): boolean {
  return !document.hidden && !matchMedia('(prefers-reduced-motion: reduce)').matches
}

function sync() {
  const on = wanted()
  document.documentElement.classList.toggle('still', !on)
  if (on && timer === undefined) {
    tick()
    timer = setInterval(tick, BEAT_MS)
    blinkTimer = setInterval(blink, BLINK_EVERY_MS)
  } else if (!on && timer !== undefined) {
    clearInterval(timer)
    clearInterval(blinkTimer)
    timer = blinkTimer = undefined
  }
}

/** Start the beat and keep it in step with whether anyone can see the window. */
export function startBeat(): () => void {
  const motion = matchMedia('(prefers-reduced-motion: reduce)')
  document.addEventListener('visibilitychange', sync)
  motion.addEventListener('change', sync)
  sync()
  return () => {
    document.removeEventListener('visibilitychange', sync)
    motion.removeEventListener('change', sync)
    clearInterval(timer)
    clearInterval(blinkTimer)
    timer = blinkTimer = undefined
  }
}

// agentmoshpit.com: two small things, and the page is whole without either.
//
// 1. The picture of the app at the top is replaced by the app's own window, running
//    here with its demo data (site/demo, made by `npm run build:site`).
// 2. Each install command gets a button that copies it.
//
// This script sends nothing anywhere and stores nothing.
;(() => {
  'use strict'

  // ── a command to paste: a button that copies it ──
  for (const button of document.querySelectorAll('button.copy[data-copy]')) {
    if (!navigator.clipboard) continue
    button.hidden = false
    button.addEventListener('click', () => {
      navigator.clipboard.writeText(button.dataset.copy).then(
        () => {
          button.textContent = 'copied'
          setTimeout(() => (button.textContent = 'copy'), 1600)
        },
        () => {}
      )
    })
  }

  // ── the app itself, where the picture of it is ──

  /**
   * Keep the page where the reader has it for a moment. The app brings a desk it opens
   * into view, and inside a frame that would move this page too. The first thing the
   * reader does themselves ends the hold.
   */
  function hold(ms) {
    const [left, top] = [scrollX, scrollY]
    const html = document.documentElement
    const signs = ['wheel', 'touchstart', 'keydown', 'pointerdown']
    const back = () => scrollTo({ left, top, behavior: 'instant' })
    const free = () => {
      removeEventListener('scroll', back)
      signs.forEach(sign => removeEventListener(sign, free))
      html.style.scrollBehavior = ''
    }
    html.style.scrollBehavior = 'auto'
    addEventListener('scroll', back)
    signs.forEach(sign => addEventListener(sign, free, { passive: true }))
    setTimeout(free, ms)
  }

  /** Open three terminals beside the floor, as in the picture this takes the place of. */
  function watch(app) {
    const office = app.contentDocument
    const desk = n => office.querySelector(`.floor button[data-desk="demo-${n}"]`)
    try {
      hold(1200)
      desk(2)?.click()
      for (const n of [5, 6]) desk(n)?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, ctrlKey: true, view: app.contentWindow }))
      // A terminal that opens takes the keyboard. Nobody asked it to here: the page keeps it,
      // and the floor goes back to its first room.
      setTimeout(() => {
        office.activeElement?.blur?.()
        app.blur()
        office.querySelector('.floor')?.scrollTo(0, 0)
      }, 350)
    } catch {
      // the floor alone is still the app
    }
  }

  function live() {
    const frame = document.getElementById('window')
    const glass = frame && frame.querySelector('.glass')
    const caption = document.getElementById('window-words')
    if (!frame || !glass || !caption) return
    // A wide window with a mouse: the app is a desktop app, and a phone gets its picture.
    const roomy = window.matchMedia('(min-width: 960px) and (hover: hover) and (pointer: fine)').matches
    const saving = navigator.connection && navigator.connection.saveData
    if (!roomy || saving) return
    const still = window.matchMedia('(prefers-reduced-motion: reduce)').matches
    const app = document.createElement('iframe')
    app.title = 'Agent Moshpit, running with demo data'
    app.loading = 'lazy'
    app.src = `demo/?demo=office${still ? '&still' : ''}`
    app.addEventListener('load', () => {
      // Shown only once the office is drawn; a missing demo leaves the picture as it is.
      let tries = 0
      const drawn = () => {
        let ready = false
        try {
          ready = Boolean(app.contentDocument && app.contentDocument.querySelector('.floor button[data-desk]'))
        } catch {
          ready = false
        }
        if (ready) {
          watch(app)
          frame.classList.add('live')
          caption.textContent = 'This is the app itself, running here with its demo data. Click a desk, or type in a terminal.'
        } else if (++tries < 40) {
          setTimeout(drawn, 150)
        } else {
          app.remove()
        }
      }
      drawn()
    })
    glass.append(app)
  }

  // `?card` is the page as a link to it shows it (tools/site-shots.mjs): the picture, not the app.
  if (new URLSearchParams(location.search).has('card')) return
  if (document.readyState === 'complete') live()
  else window.addEventListener('load', live, { once: true })
})()

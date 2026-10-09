// agentmoshpit.com: two small things, and the page is whole without either.
//
// 1. The download button is pointed at the file for the system this is read on.
//    The links themselves are in the page (the "download" room); this only picks one.
// 2. The picture of the app at the top is replaced by the app's own window, running
//    here with its demo data (site/demo, made by `npm run build:site`).
//
// This script sends nothing anywhere and stores nothing.
;(() => {
  'use strict'

  // ── which system is this ──
  const ua = navigator.userAgent || ''
  const platform = (navigator.userAgentData && navigator.userAgentData.platform) || navigator.platform || ''

  function system() {
    // An iPad says it is a Mac; a Mac has no touch screen.
    if (/Android|iPhone|iPad|iPod/i.test(ua) || (/Mac/i.test(platform) && navigator.maxTouchPoints > 1)) return 'phone'
    if (/Win/i.test(platform) || /Windows/i.test(ua)) return 'windows'
    if (/Mac/i.test(platform) || /Macintosh/i.test(ua)) return 'mac'
    if (/Linux|X11|CrOS/i.test(platform) || /Linux|X11/i.test(ua)) return 'linux'
    return ''
  }

  /** The file to offer first, by its `data-file` in the page, and what to call it. */
  async function pick(os) {
    if (os === 'windows') return { file: 'windows', words: 'Download for Windows' }
    if (os === 'linux') {
      if (/Ubuntu|Debian|Mint|Pop!_OS/i.test(ua)) return { file: 'linux-deb', words: 'Download for Linux (.deb)' }
      if (/Fedora|Red Hat|CentOS|SUSE/i.test(ua)) return { file: 'linux-rpm', words: 'Download for Linux (.rpm)' }
      return { file: 'linux-appimage', words: 'Download for Linux (AppImage)' }
    }
    if (os === 'mac') {
      // Most Macs sold since 2020 are Apple Silicon, and no browser says "Intel" reliably:
      // Apple Silicon is offered unless the browser says otherwise.
      let intel = false
      try {
        const high = await navigator.userAgentData?.getHighEntropyValues?.(['architecture'])
        intel = high?.architecture === 'x86'
      } catch {
        // not a browser that says
      }
      return intel ? { file: 'mac-intel', words: 'Download for Mac (Intel)' } : { file: 'mac-arm', words: 'Download for Mac (Apple Silicon)' }
    }
    return null
  }

  const ALL = '<a href="https://github.com/YashIngole/agent-moshpit/releases/latest">All downloads</a>'
  const OTHERS = {
    windows: `Free and open source. Also for <a href="#macos">macOS</a> and <a href="#linux">Linux</a>. ${ALL}`,
    mac: `Free and open source. <a href="#macos">An Intel Mac?</a> Also for <a href="#windows">Windows</a> and <a href="#linux">Linux</a>. ${ALL}`,
    'mac-intel': `Free and open source. <a href="#macos">Apple Silicon?</a> Also for <a href="#windows">Windows</a> and <a href="#linux">Linux</a>. ${ALL}`,
    linux: `Free and open source. Also as <a href="#linux">.deb, .rpm and AppImage</a>, and for <a href="#windows">Windows</a> and <a href="#macos">macOS</a>. ${ALL}`,
    phone: `It is a desktop app: free and open source, for <a href="#windows">Windows</a>, <a href="#macos">macOS</a> and <a href="#linux">Linux</a>. ${ALL}`
  }

  async function download() {
    const os = system()
    const button = document.getElementById('get')
    const words = document.getElementById('get-words')
    const also = document.getElementById('also')
    if (!button || !words || !also) return
    if (os === 'phone') {
      words.textContent = 'See the downloads'
      also.innerHTML = OTHERS.phone
      return
    }
    const chosen = await pick(os)
    const link = chosen && document.querySelector(`a[data-file="${chosen.file}"]`)
    if (!link) return
    button.href = link.href
    words.textContent = chosen.words
    link.classList.add('yours')
    also.innerHTML = OTHERS[chosen.file === 'mac-intel' ? 'mac-intel' : os]
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

  // `?card` is the page as a link to it shows it (tools/site-shots.mjs): for no system in particular.
  if (new URLSearchParams(location.search).has('card')) return
  download()
  if (document.readyState === 'complete') live()
  else window.addEventListener('load', live, { once: true })
})()

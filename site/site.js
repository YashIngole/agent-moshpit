// agentmoshpit.com: three small things, and the page is whole without any of them.
//
// 1. The picture of the app at the top is replaced by the app's own window, running
//    here with its demo data (site/demo, made by `npm run build:site`).
// 2. Each install command gets a button that copies it.
// 3. The bar's download button waits until the opening's own is out of sight.
//
// This script sends nothing anywhere and stores nothing.
;(() => {
  'use strict'

  // ── a command to paste: a button that copies it ──
  const buttons = [...document.querySelectorAll('button.copy[data-copy]')]
  if (buttons.length && navigator.clipboard) {
    // Said aloud to a screen reader; the button's own word changes for everyone else.
    const said = document.createElement('p')
    said.className = 'unseen'
    said.setAttribute('role', 'status')
    document.body.append(said)
    for (const button of buttons) {
      button.hidden = false
      button.addEventListener('click', () => {
        navigator.clipboard.writeText(button.dataset.copy).then(
          () => {
            button.textContent = 'copied'
            said.textContent = 'Copied.'
            setTimeout(() => {
              button.textContent = 'copy'
              said.textContent = ''
            }, 1600)
          },
          () => {
            // The browser would not write to the clipboard: select the command, ready for Ctrl+C.
            const command = button.parentElement.querySelector('code')
            if (command) getSelection().selectAllChildren(command)
            said.textContent = 'Could not copy. The command is selected.'
          }
        )
      })
    }
  }

  // ── one download button in sight at a time ──
  // The bar's is hidden in the markup while the page opens on its own (class "at-top").
  const bar = document.querySelector('.bar.at-top')
  const first = document.querySelector('.get .button')
  if (bar && first && 'IntersectionObserver' in window) {
    const under = getComputedStyle(bar).height
    new IntersectionObserver(([seen]) => bar.classList.toggle('at-top', seen.isIntersecting), { rootMargin: `-${under} 0px 0px 0px` }).observe(first)
  } else if (bar) {
    bar.classList.remove('at-top')
  }

  // ── the app itself, where the picture of it is ──

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
    // The words under the window say how to use it, and the key that leaves a terminal.
    app.setAttribute('aria-describedby', caption.id)
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
          frame.classList.add('live')
          // A terminal keeps Tab for its program, as in the app: the caption names the key that leaves one.
          const key = document.createElement('kbd')
          key.textContent = 'Ctrl+`'
          caption.replaceChildren('This is the app itself, running here with its demo data. Press “open their terminal” or click any desk; ', key, ' puts the terminals away again.')
          // The picture has done its job: it is no longer there to be read out a second time.
          const picture = glass.querySelector('img')
          if (picture) setTimeout(() => (picture.hidden = true), 600)
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

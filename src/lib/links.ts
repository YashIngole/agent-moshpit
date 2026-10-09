// Which addresses printed in a terminal may be opened in the browser.

const LOCAL = /^(localhost|127\.0\.0\.1|\[::1\])$/

/**
 * A web address this app will open, and the host it leads to; null for anything else.
 * https anywhere, plain http only on this computer (a development server), and never
 * an address with a name before an @, which reads as one site and goes to another.
 * The core checks again before opening anything (`openable` in `src-tauri/src/lib.rs`).
 */
export function webAddress(address: string): { href: string; host: string } | null {
  if (/[\s\\@]/.test(address.split(/[/?#]/, 3)[2] ?? '')) return null
  let url: URL
  try {
    url = new URL(address)
  } catch {
    return null
  }
  if (url.username || url.password || !url.hostname) return null
  const local = url.protocol === 'http:' && LOCAL.test(url.hostname)
  if (url.protocol !== 'https:' && !local) return null
  return { href: address, host: url.host }
}

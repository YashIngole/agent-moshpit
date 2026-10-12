// agentmoshpit.com: the files in site/ as they are, and a count of how often each
// installer is asked for. Nothing about who asked is kept: one number for each day,
// file, way it was asked for and country.
//
//   /get/<installer>            counted, then sent on to that file of the newest release
//   /install.sh, /install.ps1   counted, then served like any other file
//
// Counting never stands in the way of a download: the answer does not wait for it, and
// a count that cannot be kept is lost.

const RELEASE = 'https://github.com/YashIngole/agent-moshpit/releases/latest/download/'

/** The installers of a release, by the names the release pipeline gives them. */
export const INSTALLERS = [
  'agent-moshpit_windows_x64-setup.exe',
  'agent-moshpit_windows_x64.msi',
  'agent-moshpit_darwin_aarch64.dmg',
  'agent-moshpit_darwin_x64.dmg',
  'agent-moshpit_linux_amd64.AppImage',
  'agent-moshpit_linux_amd64.deb',
  'agent-moshpit_linux_x86_64.rpm'
]
export const SCRIPTS = ['install.sh', 'install.ps1']

/** Programs that fetch a page to look at it, not for a person who wants the file. */
const CRAWLER = /bot\b|bot\/|crawl|spider|slurp|preview|monitor|headless|playwright|lighthouse|facebookexternalhit|embedly|whatsapp|telegram|discord|slack|python|go-http|java\/|okhttp|axios|node-fetch|undici|libwww|scrapy|httpclient/i
/** What the install commands on the site fetch a script with. */
const SHELL = /^(curl|wget)\/|powershell/i

/**
 * What an address asks for: an installer to be sent on to, a script to be served, or
 * nothing this counts.
 */
export function asked(pathname) {
  if (pathname.startsWith('/get/')) {
    const file = pathname.slice(5)
    return INSTALLERS.includes(file) ? { file, to: RELEASE + file } : null
  }
  const file = pathname.slice(1)
  return SCRIPTS.includes(file) ? { file, to: null } : null
}

/**
 * How it was asked for, from what the browser or program says of itself:
 * `page` a click on this site; `direct` a link from somewhere else, or the address typed;
 * `script` an install command fetching its script; `read` a person opening the script to
 * read it; `bot` a program looking, not a person downloading.
 */
export function via(file, headers) {
  const agent = headers.get('user-agent') ?? ''
  if (SCRIPTS.includes(file)) {
    if (SHELL.test(agent)) return 'script'
    if (!agent || CRAWLER.test(agent)) return 'bot'
    return 'read'
  }
  if (!agent || CRAWLER.test(agent)) return 'bot'
  // A browser says where a click came from; an older one at least says which page.
  const site = headers.get('sec-fetch-site')
  if (site === 'same-origin') return 'page'
  if (site === null && /^https:\/\/agentmoshpit\.com\//.test(headers.get('referer') ?? '')) return 'page'
  return 'direct'
}

/** One more for today. The country is the two letters Cloudflare gives, or nothing. */
export async function count(db, file, how, country, now = new Date()) {
  if (!db) return
  try {
    await db
      .prepare('INSERT INTO downloads (day, file, via, country, count) VALUES (?1, ?2, ?3, ?4, 1) ON CONFLICT (day, file, via, country) DO UPDATE SET count = count + 1')
      .bind(now.toISOString().slice(0, 10), file, how, /^[A-Z]{2}$/.test(country ?? '') ? country : '')
      .run()
  } catch {
    // A count that cannot be kept is lost. The download is not.
  }
}

export default {
  async fetch(request, env, ctx) {
    const what = asked(new URL(request.url).pathname)
    // Only what is fetched is counted: a HEAD is somebody checking that the link works.
    if (what && request.method === 'GET') ctx.waitUntil(count(env.DB, what.file, via(what.file, request.headers), request.cf?.country))
    if (what?.to) return new Response(null, { status: 302, headers: { Location: what.to, 'Cache-Control': 'no-store', 'X-Robots-Tag': 'noindex' } })
    return env.ASSETS.fetch(request)
  }
}

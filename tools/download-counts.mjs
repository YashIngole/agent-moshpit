// How many times the installers were asked for: on agentmoshpit.com, where the site counts
// them (tools/cloudflare/site.js), and on GitHub, which counts every fetch of a release file.
//
//   node tools/download-counts.mjs        the last 30 days, and all time
//   node tools/download-counts.mjs 7      the last 7 days, and all time
//
// The site's count needs `wrangler` signed in to the Cloudflare account that has the site;
// GitHub's needs `gh`. Either is left out, with a line saying so, when it cannot be had.
//
// The two are not the same thing. The site counts a click on a download and a fetch of an
// install script. GitHub counts every fetch of a file: those clicks once they are sent on,
// what the install scripts fetch, every update the app fetches for itself, and one fetch of
// each file by the release pipeline, which is taken off here.
import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const DATABASE = 'agentmoshpit-downloads'
const REPO = 'YashIngole/agent-moshpit'
/** The ways of asking that are a person wanting the file: a click on the site, a link from elsewhere, an install command. */
const PEOPLE = ['page', 'direct', 'script']
const WAYS = [...PEOPLE, 'read', 'bot']

/** Rows of `{file, via, n}` as one line a file: its count for each way, and how many of them were people. */
export function tally(rows) {
  const files = new Map()
  for (const { file, via, n } of rows) {
    const line = files.get(file) ?? Object.fromEntries([...WAYS, 'people'].map(way => [way, 0]))
    if (via in line) line[via] += Number(n)
    if (PEOPLE.includes(via)) line.people += Number(n)
    files.set(file, line)
  }
  return [...files].map(([file, line]) => ({ file, ...line })).sort((a, b) => b.people - a.people || a.file.localeCompare(b.file))
}

/**
 * GitHub's releases as one line a kind of file, with the release pipeline's own fetch taken
 * off: it fetches each installer once, to write the list of hashes. Signatures, the update
 * manifest and that list are not installers and are left out.
 */
export function github(releases) {
  const kinds = new Map()
  for (const release of releases) {
    if (release.draft) continue
    for (const asset of release.assets ?? []) {
      if (/\.sig$|\.json$|SHA256SUMS/.test(asset.name)) continue
      const kind = asset.name.replace(/^agent-moshpit_/, '')
      const line = kinds.get(kind) ?? { file: kind, fetched: 0, releases: 0 }
      line.fetched += Math.max(0, asset.download_count - 1)
      line.releases += 1
      kinds.set(kind, line)
    }
  }
  return [...kinds.values()].sort((a, b) => b.fetched - a.fetched || a.file.localeCompare(b.file))
}

/** Lines of text with their columns lined up: the first to the left, numbers to the right. */
export function table(head, lines) {
  const cells = [head, ...lines.map(line => line.map(String))]
  const widths = head.map((_, i) => Math.max(...cells.map(row => row[i].length)))
  return cells.map(row => row.map((cell, i) => (i === 0 ? cell.padEnd(widths[i]) : cell.padStart(widths[i]))).join('  ')).join('\n')
}

/** What the site's database answers to each of these questions, or why it could not be asked. */
function ask(statements) {
  const sql = statements.join('; ')
  // It goes through a shell (npx is a script on Windows), between double quotes: the
  // questions are written here, and hold none.
  if (/["`$\\]/.test(sql)) return { why: 'a question holds a character a shell would read' }
  const config = path.join(root, 'tools', 'cloudflare', 'site.jsonc')
  const ran = spawnSync(`npx --yes wrangler d1 execute ${DATABASE} --remote --json --config "${config}" --command "${sql}"`, { encoding: 'utf8', cwd: root, shell: true })
  if (ran.status !== 0) return { why: (ran.stderr || ran.stdout || 'wrangler could not be run').trim().split('\n').slice(-3).join(' ') }
  try {
    return { answers: JSON.parse(ran.stdout.slice(ran.stdout.indexOf('['))).map(answer => answer.results ?? []) }
  } catch {
    return { why: 'wrangler answered something that is not a list of results' }
  }
}

function main() {
  const days = Math.max(1, Math.min(3650, Number.parseInt(process.argv[2] ?? '30', 10) || 30))
  // Only digits and dashes: it goes into the question as it is.
  const since = new Date(Date.now() - (days - 1) * 86_400_000).toISOString().slice(0, 10)
  const people = PEOPLE.map(way => `'${way}'`).join(', ')
  const site = ask([
    `SELECT file, via, SUM(count) AS n FROM downloads WHERE day >= '${since}' GROUP BY file, via`,
    'SELECT file, via, SUM(count) AS n FROM downloads GROUP BY file, via',
    `SELECT day, SUM(count) AS n FROM downloads WHERE day >= '${since}' AND via IN (${people}) GROUP BY day ORDER BY day`,
    `SELECT country, SUM(count) AS n FROM downloads WHERE day >= '${since}' AND via IN (${people}) GROUP BY country ORDER BY n DESC LIMIT 10`,
    'SELECT MIN(day) AS first FROM downloads'
  ])
  const head = ['', 'people', ...WAYS]
  const lines = rows => {
    const each = tally(rows)
    const sum = key => each.reduce((total, line) => total + line[key], 0)
    return [...each.map(line => [line.file, line.people, ...WAYS.map(way => line[way])]), ['all', sum('people'), ...WAYS.map(sum)]]
  }
  console.log('On agentmoshpit.com')
  if (site.why) console.log(`  not read: ${site.why}`)
  else {
    const [recent, ever, byDay, byCountry, [{ first } = {}]] = site.answers
    if (!first) console.log('  nothing counted yet')
    else {
      console.log(`\nThe last ${days} days (since ${since})\n${table(head, lines(recent))}`)
      console.log(`\nSince counting began (${first})\n${table(head, lines(ever))}`)
      if (byDay.length) console.log(`\nPeople, by day\n${table(['day', 'people'], byDay.map(row => [row.day, row.n]))}`)
      if (byCountry.length) console.log(`\nPeople, by country, the last ${days} days\n${table(['country', 'people'], byCountry.map(row => [row.country || 'not known', row.n]))}`)
      console.log('\npeople = page (a click on the site) + direct (a link from elsewhere) + script (an install command).\nread is the script opened to be read; bot is a program looking.')
    }
  }
  console.log('\nOn GitHub, every release so far, without the release pipeline\'s own fetch of each file')
  const gh = spawnSync('gh', ['api', `repos/${REPO}/releases?per_page=100`], { encoding: 'utf8' })
  if (gh.status !== 0) console.log(`  not read: ${(gh.stderr || 'gh could not be run').trim().split('\n')[0]}`)
  else {
    const each = github(JSON.parse(gh.stdout))
    console.log(table(['', 'fetched', 'releases'], [...each.map(line => [line.file, line.fetched, line.releases]), ['all', each.reduce((total, line) => total + line.fetched, 0), '']]))
    console.log('\nThis includes updates the app fetches for itself and what the install scripts fetch.')
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main()

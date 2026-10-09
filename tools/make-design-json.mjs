// Builds .impeccable/design.json from DESIGN.md (narrative, colours, type) plus the
// component snippets and extension tokens below. Run it after changing DESIGN.md,
// rather than editing the JSON by hand:
//
//   node tools/make-design-json.mjs
//
// It stops with a list if DESIGN.md and src/styles/tokens.css disagree about a
// colour, a radius or a spacing step, so the two cannot drift apart unnoticed.
import { readFileSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const md = readFileSync(path.join(root, 'DESIGN.md'), 'utf8').replace(/\r\n/g, '\n')
const [, front, body] = md.match(/^---\n([\s\S]*?)\n---\n([\s\S]*)$/)

// ── the frontmatter: groups of `key: "value"`, two or three levels deep ──
const tokens = {}
const at = []
for (const line of front.split('\n')) {
  const found = line.match(/^( *)([\w-]+):(?: (.+))?$/)
  if (!found) continue
  const depth = found[1].length / 2
  at.length = depth
  at[depth] = found[2]
  if (found[3] === undefined) continue
  let group = tokens
  for (const key of at.slice(0, depth)) group = group[key] ??= {}
  group[found[2]] = found[3].replace(/^"(.*)"$/, '$1')
}
const colours = Object.entries(tokens.colors)

// ── DESIGN.md against the stylesheet it describes ──
const css = readFileSync(path.join(root, 'src', 'styles', 'tokens.css'), 'utf8')
const inCss = Object.fromEntries([...css.matchAll(/^\s*--([\w-]+):\s*([^;]+);/gm)].map(m => [m[1], m[2].trim()]))
const drift = []
for (const group of ['colors', 'rounded', 'spacing']) {
  for (const [key, value] of Object.entries(tokens[group])) {
    if (key === 'pill') continue // written out as 999px where it is used; not a custom property
    const there = inCss[key]
    if (there === undefined) drift.push(`${group}.${key} is not in tokens.css`)
    else if (there.toLowerCase() !== value.toLowerCase()) drift.push(`${group}.${key} is ${value} in DESIGN.md and ${there} in tokens.css`)
  }
}
for (const [role, family] of [['body', '--font'], ['sign', '--mono']]) {
  if (tokens.typography[role].fontFamily !== inCss[family.slice(2)]) drift.push(`typography.${role}.fontFamily is not ${family} from tokens.css`)
}
for (const [name, component] of Object.entries(tokens.components)) {
  for (const value of Object.values(component)) {
    const ref = value.match(/^\{(\w+)\.([\w-]+)\}$/)
    if (ref && tokens[ref[1]]?.[ref[2]] === undefined) drift.push(`components.${name} points at ${value}, which is not defined`)
  }
}
if (drift.length > 0) {
  console.error('DESIGN.md and src/styles/tokens.css disagree:\n  ' + drift.join('\n  '))
  process.exit(1)
}

// ── colours: a name, a role and a ramp for each ──
const toLinear = c => (c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4))
/** Red, green and blue, 0 to 255, from `#rrggbb` or `rgba(r, g, b, a)`. */
function channels(colour) {
  if (colour.startsWith('#')) {
    const n = parseInt(colour.slice(1), 16)
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255]
  }
  return colour.match(/[\d.]+/g).slice(0, 3).map(Number)
}
function oklch(colour) {
  const [r, g, b] = channels(colour).map(c => toLinear(c / 255))
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b)
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b)
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b)
  const L = 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s
  const A = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s
  const B = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s
  const C = Math.hypot(A, B)
  const H = (Math.atan2(B, A) * 180) / Math.PI
  return { L, C, H: (H + 360) % 360 }
}
const fmt = ({ L, C, H }) => 'oklch(' + (L * 100).toFixed(1) + '% ' + C.toFixed(3) + ' ' + H.toFixed(1) + ')'
const ramp = colour => {
  const base = oklch(colour)
  return Array.from({ length: 8 }, (_, i) => fmt({ L: 0.15 + (0.8 * i) / 7, C: base.C, H: base.H }))
}

const NAMES = {
  needs: ['primary', 'Amber'],
  'needs-ink': ['primary', 'Ink On Amber'],
  done: ['secondary', 'Done Green'],
  trouble: ['secondary', 'Trouble Red'],
  'trouble-ink': ['secondary', 'Trouble Red, For Words'],
  'trouble-wash': ['secondary', 'Trouble Wash'],
  work: ['secondary', 'Screen Blue'],
  quiet: ['neutral', 'Quiet'],
  bg: ['neutral', 'The Building'],
  carpet: ['neutral', 'Room Graphite'],
  grid: ['neutral', 'Grid Line'],
  wall: ['neutral', 'Wall'],
  chrome: ['neutral', 'Top Bar'],
  panel: ['neutral', 'Panel'],
  inset: ['neutral', 'Field Fill'],
  line: ['neutral', 'Divider'],
  'field-line': ['neutral', 'Field Edge'],
  select: ['neutral', 'Selected Text'],
  ink: ['neutral', 'Ink'],
  'ink-2': ['neutral', 'Supporting Ink'],
  'ink-3': ['neutral', 'Quiet Ink'],
  button: ['neutral', 'Button'],
  'button-ink': ['neutral', 'Button Label'],
  'button-hover': ['neutral', 'Button, Hovered'],
  focus: ['neutral', 'Focus Ring'],
  chair: ['tertiary', 'Chair'],
  'chair-edge': ['tertiary', 'Chair Edge'],
  desk: ['tertiary', 'Desk'],
  'desk-edge': ['tertiary', 'Desk Edge'],
  'desk-leg': ['tertiary', 'Desk Leg'],
  mat: ['tertiary', 'Desk Mat'],
  kbd: ['tertiary', 'Keyboard'],
  keycaps: ['tertiary', 'Key Caps'],
  bezel: ['tertiary', 'Monitor Bezel'],
  screen: ['tertiary', 'Screen'],
  'screen-off': ['tertiary', 'Dark Screen'],
  term: ['neutral', 'Terminal Glass'],
  'term-ink': ['neutral', 'Terminal Text'],
  'term-dim': ['neutral', 'Pane Edge, Held'],
  'term-line': ['neutral', 'Line Between Panes'],
  'term-select': ['neutral', 'Terminal Selection']
}
const ANSI = ['Black', 'Red', 'Green', 'Yellow', 'Blue', 'Magenta', 'Cyan', 'White']
const colorMeta = {}
for (const [name, value] of colours) {
  const n = name.startsWith('ansi-') ? Number(name.slice(5)) : -1
  const [role, displayName] = n >= 0 ? ['neutral', 'Terminal ' + (n > 7 ? 'Bright ' : '') + ANSI[n % 8]] : (NAMES[name] ?? ['neutral', name])
  colorMeta[name] = { role, displayName, canonical: value.toLowerCase(), tonalRamp: ramp(value) }
}

// ── narrative, copied from DESIGN.md word for word ──
const northStar = body.match(/\*\*Creative North Star: "(.+?)"\*\*/)[1]
const overviewText = body.match(/\*\*Creative North Star: ".+?"\*\*\n\n([\s\S]*?)\n\n\*\*Key Characteristics:\*\*/)[1]
const keyCharacteristics = body
  .match(/\*\*Key Characteristics:\*\*\n([\s\S]*?)\n\n## /)[1]
  .split('\n')
  .map(line => line.replace(/^- /, ''))
const rules = []
let section = ''
for (const line of body.split('\n')) {
  const heading = line.match(/^## (.+)$/)
  if (heading) section = heading[1].toLowerCase().split(/[ &]/)[0]
  const rule = line.match(/^\*\*(The .+? Rule)\.\*\* (.+)$/)
  if (rule) rules.push({ name: rule[1], body: rule[2], section })
}
const bullets = title =>
  body
    .match(new RegExp('### ' + title + '\\n([\\s\\S]*?)(?:\\n\\n|\\n?$)'))[1]
    .split('\n')
    .map(line => line.replace(/^- /, ''))
const dos = bullets('Do:')
const donts = bullets("Don't:")

// ── components: self-contained snippets that read the live tokens ──
// Each colour is written as the app's custom property with DESIGN.md's value to fall back on.
const v = name => {
  if (tokens.colors[name] === undefined) throw new Error('No colour called ' + name + ' in DESIGN.md')
  return 'var(--' + name + ', ' + tokens.colors[name] + ')'
}
const FONT = tokens.typography.body.fontFamily
const MONO = tokens.typography.sign.fontFamily
const EASE = 'cubic-bezier(0.16, 1, 0.3, 1)'
const R1 = tokens.rounded['r-1']
const R2 = tokens.rounded['r-2']
const R3 = tokens.rounded['r-3']
const buttonBase =
  'display: inline-flex; align-items: center; gap: 8px; min-height: 32px; padding: 0 12px; border-radius: ' +
  R1 +
  '; font: 650 0.8125rem/1.4 ' +
  MONO +
  '; white-space: nowrap; cursor: pointer; transition: background 180ms ' +
  EASE +
  ', transform 180ms ' +
  EASE +
  ';'
const focus = ' outline: 2px solid ' + v('focus') + '; outline-offset: 2px;'
const keyCap = ' kbd { padding: 0 5px; border: 1px solid currentColor; border-radius: 4px; font: 500 0.6875rem/1.4 ' + MONO + '; opacity: 0.6; }'
const dot = 'display: inline-block; flex: none; width: 7px; height: 7px; border-radius: 50%;'
const tag =
  'padding: 0 6px; border: 1px solid ' + v('wall') + '; border-radius: 5px; color: ' + v('ink-2') + '; font: 400 11px/1.6 ' + MONO + '; text-transform: lowercase; white-space: nowrap;'

const components = [
  {
    name: 'Primary Button',
    kind: 'button',
    refersTo: 'button-primary',
    description: 'The one main action of a place: new agent, Start agent, Carry on.',
    html: '<button class="ds-btn">+ new agent <kbd>n</kbd></button>',
    css:
      '.ds-btn { ' +
      buttonBase +
      ' border: 1px solid transparent; background: ' +
      v('button') +
      '; color: ' +
      v('button-ink') +
      '; }' +
      ' .ds-btn:hover { background: ' +
      v('button-hover') +
      '; }' +
      ' .ds-btn:active { transform: translateY(1px); }' +
      ' .ds-btn:focus-visible {' +
      focus +
      ' }' +
      ' .ds-btn' +
      keyCap
  },
  {
    name: 'Quiet Button',
    kind: 'button',
    refersTo: 'button-quiet',
    description: 'The action beside a primary one: back to the floor, Cancel, Browse, Dismiss.',
    html: '<button class="ds-btn-quiet">back to the floor <kbd>ctrl `</kbd></button>',
    css:
      '.ds-btn-quiet { ' +
      buttonBase +
      ' border: 1px solid ' +
      v('wall') +
      '; background: transparent; color: ' +
      v('ink') +
      '; }' +
      ' .ds-btn-quiet:hover { background: ' +
      v('inset') +
      '; }' +
      ' .ds-btn-quiet:active { transform: translateY(1px); }' +
      ' .ds-btn-quiet:focus-visible {' +
      focus +
      ' }' +
      ' .ds-btn-quiet' +
      keyCap
  },
  {
    name: 'Danger Button',
    kind: 'button',
    refersTo: 'button-danger',
    description: 'Only inside the box that asks before a busy agent is stopped, restarted or removed.',
    html: '<button class="ds-btn-danger">Remove</button>',
    css:
      '.ds-btn-danger { ' +
      buttonBase +
      ' border: 1px solid currentColor; background: transparent; color: ' +
      v('trouble-ink') +
      '; }' +
      ' .ds-btn-danger:hover { background: ' +
      v('trouble-wash') +
      '; }' +
      ' .ds-btn-danger:focus-visible {' +
      focus +
      ' }'
  },
  {
    name: 'Field',
    kind: 'input',
    refersTo: 'field',
    description: 'Anything typed into. The edge holds 3:1 against the fill and the panel.',
    html: '<input class="ds-field" placeholder="The project they work in" />',
    css:
      '.ds-field { display: block; box-sizing: border-box; width: 260px; min-height: 34px; padding: 7px 12px; border: 1px solid ' +
      v('field-line') +
      '; border-radius: ' +
      R1 +
      '; background: ' +
      v('inset') +
      '; color: ' +
      v('ink') +
      '; font: 400 0.875rem/1.4 ' +
      FONT +
      '; }' +
      ' .ds-field::placeholder { color: ' +
      v('ink-3') +
      '; opacity: 1; }' +
      ' .ds-field:hover { border-color: ' +
      v('ink-2') +
      '; }' +
      ' .ds-field:focus-visible { outline: 2px solid ' +
      v('focus') +
      '; outline-offset: 0; border-color: ' +
      v('focus') +
      '; }'
  },
  {
    name: 'Choice',
    kind: 'input',
    refersTo: 'choice',
    description: 'One of a few, held in a single control: which program takes the task, which editor opens files.',
    html:
      '<div class="ds-pick" role="radiogroup" aria-label="Who should take it?"><label class="ds-on"><input type="radio" name="ds-pick" checked />Claude Code</label><label><input type="radio" name="ds-pick" />Codex</label><label><input type="radio" name="ds-pick" />Gemini CLI</label></div>',
    css:
      '.ds-pick { display: flex; flex-wrap: wrap; gap: 3px; width: 320px; padding: 3px; border: 1px solid ' +
      v('field-line') +
      '; border-radius: ' +
      R1 +
      '; background: ' +
      v('inset') +
      '; }' +
      ' .ds-pick label { display: grid; flex: 1 1 auto; min-height: 28px; padding: 0 12px; place-items: center; border-radius: 4px; color: ' +
      v('ink-2') +
      '; font: 650 0.8125rem/1.4 ' +
      FONT +
      '; white-space: nowrap; cursor: pointer; }' +
      ' .ds-pick label:hover { color: ' +
      v('ink') +
      '; }' +
      ' .ds-pick label.ds-on, .ds-pick label:has(input:checked) { background: ' +
      v('button') +
      '; color: ' +
      v('button-ink') +
      '; }' +
      ' .ds-pick label:has(input:focus-visible) {' +
      focus +
      ' }' +
      ' .ds-pick input { position: absolute; opacity: 0; pointer-events: none; }'
  },
  {
    name: 'Status And Program Tag',
    kind: 'chip',
    refersTo: 'status-chip',
    description: 'Every status is a dot and its word; away and starting are a hollow ring. Which program an agent is, is a word in a tag.',
    html:
      '<div class="ds-states"><span class="ds-status ds-needs"><i></i>needs you</span><span class="ds-status ds-working"><i></i>working</span><span class="ds-status ds-done"><i></i>done</span><span class="ds-status ds-trouble"><i></i>trouble</span><span class="ds-status"><i></i>idle</span><span class="ds-status ds-away"><i></i>away</span><span class="ds-tag">claude</span></div>',
    css:
      '.ds-states { display: flex; flex-wrap: wrap; align-items: center; gap: 8px 16px; width: 320px; }' +
      ' .ds-status { display: inline-flex; align-items: center; gap: 7px; color: ' +
      v('ink-2') +
      '; font: 400 11.5px/1.4 ' +
      MONO +
      '; white-space: nowrap; }' +
      ' .ds-status i { ' +
      dot +
      ' background: ' +
      v('quiet') +
      '; }' +
      ' .ds-needs { color: ' +
      v('needs') +
      '; font-weight: 600; } .ds-needs i { background: ' +
      v('needs') +
      '; }' +
      ' .ds-working i { background: ' +
      v('work') +
      '; } .ds-done i { background: ' +
      v('done') +
      '; }' +
      ' .ds-trouble { color: ' +
      v('trouble-ink') +
      '; } .ds-trouble i { background: ' +
      v('trouble') +
      '; }' +
      ' .ds-away i { background: transparent; box-shadow: inset 0 0 0 1.5px ' +
      v('quiet') +
      '; }' +
      ' .ds-tag { ' +
      tag +
      ' }'
  },
  {
    name: 'The Band',
    kind: 'custom',
    refersTo: 'band',
    description: 'Exists only while an agent waits whose terminal is not in front: who, which program, where, and the one way to their terminal.',
    html:
      '<div class="ds-band"><div class="ds-band-who"><span>needs you · 48 sec</span><strong>Fix the checkout total</strong></div><p><code>claude</code> is waiting for your answer <em>in shop · fix/checkout-total</em></p><button class="ds-band-go">open their terminal</button></div>',
    css:
      '.ds-band { display: flex; flex-wrap: wrap; align-items: center; gap: 8px 16px; padding: 10px 14px 10px 18px; background: ' +
      v('needs') +
      '; color: ' +
      v('needs-ink') +
      '; font: 400 0.875rem/1.4 ' +
      FONT +
      '; }' +
      ' .ds-band-who { display: grid; gap: 2px; }' +
      ' .ds-band-who span { font: 600 11.5px/1.4 ' +
      MONO +
      '; letter-spacing: 0.06em; text-transform: uppercase; opacity: 0.75; }' +
      ' .ds-band-who strong { font-size: 1rem; font-weight: 650; line-height: 1.25; }' +
      ' .ds-band p { flex: 1 1 260px; margin: 0; }' +
      ' .ds-band code { padding: 2px 6px; border-radius: 4px; background: rgba(20, 14, 0, 0.12); font: 400 0.8125rem/1.4 ' +
      MONO +
      '; }' +
      ' .ds-band em { margin-left: 6px; font: 400 0.75rem/1.4 ' +
      MONO +
      '; opacity: 0.7; }' +
      ' .ds-band-go { height: 34px; padding: 0 14px; border: 0; border-radius: ' +
      R1 +
      '; background: ' +
      v('needs-ink') +
      '; color: ' +
      v('needs') +
      '; font: 600 0.8125rem/1.4 ' +
      MONO +
      '; text-transform: lowercase; cursor: pointer; }' +
      ' .ds-band-go:hover { background: #2a1f04; }' +
      ' .ds-band-go:focus-visible { outline: 2px solid ' +
      v('needs-ink') +
      '; outline-offset: 2px; }'
  },
  {
    name: 'Desk Nameplate',
    kind: 'custom',
    refersTo: 'desk',
    description: "Under every drawing: the name, the status line with how long and which program, and a line or two about what is going on. This one's terminal is open.",
    html:
      '<button type="button" class="ds-plate"><span class="ds-plate-name">Refactor auth middleware</span><span class="ds-plate-status"><span><i></i>working</span><span class="ds-plate-since">12 min</span><span class="ds-plate-tag">codex</span></span><span class="ds-plate-doing">Working</span></button>',
    css:
      '.ds-plate { display: grid; gap: 4px; width: 220px; padding: 8px 12px 12px; border: 0; border-radius: ' +
      R2 +
      '; background: rgba(130, 170, 255, 0.07); box-shadow: inset 0 0 0 1px rgba(130, 170, 255, 0.24); text-align: left; cursor: pointer; font-family: ' +
      FONT +
      '; }' +
      ' .ds-plate:focus-visible {' +
      focus +
      ' }' +
      ' .ds-plate-name { color: ' +
      v('ink') +
      '; font-size: 0.875rem; font-weight: 600; line-height: 1.25; }' +
      ' .ds-plate-status { display: flex; align-items: center; gap: 7px; color: ' +
      v('ink-2') +
      '; font: 400 11.5px/1.4 ' +
      MONO +
      '; }' +
      ' .ds-plate-status > span:first-child { display: inline-flex; align-items: center; gap: 7px; }' +
      ' .ds-plate-status i { ' +
      dot +
      ' background: ' +
      v('work') +
      '; }' +
      ' .ds-plate-since { color: ' +
      v('ink-3') +
      '; }' +
      ' .ds-plate-tag { margin-left: auto; ' +
      tag +
      ' }' +
      ' .ds-plate-doing { color: ' +
      v('ink-3') +
      '; font-size: 12.5px; line-height: 1.35; }'
  },
  {
    name: 'Room',
    kind: 'card',
    refersTo: 'room',
    description: 'One per project: graphite under a faint grid inside a 1px wall, with a sign and an empty desk for adding someone.',
    html:
      '<section class="ds-room"><h3>shop<span>/</span> <small>3 agents</small></h3><button type="button" class="ds-spare"><i><svg viewBox="0 0 16 16" aria-hidden="true"><path d="M8 3v10M3 8h10"/></svg></i>add an agent</button></section>',
    css:
      '.ds-room { width: 280px; padding: 12px 14px 14px; border: 1px solid ' +
      v('wall') +
      '; border-radius: ' +
      R3 +
      '; background: linear-gradient(' +
      v('grid') +
      ' 1px, transparent 1px) 0 0 / 28px 28px, linear-gradient(90deg, ' +
      v('grid') +
      ' 1px, transparent 1px) 0 0 / 28px 28px, ' +
      v('carpet') +
      '; }' +
      ' .ds-room h3 { margin: 0 0 6px; padding: 0 6px; color: ' +
      v('ink') +
      '; font: 600 0.875rem/1.3 ' +
      MONO +
      '; }' +
      ' .ds-room h3 span { color: ' +
      v('ink-3') +
      '; }' +
      ' .ds-room small { margin-left: 8px; color: ' +
      v('ink-3') +
      '; font-size: 0.75rem; font-weight: 400; }' +
      ' .ds-spare { display: grid; gap: 8px; place-content: center; justify-items: center; width: 100%; min-height: 120px; padding: 12px; border: 1px dashed ' +
      v('wall') +
      '; border-radius: ' +
      R2 +
      '; background: transparent; color: ' +
      v('ink-3') +
      '; font: 400 0.8125rem/1.4 ' +
      MONO +
      '; cursor: pointer; transition: background 180ms ' +
      EASE +
      ', color 180ms ' +
      EASE +
      ', border-color 180ms ' +
      EASE +
      '; }' +
      ' .ds-spare:hover, .ds-spare:focus-visible { border-color: ' +
      v('field-line') +
      '; background: rgba(255, 255, 255, 0.025); color: ' +
      v('ink') +
      '; }' +
      ' .ds-spare i { display: grid; width: 34px; height: 34px; place-items: center; border: 1px solid ' +
      v('wall') +
      '; border-radius: 9px; }' +
      ' .ds-spare svg { width: 14px; height: 14px; fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; }'
  },
  {
    name: 'Pane Header',
    kind: 'custom',
    refersTo: 'pane-header',
    description: "Where the office's part of a terminal ends: name, status, how long, where, and which program. The line of ink along the top says the keyboard goes here.",
    html:
      '<header class="ds-pane"><div><strong>Refactor auth middleware</strong><span><i></i>working · 12 min · shop · refactor/auth</span></div><em>codex</em><button type="button" aria-label="Put this terminal away"><svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 3.5l9 9M12.5 3.5l-9 9"/></svg></button></header>',
    css:
      '.ds-pane { display: flex; align-items: center; gap: 8px; width: 360px; padding: 7px 8px 7px 10px; border-top: 2px solid ' +
      v('ink') +
      '; border-bottom: 1px solid ' +
      v('line') +
      '; background: ' +
      v('panel') +
      '; font-family: ' +
      FONT +
      '; }' +
      ' .ds-pane div { display: grid; flex: 1; gap: 2px; min-width: 0; }' +
      ' .ds-pane strong { color: ' +
      v('ink') +
      '; font-size: 13.5px; font-weight: 600; line-height: 1.25; }' +
      ' .ds-pane span { display: flex; align-items: center; gap: 6px; color: ' +
      v('ink-3') +
      '; font: 400 11px/1.4 ' +
      MONO +
      '; white-space: nowrap; }' +
      ' .ds-pane i { ' +
      dot +
      ' background: ' +
      v('work') +
      '; }' +
      ' .ds-pane em { font-style: normal; ' +
      tag +
      ' }' +
      ' .ds-pane button { display: grid; flex: none; width: 28px; height: 28px; place-items: center; border: 0; border-radius: ' +
      R1 +
      '; background: transparent; color: ' +
      v('ink-2') +
      '; cursor: pointer; }' +
      ' .ds-pane button:hover { background: ' +
      v('inset') +
      '; color: ' +
      v('ink') +
      '; }' +
      ' .ds-pane button:focus-visible {' +
      focus +
      ' }' +
      ' .ds-pane svg { width: 16px; height: 16px; fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; }'
  },
  {
    name: 'Menu',
    kind: 'custom',
    refersTo: 'menu-sheet',
    description: "A desk's or a terminal's menu: each item a name with a quiet line under it. One of the few things that float.",
    html:
      '<div class="ds-menu" role="menu"><button type="button" role="menuitem">Open beside the others<span>Ctrl+Enter, or Ctrl and a click</span></button><button type="button" role="menuitem">Rename<span>F2</span></button><hr /><button type="button" role="menuitem" class="ds-menu-danger">Remove this desk<span>Ends their program; Undo for a moment. Delete</span></button></div>',
    css:
      '.ds-menu { display: grid; gap: 2px; width: 280px; padding: 8px; border: 1px solid ' +
      v('line') +
      '; border-radius: ' +
      R2 +
      '; background: ' +
      v('panel') +
      '; box-shadow: 0 1px 2px rgba(0, 0, 0, 0.45), 0 12px 32px -10px rgba(0, 0, 0, 0.62); }' +
      ' .ds-menu button { display: grid; padding: 6px 12px; border: 0; border-radius: ' +
      R1 +
      '; background: transparent; color: ' +
      v('ink') +
      '; font: 600 0.875rem/1.4 ' +
      FONT +
      '; text-align: left; cursor: pointer; }' +
      ' .ds-menu button:hover, .ds-menu button:focus-visible { background: ' +
      v('inset') +
      '; outline: none; }' +
      ' .ds-menu span { color: ' +
      v('ink-3') +
      '; font-size: 0.75rem; font-weight: 400; line-height: 1.35; }' +
      ' .ds-menu .ds-menu-danger { color: ' +
      v('trouble-ink') +
      '; }' +
      ' .ds-menu hr { margin: 4px 0; border: 0; border-top: 1px solid ' +
      v('line') +
      '; }'
  }
]
for (const component of components) {
  if (tokens.components[component.refersTo] === undefined) throw new Error(component.name + ' refers to ' + component.refersTo + ', which DESIGN.md does not have')
}

const design = {
  schemaVersion: 2,
  generatedAt: new Date().toISOString(),
  title: 'Design System: Agent Moshpit',
  extensions: {
    colorMeta,
    typographyMeta: {
      headline: { displayName: 'Headline', purpose: 'The title of a side panel, and the one line of the empty office.' },
      title: { displayName: 'Title', purpose: "The agent's name on the band." },
      body: { displayName: 'Body', purpose: "Sentences and fields. A desk's name at weight 600, a menu item at 600 to 650." },
      small: { displayName: 'Small', purpose: 'A toast, a notice, the list of keys. A form label and an option in a choice at weight 650.' },
      note: { displayName: 'Note', purpose: 'The hint under a field, a note in a panel, the second line of a menu item.' },
      sign: { displayName: 'Sign', purpose: "A room's sign: the folder's name with a slash after it. The wordmark is the same voice at 15px." },
      button: { displayName: 'Button', purpose: 'The words on a button.' },
      meta: { displayName: 'Meta', purpose: 'The counts in the top bar, the head count on a sign, a version number.' },
      status: { displayName: 'Status', purpose: "The lowercase line under a desk's name: dot, word, how long, program tag. 11px in a pane's header and in the list." },
      label: { displayName: 'Label', purpose: 'The uppercase heads of the sections inside a panel, and "also waiting" on the band.' },
      terminal: { displayName: 'Terminal', purpose: "A program's own screen. The user can step it between 9 and 24px." }
    },
    shadows: [
      { name: 'lift', value: '0 1px 2px rgba(0, 0, 0, 0.45), 0 12px 32px -10px rgba(0, 0, 0, 0.62)', purpose: 'What floats: a menu, the toast, the find bar, the note while files are dragged over a pane, and the side panel when it lies over terminals.' },
      { name: 'ground', value: 'an ellipse filled rgba(0, 0, 0, 0.45), drawn under each desk', purpose: 'Furniture standing on the floor. Part of the drawing, not a box shadow.' },
      { name: 'spill', value: "an ellipse in the screen's colour at 16% opacity, blurred 6px", purpose: 'The light of a screen falling on the desk: blue at work, amber, green or red by status, grey when idle or starting, none when away.' }
    ],
    motion: [
      { name: 'ease', value: EASE, purpose: 'Every state change: an exponential ease-out from an already visible default.' },
      { name: 'quick', value: '180ms', purpose: 'Hover and press on controls and desks, a menu unfolding.' },
      { name: 'settle', value: '240ms', purpose: 'The band dropping in, the side panel arriving, the toast rising, a tag popping over a desk.' },
      { name: 'beat', value: '320ms', purpose: 'The shared clock that switches people between fixed poses. Off while the window is hidden or under reduced motion.' },
      { name: 'blink', value: '130ms every 4300ms', purpose: 'Working and idle people blink.' },
      { name: 'walk-in', value: '720ms, once', purpose: 'A new agent walks to their chair. Never on window open.' },
      { name: 'found', value: '1.4s, once', purpose: 'The ring of ink on a desk found from a count in the top bar.' }
    ],
    breakpoints: [
      { name: 'bar-key-hints-hide', value: '860px' },
      { name: 'terminals-or-panel-take-the-window', value: '760px' },
      { name: 'band-on-one-line', value: '720px' },
      { name: 'wordmark-and-counts-hide', value: '620px' },
      { name: 'bar-buttons-shorten', value: '520px' },
      { name: 'desks-in-rows', value: '560px (width of the floor, a container query)' },
      { name: 'desks-lose-their-line', value: '340px (width of the floor, a container query)' },
      { name: 'floor-as-a-list', value: '300px (width the floor was dragged to, beside terminals)' },
      { name: 'pane-header-drops-place-and-tag', value: "430px (width of the pane's header, a container query)" },
      { name: 'pane-header-drops-time', value: "260px (width of the pane's header, a container query)" },
      { name: 'smallest-window', value: '340px by 420px' }
    ]
  },
  components,
  narrative: { northStar, overview: overviewText, keyCharacteristics, rules, dos, donts }
}

const target = path.join(root, '.impeccable', 'design.json')
writeFileSync(target, JSON.stringify(design, null, 2) + '\n')
const back = JSON.parse(readFileSync(target, 'utf8'))
console.log('colours', Object.keys(back.extensions.colorMeta).length)
console.log('type roles', Object.keys(back.extensions.typographyMeta).length)
console.log('components', back.components.length)
console.log('north star:', back.narrative.northStar)
console.log('overview paragraphs', back.narrative.overview.split('\n\n').length)
console.log('key characteristics', back.narrative.keyCharacteristics.length)
console.log('rules', back.narrative.rules.map(r => r.name + ' [' + r.section + ']').join('; '))
console.log('dos', back.narrative.dos.length, 'donts', back.narrative.donts.length)
console.log('last dont:', back.narrative.donts.at(-1))

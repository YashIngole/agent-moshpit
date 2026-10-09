// A pretend office, for looking at the window without any program behind it.
//
//   npm run dev, then http://localhost:1420/?demo=office
//
// Scenes: office (the default), calm, empty, crowd.
// Add `&still` to stop anything from changing by itself, and `&update` for an office
// that has a newer version of itself to offer (`&update=fails` when it cannot be had).
//
// The terminals here are typed by hand: they show what a program might print and
// echo what is typed into them. Nothing is started and nothing is sent anywhere.
import type { Bridge, OnTerminal } from './bridge'
import type { Agent, Harness, Job, ModelChoice, NewAgentSpec, Phase, Snapshot } from './types'
import { shorten, titleFrom, uniqueTitle } from './words'
import { cleanVoice, DEFAULT_VOICE, EMPTY_VOICE, type VoiceSettings, type VoiceView } from './voice'

const MIN = 60_000

const program = (over: Partial<Harness> & Pick<Harness, 'id' | 'name' | 'tag'>): Harness => ({
  installed: true,
  launch: 'none',
  version: '',
  takes_task: false,
  worktree: false,
  latest: '',
  outdated: false,
  install_line: '',
  update_line: '',
  ...over
})

const HARNESSES: Harness[] = [
  program({ id: 'claude', launch: 'claude', name: 'Claude Code', tag: 'Claude', version: '2.1.290', latest: '2.1.294', outdated: true, takes_task: true, worktree: true, install_line: 'npm install -g @anthropic-ai/claude-code', update_line: 'claude update' }),
  program({ id: 'codex', launch: 'codex', name: 'Codex', tag: 'Codex', version: '0.161.0', latest: '0.161.0', takes_task: true, worktree: true, install_line: 'npm install -g @openai/codex', update_line: 'codex update' }),
  program({ id: 'antigravity', name: 'Antigravity CLI', tag: 'Antigravity', version: '1.2.4', takes_task: true, update_line: 'agy update' }),
  program({ id: 'hermes', name: 'Hermes', tag: 'Hermes', version: '0.21.2', worktree: true, update_line: 'hermes update' }),
  program({ id: 'gemini', name: 'Gemini CLI', tag: 'Gemini', version: '0.2.1', latest: '0.63.0', outdated: true, takes_task: true, install_line: 'npm install -g @google/gemini-cli', update_line: 'npm install -g @google/gemini-cli@latest' }),
  program({ id: 'opencode', name: 'OpenCode', tag: 'OpenCode', installed: false, install_line: 'npm install -g opencode-ai', update_line: 'npm install -g opencode-ai@latest' }),
  program({ id: 'qwen', name: 'Qwen Code', tag: 'Qwen', installed: false, install_line: 'npm install -g @qwen-code/qwen-code', update_line: 'npm install -g @qwen-code/qwen-code@latest' }),
  program({ id: 'cursor', name: 'Cursor CLI', tag: 'Cursor', installed: false })
]

function agent(n: number, title: string, harness: string, repo: string, branch: string, phase: Phase, ago: number, activity = ''): Agent {
  const kind = HARNESSES.find(h => h.id === harness)!
  return {
    id: `demo-${n}`,
    title,
    phase,
    activity,
    harness,
    harness_name: kind.name,
    harness_tag: kind.tag,
    project: `C:/code/${repo}`,
    repo,
    branch,
    cwd: `C:/code/${repo}`,
    since_ms: Date.now() - ago,
    look: n * 7919 + 13,
    running: phase !== 'asleep' && phase !== 'failed',
    resumable: harness !== 'gemini',
    resume_scope: harness === 'gemini' ? 'none' : harness === 'antigravity' ? 'folder' : harness === 'hermes' ? 'latest' : 'desk',
    resume_note: '',
    // Busy and out of sight: something new in their terminal.
    unread: phase === 'working' && n % 2 === 1
  }
}

function scene(name: string): Agent[] {
  const office = [
    agent(1, 'Fix the checkout total', 'claude', 'shop', 'fix/checkout-total', 'needs_you', 48_000, 'Waiting for your answer in the terminal'),
    agent(2, 'Refactor auth middleware', 'codex', 'shop', 'refactor/auth', 'working', 12 * MIN),
    agent(3, 'Flaky test hunt', 'claude', 'shop', 'main', 'asleep', 26 * 60 * MIN),
    agent(4, 'Type the orders API', 'codex', 'api', 'types/orders', 'done', 3 * MIN),
    agent(5, 'Docs pass for the API', 'antigravity', 'api', 'main', 'working', 9 * MIN),
    agent(6, 'Bump dependencies', 'hermes', 'api', 'main', 'idle', 4 * MIN),
    agent(7, 'Seed the staging data', 'claude', 'infra', 'main', 'failed', 2 * MIN, 'It stopped with an error as soon as it started. Open the desk to see what it said.')
  ]
  if (name === 'empty') return []
  if (name === 'calm') return office.slice(1, 4).map(a => (a.phase === 'done' ? { ...a, phase: 'idle' as Phase } : a))
  if (name === 'crowd') {
    const repos = ['shop', 'api', 'infra', 'web', 'mobile']
    const phases: Phase[] = ['working', 'working', 'idle', 'done', 'needs_you', 'asleep', 'working', 'failed']
    return Array.from({ length: 20 }, (_, i) =>
      agent(i + 1, `Task ${i + 1} in ${repos[i % 5]}`, HARNESSES[i % 4]!.id, repos[i % 5]!, 'main', phases[i % phases.length]!, (i + 1) * MIN)
    )
  }
  return office
}

const ESC = '\x1b['
const dim = (text: string) => `${ESC}2m${text}${ESC}0m`
const bold = (text: string) => `${ESC}1m${text}${ESC}0m`
const cyan = (text: string) => `${ESC}36m${text}${ESC}0m`
const green = (text: string) => `${ESC}32m${text}${ESC}0m`

/** What a pretend program has on its screen. */
function screen(who: Agent): string {
  const lines: string[] = []
  if (who.phase === 'asleep' || who.phase === 'failed') {
    lines.push(dim(`${who.harness_name} is not running in this pretend office.`))
  } else if (who.harness === 'codex') {
    lines.push(`${dim('>')} ${who.title}`, '', `${bold('• Explored')}`, dim('  └ Read src/middleware/auth.ts, src/routes/orders.ts'), '')
    lines.push(`${bold('• Edited')} src/middleware/auth.ts ${dim('(+18 -4)')}`, green('    21 +  export function requireSession(req, res, next) {'), green('    22 +    const session = readSession(req)'), '')
    lines.push(who.phase === 'working' ? `${cyan('  Working')} ${dim('(2m 14s · esc to interrupt)')}` : `${dim('>')} `)
  } else {
    lines.push(`${dim('>')} ${who.title}`, '', `${green('●')} ${bold('Read')}(src/cart/total.ts)`, dim('  └  Read 48 lines'), '')
    lines.push(`${green('●')} ${bold('Update')}(src/cart/total.ts)`, dim('  └  Updated src/cart/total.ts with 1 addition and 2 removals'), '')
    if (who.phase === 'needs_you') {
      lines.push(`${dim('●')} ${bold('Bash')}(rm -rf dist && pnpm build)`, dim('  └  Waiting for permission'), '', cyan(bold('Bash command')), '', '  rm -rf dist && pnpm build', '')
      lines.push('Do you want to proceed?', cyan('> 1. Yes'), "  2. Yes, and don't ask again for pnpm build", '  3. No, and tell Claude what to do differently')
    } else {
      lines.push(`${dim('>')} `)
    }
  }
  return lines.join('\r\n')
}

/** What the pretend office was asked to do, for checks to read: `window.__demo` in a demo page. */
interface Seen {
  /** How many times the newer version was asked for. */
  updates: number
  watched: string[]
  typed: { agent: string; data: string }[]
  opened: { agent: string | null; path: string; line: number | null; editor: string }[]
}

export function demoBridge(): Bridge {
  const params = new URLSearchParams(location.search)
  const seen: Seen = { updates: 0, watched: [], typed: [], opened: [] }
  ;(window as unknown as { __demo: Seen }).__demo = seen
  const still = params.has('still')
  let agents = scene(params.get('demo') ?? 'office')
  /** The name each desk was given when it was seated. */
  const given = new Map(agents.map(a => [a.id, a.title]))
  let harnesses = HARNESSES.map(h => ({ ...h }))
  let jobs: Job[] = []
  let made = 0
  let closeQuits = false
  let voiceSettings: VoiceSettings = { ...DEFAULT_VOICE }
  let voice: VoiceView = { ...EMPTY_VOICE, models: [{ id: 'small', bytes: 190085487, ready: false }, { id: 'base', bytes: 59707625, ready: false }] }
  let voiceEpoch = 0
  let downloadEpoch = 0
  let recordingLimit: ReturnType<typeof setTimeout> | undefined
  const voiceChange = (patch: Partial<VoiceView>) => { voice = { ...voice, ...patch, sequence: voice.sequence + 1 } }
  const cancelVoice = () => {
    voiceEpoch += 1
    clearTimeout(recordingLimit)
    voiceChange({ phase: 'idle', agent: null, started_ms: null, message: '', busy: false })
  }
  const finishVoice = () => {
    if (voice.phase !== 'listening') throw 'Voice is not listening.'
    clearTimeout(recordingLimit)
    const epoch = voiceEpoch
    const target = voice.agent!
    voiceChange({ phase: 'transcribing' })
    setTimeout(() => {
      if (epoch !== voiceEpoch || !agents.some(a => a.id === target && a.running) || !followers.get(target)?.size) return
      if (['silence', 'short', 'inference-error'].includes(params.get('voice') ?? '')) {
        voiceChange({ phase: 'error', agent: null, started_ms: null, busy: false, message: params.get('voice') === 'silence' ? 'No clear speech was heard. Check the default microphone and try again.' : params.get('voice') === 'short' ? 'That recording was too short. Speak for at least half a second, then stop.' : 'Local transcription stopped. Try a shorter recording.' })
        return
      }
      const text = cleanVoice('Fix the\ncheckout total\x1b[31m\x1b[0m\x03')
      seen.typed.push({ agent: target, data: text })
      print(target, text)
      voiceChange({ phase: 'idle', agent: null, started_ms: null, busy: false, message: 'Text inserted. Review it in the terminal; press Enter yourself to send.' })
    }, 600)
  }
  const listeners = new Set<(snapshot: Snapshot) => void>()
  const followers = new Map<string, Set<OnTerminal>>()
  const encoder = new TextEncoder()

  const snapshot = (): Snapshot => ({ agents: agents.map(a => ({ ...a })), harnesses: harnesses.map(h => ({ ...h })), jobs: jobs.map(j => ({ ...j })), now_ms: Date.now() })
  const publish = () => listeners.forEach(fn => fn(snapshot()))
  const turn = (id: string, phase: Phase, activity = '') => {
    agents = agents.map(a => (a.id === id ? { ...a, phase, activity, since_ms: Date.now(), running: phase !== 'asleep' && phase !== 'failed' } : a))
    publish()
  }
  /** What each terminal has printed so far, for a pane that opens later: a job's lines, a desk's screen and what was typed. */
  const printed = new Map<string, string>()
  const print = (id: string, text: string) => {
    if (printed.has(id)) printed.set(id, printed.get(id) + text)
    followers.get(id)?.forEach(fn => fn(encoder.encode(text), false))
  }

  /** A pretend install or update: a few lines, then done, and the program is there. */
  function job(harness: string, kind: 'install' | 'update'): string {
    const kindOf = harnesses.find(h => h.id === harness)
    const line = kind === 'install' ? kindOf?.install_line : kindOf?.update_line
    if (!kindOf || !line) throw `The office does not know how to ${kind} ${kindOf?.name ?? 'that'}.`
    const id = `job-${kind}-${harness}`
    jobs = [...jobs.filter(j => j.id !== id), { id, harness, harness_name: kindOf.name, kind, running: true, ok: null }]
    printed.set(id, `${dim('$')} ${line}\r\n`)
    publish()
    const steps = ['', 'added 1 package in 3s', '', `${green('✓')} ${kindOf.name} ${kind === 'install' ? 'installed' : 'up to date'}`]
    steps.forEach((step, i) =>
      setTimeout(() => {
        print(id, `${step}\r\n`)
        if (i < steps.length - 1) return
        harnesses = harnesses.map(h => (h.id === harness ? { ...h, installed: true, version: h.latest || '1.0.0', outdated: false } : h))
        jobs = jobs.map(j => (j.id === id ? { ...j, running: false, ok: true } : j))
        publish()
      }, still ? 0 : 500 + i * 450)
    )
    return id
  }

  return {
    demo: true,
    snapshot: async () => snapshot(),
    onSnapshot: fn => {
      listeners.add(fn)
      return () => listeners.delete(fn)
    },
    onNewAgent: fn => { queueMicrotask(fn); return () => {} },
    takeNewAgent: async () => { const pending = params.has('new-agent'); params.delete('new-agent'); return pending },
    onStorageProblem: () => () => {},
    onOpenDesk: () => () => {},
    // `&open=demo-4` stands for a click on a notification about that desk.
    takeOpening: async () => params.get('open'),
    modelCatalog: async (harness, _cwd, _profile, refresh) => {
      if (params.has('catalog-fails')) throw 'The model catalog could not be read. Try Refresh, or enter a model ID.'
      const claude = harness === 'claude'
      const models: ModelChoice[] = claude
        ? [{ id: 'opus', name: 'Opus', description: 'Demo model catalog', efforts: ['low', 'medium', 'high', 'xhigh', 'max'], resolved: 'claude-opus-demo', auto_mode: true }]
        : [{ id: 'codex-demo', name: 'Codex demo', description: 'Demo model catalog', efforts: ['low', 'medium', 'high', 'xhigh'], resolved: 'codex-demo', auto_mode: null }]
      if (refresh && params.has('catalog-new')) models.push({ id: 'newly-released-model', name: 'Newly released model', description: 'A new entry returned by the provider', efforts: ['new-effort'], resolved: 'newly-released-model', auto_mode: true })
      return {
        models,
        permissions: [
          { id: '', name: 'Use CLI settings', description: 'Inherits your configured permission settings.' },
          ...(claude ? [
            { id: 'default', name: 'Manual', description: 'Asks you to approve actions that need permission.' },
            { id: 'auto', name: 'Auto', description: 'Reviews actions automatically.' },
            { id: 'acceptEdits', name: 'Accept edits', description: 'Accepts edits; other actions follow your rules.' },
            { id: 'plan', name: 'Plan', description: 'Explores the project and prepares a plan.' },
            { id: 'dontAsk', name: "Don't ask", description: 'Denies actions that would need a permission prompt.' },
            { id: 'bypassPermissions', name: 'Bypass permissions', description: 'Skips permission checks for this session.' }
          ] : [
            { id: 'workspace', name: 'Workspace with approvals', description: 'Works in the workspace sandbox and can request approval.' },
            { id: 'read-only', name: 'Read-only sandbox', description: 'Starts in a read-only sandbox and can request approval.' },
            { id: 'auto', name: 'Automatic approval review', description: 'Reviews approvals in the workspace sandbox.' },
            { id: 'yolo', name: 'YOLO / full access', description: 'Skips approvals and runs without the Codex sandbox.' },
            { id: 'custom', name: 'Custom sandbox and approvals', description: 'Choose sandbox and approval policy under Advanced.' }
          ])
        ],
        features: claude ? ['model', 'effort', 'additional_dirs', 'tools', 'chrome', 'instructions'] : ['model', 'effort', 'additional_dirs', 'profile', 'search'],
        fetched_ms: Date.now(), source: 'Demo model catalog', problem: ''
      }
    },
    install: async harness => job(harness, 'install'),
    update: async harness => job(harness, 'update'),
    forgetJob: id => {
      jobs = jobs.filter(j => j.id !== id)
      printed.delete(id)
      publish()
    },
    newAgent: async (spec: NewAgentSpec) => {
      const kind = harnesses.find(h => h.id === spec.harness)
      if (!kind?.installed) throw `${kind?.name ?? 'That program'} was not found on this computer.`
      made += 1
      const repo = spec.cwd.replace(/[\\/]+$/, '').split(/[\\/]/).pop() || 'project'
      // Named the way the core names a desk: from the task, else program and folder, numbered when taken.
      const auto = uniqueTitle(spec.prompt.trim() ? titleFrom(spec.prompt) : `${kind.tag} in ${repo}`, agents.map(a => a.title))
      const title = spec.title?.trim() ? shorten(spec.title, 60) : auto
      const fresh = { ...agent(100 + made, title, kind.id, repo, 'main', 'starting', 0), id: `demo-new-${made}`, project: spec.cwd, cwd: spec.cwd, launch: structuredClone(spec.launch ?? {}) }
      given.set(fresh.id, auto)
      agents = [...agents, fresh]
      publish()
      if (!still) setTimeout(() => turn(fresh.id, spec.prompt ? 'working' : 'idle'), 900)
      return fresh.id
    },
    wake: async id => {
      const who = agents.find(a => a.id === id)
      if (who && !who.running) turn(id, 'idle')
    },
    follow: async (id, fn) => {
      const who = agents.find(a => a.id === id)
      if (who && !printed.has(id)) printed.set(id, screen(who))
      fn(encoder.encode(printed.get(id) ?? ''), true)
      if (!followers.has(id)) followers.set(id, new Set())
      followers.get(id)!.add(fn)
      return () => { followers.get(id)?.delete(fn); if (voice.agent === id) cancelVoice() }
    },
    type: (id, data) => {
      seen.typed.push({ agent: id, data })
      // A pretend program shows what is typed, and takes Enter as an answer.
      print(id, data === '\r' ? '\r\n' : data === '\x7f' ? '\b \b' : data)
      const who = agents.find(a => a.id === id)
      if (data === '\r' && who?.phase === 'needs_you') turn(id, 'working')
    },
    resize: () => {},
    stop: id => { if (voice.agent === id) cancelVoice(); turn(id, 'asleep') },
    restart: async id => { if (voice.agent === id) cancelVoice(); turn(id, 'idle') },
    showFolder: async () => {},
    editors: async () => (params.has('noeditor') ? [] : [{ id: 'cursor', name: 'Cursor' }, { id: 'zed', name: 'Zed' }]),
    openFile: async (agent, path, line, editor) => {
      seen.opened.push({ agent, path, line, editor })
      return params.has('noeditor') ? 'folder' : 'editor'
    },
    openFolder: async (agent, editor) => void seen.opened.push({ agent, path: '', line: null, editor }),
    savePastedImage: async () => 'C:/Users/you/AppData/Roaming/agent-moshpit/pasted/pasted-1.png',
    startupProblems: async () => {
      const file = 'C:/Users/you/AppData/Roaming/io.github.yashingole.agentmoshpit/harnesses.json'
      const text = `harnesses.json could not be read (expected \`,\` or \`}\` at line 3 column 5), so the programs in it are not offered. It is ${file}.`
      return params.has('problem') ? [{ text, file, line: 3 }] : []
    },
    // In a browser a dropped file has a name but no path; the name stands in for it.
    onDrop: fn => {
      const where = (event: DragEvent) => ({ x: event.clientX, y: event.clientY })
      const over = (event: DragEvent) => {
        if (!event.dataTransfer?.types.includes('Files')) return
        event.preventDefault()
        fn({ kind: 'over', paths: [], ...where(event) })
      }
      const drop = (event: DragEvent) => {
        if (!event.dataTransfer?.files.length) return
        event.preventDefault()
        fn({ kind: 'drop', paths: [...event.dataTransfer.files].map(file => `C:/Users/you/Desktop/${file.name}`), ...where(event) })
      }
      const leave = (event: DragEvent) => {
        if (event.relatedTarget === null) fn({ kind: 'leave', paths: [], x: 0, y: 0 })
      }
      window.addEventListener('dragover', over)
      window.addEventListener('drop', drop)
      window.addEventListener('dragleave', leave)
      return () => {
        window.removeEventListener('dragover', over)
        window.removeEventListener('drop', drop)
        window.removeEventListener('dragleave', leave)
      }
    },
    dismiss: id => {
      if (voice.agent === id) cancelVoice()
      agents = agents.filter(a => a.id !== id)
      followers.delete(id)
      publish()
    },
    rename: (id, title) => {
      // Emptied, a name goes back to the one it was given, as in the core.
      agents = agents.map(a => (a.id === id ? { ...a, title: title.trim() || given.get(id) || a.title } : a))
      publish()
    },
    watch: ids => {
      seen.watched = [...ids]
      if (voice.agent && !ids.includes(voice.agent)) cancelVoice()
      if (agents.some(a => ids.includes(a.id) && a.unread)) {
        agents = agents.map(a => (ids.includes(a.id) ? { ...a, unread: false } : a))
        publish()
      }
      const finished = agents.filter(a => ids.includes(a.id) && a.phase === 'done')
      for (const a of finished) turn(a.id, 'idle')
    },
    openPage: url => void window.open(url, '_blank', 'noopener'),
    pickFolder: async () => 'C:/code/shop',
    quit: () => {},
    settings: async () => ({ close_quits: closeQuits, voice: { ...voiceSettings } }),
    setCloseQuits: on => void (closeQuits = on),
    version: async () => '0.3.0',
    newer: async () => (params.has('update') ? { version: '0.3.1' } : null),
    onNewer: () => () => {},
    // Nothing is fetched in a pretend office: it says so the way a failure would, or is asked for and noted.
    voiceView: async () => structuredClone(voice),
    voiceConfig: async settings => { cancelVoice(); voiceSettings = { ...settings } },
    voiceStart: async target => {
      if (!voiceSettings.enabled) throw 'Enable local voice in the Voice input panel first.'
      if (!voice.models.some(m => m.id === voiceSettings.model && m.ready)) throw 'Download the selected voice model first, then try again.'
      if (!agents.some(a => a.id === target && a.running) || !followers.get(target)?.size) throw 'Select a visible terminal whose program is running before recording.'
      if (voice.busy) throw 'The previous voice worker is finishing. Try again in a moment.'
      cancelVoice()
      if (params.get('voice') === 'mic-error') { voiceChange({ phase: 'error', message: 'The microphone could not start. Check system microphone access and the default input device.' }); return }
      voiceChange({ phase: 'listening', agent: target, started_ms: Date.now(), busy: true })
      recordingLimit = setTimeout(finishVoice, 60_000)
    },
    voiceStop: async () => finishVoice(),
    voiceCancel: async () => cancelVoice(),
    voiceDownload: async model => {
      if (voice.downloading) throw 'A model is already being downloaded.'
      const download = ++downloadEpoch
      voiceChange({ downloading: model, received: 0, download_error: '' })
      setTimeout(() => {
        if (download !== downloadEpoch || voice.downloading !== model) return
        if (params.get('voice') === 'download-error') voiceChange({ downloading: null, download_error: 'The model did not pass its size and SHA-256 check. Download it again.' })
        else voiceChange({ downloading: null, models: voice.models.map(m => m.id === model ? { ...m, ready: true } : m) })
      }, 400)
    },
    voiceCancelDownload: async () => { downloadEpoch += 1; voiceChange({ downloading: null, download_error: 'Download cancelled.' }) },
    voiceRemove: async model => { cancelVoice(); voiceChange({ models: voice.models.map(m => m.id === model ? { ...m, ready: false } : m) }) },
    updateNow: async () => {
      seen.updates += 1
      if (params.get('update') === 'fails') throw 'The update could not be fetched (no network). Nothing was changed.'
      return false
    }
  }
}

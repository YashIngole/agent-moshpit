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
import { version as appVersion } from '../../package.json'
import type { Bridge, OnTerminal } from './bridge'
import type { Agent, Harness, Job, ModelChoice, NewAgentSpec, Phase, Snapshot } from './types'
import { shorten, titleFrom, uniqueTitle } from './words'
import { cleanVoice, DEFAULT_VOICE, EMPTY_VOICE, engineName, isPhrased, type FieldText, type VoiceProvider, type VoiceSettings, type VoiceView } from './voice'

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
  install_needs: '',
  signed_in: null,
  account: '',
  login_line: '',
  ...over
})

const HARNESSES: Harness[] = [
  program({ id: 'claude', launch: 'claude', name: 'Claude Code', tag: 'Claude', version: '2.1.290', latest: '2.1.294', outdated: true, takes_task: true, worktree: true, install_line: 'npm install -g @anthropic-ai/claude-code', update_line: 'claude update', signed_in: true, account: 'Signed in as you@example.com · Max', login_line: 'claude auth login' }),
  program({ id: 'codex', launch: 'codex', name: 'Codex', tag: 'Codex', version: '0.161.0', latest: '0.161.0', takes_task: true, worktree: true, install_line: 'npm install -g @openai/codex', update_line: 'codex update', signed_in: true, account: 'Logged in using ChatGPT', login_line: 'codex login' }),
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
    // Out of sight since it printed: something new in their terminal. On a desk at work it is not drawn.
    unread: (phase === 'working' && n % 2 === 1) || (phase === 'done' && n % 2 === 0)
  }
}

function scene(name: string): Agent[] {
  const office = [
    agent(1, 'Fix the checkout total', 'claude', 'shop', 'fix/checkout-total', 'needs_you', 48_000, 'Waiting for your answer in the terminal'),
    agent(2, 'Refactor auth middleware', 'codex', 'shop', 'refactor/auth', 'working', 12 * MIN, 'Moving the session check into middleware'),
    agent(3, 'Flaky test hunt', 'claude', 'shop', 'main', 'asleep', 26 * 60 * MIN),
    agent(4, 'Type the orders API', 'codex', 'api', 'types/orders', 'done', 3 * MIN, 'Typed the orders routes; 14 tests pass'),
    agent(5, 'Docs pass for the API', 'antigravity', 'api', 'main', 'working', 9 * MIN),
    agent(6, 'Bump dependencies', 'hermes', 'api', 'main', 'idle', 4 * MIN),
    agent(7, 'Seed the staging data', 'claude', 'infra', 'main', 'failed', 2 * MIN, 'It stopped with an error as soon as it started.')
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
  visible: string[]
  typed: { agent: string; data: string }[]
  opened: { agent: string | null; path: string; line: number | null; editor: string }[]
  /** How many pretend keys were kept. The keys themselves are not. */
  keys: number
  /** Each program started in a terminal that was there before, and the size it was started at. */
  started: { agent: string; cols: number; rows: number }[]
  /** Each size a terminal's program was told. */
  sized: { agent: string; cols: number; rows: number }[]
  /** How many times the programs were asked who is signed in. */
  asked: number
  /** How many times the programs were looked for again. */
  looked: number
  /** Node.js arrives (`&nonode`), as when its installer has finished: the next look finds it. */
  findNode: () => void
  /** Each agent that was asked for, as it was asked for. */
  specs: NewAgentSpec[]
  /** Let a sign-in that is waiting (`&signin=waits`) end, as it does when the browser comes back. */
  finishSignIn: () => void
}

export function demoBridge(): Bridge {
  const params = new URLSearchParams(location.search)
  const seen: Seen = { updates: 0, watched: [], visible: [], typed: [], opened: [], keys: 0, started: [], sized: [], asked: 0, looked: 0, findNode: () => {}, specs: [], finishSignIn: () => {} }
  ;(window as unknown as { __demo: Seen }).__demo = seen
  const still = params.has('still')
  let agents = scene(params.get('demo') ?? 'office')
  /** The name each desk was given when it was seated. */
  const given = new Map(agents.map(a => [a.id, a.title]))
  // `&signedout=claude` (or `claude,codex`): nobody is signed in to those.
  const signedOut = (params.get('signedout') ?? '').split(',')
  // `&nonode`: Node.js is not on this computer, so nothing npm installs can be installed yet.
  let noNode = params.has('nonode')
  const needs = (h: Harness): Harness => ({ ...h, install_needs: noNode && !h.installed && h.install_line.startsWith('npm ') ? 'Node.js' : '' })
  let harnesses = HARNESSES.map(h => needs(signedOut.includes(h.id) ? { ...h, signed_in: false, account: 'Signed out' } : { ...h }))
  seen.findNode = () => { noNode = false }
  let jobs: Job[] = []
  let made = 0
  let closeQuits = false
  let voiceSettings: VoiceSettings = { ...DEFAULT_VOICE }
  // `?voice=<scene>` plays one way a recording or a key can go wrong.
  const voiceScene = params.get('voice') ?? ''
  let voice: VoiceView = {
    ...EMPTY_VOICE,
    unsupported: voiceScene === 'unsupported' ? "The local speech engine needs a processor with AVX2 (Intel from 2013 or AMD from 2015), and this computer's processor does not have it." : '',
    models: [
      { id: 'nemotron-streaming-en-q8', label: 'Nemotron Speech Streaming', bytes: 729650176, ready: false, recommended: true },
      { id: 'nemotron-streaming-en-q4', label: 'Nemotron Speech Streaming, smaller', bytes: 475436032, ready: false, recommended: false }
    ],
    providers: (['deepgram', 'openai', 'assemblyai', 'elevenlabs', 'openrouter'] as const).map(id => ({ id, name: engineName(id), configured: false, last4: '', session_only: false }))
  }
  let voiceEpoch = 0
  let downloadEpoch = 0
  let recordingLimit: ReturnType<typeof setTimeout> | undefined
  let hearing: ReturnType<typeof setInterval> | undefined
  // What the pretend speaker says, in the pieces an engine settles; the first carries controls the typing must strip.
  const spoken = [['Fix', 'the\n', 'checkout', 'total\x1b[31m\x1b[0m\x03'], ['and', 'add', 'a', 'test']]
  let heard = 0
  let settled = 0
  const voiceChange = (patch: Partial<VoiceView>) => { voice = { ...voice, ...patch, sequence: voice.sequence + 1 } }
  const providerName = (id: VoiceProvider) => engineName(id)
  // Where the words of a recording for a field in the window go; a terminal's are typed.
  let sink: ((message: FieldText) => void) | null = null
  /** The field is told, last and once, that its recording is over. */
  const endField = () => {
    const end = sink
    sink = null
    end?.({ text: '', space: false, done: true })
  }
  const cancelVoice = () => {
    voiceEpoch += 1
    clearTimeout(recordingLimit)
    clearInterval(hearing)
    voiceChange({ phase: 'idle', agent: null, started_ms: null, message: '', detail: '', busy: false, level: 0, transcribing_ms: null, partial: '', field: false })
    endField()
  }
  /** Settled pieces go where the recording is for: `null` is the field in the window. */
  const typeSettled = (target: string | null, upTo: number) => {
    while (settled < upTo) {
      const words = cleanVoice(spoken[settled]!.join(' '))
      if (target === null) sink?.({ text: words, space: settled > 0, done: false })
      else {
        const text = (settled ? ' ' : '') + words
        seen.typed.push({ agent: target, data: text })
        print(target, text)
      }
      settled += 1
    }
  }
  const failVoice = (message: string) => {
    clearTimeout(recordingLimit)
    clearInterval(hearing)
    voiceChange({ phase: 'error', agent: null, started_ms: null, busy: false, partial: '', detail: '', level: 0, transcribing_ms: null, message })
    endField()
  }
  const finishVoice = () => {
    if (voice.phase !== 'listening' && voice.phase !== 'preparing') throw 'Voice is not listening.'
    clearTimeout(recordingLimit)
    clearInterval(hearing)
    const epoch = voiceEpoch
    const target = voice.field ? null : voice.agent!
    voiceChange({ phase: 'transcribing', level: 0, transcribing_ms: Date.now() })
    setTimeout(() => {
      if (epoch !== voiceEpoch || (target !== null && (!agents.some(a => a.id === target && a.running) || !followers.get(target)?.size))) return
      if (['silence', 'short', 'inference-error'].includes(voiceScene) && !settled) {
        failVoice(voiceScene === 'silence' ? 'No clear speech was heard. Check the microphone and try again.' : voiceScene === 'short' ? 'That recording was too short. Speak for at least half a second, then stop.' : 'The local speech engine stopped unexpectedly. Words already typed stay. Try again.')
        return
      }
      // The engine settles the rest of what it heard: a whole piece, even one cut short.
      typeSettled(target, Math.max(settled, Math.ceil(heard / 4), 1))
      voiceChange({ phase: 'idle', agent: target, started_ms: null, busy: false, partial: '', message: target === null ? 'Added to the task.' : 'Inserted. Press Enter in the terminal to send.' })
      endField()
      // `?voice=slow-finish`: the last words take their time, long enough to change one's mind.
    }, voiceScene === 'slow-finish' ? 1500 : 300)
  }
  /** Start a recording: for a terminal, or with `onText` for a field in the window. */
  const startVoice = async (target: string | null, onText: ((message: FieldText) => void) | null) => {
    if (!voiceSettings.enabled) throw 'Turn on voice input in the Voice input panel first.'
    const engine = voiceSettings.engine
    if (engine === 'local' && voice.unsupported) throw `${voice.unsupported} Choose a cloud provider in Voice input instead.`
    if (engine !== 'local' && !voice.providers.some(p => p.id === engine && p.configured)) throw `Add your ${providerName(engine)} key in Voice input first.`
    if (target !== null && (!agents.some(a => a.id === target && a.running) || !followers.get(target)?.size)) throw 'Select a visible terminal whose program is running before recording.'
    if (voice.busy) throw 'The previous recording is finishing. Try again in a moment.'
    if (engine === 'local' && !voice.models.some(m => m.id === voiceSettings.local_model && m.ready)) throw 'Download the local speech model in Voice input first, then try again.'
    cancelVoice()
    if (voiceSettings.microphone && !['Built-in microphone', 'USB microphone'].includes(voiceSettings.microphone)) {
      voiceChange({ phase: 'error', field: target === null, message: 'The selected microphone is disconnected. Open Voice input, refresh microphones and choose an available input.' }); return
    }
    if (voiceScene === 'mic-error') { voiceChange({ phase: 'error', field: target === null, message: 'The microphone could not start. Check system microphone access and the default input device.' }); return }
    const epoch = voiceEpoch
    heard = 0
    settled = 0
    sink = onText
    voiceChange({ phase: 'preparing', agent: target, field: target === null, engine, busy: true, detail: engine === 'local' ? 'Loading the local speech model…' : `Connecting to ${providerName(engine)}…` })
    await new Promise(resolve => setTimeout(resolve, 150))
    if (epoch !== voiceEpoch) return
    if (engine !== 'local') {
      const name = providerName(engine)
      const fault = voiceScene === 'auth' ? `${name} did not accept the key. Check it, or add it again, in Voice input.`
        : voiceScene === 'quota' ? `${name} says the account is out of credit or over its limit. Add credit with ${name}, or switch engine in Voice input.`
        : voiceScene === 'offline' ? `${name} could not be reached. Check your connection, or use the local engine in Voice input.` : ''
      if (fault) { failVoice(fault); return }
    }
    voiceChange({ phase: 'listening', started_ms: Date.now(), detail: '', microphone: voiceSettings.microphone ?? 'Built-in microphone', level: voiceScene === 'silence' ? 0 : 36 })
    recordingLimit = setTimeout(finishVoice, 300_000)
    if (voiceScene === 'silence') return
    // Words show as they are heard; a piece is typed once the engine settles it, two words later.
    const words = spoken.flat()
    hearing = setInterval(() => {
      if (epoch !== voiceEpoch || heard >= words.length) return
      heard += 1
      if (engine === 'local' && voiceScene === 'worker-crash' && heard === 7) { failVoice('The local speech engine stopped unexpectedly. Words already typed stay. Try again.'); return }
      if (heard >= (settled + 1) * 4 + 2 && (voice.agent || voice.field)) typeSettled(voice.field ? null : voice.agent, settled + 1)
      // A provider that is sent whole phrases has no words to show while one is being said.
      voiceChange({ partial: isPhrased(engine) ? '' : cleanVoice(words.slice(settled * 4, heard).join(' ')) })
    }, 300)
  
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

  /** A pretend install, update or sign-in: a few lines, then done, and the program is there, or someone is signed in. */
  function job(harness: string, kind: Job['kind']): string {
    const kindOf = harnesses.find(h => h.id === harness)
    const line = kind === 'install' ? kindOf?.install_line : kind === 'update' ? kindOf?.update_line : kindOf?.login_line
    if (!kindOf || !line) throw kind === 'signin' ? `The office does not know how to sign in to ${kindOf?.name ?? 'that'}. Start one of its desks and sign in from its terminal.` : `The office does not know how to ${kind} ${kindOf?.name ?? 'that'}.`
    if (kind === 'install' && noNode && line.startsWith('npm ')) throw `${kindOf.name} is installed with npm, which comes with Node.js, and Node.js was not found on this computer. Install it from nodejs.org, then try again here: the office does not need restarting.`
    const id = `job-${kind}-${harness}`
    jobs = [...jobs.filter(j => j.id !== id), { id, harness, harness_name: kindOf.name, kind, running: true, ok: null }]
    printed.set(id, `${dim('$')} ${line}\r\n`)
    publish()
    const account = harness === 'claude' ? 'Signed in as other@example.com · Max' : 'Logged in using ChatGPT'
    const steps = kind === 'signin' ? ['', 'Opening your browser to sign in…', '', `${green('✓')} ${account}`] : ['', 'added 1 package in 3s', '', `${green('✓')} ${kindOf.name} ${kind === 'install' ? 'installed' : 'up to date'}`]
    const finish = () => {
      if (!jobs.some(j => j.id === id && j.running)) return
      print(id, `${steps.at(-1)}\r\n`)
      harnesses = harnesses.map(h => (h.id !== harness ? h : kind === 'signin' ? { ...h, signed_in: true, account } : { ...h, installed: true, version: h.latest || '1.0.0', outdated: false }))
      jobs = jobs.map(j => (j.id === id ? { ...j, running: false, ok: true } : j))
      publish()
    }
    // `&signin=waits`: a sign-in stays open on the browser until it is let through, or stopped.
    const waits = kind === 'signin' && params.get('signin') === 'waits'
    if (waits) seen.finishSignIn = finish
    steps.forEach((step, i) =>
      setTimeout(() => {
        if (i < steps.length - 1) print(id, `${step}\r\n`)
        else if (!waits) finish()
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
        plugins: claude ? [{ id: 'rust-analyzer-lsp@claude-plugins-official', enabled: true }, { id: 'figma@claude-plugins-official', enabled: false }] : [],
        features: claude ? ['model', 'effort', 'additional_dirs', 'tools', 'chrome', 'instructions', 'plugins'] : ['model', 'effort', 'additional_dirs', 'profile', 'search'],
        fetched_ms: Date.now(), source: 'Demo model catalog', problem: ''
      }
    },
    install: async harness => job(harness, 'install'),
    update: async harness => job(harness, 'update'),
    signIn: async harness => job(harness, 'signin'),
    refreshPrograms: accounts => {
      seen.looked += 1
      if (accounts) seen.asked += 1
      // What has changed on the computer since the last look is found.
      if (harnesses.some(h => h.install_needs !== needs(h).install_needs)) {
        harnesses = harnesses.map(needs)
        publish()
      }
    },
    forgetJob: id => {
      jobs = jobs.filter(j => j.id !== id)
      printed.delete(id)
      publish()
    },
    newAgent: async (spec: NewAgentSpec) => {
      const kind = harnesses.find(h => h.id === spec.harness)
      if (!kind?.installed) throw `${kind?.name ?? 'That program'} was not found on this computer.`
      seen.specs.push(structuredClone(spec))
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
    wake: async (id, cols, rows) => {
      const who = agents.find(a => a.id === id)
      if (who && !who.running) {
        seen.started.push({ agent: id, cols, rows })
        turn(id, 'idle')
      }
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
    resize: (id, cols, rows) => void seen.sized.push({ agent: id, cols, rows }),
    stop: id => { if (voice.agent === id) cancelVoice(); turn(id, 'asleep') },
    restart: async (id, cols, rows) => { if (voice.agent === id) cancelVoice(); seen.started.push({ agent: id, cols, rows }); turn(id, 'idle') },
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
    watch: (ids, visible = ids) => {
      seen.watched = [...ids]
      seen.visible = [...visible]
      if (voice.agent && !visible.includes(voice.agent)) cancelVoice()
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
    version: async () => appVersion,
    newer: async () => (params.has('update') ? { version: '99.0.0' } : null),
    onNewer: () => () => {},
    // Nothing is fetched in a pretend office: it says so the way a failure would, or is asked for and noted.
    voiceView: async () => structuredClone(voice),
    onVoiceView: () => () => {},
    voiceInputs: async () => {
      if (params.get('voice') === 'inputs-error') throw 'Microphones could not be listed. Reconnect your microphone and refresh.'
      return { devices: ['Built-in microphone', 'USB microphone'], default: 'Built-in microphone' }
    },
    voiceConfig: async settings => { cancelVoice(); voiceSettings = { ...settings } },
    voiceStart: target => startVoice(target, null),
    voiceStartField: async onText => {
      // However it goes, the field hears that it is over exactly once.
      let over = false
      const once = (message: FieldText) => {
        if (over) return
        over = message.done
        onText(message)
      }
      try { await startVoice(null, once) }
      finally { if (sink !== once) once({ text: '', space: false, done: true }) }
    },
    voiceStop: async () => finishVoice(),
    voiceCancel: async () => cancelVoice(),
    voiceDownload: async model => {
      if (voice.downloading) throw 'A model is already being downloaded.'
      const download = ++downloadEpoch
      voiceChange({ downloading: model, received: 0, download_error: '' })
      setTimeout(() => {
        if (download !== downloadEpoch || voice.downloading !== model) return
        if (voiceScene === 'download-error') voiceChange({ downloading: null, download_error: 'The model did not pass its size and SHA-256 check. Download it again.' })
        else voiceChange({ downloading: null, models: voice.models.map(m => m.id === model ? { ...m, ready: true } : m) })
      }, 400)
    },
    voiceCancelDownload: async () => { downloadEpoch += 1; voiceChange({ downloading: null, download_error: 'Download cancelled.' }) },
    voiceRemove: async model => { cancelVoice(); voiceChange({ models: voice.models.map(m => m.id === model ? { ...m, ready: false } : m) }) },
    // A pretend key is checked by its scene, and only its last four characters are kept.
    voiceKeySet: async (provider, key, sessionOnly, check) => {
      const name = providerName(provider)
      key = key.trim()
      if (key.length < 8 || /\s/.test(key)) throw "That does not look like an API key. Copy it again from the provider's dashboard."
      await new Promise(resolve => setTimeout(resolve, 200))
      if (check && voiceScene === 'auth') throw `${name} did not accept the key. Check it, or add it again, in Voice input.`
      if (check && voiceScene === 'offline') throw `unreached:${name} could not be reached to check the key. Check your connection, or save the key without checking.`
      if (!sessionOnly && voiceScene === 'store-unavailable') throw 'keychain:The system keychain could not be opened: no Secret Service is running.'
      seen.keys += 1
      voiceChange({ providers: voice.providers.map(p => p.id === provider ? { ...p, configured: true, last4: key.slice(-4), session_only: sessionOnly } : p) })
    },
    voiceKeyCheck: async provider => {
      const name = providerName(provider)
      await new Promise(resolve => setTimeout(resolve, 200))
      if (voiceScene === 'auth') throw `${name} did not accept the key. Check it, or add it again, in Voice input.`
      if (voiceScene === 'offline') throw `unreached:${name} could not be reached to check the key. Check your connection, or save the key without checking.`
    },
    voiceKeyRemove: async provider => { voiceChange({ providers: voice.providers.map(p => p.id === provider ? { ...p, configured: false, last4: '', session_only: false } : p) }) },
    updateNow: async () => {
      seen.updates += 1
      if (params.get('update') === 'fails') throw 'The update could not be fetched (no network). Nothing was changed.'
      return false
    }
  }
}

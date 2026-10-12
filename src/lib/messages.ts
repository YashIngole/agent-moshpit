// What agents have sent each other, and how far each thing got on its way. The core keeps
// it (the office's MCP service); the window only shows it. Every word here says what the
// office did or was told, and no more: that something was pushed is not that it arrived,
// and that it was handed over is not that it was acted on.

/** How something new is brought to an agent's attention, as the user has set it. */
export interface MessageDelivery {
  /** Claude Code is given hooks that say what is new at a step of its work. */
  hooks: boolean
  /** One of them waits while the session is idle, and wakes it. */
  wake: boolean
  /** Claude Code is started with its Channels preview: new things are pushed into the session. */
  channels: boolean
}
export const DEFAULT_DELIVERY: MessageDelivery = { hooks: true, wake: true, channels: false }

/** How far one item got on its way to the session it is for. */
export interface Delivery {
  state: 'saved' | 'notified' | 'fetched' | 'acknowledged'
  revision: number
  notified_ms: number | null
  /** `channel`, `hook`, `stop`, `wake`, `hint` or `inbox`; empty until it has been told. */
  notified_via: string
  fetched_ms: number | null
  acknowledged_ms: number | null
}

/** What is known of one session's connection, in this run of its program. */
export interface Link {
  /** Started from the office and not stopped since. */
  granted: boolean
  /** Its program has connected and can read what it is told of. */
  live: boolean
  client: string
  push: 'channel' | 'hooks' | 'poll'
  channel: 'off' | 'requested' | 'confirmed'
  hooks: 'off' | 'configured' | 'seen'
  hook_seen_ms: number | null
  wake: boolean
  /** Why something it could have been given was left out, in a sentence. */
  note: string
}

export interface MailDesk {
  id: string
  title: string
  harness: string
  harness_name: string
  running: boolean
  /** Things for it that it has not read. */
  waiting: number
  link: Link
}

export interface MailItem {
  kind: 'message' | 'task'
  id: string
  from_title: string
  to_title: string
  text: string
  created_ms: number
  /** A task's own state: queued, in_progress, completed, blocked, failed. */
  status?: string
  /** Handed over as its session started: there was nothing to tell. */
  spawned?: boolean
  delivery: Delivery | null
  /** For a task that has been reported on: how far the result got back to whoever asked. */
  result?: Delivery | null
}

export interface Mailbox {
  desks: MailDesk[]
  items: MailItem[]
  push: MessageDelivery
}

const clock = (ms: number | null) => (ms ? new Date(ms).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' }) : '')

/** How far an item got, in a sentence that claims only what is known. */
export function deliveryWords(delivery: Delivery | null): string {
  if (!delivery) return 'Handed over as its session started.'
  if (delivery.state === 'acknowledged') return `Acknowledged ${clock(delivery.acknowledged_ms)}.`
  if (delivery.state === 'fetched') return `Read in full ${clock(delivery.fetched_ms)}. Not acknowledged.`
  if (delivery.state === 'saved') return 'Saved. Not told yet.'
  const at = clock(delivery.notified_ms)
  switch (delivery.notified_via) {
    case 'channel': return `Pushed ${at}. Not known to have arrived.`
    case 'hook': return `Said at a step of its work ${at}. Not read yet.`
    case 'stop': return `Said as its turn ended ${at}. Not read yet.`
    case 'wake': return `It was woken for this ${at}. Not read yet.`
    case 'hint': return `Said with a tool's answer ${at}. Not read yet.`
    default: return `Its start was listed in its inbox ${at}. Not read in full.`
  }
}

/** How a session is reached, in a sentence that claims only what is known. */
export function linkWords(desk: MailDesk): string {
  const { link } = desk
  if (!desk.running) return 'Not running. What is sent is kept, and said when it is carried on.'
  if (!link.live) return 'Started. Its program has not connected to the office yet.'
  const hooks = link.hooks === 'seen' ? (link.wake ? 'told at a step of its work, and woken when idle' : 'told at a step of its work or its next prompt') : link.hooks === 'configured' ? 'given hooks, which have not run yet' : ''
  if (link.channel !== 'off') {
    const pushed = link.channel === 'confirmed' ? 'Pushes reach it' : 'Pushes are on; none is known to have arrived yet'
    return hooks ? `${pushed}. Also ${hooks}.` : `${pushed}.`
  }
  if (hooks) return `Connected: ${hooks}.`
  return 'Connected. It finds what is new with its next call to the office, or when it checks its inbox.'
}

/** A task's own state, in the office's words. */
export function taskWords(status: string | undefined): string {
  return ({ starting: 'starting', queued: 'not taken up yet', in_progress: 'taken up', completed: 'done', blocked: 'blocked', failed: 'failed' } as Record<string, string>)[status ?? ''] ?? ''
}

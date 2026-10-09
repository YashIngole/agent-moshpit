<script lang="ts" module>
  /**
   * floor: nobody open. terminal: one terminal, the floor as a strip. side: one terminal
   * beside a floor that keeps its desks. split: four terminals at once. new, elsewhere:
   * the two side panels.
   */
  export type Screen = 'floor' | 'terminal' | 'side' | 'split' | 'new' | 'elsewhere'
</script>

<script lang="ts">
  // Pictures of the office once it sits over Claude Code and Codex instead of Hermes.
  // The drawings are the app's own components; the layout rules are copied from Floor, Desk,
  // TopBar, KnockBand and NewAgent so the pictures match what would be built.
  import Person from '../../../src/components/Person.svelte'
  import SpareDesk from '../../../src/components/SpareDesk.svelte'
  import StatusMark from '../../../src/components/StatusMark.svelte'
  import type { Phase } from '../../../src/lib/types'

  type Harness = 'Claude' | 'Codex'
  /** here: its terminal can be opened in the office. app, outside: it lives somewhere else. */
  type Where = 'here' | 'app' | 'outside'

  interface Seat {
    id: string
    title: string
    phase: Phase
    tool?: string
    harness: Harness
    since: string
    doing: string
    look: number
    where: Where
    branch: string
  }

  let { screen }: { screen: Screen } = $props()

  /** Nobody has a hand up in the pictures that are about something else. */
  const calm = screen === 'new' || screen === 'elsewhere'
  /** Which desks have their terminal on screen, in the order of the panes. */
  const openIds: string[] =
    screen === 'split' ? ['checkout', 'auth', 'docs', 'orders'] : screen === 'terminal' || screen === 'side' ? ['checkout'] : screen === 'elsewhere' ? ['deps'] : []
  const terminals = screen === 'terminal' || screen === 'side' || screen === 'split'

  const WORD: Record<Phase, string> = {
    needs_you: 'Needs you',
    working: 'Working',
    done: 'Done',
    idle: 'Idle',
    failed: 'Trouble',
    starting: 'Starting',
    asleep: 'Away'
  }

  const checkout: Seat = calm
    ? { id: 'checkout', title: 'Fix the checkout total', phase: 'working', tool: 'run', harness: 'Claude', since: '6 min', doing: 'Running pnpm build', look: 11, where: 'here', branch: 'fix/checkout-total' }
    : { id: 'checkout', title: 'Fix the checkout total', phase: 'needs_you', harness: 'Claude', since: '48 sec', doing: 'Waiting for your answer in the terminal', look: 11, where: 'here', branch: 'fix/checkout-total' }

  // Four terminals at once need four people who are in the office.
  const docs: Seat =
    screen === 'split'
      ? { id: 'docs', title: 'Docs pass for the API', phase: 'working', tool: 'read', harness: 'Claude', since: '9 min', doing: 'Reading docs/api/errors.md', look: 5, where: 'here', branch: 'main' }
      : { id: 'docs', title: 'Docs pass for the API', phase: 'working', tool: 'read', harness: 'Claude', since: '9 min', doing: 'In the Claude app', look: 5, where: 'app', branch: 'main' }

  const everyRoom: { repo: string; seats: Seat[] }[] = [
    {
      repo: 'shop',
      seats: [
        checkout,
        { id: 'auth', title: 'Refactor auth middleware', phase: 'working', tool: 'edit', harness: 'Codex', since: '12 min', doing: 'Editing src/middleware/auth.ts', look: 7, where: 'here', branch: 'refactor/auth' },
        { id: 'flaky', title: 'Flaky test hunt', phase: 'asleep', harness: 'Claude', since: '26 hr', doing: 'Stopped. Open their terminal to carry on.', look: 23, where: 'here', branch: 'main' }
      ]
    },
    {
      repo: 'api',
      seats: [
        { id: 'orders', title: 'Type the orders API', phase: 'done', harness: 'Codex', since: '3 min', doing: 'Finished. Open their terminal to see what they did.', look: 42, where: 'here', branch: 'types/orders' },
        docs,
        { id: 'deps', title: 'Bump dependencies', phase: 'idle', harness: 'Codex', since: '4 min', doing: 'In a terminal outside the office', look: 31, where: 'outside', branch: 'main' }
      ]
    }
  ]

  // The room with the open desk comes first, so the picture shows it.
  const rooms = screen === 'elsewhere' ? [...everyRoom].reverse() : everyRoom
  const seat = (id: string) => everyRoom.flatMap(room => room.seats.map(s => ({ ...s, repo: room.repo }))).find(s => s.id === id)!

  const summary = calm ? '3 working, 1 done' : screen === 'split' ? '1 needs you, 2 working, 1 done' : '1 needs you, 2 working, 1 done'
  const count = (n: number) => (n === 1 ? '1 agent' : `${n} agents`)

  /** A terminal line. Text is written as it would be printed; `tone` colours the whole line. */
  interface Line {
    html: string
    tone?: 'you' | 'add' | 'del'
  }
  /** What one program has on its screen: lines, then perhaps a box it drew, then lines. */
  interface Shown {
    lines: Line[]
    box?: Line[]
    /** A dim line inside a box of its own: where the next message is typed. */
    input?: string
  }
  const bullet = (kind: 'say' | 'ok' | 'wait') => `<i class="bullet ${kind}"></i>`
  const blank: Line = { html: '&nbsp;' }
  const question: Line[] = [
    { html: '<b class="cyan">Bash command</b>' },
    blank,
    { html: '  rm -rf dist &amp;&amp; pnpm build' },
    { html: '  <span class="dim">Clear the old build, then rebuild</span>' },
    blank,
    { html: 'Do you want to proceed?' },
    { html: '<span class="cyan">&gt; 1. Yes</span>' },
    { html: "  2. Yes, and don't ask again for pnpm build" },
    { html: '  3. No, and tell Claude what to do differently <span class="dim">(esc)</span>' }
  ]

  /** The one terminal that has room for the whole story. */
  const long: Shown = {
    lines: [
      { html: '<span class="dim">&gt;</span> The checkout total is wrong when a coupon is applied twice. Fix it and make sure the build still passes.', tone: 'you' },
      blank,
      { html: `${bullet('say')} I'll look at how the total is worked out.` },
      blank,
      { html: `${bullet('ok')} <b>Read</b>(src/cart/total.ts)` },
      { html: '  <span class="dim">└  Read 48 lines</span>' },
      blank,
      { html: `${bullet('ok')} <b>Update</b>(src/cart/total.ts)` },
      { html: '  <span class="dim">└  Updated src/cart/total.ts with 1 addition and 2 removals</span>' },
      { html: '       <span class="dim">12</span>    export function total(lines: Line[], coupon?: Coupon) {' },
      { html: '       <span class="dim">13</span> -    const once = applyCoupon(lines, coupon)', tone: 'del' },
      { html: '       <span class="dim">14</span> -    return applyCoupon(once, coupon)', tone: 'del' },
      { html: '       <span class="dim">13</span> +    return applyCoupon(lines, coupon)', tone: 'add' },
      { html: '       <span class="dim">15</span>    }' },
      blank,
      { html: `${bullet('say')} The coupon was being applied twice. Rebuilding to check nothing else broke.` },
      blank,
      { html: `${bullet('wait')} <b>Bash</b>(rm -rf dist &amp;&amp; pnpm build)` },
      { html: '  <span class="dim">└  Waiting for permission</span>' },
      blank
    ],
    box: question
  }

  /** Four at once: each pane shows the end of its story. */
  const panes: Record<string, Shown> = {
    checkout: {
      lines: [
        { html: `${bullet('say')} The coupon was being applied twice. Rebuilding to check.` },
        blank,
        { html: `${bullet('wait')} <b>Bash</b>(rm -rf dist &amp;&amp; pnpm build)` },
        { html: '  <span class="dim">└  Waiting for permission</span>' },
        blank
      ],
      box: question
    },
    auth: {
      lines: [
        { html: '<span class="dim">&gt;</span> Move the session checks out of the route handlers and into', tone: 'you' },
        { html: '  middleware. Keep behaviour the same.', tone: 'you' },
        blank,
        { html: '• <b>Explored</b>' },
        { html: '  <span class="dim">└ Read src/middleware/auth.ts, src/routes/orders.ts</span>' },
        blank,
        { html: '• <b>Edited</b> src/middleware/auth.ts <span class="dim">(+18 -4)</span>' },
        { html: '    <span class="dim">21</span> +  export function requireSession(req, res, next) {', tone: 'add' },
        { html: '    <span class="dim">22</span> +    const session = readSession(req)', tone: 'add' },
        { html: '    <span class="dim">23</span> +    if (!session) return res.status(401).end()', tone: 'add' },
        { html: '    <span class="dim">24</span> +    next()', tone: 'add' },
        blank,
        { html: '• <b>Running</b> pnpm test auth' },
        blank,
        { html: '<span class="cyan">  Working</span> <span class="dim">(2m 14s · esc to interrupt)</span>' }
      ]
    },
    docs: {
      lines: [
        { html: '<span class="dim">&gt;</span> Go through docs/ and fix anything the orders change made', tone: 'you' },
        { html: '  untrue.', tone: 'you' },
        blank,
        { html: `${bullet('ok')} <b>Read</b>(docs/orders.md)` },
        { html: '  <span class="dim">└  Read 212 lines</span>' },
        blank,
        { html: `${bullet('ok')} <b>Search</b>(pattern: "applyCoupon", path: "docs")` },
        { html: '  <span class="dim">└  Found 3 files</span>' },
        blank,
        { html: `${bullet('ok')} <b>Update</b>(docs/checkout.md)` },
        { html: '  <span class="dim">└  Updated docs/checkout.md with 4 additions and 6 removals</span>' },
        blank,
        { html: '<span class="cyan">* Reading docs/api/errors.md</span> <span class="dim">(41s · esc to interrupt)</span>' }
      ]
    },
    orders: {
      lines: [
        { html: '• <b>Ran</b> pnpm typecheck' },
        { html: '  <span class="dim">└ No errors</span>' },
        blank,
        { html: '• <b>Ran</b> pnpm test orders' },
        { html: '  <span class="dim">└ 31 passed</span>' },
        blank,
        { html: '• The orders API is typed end to end.' },
        { html: '  - OrderInput and OrderResult live in src/types/orders.ts' },
        { html: '  - the handlers in src/routes/orders.ts use them' },
        { html: '  - 4 files changed, tests pass' },
        blank
      ],
      input: 'Ask for follow-up changes'
    }
  }
</script>

{#snippet outside()}
  <svg class="out" viewBox="0 0 12 12" aria-hidden="true" focusable="false">
    <path d="M5 2.5H3A1.5 1.5 0 0 0 1.5 4v5A1.5 1.5 0 0 0 3 10.5h5A1.5 1.5 0 0 0 9.5 9V7M7 1.5h3.5V5M10.5 1.5 5.5 6.5" />
  </svg>
{/snippet}

{#snippet closeMark()}
  <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 3.5l9 9M12.5 3.5l-9 9" /></svg>
{/snippet}

{#snippet desk(s: Seat)}
  <button type="button" class="desk {s.phase}" class:open={openIds.includes(s.id)}>
    <span class="scene"><Person look={s.look} phase={s.phase} tool={s.tool ?? ''} /></span>
    <span class="plate">
      <span class="name">{s.title}</span>
      <span class="status">
        <span class="chip"><StatusMark phase={s.phase} />{WORD[s.phase]}</span>
        <span class="since">{s.since}</span>
        <span class="tag">{s.harness}</span>
      </span>
      <span class="doing">
        {#if s.where !== 'here'}{@render outside()}{/if}{s.doing}
      </span>
    </span>
  </button>
{/snippet}

{#snippet floor()}
  <main class="floor">
    {#each rooms as room (room.repo)}
      <section class="room" style:--desks={Math.min(room.seats.length + 1, 5)}>
        <header class="sign">
          <h2>{room.repo}</h2>
          <span>{count(room.seats.length)}</span>
        </header>
        <div class="desks">
          {#each room.seats as s (s.id)}
            {@render desk(s)}
          {/each}
          <SpareDesk label="Add an agent" key="spare:{room.repo}" home={false} onchoose={() => {}} />
        </div>
      </section>
    {/each}
  </main>
{/snippet}

<!-- One program's terminal. `typing` marks the one the keyboard goes to. -->
{#snippet term(id: string, shown: Shown, pane: boolean, typing: boolean)}
  {@const s = seat(id)}
  <section class="term" class:pane class:typing aria-label="{s.title}, in {s.harness === 'Claude' ? 'Claude Code' : 'Codex'}">
    <header>
      <span class="face small"><Person look={s.look} phase={s.phase} tool={s.tool ?? ''} portrait /></span>
      <div class="title">
        <h2>{s.title}</h2>
        <p class="status">
          <span class="chip {s.phase}"><StatusMark phase={s.phase} />{WORD[s.phase]}</span>
          <span>{s.since}</span>
          {#if !pane}<span class="meta">{s.repo} · {s.branch}</span>{/if}
        </p>
      </div>
      <span class="tag">{s.harness === 'Claude' ? 'Claude Code' : 'Codex'}</span>
      <button type="button" class="tool" aria-label="More for this agent">
        <svg viewBox="0 0 16 16" aria-hidden="true"><circle class="solid" cx="3" cy="8" r="1.2" /><circle class="solid" cx="8" cy="8" r="1.2" /><circle class="solid" cx="13" cy="8" r="1.2" /></svg>
      </button>
      <button type="button" class="tool" aria-label="Put this terminal away">{@render closeMark()}</button>
    </header>
    <div class="screen">
      {#each shown.lines as line, i (i)}<div class="ln {line.tone ?? ''}">{@html line.html}</div>{/each}
      {#if shown.box}
        <div class="prompt">
          {#each shown.box as line, i (i)}<div class="ln">{@html line.html}</div>{/each}
        </div>
      {/if}
      {#if shown.input}
        <div class="prompt input"><span class="dim">&gt; {shown.input}</span></div>
      {/if}
      {#if typing}<div class="ln"><span class="caret"></span></div>{/if}
    </div>
  </section>
{/snippet}

<div class="app" class:wide={screen === 'terminal' || screen === 'split'} class:side={screen === 'side'}>
  <header class="bar">
    <div class="brand">
      <svg viewBox="0 0 64 64" aria-hidden="true" focusable="false">
        <rect x="2" y="2" width="60" height="60" rx="14" fill="#15302e" />
        <rect x="19" y="27" width="26" height="18" rx="9" fill="#4c9be8" />
        <circle cx="32" cy="23" r="10.5" fill="#f2c9a1" />
        <path d="M21.5 23a10.5 10.5 0 0 1 21 0c-2.6-4.2-6.1-6.4-10.5-6.4s-7.9 2.2-10.5 6.4z" fill="#2b2b3a" />
        <rect x="10" y="38" width="44" height="15" rx="5" fill="#d0a06e" />
      </svg>
      <h1>Agent Moshpit</h1>
    </div>
    <p class="summary">{summary}</p>
    {#if terminals}
      <button type="button" class="button quiet back">
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M2.5 3.5h4v4h-4zM9.5 3.5h4v4h-4zM2.5 9.5h4v3h-4zM9.5 9.5h4v3h-4z" /></svg>
        Back to the floor
        <span class="keys">Ctrl + `</span>
      </button>
    {/if}
    <button type="button" class="button">New agent</button>
    <button type="button" class="more" aria-label="More">
      <svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="3" cy="8" r="1.4" /><circle cx="8" cy="8" r="1.4" /><circle cx="13" cy="8" r="1.4" /></svg>
    </button>
  </header>

  {#if screen === 'floor'}
    <section class="band" aria-label="Needs you">
      <div class="who">
        <span class="face"><Person look={checkout.look} phase="needs_you" portrait /></span>
        <div class="names">
          <h2>{checkout.title}</h2>
          <p>shop · fix/checkout-total</p>
        </div>
      </div>
      <div class="ask">
        <div class="said">
          <p class="lead">Claude Code is waiting for your answer</p>
          <p>They asked 48 seconds ago. The question is on their screen.</p>
        </div>
        <button type="button" class="button answer">Open their terminal</button>
      </div>
    </section>
  {/if}

  <div class="work">
    {@render floor()}

    {#if screen === 'terminal' || screen === 'side'}
      {@render term('checkout', long, false, true)}
    {:else if screen === 'split'}
      <div class="panes">
        {#each openIds as id, i (id)}
          {@render term(id, panes[id]!, true, i === 0)}
        {/each}
      </div>
    {:else if screen === 'new'}
      <aside class="panel" aria-label="New agent">
        <header class="plain">
          <h2 class="big">New agent</h2>
          <button type="button" class="tool" aria-label="Close">{@render closeMark()}</button>
        </header>
        <form>
          <div>
            <span class="label">Who should take it?</span>
            <div class="pick" role="radiogroup" aria-label="Who should take it?">
              <label class="on"><input type="radio" name="who" checked />Claude Code</label>
              <label><input type="radio" name="who" />Codex</label>
            </div>
            <p class="hint">Your own Claude Code: its sign-in, settings, model and tools. Nothing is set up again here.</p>
          </div>
          <div>
            <label class="label" for="task">What should they do?</label>
            <textarea id="task" class="field" rows="3">Add rate limiting to the orders endpoint and cover it with tests.</textarea>
          </div>
          <div>
            <label class="label" for="cwd">Folder</label>
            <div class="pair">
              <input id="cwd" class="field" value="C:\Users\Yash\Documents\api" />
              <button type="button" class="button quiet">Browse</button>
            </div>
            <div class="recent">
              <button type="button" class="folder">shop</button>
              <button type="button" class="folder">web</button>
              <button type="button" class="folder">agent-moshpit</button>
            </div>
          </div>
          <div>
            <label class="check"><input type="checkbox" checked />Work on a separate copy</label>
            <p class="hint">Claude Code makes a git worktree on a new branch, so several agents can change one project without colliding.</p>
          </div>
          <div>
            <label class="label" for="name">Name <span class="optional">optional</span></label>
            <input id="name" class="field" placeholder="Taken from the task if you leave it empty" />
          </div>
          <p class="heads-up">They get a desk here, and their terminal opens beside the floor.</p>
          <div class="end">
            <button type="button" class="button">Start agent</button>
            <button type="button" class="button quiet">Cancel</button>
          </div>
        </form>
      </aside>
    {:else if screen === 'elsewhere'}
      <aside class="panel" aria-label="Bump dependencies">
        <header>
          <span class="face"><Person look={31} phase="idle" portrait /></span>
          <div class="title">
            <h2>Bump dependencies</h2>
            <p class="status">
              <span class="chip"><StatusMark phase="idle" />Idle</span>
              <span>4 min</span>
            </p>
            <p class="meta">api · main</p>
            <p class="meta">Codex</p>
          </div>
          <button type="button" class="tool" aria-label="Close">{@render closeMark()}</button>
        </header>
        <div class="away">
          <h3>This desk is in a terminal outside the office</h3>
          <p>
            This Codex session was started in Windows Terminal, so its screen belongs to that window. The office can see how
            it is doing, but cannot show its terminal while it lives there.
          </p>
          <div class="act">
            <button type="button" class="button">Bring them into the office</button>
            <p class="hint">
              Carries on the same conversation in a terminal here. Close it in the other window first; if it is still open
              there, you get a copy.
            </p>
          </div>
          <dl>
            <dt>Folder</dt>
            <dd class="code">C:\Users\Yash\Documents\api</dd>
            <dt>Started</dt>
            <dd>Today, 2:14 pm</dd>
            <dt>Session</dt>
            <dd class="code">0a1b2c3d-4e5f-6071-8293</dd>
          </dl>
        </div>
      </aside>
    {/if}
  </div>
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
    background: var(--carpet-deep);
    /* The terminal is a lit screen in the room: dark by day and by night. */
    --term: #0d2221;
    --term-ink: #dbe8e1;
    --term-dim: #8aa79e;
    --term-line: #2b4b46;
    --term-cyan: #8fe6f0;
    --term-add: #17463f;
    --term-del: #43341f;
  }
  @media (prefers-color-scheme: dark) {
    .app {
      --term: #081716;
      --term-line: #21403b;
    }
  }
  .app > * {
    flex: none;
    min-width: 0;
  }
  .work {
    display: grid;
    flex: 1 1 0;
    grid-template-columns: minmax(0, 1fr) auto;
    min-height: 0;
  }
  .wide .work {
    grid-template-columns: var(--strip) minmax(0, 1fr);
  }
  .wide .floor {
    gap: var(--s-2);
    padding: var(--s-2);
  }
  /* The floor keeps two desks to a row and the terminal takes the rest. */
  .side .work {
    grid-template-columns: 600px minmax(0, 1fr);
  }

  /* ── the bar ── */
  .bar {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    height: var(--bar);
    padding: 0 var(--s-3) 0 var(--s-5);
    border-bottom: 1px solid var(--line);
    background: var(--chrome);
  }
  .brand {
    display: flex;
    flex: none;
    align-items: center;
    gap: var(--s-2);
  }
  .brand svg {
    width: 24px;
    height: 24px;
  }
  h1 {
    font-size: var(--t-lg);
    font-weight: 800;
    letter-spacing: -0.01em;
    white-space: nowrap;
  }
  .summary {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font-size: var(--t-sm);
    color: var(--ink-2);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .more {
    display: grid;
    width: 32px;
    height: 32px;
    place-items: center;
    border: 0;
    border-radius: var(--r-1);
    background: transparent;
    color: var(--ink-2);
  }
  .more svg {
    width: 16px;
    height: 16px;
    fill: currentColor;
  }
  .back svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
    stroke-linejoin: round;
  }
  .keys {
    font-weight: 400;
    color: var(--ink-2);
  }

  /* ── the band ── */
  .band {
    display: grid;
    grid-template-columns: minmax(180px, 250px) minmax(0, 1fr);
    gap: var(--s-3) var(--s-5);
    align-items: center;
    padding: var(--s-3) var(--s-5) var(--s-4);
    background: var(--needs);
    color: var(--needs-ink);
    --focus: var(--needs-ink);
  }
  .who {
    display: flex;
    gap: var(--s-3);
    align-items: center;
    min-width: 0;
  }
  .band .face {
    width: 56px;
    background: rgba(42, 31, 0, 0.12);
  }
  .names {
    display: grid;
    gap: 2px;
    min-width: 0;
  }
  .band h2 {
    overflow: hidden;
    font-size: var(--t-lg);
    font-weight: 800;
    line-height: 1.25;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .names p,
  .said p:not(.lead) {
    font-size: var(--t-sm);
    color: #5a4300;
  }
  .ask {
    display: flex;
    align-items: center;
    gap: var(--s-4);
    min-width: 0;
  }
  .said {
    display: grid;
    flex: 1;
    gap: 2px;
    min-width: 0;
  }
  .lead {
    font-size: var(--t-lg);
    font-weight: 650;
    line-height: 1.25;
  }
  .answer {
    background: var(--needs-ink);
    color: #ffe9a8;
  }

  /* ── the floor ── */
  .floor {
    display: flex;
    flex-wrap: wrap;
    align-content: flex-start;
    align-items: flex-start;
    gap: var(--s-5);
    min-width: 0;
    min-height: 0;
    padding: var(--s-5);
    overflow: auto;
    background: var(--carpet-deep);
    container: floor / inline-size;
  }
  .side .floor {
    gap: var(--s-4);
    padding: var(--s-4);
  }
  .room {
    flex: 1 1 calc(var(--desks, 2) * 204px + 2 * var(--s-4));
    min-width: 0;
    max-width: 100%;
    padding: var(--s-3) var(--s-4) var(--s-4);
    border: 3px solid var(--wall);
    border-radius: var(--r-3);
    background: repeating-conic-gradient(var(--carpet) 0 25%, var(--carpet-line) 0 50%) 0 0 / 72px 72px;
  }
  .sign {
    display: flex;
    align-items: baseline;
    gap: var(--s-3);
    margin-bottom: var(--s-2);
    padding: 0 var(--s-2);
  }
  .sign h2 {
    overflow: hidden;
    font-size: var(--t-lg);
    font-weight: 750;
    line-height: 1.3;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sign span {
    flex: none;
    font-size: var(--t-sm);
    color: var(--ink-2);
  }
  .desks {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(184px, 1fr));
    gap: var(--s-2) var(--s-3);
  }

  .desk {
    display: grid;
    grid-template-rows: auto auto;
    align-content: start;
    align-self: start;
    gap: var(--s-1);
    width: 100%;
    padding: var(--s-2) var(--s-2) var(--s-3);
    border: 0;
    border-radius: var(--r-2);
    background: transparent;
    text-align: left;
  }
  .desk.open {
    background: var(--carpet-deep);
  }
  .scene {
    display: block;
    padding: 0 var(--s-1);
  }
  .plate {
    display: grid;
    gap: 3px;
    min-width: 0;
    padding: 0 var(--s-1);
  }
  .name {
    overflow: hidden;
    font-size: var(--t-md);
    font-weight: 700;
    line-height: 1.25;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .status {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    min-width: 0;
    font-size: var(--t-xs);
    color: var(--ink-2);
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--ink);
    white-space: nowrap;
  }
  .desk.needs_you .chip {
    margin-left: -6px;
  }
  .desk.needs_you .chip,
  .chip.needs_you {
    padding: 1px 7px 1px 6px;
    border-radius: 999px;
    background: var(--needs);
    color: var(--needs-ink);
  }
  .since {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* Which program this person is: a word, never a colour. */
  .tag {
    flex: none;
    padding: 0 7px;
    border: 1px solid var(--field-line);
    border-radius: 999px;
    font-size: var(--t-xs);
    font-weight: 650;
    line-height: 1.5;
    color: var(--ink-2);
    white-space: nowrap;
  }
  .status .tag {
    margin-left: auto;
  }
  .doing {
    display: -webkit-box;
    overflow: hidden;
    font-size: var(--t-sm);
    line-height: 1.3;
    color: var(--ink-2);
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow-wrap: anywhere;
  }
  .out {
    display: inline-block;
    width: 12px;
    height: 12px;
    margin-right: 5px;
    vertical-align: -1px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  @container floor (max-width: 560px) {
    .room {
      flex-basis: 100%;
      padding: var(--s-2) var(--s-2) var(--s-3);
    }
    .desks {
      grid-template-columns: minmax(0, 1fr);
      gap: 0;
    }
    .desk {
      grid-template-columns: 116px minmax(0, 1fr);
      grid-template-rows: auto;
      align-items: center;
      gap: var(--s-2);
      padding: var(--s-2);
    }
    .scene {
      padding: 0;
    }
  }
  @container floor (max-width: 300px) {
    .room {
      padding: var(--s-1) var(--s-1) var(--s-2);
      border-radius: var(--r-2);
    }
    .sign {
      margin-bottom: 0;
      padding: var(--s-1) var(--s-2) 0;
    }
    .sign h2 {
      font-size: var(--t-md);
    }
    .desk {
      grid-template-columns: 76px minmax(0, 1fr);
      padding: var(--s-1) var(--s-2);
    }
    .doing,
    .desk .tag {
      display: none;
    }
  }

  /* ── what a room's header shares, in the side panel and above a terminal ── */
  .face {
    flex: none;
    width: 52px;
    border-radius: 50%;
    background: var(--inset);
    overflow: hidden;
  }
  .face.small {
    width: 36px;
  }
  .title {
    display: grid;
    flex: 1;
    gap: 3px;
    min-width: 0;
  }
  .title h2 {
    overflow: hidden;
    font-size: var(--t-lg);
    font-weight: 750;
    line-height: 1.25;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    overflow: hidden;
    font-size: var(--t-sm);
    color: var(--ink-2);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tool {
    display: grid;
    flex: none;
    width: 30px;
    height: 30px;
    place-items: center;
    border: 0;
    border-radius: var(--r-1);
    background: transparent;
    color: var(--ink-2);
  }
  .tool svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
  }
  .tool .solid {
    fill: currentColor;
    stroke: none;
  }

  /* ── terminals ── */
  .term {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    min-width: 0;
    min-height: 0;
    border-left: 1px solid var(--line);
    background: var(--term);
  }
  .term header {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    padding: var(--s-2) var(--s-2) var(--s-2) var(--s-4);
    /* The terminal the keyboard goes to wears a line of ink along its top. */
    border-top: 2px solid transparent;
    border-bottom: 1px solid var(--line);
    background: var(--panel);
  }
  .term header .title {
    margin-left: var(--s-1);
  }
  .term header .tag {
    margin-right: var(--s-1);
  }
  .term .status .meta {
    padding-left: var(--s-2);
    border-left: 1px solid var(--line);
    font-size: var(--t-xs);
  }
  /* Several at once: a grid of them, parted by one line. */
  .panes {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    grid-template-rows: repeat(2, minmax(0, 1fr));
    gap: 1px;
    min-width: 0;
    min-height: 0;
    border-left: 1px solid var(--line);
    background: var(--line);
  }
  .term.pane {
    border-left: 0;
  }
  .pane.typing header {
    border-top-color: var(--ink);
  }
  .pane header {
    padding-left: var(--s-3);
  }
  .pane .face.small {
    width: 32px;
  }
  .pane .title h2 {
    font-size: var(--t-md);
  }
  /* What the program itself prints. The office draws none of it. */
  .screen {
    padding: var(--s-3) var(--s-4);
    overflow: hidden;
    font-family: var(--mono);
    font-size: var(--t-sm);
    line-height: 1.5;
    color: var(--term-ink);
    white-space: pre;
  }
  .pane .screen {
    padding: var(--s-3);
    font-size: var(--t-xs);
  }
  .ln.you {
    /* A terminal wraps what does not fit; the rest of these lines are short enough. */
    white-space: pre-wrap;
    margin: 0 calc(-1 * var(--s-2));
    padding: 2px var(--s-2);
    background: rgba(255, 255, 255, 0.07);
  }
  .ln.add {
    background: var(--term-add);
  }
  .ln.del {
    background: var(--term-del);
  }
  .screen :global(.dim) {
    color: var(--term-dim);
  }
  .screen :global(.cyan) {
    color: var(--term-cyan);
  }
  .screen :global(b) {
    font-weight: 700;
  }
  .screen :global(.bullet) {
    display: inline-block;
    width: 0.55em;
    height: 0.55em;
    margin: 0 0.45em 0.08em 0;
    border-radius: 50%;
    background: var(--term-ink);
  }
  .screen :global(.bullet.ok) {
    background: #7fd69a;
  }
  .screen :global(.bullet.wait) {
    background: var(--term-dim);
  }
  .prompt {
    width: fit-content;
    min-width: min(100%, 78ch);
    padding: var(--s-2) var(--s-3);
    border: 1px solid var(--term-cyan);
    border-radius: 6px;
  }
  .pane .prompt {
    min-width: 100%;
  }
  .prompt.input {
    border-color: var(--term-line);
  }
  .caret {
    display: inline-block;
    width: 0.6em;
    height: 1.1em;
    margin-top: var(--s-2);
    background: var(--term-ink);
    vertical-align: text-bottom;
  }

  /* ── the side panel ── */
  .panel {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    width: var(--panel-width);
    min-width: 0;
    min-height: 0;
    border-left: 1px solid var(--line);
    background: var(--panel);
  }
  .panel header {
    display: flex;
    gap: var(--s-3);
    align-items: flex-start;
    padding: var(--s-4) var(--s-4) var(--s-3);
    border-bottom: 1px solid var(--line);
  }
  .panel header.plain {
    align-items: center;
    justify-content: space-between;
  }
  .big {
    font-size: var(--t-xl);
    font-weight: 750;
    line-height: 1.2;
  }
  form {
    display: grid;
    align-content: start;
    gap: var(--s-4);
    padding: var(--s-4);
    overflow-y: auto;
  }
  .pair {
    display: flex;
    gap: var(--s-2);
  }
  /* One choice of two, held in a single control. */
  .pick {
    display: grid;
    grid-template-columns: 1fr 1fr;
    padding: 3px;
    border: 1px solid var(--field-line);
    border-radius: var(--r-1);
    background: var(--inset);
  }
  .pick label {
    display: grid;
    height: 28px;
    place-items: center;
    border-radius: 4px;
    font-size: var(--t-sm);
    font-weight: 650;
    color: var(--ink-2);
  }
  .pick label.on {
    background: var(--button);
    color: var(--button-ink);
  }
  .pick input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .check {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    font-size: var(--t-md);
    font-weight: 650;
  }
  .check input {
    width: 16px;
    height: 16px;
    margin: 0;
    accent-color: var(--button);
  }
  .optional {
    font-weight: 400;
    color: var(--ink-2);
  }
  .recent {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s-1);
    margin-top: var(--s-2);
  }
  .folder {
    padding: 2px var(--s-3);
    border: 1px solid var(--line);
    border-radius: 999px;
    background: transparent;
    font-size: var(--t-sm);
  }
  .heads-up {
    font-size: var(--t-sm);
    line-height: 1.45;
    color: var(--ink-2);
  }
  .end {
    display: flex;
    gap: var(--s-2);
    padding-top: var(--s-1);
  }

  /* ── a desk that lives somewhere else ── */
  .away {
    display: grid;
    align-content: start;
    gap: var(--s-4);
    padding: var(--s-4);
    overflow-y: auto;
    line-height: 1.5;
  }
  .away h3 {
    margin: 0;
    font-size: var(--t-lg);
    font-weight: 750;
    line-height: 1.25;
  }
  .away > p {
    color: var(--ink-2);
  }
  .act {
    display: grid;
    gap: var(--s-2);
    justify-items: start;
  }
  .act .hint {
    margin: 0;
    font-size: var(--t-sm);
  }
  dl {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: var(--s-2) var(--s-4);
    margin: 0;
    padding-top: var(--s-4);
    border-top: 1px solid var(--line);
    font-size: var(--t-sm);
  }
  dt {
    color: var(--ink-2);
  }
  dd {
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>

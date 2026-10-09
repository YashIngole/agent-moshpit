<script lang="ts">
  // The same office, after hours: the floor, desks and people as before, drawn
  // for a room of developers rather than a nursery. Pictures to judge, not the app.
  import '@fontsource-variable/geist'
  import '@fontsource-variable/geist-mono'
  import type { Dev } from './crew'
  import DevDesk from './DevDesk.svelte'

  type Phase = 'starting' | 'working' | 'needs_you' | 'done' | 'idle' | 'failed' | 'asleep'
  interface Seat {
    id: string
    title: string
    phase: Phase
    tool?: string
    program: string
    since: string
    doing: string
    dev: Dev
    branch: string
  }

  let { screen }: { screen: 'floor' | 'split' | 'cast' } = $props()

  const base = { ink: '#1d1b22', glasses: 'none', beard: 'none', dual: false } as const
  const crew: Record<string, Dev> = {
    maya: { ...base, skin: '#a8734e', hair: '#17171c', hairStyle: 'long', top: 'hoodie', cloth: '#16181d', hat: '#121418', headwear: 'headphones', item: 'can', dual: true },
    theo: { ...base, skin: '#f3cfae', hair: '#5a3c28', hairStyle: 'messy', top: 'tee', cloth: '#1d2a45', hat: '#121418', headwear: 'none', glasses: 'square', item: 'mug' },
    ren: { ...base, skin: '#e6b48c', hair: '#8a62e0', hairStyle: 'swept', top: 'hoodie', cloth: '#3a4049', hat: '#121418', headwear: 'none', item: 'lamp' },
    kofi: { ...base, skin: '#5f3b28', ink: '#140c08', hair: '#17171c', hairStyle: 'buzz', top: 'jacket', cloth: '#272b33', hat: '#2b2f37', headwear: 'cap', beard: 'full', item: 'plant', dual: true },
    ines: { ...base, skin: '#d09a6c', hair: '#2e221b', hairStyle: 'curly', top: 'hoodie', cloth: '#3f325f', hat: '#121418', headwear: 'beanie', glasses: 'round', item: 'cans' },
    sam: { ...base, skin: '#80523a', ink: '#140c08', hair: '#d9d4ca', hairStyle: 'bun', top: 'tee', cloth: '#dcd7cb', hat: '#121418', headwear: 'none', item: 'mug' },
    jun: { ...base, skin: '#f3cfae', hair: '#17171c', hairStyle: 'swept', top: 'hoodie', cloth: '#2f4b72', hat: '#121418', headwear: 'none', glasses: 'square', beard: 'stubble', item: 'can' },
    ola: { ...base, skin: '#e6b48c', hair: '#7d4a2b', hairStyle: 'buzz', top: 'hoodie', cloth: '#272b33', hat: '#3f325f', headwear: 'beanie', item: 'lamp' }
  }

  const rooms: { name: string; seats: Seat[] }[] = [
    {
      name: 'shop',
      seats: [
        { id: 'a', title: 'Fix the checkout total', phase: 'needs_you', program: 'claude', since: '48s', doing: 'Waiting on you: rm -rf dist && pnpm build', dev: crew.maya!, branch: 'fix/checkout-total' },
        { id: 'b', title: 'Refactor auth middleware', phase: 'working', tool: 'edit', program: 'codex', since: '12m', doing: 'Editing src/middleware/auth.ts', dev: crew.theo!, branch: 'refactor/auth' },
        { id: 'c', title: 'Flaky test hunt', phase: 'asleep', program: 'claude', since: '26h', doing: 'Not running. Open to carry on.', dev: crew.ines!, branch: 'main' }
      ]
    },
    {
      name: 'api',
      seats: [
        { id: 'd', title: 'Type the orders API', phase: 'done', program: 'codex', since: '3m', doing: 'Finished. Open to see what changed.', dev: crew.kofi!, branch: 'types/orders' },
        { id: 'e', title: 'Docs pass for the API', phase: 'working', tool: 'read', program: 'gemini', since: '9m', doing: 'Reading docs/api/errors.md', dev: crew.ren!, branch: 'main' },
        { id: 'f', title: 'Seed the staging data', phase: 'failed', program: 'claude', since: '2m', doing: 'Stopped with an error on start', dev: crew.sam!, branch: 'main' }
      ]
    }
  ]

  const WORD: Record<Phase, string> = { starting: 'starting', working: 'working', needs_you: 'needs you', done: 'done', idle: 'idle', failed: 'trouble', asleep: 'away' }
  const open = screen === 'split' ? ['a', 'b', 'd', 'e'] : []
  const seat = (id: string) => rooms.flatMap(r => r.seats.map(s => ({ ...s, repo: r.name }))).find(s => s.id === id)!

  const terminal: Record<string, string[]> = {
    a: [
      '<i>›</i> The checkout total is wrong when a coupon is applied twice.',
      '',
      '<g>●</g> <b>Update</b>(src/cart/total.ts)',
      '  <i>└ 1 addition, 2 removals</i>',
      '<del>   13 -   const once = applyCoupon(lines, coupon)</del>',
      '<add>   13 +   return applyCoupon(lines, coupon)</add>',
      '',
      '<i>●</i> <b>Bash</b>(rm -rf dist &amp;&amp; pnpm build)',
      '',
      '<box>Do you want to proceed?</box>',
      '<box><c>› 1. Yes</c></box>',
      '<box>  2. No, and tell Claude what to do</box>'
    ],
    b: ['<i>›</i> Move the session checks into middleware.', '', '<b>• Explored</b>', '  <i>└ Read auth.ts, orders.ts</i>', '', '<b>• Edited</b> src/middleware/auth.ts <i>(+18 -4)</i>', '<add>   21 +  export function requireSession(req, res, next) {</add>', '<add>   22 +    const session = readSession(req)</add>', '', '<c>  Working</c> <i>(2m 14s · esc to interrupt)</i>'],
    d: ['<b>• Ran</b> pnpm typecheck', '  <i>└ No errors</i>', '', '<b>• Ran</b> pnpm test orders', '  <i>└ 31 passed</i>', '', '• The orders API is typed end to end.', '  - 4 files changed, tests pass', '', '<i>› Ask for follow-up changes</i>'],
    e: ['<i>›</i> Fix anything the orders change made untrue.', '', '<g>●</g> <b>Read</b>(docs/orders.md)', '  <i>└ 212 lines</i>', '', '<g>●</g> <b>Search</b>("applyCoupon", docs)', '  <i>└ 3 files</i>', '', '<c>* Reading docs/api/errors.md</c> <i>(41s)</i>']
  }

  const castLine: { dev: Dev; phase: Phase; tool?: string; label: string }[] = [
    { dev: crew.theo!, phase: 'working', tool: 'edit', label: 'working · typing' },
    { dev: crew.ren!, phase: 'working', tool: 'read', label: 'working · reading' },
    { dev: crew.jun!, phase: 'working', tool: 'think', label: 'working · thinking' },
    { dev: crew.maya!, phase: 'needs_you', label: 'needs you' },
    { dev: crew.kofi!, phase: 'done', label: 'done' },
    { dev: crew.sam!, phase: 'failed', label: 'trouble' },
    { dev: crew.ola!, phase: 'idle', label: 'idle' },
    { dev: crew.ines!, phase: 'asleep', label: 'away' }
  ]
</script>

{#snippet mark()}
  <svg class="mark" viewBox="0 0 24 24" aria-hidden="true">
    <rect x="1" y="1" width="22" height="22" rx="6" fill="#1b1f25" stroke="#2c323a" />
    <rect x="6" y="10" width="12" height="9" rx="4" fill="#3a4049" />
    <circle cx="12" cy="8.500" r="4" fill="#d09a6c" />
    <path d="M7.800 8.300a4.200 4.200 0 0 1 8.400 0z" fill="#17171c" />
    <rect x="4" y="15" width="16" height="4" rx="1.500" fill="#2a2f36" />
  </svg>
{/snippet}

{#snippet desk(s: Seat, small = false)}
  <button type="button" class="desk {s.phase}" class:small class:open={open.includes(s.id)}>
    <span class="scene"><DevDesk dev={s.dev} phase={s.phase} tool={s.tool} /></span>
    <span class="plate">
      <span class="name">{s.title}</span>
      <span class="status"><i class="dot"></i><span class="word">{WORD[s.phase]}</span><span class="since">{s.since}</span><span class="tag">{s.program}</span></span>
      {#if !small}<span class="doing">{s.doing}</span>{/if}
    </span>
  </button>
{/snippet}

{#snippet bar()}
  <header class="bar">
    {@render mark()}
    <h1>agent moshpit</h1>
    <p class="summary">
      <span class="n needs"><i class="dot"></i>1 needs you</span>
      <span class="n working"><i class="dot"></i>2 working</span>
      <span class="n done"><i class="dot"></i>1 done</span>
      <span class="n failed"><i class="dot"></i>1 trouble</span>
    </p>
    {#if screen === 'split'}
      <button type="button" class="ghost">back to the floor <kbd>ctrl `</kbd></button>
    {/if}
    <button type="button" class="primary">+ new agent <kbd>n</kbd></button>
  </header>
{/snippet}

<div class="night">
  {#if screen === 'cast'}
    <header class="bar">
      {@render mark()}
      <h1>agent moshpit</h1>
      <p class="summary">the crew, in every state</p>
    </header>
    <main class="cast">
      {#each castLine as one, i (i)}
        <figure>
          <DevDesk dev={one.dev} phase={one.phase} tool={one.tool} />
          <figcaption class={one.phase}><i class="dot"></i>{one.label}</figcaption>
        </figure>
      {/each}
      <div class="faces">
        {#each Object.values(crew) as dev, i (i)}
          <span class="face"><DevDesk {dev} phase="idle" portrait /></span>
        {/each}
      </div>
    </main>
  {:else}
    {@render bar()}
    {#if screen === 'floor'}
      <section class="band">
        <span class="face"><DevDesk dev={crew.maya!} phase="needs_you" portrait /></span>
        <div class="who">
          <p class="kicker">needs you · 48s</p>
          <h2>Fix the checkout total</h2>
        </div>
        <p class="what"><code>claude</code> wants to run <code>rm -rf dist &amp;&amp; pnpm build</code></p>
        <button type="button">open terminal</button>
      </section>
    {/if}
    <div class="work" class:split={screen === 'split'}>
      <main class="floor">
        {#each rooms as room (room.name)}
          <section class="room">
            <header class="sign"><h2>~/{room.name}</h2><span>{room.seats.length} agents</span></header>
            <div class="desks">
              {#each room.seats as s (s.id)}{@render desk(s, screen === 'split')}{/each}
              {#if screen === 'floor'}
                <button type="button" class="spare"><span class="plus">+</span><span>add an agent</span></button>
              {/if}
            </div>
          </section>
        {/each}
      </main>
      {#if screen === 'split'}
        <section class="panes">
          {#each open as id, i (id)}
            {@const s = seat(id)}
            <article class="pane" class:typing={i === 0}>
              <header>
                <span class="face"><DevDesk dev={s.dev} phase={s.phase} portrait /></span>
                <div class="title">
                  <h3>{s.title}</h3>
                  <p class={s.phase}><i class="dot"></i>{WORD[s.phase]} · {s.since} · {s.repo}/{s.branch}</p>
                </div>
                <span class="tag">{s.program}</span>
                <span class="x">×</span>
              </header>
              <div class="term">
                {#each terminal[id] ?? [] as line, n (n)}<div class="ln">{@html line || '&nbsp;'}</div>{/each}
                {#if i === 0}<div class="ln"><span class="caret"></span></div>{/if}
              </div>
            </article>
          {/each}
        </section>
      {/if}
    </div>
  {/if}
</div>

<style>
  .night {
    --bg: #0a0c0f;
    --floor: #111418;
    --grid: rgba(255, 255, 255, 0.028);
    --wall: #252b33;
    --chrome: #0d0f12;
    --panel: #13161a;
    --line: #20252c;
    --ink: #e7eaf0;
    --ink-2: #a0a8b4;
    --ink-3: #6c7684;
    --amber: #ffb224;
    --green: #3ccf91;
    --red: #ff5f56;
    --blue: #82aaff;
    --sans: 'Geist Variable', system-ui, sans-serif;
    --mono: 'Geist Mono Variable', ui-monospace, monospace;
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg);
    color: var(--ink);
    font-family: var(--sans);
    font-feature-settings: 'tnum';
  }
  .dot {
    display: inline-block;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--ink-3);
    flex: none;
  }
  .needs .dot,
  .needs_you .dot {
    background: var(--amber);
  }
  .working .dot {
    background: var(--blue);
  }
  .done .dot {
    background: var(--green);
  }
  .failed .dot {
    background: var(--red);
  }
  .asleep .dot {
    background: transparent;
    box-shadow: inset 0 0 0 1.500px var(--ink-3);
  }
  kbd {
    margin-left: 8px;
    padding: 1px 5px;
    border: 1px solid currentColor;
    border-radius: 4px;
    font-family: var(--mono);
    font-size: 11px;
    opacity: 0.6;
  }

  /* ── the bar ── */
  .bar {
    display: flex;
    align-items: center;
    gap: 14px;
    height: 52px;
    padding: 0 14px 0 18px;
    border-bottom: 1px solid var(--line);
    background: var(--chrome);
  }
  .mark {
    width: 24px;
    height: 24px;
  }
  .bar h1 {
    margin: 0;
    font-family: var(--mono);
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .summary {
    display: flex;
    flex: 1;
    gap: 16px;
    margin: 0 0 0 10px;
    font-family: var(--mono);
    font-size: 12px;
    color: var(--ink-2);
  }
  .summary .n {
    display: inline-flex;
    align-items: center;
    gap: 7px;
  }
  .bar button {
    height: 32px;
    padding: 0 12px;
    border-radius: 7px;
    font-family: var(--mono);
    font-size: 12.500px;
    font-weight: 600;
  }
  .primary {
    border: 0;
    background: var(--ink);
    color: var(--bg);
  }
  .ghost {
    border: 1px solid var(--wall);
    background: transparent;
    color: var(--ink);
  }

  /* ── someone waiting ── */
  .band {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 10px 18px;
    background: var(--amber);
    color: #140e00;
  }
  .band .face {
    width: 44px;
    border-radius: 10px;
    background: rgba(20, 14, 0, 0.14);
    overflow: hidden;
  }
  .kicker {
    margin: 0;
    font-family: var(--mono);
    font-size: 11.500px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    opacity: 0.75;
  }
  .band h2 {
    margin: 2px 0 0;
    font-size: 16px;
    font-weight: 650;
  }
  .what {
    flex: 1;
    margin: 0 0 0 12px;
    font-size: 14px;
  }
  .what code {
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(20, 14, 0, 0.12);
    font-family: var(--mono);
    font-size: 13px;
  }
  .band button {
    height: 34px;
    padding: 0 14px;
    border: 0;
    border-radius: 7px;
    background: #140e00;
    color: var(--amber);
    font-family: var(--mono);
    font-size: 13px;
    font-weight: 600;
  }

  /* ── the floor ── */
  .work {
    display: grid;
    flex: 1;
    grid-template-columns: minmax(0, 1fr);
    min-height: 0;
  }
  .work.split {
    grid-template-columns: 300px minmax(0, 1fr);
  }
  .floor {
    display: flex;
    flex-wrap: wrap;
    align-content: flex-start;
    gap: 20px;
    min-height: 0;
    padding: 20px;
    overflow: hidden;
    container: floor / inline-size;
  }
  .split .floor {
    gap: 12px;
    padding: 12px;
    border-right: 1px solid var(--line);
  }
  .room {
    flex: 1 1 860px;
    padding: 12px 14px 14px;
    border: 1px solid var(--wall);
    border-radius: 12px;
    background:
      linear-gradient(var(--grid) 1px, transparent 1px) 0 0 / 28px 28px,
      linear-gradient(90deg, var(--grid) 1px, transparent 1px) 0 0 / 28px 28px,
      var(--floor);
  }
  .sign {
    display: flex;
    align-items: baseline;
    gap: 12px;
    margin: 0 0 6px;
    padding: 0 6px;
  }
  .sign h2 {
    margin: 0;
    font-family: var(--mono);
    font-size: 14px;
    font-weight: 600;
  }
  .sign span {
    font-family: var(--mono);
    font-size: 12px;
    color: var(--ink-3);
  }
  .desks {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 6px 12px;
  }
  .split .desks {
    grid-template-columns: minmax(0, 1fr);
    gap: 0;
  }
  .desk,
  .spare {
    display: grid;
    gap: 4px;
    align-content: start;
    padding: 8px 8px 12px;
    border: 0;
    border-radius: 10px;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
  }
  .desk.open {
    background: rgba(130, 170, 255, 0.07);
    box-shadow: inset 0 0 0 1px rgba(130, 170, 255, 0.22);
  }
  .desk.small {
    grid-template-columns: 92px minmax(0, 1fr);
    align-items: center;
    gap: 10px;
    padding: 4px 6px;
  }
  .plate {
    display: grid;
    gap: 4px;
    min-width: 0;
    padding: 0 4px;
  }
  .name {
    overflow: hidden;
    font-size: 14px;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .small .name {
    font-size: 13px;
  }
  .small .tag {
    display: none;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 7px;
    font-family: var(--mono);
    font-size: 11.500px;
    color: var(--ink-2);
  }
  .needs_you .word {
    color: var(--amber);
    font-weight: 600;
  }
  .failed .word {
    color: var(--red);
  }
  .since {
    color: var(--ink-3);
  }
  .tag {
    margin-left: auto;
    padding: 0 6px;
    border: 1px solid var(--wall);
    border-radius: 5px;
    font-family: var(--mono);
    font-size: 11px;
    line-height: 1.6;
    color: var(--ink-2);
  }
  .doing {
    overflow: hidden;
    font-size: 12.500px;
    color: var(--ink-3);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .spare {
    place-content: center;
    justify-items: center;
    min-height: 100%;
    border: 1px dashed var(--wall);
    color: var(--ink-3);
    font-family: var(--mono);
    font-size: 12.500px;
  }
  .plus {
    display: grid;
    width: 34px;
    height: 34px;
    place-items: center;
    border: 1px solid var(--wall);
    border-radius: 9px;
    font-size: 18px;
    color: var(--ink-2);
  }

  /* ── terminals ── */
  .panes {
    display: grid;
    grid-template-columns: 1fr 1fr;
    grid-template-rows: 1fr 1fr;
    gap: 1px;
    min-height: 0;
    background: var(--line);
  }
  .pane {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    min-height: 0;
    background: #0b0d10;
  }
  .pane header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 10px;
    border-top: 2px solid transparent;
    border-bottom: 1px solid var(--line);
    background: var(--panel);
  }
  .pane.typing header {
    border-top-color: var(--ink);
  }
  .pane .face {
    width: 30px;
    border-radius: 8px;
    background: #1b1f25;
    overflow: hidden;
  }
  .title {
    flex: 1;
    min-width: 0;
  }
  .title h3 {
    margin: 0;
    overflow: hidden;
    font-size: 13.500px;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .title p {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 2px 0 0;
    font-family: var(--mono);
    font-size: 11px;
    color: var(--ink-3);
  }
  .pane .tag {
    margin-left: 0;
  }
  .x {
    width: 22px;
    color: var(--ink-3);
    font-size: 16px;
    text-align: center;
  }
  .term {
    padding: 10px 12px;
    overflow: hidden;
    font-family: var(--mono);
    font-size: 12px;
    line-height: 1.55;
    color: #d5dae2;
    white-space: pre;
  }
  .term :global(i) {
    font-style: normal;
    color: #6c7684;
  }
  .term :global(b) {
    font-weight: 600;
    color: #e7eaf0;
  }
  .term :global(g) {
    color: var(--green);
  }
  .term :global(c) {
    color: var(--blue);
  }
  .term :global(add) {
    display: block;
    background: rgba(60, 207, 145, 0.12);
  }
  .term :global(del) {
    display: block;
    background: rgba(255, 95, 86, 0.12);
    text-decoration: none;
  }
  .term :global(box) {
    display: block;
    width: fit-content;
    min-width: 92%;
    padding: 0 8px;
    border-right: 1px solid var(--blue);
    border-left: 1px solid var(--blue);
  }
  .caret {
    display: inline-block;
    width: 7px;
    height: 14px;
    background: #e7eaf0;
  }

  /* ── the cast ── */
  .cast {
    flex: 1;
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 18px 22px;
    padding: 26px 30px;
    overflow: hidden;
    background:
      linear-gradient(var(--grid) 1px, transparent 1px) 0 0 / 28px 28px,
      linear-gradient(90deg, var(--grid) 1px, transparent 1px) 0 0 / 28px 28px,
      var(--floor);
  }
  figure {
    margin: 0;
  }
  figcaption {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 6px;
    font-family: var(--mono);
    font-size: 12.500px;
    color: var(--ink-2);
  }
  .faces {
    display: flex;
    grid-column: 1 / -1;
    gap: 14px;
    padding-top: 14px;
    border-top: 1px solid var(--wall);
  }
  .faces .face {
    width: 64px;
    border-radius: 14px;
    background: #1b1f25;
    overflow: hidden;
  }
</style>

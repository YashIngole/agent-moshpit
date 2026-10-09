<script lang="ts">
  // Two directions for a look that is less sweet: the same office, the same six
  // agents, one terminal open. Pictures to choose from, not the app.
  //
  //   plan: the floor as a drafting sheet, the agents a crew marked on it
  //   wall: the floor as a wall of camera feeds, each watching one desk
  //
  // The lettering here is Bahnschrift and Cascadia Mono, which Windows has. They
  // stand in for faces that would be bundled.
  type Phase = 'working' | 'needs_you' | 'done' | 'idle' | 'asleep'

  interface Seat {
    id: string
    sign: string
    title: string
    harness: string
    branch: string
    phase: Phase
    since: string
  }

  let { screen }: { screen: 'plan' | 'wall' } = $props()

  const rooms: { name: string; seats: Seat[] }[] = [
    {
      name: 'shop',
      seats: [
        { id: 'a', sign: 'C1', title: 'Fix the checkout total', harness: 'Claude Code', branch: 'fix/checkout-total', phase: 'needs_you', since: '00:48' },
        { id: 'b', sign: 'X1', title: 'Refactor auth middleware', harness: 'Codex', branch: 'refactor/auth', phase: 'working', since: '12:07' },
        { id: 'c', sign: 'C2', title: 'Flaky test hunt', harness: 'Claude Code', branch: 'main', phase: 'asleep', since: '26 h' }
      ]
    },
    {
      name: 'api',
      seats: [
        { id: 'd', sign: 'X2', title: 'Type the orders API', harness: 'Codex', branch: 'types/orders', phase: 'done', since: '03:10' },
        { id: 'e', sign: 'G1', title: 'Docs pass for the API', harness: 'Gemini CLI', branch: 'main', phase: 'working', since: '09:22' },
        { id: 'f', sign: 'H1', title: 'Bump dependencies', harness: 'Hermes', branch: 'main', phase: 'idle', since: '04:01' }
      ]
    }
  ]

  const WORD: Record<Phase, string> = { working: 'Working', needs_you: 'Needs you', done: 'Done', idle: 'Idle', asleep: 'Away' }
  const open = rooms[0]!.seats[0]!
  const cam = (n: number) => String(n + 1).padStart(2, '0')

  const lines: { text: string; tone?: 'dim' | 'add' | 'del' | 'key' }[] = [
    { text: '> The checkout total is wrong when a coupon is applied twice.', tone: 'dim' },
    { text: '' },
    { text: '● Read(src/cart/total.ts)' },
    { text: '  └ Read 48 lines', tone: 'dim' },
    { text: '' },
    { text: '● Update(src/cart/total.ts)' },
    { text: '  └ Updated with 1 addition and 2 removals', tone: 'dim' },
    { text: '    13 -   const once = applyCoupon(lines, coupon)', tone: 'del' },
    { text: '    14 -   return applyCoupon(once, coupon)', tone: 'del' },
    { text: '    13 +   return applyCoupon(lines, coupon)', tone: 'add' },
    { text: '' },
    { text: '● Bash(rm -rf dist && pnpm build)' },
    { text: '  └ Waiting for permission', tone: 'dim' },
    { text: '' },
    { text: 'Do you want to proceed?' },
    { text: '> 1. Yes', tone: 'key' },
    { text: "  2. Yes, and don't ask again for pnpm build" },
    { text: '  3. No, and tell Claude what to do differently' }
  ]
</script>

{#snippet terminal()}
  <div class="screen">
    {#each lines as line, i (i)}<div class="ln {line.tone ?? ''}">{line.text || ' '}</div>{/each}
    <div class="ln"><span class="caret"></span></div>
  </div>
{/snippet}

{#if screen === 'plan'}
  <div class="plan">
    <header class="strip">
      <b>AGENT MOSHPIT</b>
      <span>FLOOR PLAN</span>
      <span>6 CREW</span>
      <span class="hot">1 ASKING</span>
      <span>2 WORKING</span>
      <span>1 DONE</span>
      <i></i>
      <button type="button">+ ADD CREW</button>
    </header>
    <div class="table">
      <main class="sheet">
        {#each rooms as room, r (room.name)}
          <section class="room">
            <h2><span>ROOM {cam(r)}</span>{room.name.toUpperCase()}<em>{room.seats.length} CREW</em></h2>
            <svg class="door" viewBox="0 0 40 40" aria-hidden="true"><path d="M2 38V2M2 38A36 36 0 0 0 38 2" /></svg>
            <div class="desks">
              {#each room.seats as s (s.id)}
                <div class="desk {s.phase}" class:open={s.id === open.id}>
                  <svg viewBox="0 0 200 132" aria-hidden="true">
                    <!-- the throw of a lit screen, drawn the way a plan draws light -->
                    {#if s.phase === 'working' || s.phase === 'needs_you' || s.phase === 'done'}
                      <path class="throw" d="M84 34 100 34 116 34 142 96H58z" />
                    {/if}
                    <rect class="line" x="34" y="26" width="132" height="46" />
                    <path class="hatch" d="M40 72 52 60M52 72 64 60M136 72 148 60M148 72 160 60" />
                    <rect class="solid" x="84" y="29" width="32" height="5" />
                    <rect class="line thin" x="78" y="48" width="44" height="12" rx="2" />
                    <g class="person" transform={s.phase === 'done' ? 'translate(0 12)' : ''}>
                      <rect class="line thin" x="76" y="84" width="48" height="34" rx="9" />
                      {#if s.phase === 'working'}<path class="line thin" d="M82 92 86 62M118 92 114 62" />{/if}
                      {#if s.phase === 'needs_you'}<path class="line" d="M118 90 140 58" /><circle class="solid" cx="141" cy="56" r="4" />{/if}
                      <ellipse class="body" cx="100" cy="96" rx="23" ry="11" />
                      <circle class="body head" cx="100" cy="92" r="9" />
                    </g>
                    {#if s.phase === 'needs_you'}
                      <!-- grease pencil, the one hand-drawn thing on the sheet -->
                      <path class="wax" d="M30 70C24 28 70 8 112 12c46 4 70 30 62 62-8 34-52 50-92 44C52 114 34 98 30 70z" />
                      <path class="wax" d="M36 76C28 40 62 16 104 16" />
                    {/if}
                    <circle class="tag-ring" cx="22" cy="16" r="13" />
                    <text class="tag-sign" x="22" y="20.500">{s.sign}</text>
                    <path class="line thin dash" d="M33 22 48 32" />
                  </svg>
                  <p class="name">{s.title}</p>
                  <p class="meta"><span class="stamp">{WORD[s.phase]}</span><span>{s.since}</span><span>{s.harness}</span></p>
                </div>
              {/each}
            </div>
          </section>
        {/each}
        <table class="block" aria-hidden="true">
          <tbody>
            <tr><td>PROJECT</td><td>AGENT MOSHPIT</td><td>SHEET</td><td>1 OF 1</td></tr>
            <tr><td>DRAWN</td><td>2026-10-08 21:42</td><td>SCALE</td><td>1 DESK = 1 AGENT</td></tr>
          </tbody>
        </table>
      </main>
      <section class="detail">
        <header>
          <span class="bubble">{open.sign}</span>
          <div>
            <h2>{open.title.toUpperCase()}</h2>
            <p><span class="stamp hot">NEEDS YOU</span><span>{open.since}</span><span>{open.harness.toUpperCase()}</span><span>SHOP / {open.branch}</span></p>
          </div>
          <span class="ref">DETAIL {open.sign}</span>
        </header>
        {@render terminal()}
      </section>
    </div>
  </div>
{:else}
  <div class="wall">
    <header class="strip">
      <b>AGENT MOSHPIT</b>
      <span>6 FEEDS</span>
      <span class="hot">1 WAITING</span>
      <span>2 WORKING</span>
      <span>1 DONE</span>
      <i></i>
      <span class="clock">2026-10-08 21:42:07</span>
      <button type="button">+ NEW AGENT</button>
    </header>
    <div class="table">
      <main class="feeds">
        {#each rooms as room (room.name)}
          {#each room.seats as s, n (s.id)}
            <div class="feed {s.phase}" class:open={s.id === open.id}>
              <svg viewBox="0 0 320 200" preserveAspectRatio="xMidYMid slice" aria-hidden="true">
                <rect class="floor" width="320" height="200" />
                <path class="tiles" d="M0 50h320M0 100h320M0 150h320M80 0v200M160 0v200M240 0v200" />
                <ellipse class="pool" cx="160" cy="112" rx="120" ry="78" />
                <rect class="desk-top" x="78" y="52" width="164" height="58" rx="3" />
                <rect class="screen-back" x="134" y="56" width="52" height="8" rx="2" />
                <rect class="glow" x="128" y="64" width="64" height="26" />
                <rect class="keys" x="136" y="92" width="48" height="12" rx="2" />
                {#if s.phase !== 'asleep'}
                  <g transform={s.phase === 'done' ? 'translate(0 16)' : ''}>
                    <rect class="chair" x="126" y="122" width="68" height="44" rx="12" />
                    <ellipse class="shoulders" cx="160" cy="138" rx="34" ry="15" />
                    <circle class="head" cx="160" cy="130" r="13" />
                    {#if s.phase === 'needs_you'}<path class="arm" d="M186 132 216 86" />{/if}
                  </g>
                {:else}
                  <rect class="chair" x="152" y="150" width="68" height="44" rx="12" transform="rotate(18 186 172)" />
                {/if}
                {#if s.phase === 'needs_you'}
                  <path class="lock" d="M96 76V60h16M224 60h16v16M240 162v16h-16M112 178H96v-16" />
                {/if}
              </svg>
              <div class="lines" aria-hidden="true"></div>
              <p class="osd tl">CAM {cam(rooms.indexOf(room) * 3 + n)} &nbsp;{room.name.toUpperCase()} / {s.branch}</p>
              <p class="osd tr">{#if s.phase !== 'asleep'}<i></i>{/if}{s.phase === 'asleep' ? 'NO SIGNAL' : `LIVE ${s.since}`}</p>
              <p class="osd bl"><b>{s.title}</b></p>
              <p class="osd br"><span class="state">{WORD[s.phase].toUpperCase()}</span> {s.harness.toUpperCase()}</p>
            </div>
          {/each}
        {/each}
      </main>
      <section class="patched">
        <header>
          <span class="cam">CAM 01</span>
          <div>
            <h2>{open.title}</h2>
            <p><span class="state">NEEDS YOU</span> {open.since} &nbsp;·&nbsp; {open.harness.toUpperCase()} &nbsp;·&nbsp; shop / {open.branch}</p>
          </div>
          <span class="ref">PATCHED THROUGH</span>
        </header>
        {@render terminal()}
      </section>
    </div>
  </div>
{/if}

<style>
  .plan,
  .wall {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    height: 100%;
    font-family: Bahnschrift, 'DIN Alternate', 'Barlow Condensed', sans-serif;
    font-stretch: 82%;
    letter-spacing: 0.04em;
    --mono: 'Cascadia Mono', Consolas, monospace;
  }
  .table {
    display: grid;
    grid-template-columns: minmax(0, 54fr) minmax(0, 46fr);
    min-height: 0;
  }
  .strip {
    display: flex;
    align-items: center;
    gap: 22px;
    height: 44px;
    padding: 0 16px 0 20px;
    font-size: 13px;
    font-weight: 600;
  }
  .strip b {
    font-size: 16px;
    font-weight: 700;
    letter-spacing: 0.14em;
  }
  .strip i {
    flex: 1;
  }
  .strip button {
    height: 28px;
    padding: 0 14px;
    border: 1px solid currentColor;
    background: transparent;
    color: inherit;
    font: inherit;
    letter-spacing: 0.1em;
  }
  .screen {
    padding: 14px 18px;
    overflow: hidden;
    font-family: var(--mono);
    font-size: 13px;
    font-stretch: 100%;
    letter-spacing: 0;
    line-height: 1.55;
    white-space: pre;
  }
  .caret {
    display: inline-block;
    width: 8px;
    height: 16px;
    margin-top: 6px;
    background: currentColor;
  }

  /* ── the plan: a drafting sheet, white line on Prussian blue ── */
  .plan {
    --paper: #10305a;
    --paper-deep: #0b2547;
    --ink: #d7e8ff;
    --ink-2: #8fb2de;
    --rule: rgba(190, 220, 255, 0.11);
    --wax: #ff7a3d;
    background: var(--paper-deep);
    color: var(--ink);
  }
  .plan .strip {
    border-bottom: 1px solid var(--ink-2);
    background: var(--paper-deep);
  }
  .plan .hot {
    color: var(--wax);
  }
  .sheet {
    position: relative;
    display: grid;
    align-content: start;
    gap: 26px;
    padding: 26px 28px 20px;
    overflow: hidden;
    background:
      linear-gradient(var(--rule) 1px, transparent 1px) 0 0 / 100% 20px,
      linear-gradient(90deg, var(--rule) 1px, transparent 1px) 0 0 / 20px 100%,
      var(--paper);
  }
  .room {
    position: relative;
    padding: 18px 16px 14px;
    /* A wall on a plan is two lines. */
    border: 1px solid var(--ink);
    outline: 1px solid var(--ink);
    outline-offset: 4px;
  }
  .room h2 {
    position: absolute;
    top: -11px;
    left: 18px;
    display: flex;
    gap: 12px;
    align-items: baseline;
    margin: 0;
    padding: 0 10px;
    background: var(--paper);
    font-size: 15px;
    font-weight: 700;
    letter-spacing: 0.16em;
  }
  .room h2 span,
  .room h2 em {
    font-size: 11px;
    font-style: normal;
    font-weight: 500;
    color: var(--ink-2);
  }
  .door {
    position: absolute;
    right: -6px;
    bottom: -6px;
    width: 46px;
    height: 46px;
    background: var(--paper);
    fill: none;
    stroke: var(--ink);
    stroke-width: 1;
    stroke-dasharray: 3 3;
  }
  .desks {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
  }
  .plan .desk {
    padding: 6px 8px 8px;
  }
  .plan .desk.open {
    background: rgba(215, 232, 255, 0.07);
    outline: 1px dashed var(--ink-2);
  }
  .plan .desk svg {
    display: block;
    width: 100%;
    overflow: visible;
  }
  .line {
    fill: none;
    stroke: var(--ink);
    stroke-width: 1.6;
  }
  .thin {
    stroke-width: 1;
  }
  .dash {
    stroke-dasharray: 3 3;
  }
  .hatch {
    fill: none;
    stroke: var(--ink-2);
    stroke-width: 0.8;
  }
  .solid {
    fill: var(--ink);
  }
  .body {
    fill: var(--paper);
    stroke: var(--ink);
    stroke-width: 1.6;
  }
  .throw {
    fill: rgba(215, 232, 255, 0.12);
    stroke: var(--ink-2);
    stroke-width: 0.8;
    stroke-dasharray: 2 4;
  }
  .needs_you .throw {
    fill: rgba(255, 122, 61, 0.2);
    stroke: var(--wax);
  }
  .needs_you .solid,
  .needs_you .line:not(.thin) {
    stroke: var(--ink);
  }
  .wax {
    fill: none;
    stroke: var(--wax);
    stroke-width: 2.4;
    stroke-linecap: round;
    opacity: 0.92;
  }
  .asleep .person {
    opacity: 0.45;
  }
  .asleep .person .body,
  .asleep .person .line {
    stroke-dasharray: 3 3;
  }
  .tag-ring {
    fill: var(--paper);
    stroke: var(--ink);
    stroke-width: 1.2;
  }
  .tag-sign {
    fill: var(--ink);
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0;
    text-anchor: middle;
  }
  .name {
    margin: 4px 0 2px;
    overflow: hidden;
    font-size: 14px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-overflow: ellipsis;
    text-transform: uppercase;
    white-space: nowrap;
  }
  .meta {
    display: flex;
    gap: 10px;
    align-items: center;
    margin: 0;
    font-size: 11px;
    color: var(--ink-2);
    text-transform: uppercase;
  }
  .stamp {
    padding: 0 6px;
    border: 1px solid currentColor;
    font-weight: 700;
    letter-spacing: 0.12em;
    color: var(--ink);
  }
  .needs_you .stamp,
  .stamp.hot {
    border-color: var(--wax);
    background: var(--wax);
    color: #2a1000;
  }
  .block {
    align-self: end;
    justify-self: end;
    border-collapse: collapse;
    font-size: 10px;
    color: var(--ink-2);
  }
  .block td {
    padding: 3px 10px;
    border: 1px solid var(--ink-2);
  }
  .block td:nth-child(even) {
    color: var(--ink);
    font-weight: 600;
  }
  .detail {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    min-height: 0;
    border-left: 1px solid var(--ink);
    background: #071a33;
  }
  .detail header,
  .patched header {
    display: flex;
    gap: 14px;
    align-items: center;
    padding: 10px 16px;
  }
  .detail header {
    border-bottom: 1px solid var(--ink-2);
    background: var(--paper-deep);
  }
  .detail h2,
  .patched h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 700;
  }
  .detail header p,
  .patched header p {
    display: flex;
    gap: 12px;
    align-items: center;
    margin: 3px 0 0;
    font-size: 11px;
    color: var(--ink-2);
  }
  .detail header div,
  .patched header div {
    flex: 1;
    min-width: 0;
  }
  .bubble {
    display: grid;
    width: 34px;
    height: 34px;
    place-items: center;
    border: 1.5px solid var(--ink);
    border-radius: 50%;
    font-size: 14px;
    font-weight: 700;
    letter-spacing: 0;
  }
  .ref {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.14em;
    color: var(--ink-2);
  }
  .plan .screen {
    color: #dbe8f8;
  }
  .plan .ln.dim {
    color: #7f9cc0;
  }
  .plan .ln.add {
    background: rgba(120, 200, 170, 0.16);
  }
  .plan .ln.del {
    background: rgba(255, 122, 61, 0.14);
  }
  .plan .ln.key {
    color: #9fd3ff;
  }

  /* ── the wall: camera feeds, phosphor grey with one amber ── */
  .wall {
    --ground: #0b0d0c;
    --frame: #1b201e;
    --ink: #c9d6cf;
    --ink-2: #7c8c85;
    --amber: #ffb224;
    --live: #9be7c4;
    background: var(--ground);
    color: var(--ink);
  }
  .wall .strip {
    border-bottom: 1px solid var(--frame);
    color: var(--ink-2);
  }
  .wall .strip b {
    color: var(--ink);
  }
  .wall .hot {
    color: var(--amber);
  }
  .clock {
    font-family: var(--mono);
    font-stretch: 100%;
    letter-spacing: 0;
  }
  .feeds {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    grid-auto-rows: minmax(0, 1fr);
    gap: 6px;
    padding: 6px;
    min-height: 0;
  }
  .feed {
    position: relative;
    overflow: hidden;
    border: 1px solid var(--frame);
    background: #101412;
  }
  .feed.open {
    border-color: var(--ink);
  }
  .feed.needs_you {
    border-color: var(--amber);
  }
  .feed svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .floor {
    fill: #141a17;
  }
  .tiles {
    fill: none;
    stroke: #1b2420;
    stroke-width: 1;
  }
  /* The only light in the room is the screen; its colour is the status. */
  .pool {
    fill: var(--light, #7d8f87);
    opacity: 0.16;
    filter: blur(14px);
  }
  .glow {
    fill: var(--light, #7d8f87);
    opacity: 0.8;
  }
  .working {
    --light: #bfeee0;
  }
  .needs_you {
    --light: var(--amber);
  }
  .done {
    --light: #5fd08a;
  }
  .idle {
    --light: #6f8f86;
  }
  .asleep .pool,
  .asleep .glow {
    opacity: 0.05;
  }
  .desk-top {
    fill: #2a322e;
  }
  .screen-back {
    fill: #0a0c0b;
  }
  .keys {
    fill: #3a4540;
  }
  .chair {
    fill: #1e2522;
  }
  .shoulders {
    fill: #56655e;
  }
  .head {
    fill: #7b8b83;
  }
  .arm {
    fill: none;
    stroke: #7b8b83;
    stroke-width: 9;
    stroke-linecap: round;
  }
  .lock {
    fill: none;
    stroke: var(--amber);
    stroke-width: 2;
  }
  .lines {
    position: absolute;
    inset: 0;
    background: repeating-linear-gradient(rgba(0, 0, 0, 0.22) 0 1px, transparent 1px 3px);
  }
  .osd {
    position: absolute;
    display: flex;
    gap: 8px;
    align-items: center;
    margin: 0;
    font-family: var(--mono);
    font-size: 11px;
    font-stretch: 100%;
    letter-spacing: 0.02em;
    color: var(--ink);
    text-shadow: 0 0 2px #000;
  }
  .osd.tl {
    top: 8px;
    left: 10px;
    color: var(--ink-2);
  }
  .osd.tr {
    top: 8px;
    right: 10px;
    color: var(--ink-2);
  }
  .osd.tr i {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--live);
  }
  .osd.bl {
    bottom: 8px;
    left: 10px;
    font-size: 13px;
  }
  .osd.bl b {
    font-weight: 600;
  }
  .osd.br {
    right: 10px;
    bottom: 8px;
    color: var(--ink-2);
  }
  .state {
    padding: 0 5px;
    background: var(--ink-2);
    color: var(--ground);
    font-weight: 700;
  }
  .needs_you .state,
  .patched .state {
    background: var(--amber);
  }
  .working .state {
    background: var(--live);
  }
  .patched {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    min-height: 0;
    border-left: 1px solid var(--frame);
    background: #070908;
  }
  .patched header {
    border-bottom: 1px solid var(--frame);
  }
  .patched header p {
    font-family: var(--mono);
    font-stretch: 100%;
    letter-spacing: 0;
  }
  .patched h2 {
    font-size: 16px;
    letter-spacing: 0.02em;
  }
  .cam {
    padding: 4px 8px;
    border: 1px solid var(--ink-2);
    font-family: var(--mono);
    font-size: 11px;
    font-stretch: 100%;
    letter-spacing: 0;
  }
  .wall .screen {
    color: #cfdcd5;
  }
  .wall .ln.dim {
    color: #6f7f78;
  }
  .wall .ln.add {
    background: rgba(95, 208, 138, 0.14);
  }
  .wall .ln.del {
    background: rgba(255, 178, 36, 0.12);
  }
  .wall .ln.key {
    color: var(--live);
  }
</style>

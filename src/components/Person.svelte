<script lang="ts">
  // One person at a desk, after hours. Their pose is the status, and so is the
  // light of their screen: code while they work, amber when they need you, a tick
  // when they are done, red when something broke, dark when they have gone home.
  //
  // Nothing here uses a running CSS animation. Loops (typing, waving, reading) are a
  // handful of fixed poses switched by the `data-b*` beat attributes on <html>, a few
  // times a second, and not at all while the window is hidden.
  import { lookFor, pale, shade } from '../lib/look'
  import type { Phase } from '../lib/types'

  interface Props {
    look: number
    phase: Phase
    /** The kind of work, while working: read and search glance, think thinks. */
    tool?: string
    /** Draw only head and shoulders, for a pane's header or the band. */
    portrait?: boolean
  }

  let { look, phase, tool = '', portrait = false }: Props = $props()

  const dev = $derived(lookFor(look))

  type Pose = 'type' | 'mouse' | 'think' | 'raise' | 'relax' | 'sleep' | 'ohno' | 'rest'
  const pose = $derived.by((): Pose => {
    switch (phase) {
      case 'needs_you':
        return 'raise'
      case 'done':
        return 'relax'
      case 'failed':
        return 'ohno'
      case 'asleep':
        return 'sleep'
      case 'working':
        if (tool === 'read' || tool === 'search' || tool === 'web') return 'mouse'
        if (tool === 'think' || tool === 'plan') return 'think'
        return 'type'
      default:
        return 'rest'
    }
  })

  const fold = $derived(shade(dev.cloth))
  const string = $derived(pale(dev.cloth) ? '#6b675e' : '#c9cbd1')
  /** Someone who has gone home with a hoodie on has the hood up. */
  const hood = $derived(dev.headwear === 'hood' || (phase === 'asleep' && dev.top === 'hoodie'))
  const alarmed = $derived(phase === 'needs_you' || phase === 'failed')
</script>

{#snippet hair()}
  {#if dev.hairStyle === 'long'}
    <rect x="66" y="27" width="36" height="36" rx="13" fill={dev.hair} />
  {:else if dev.hairStyle === 'curly'}
    <circle cx="84" cy="35" r="21" fill={dev.hair} />
  {/if}
{/snippet}

{#snippet fringe()}
  {#if dev.hairStyle === 'buzz'}
    <path d="M68 39a16.5 16.5 0 0 1 32 0c-4-7-9-10-16-10s-12 3-16 10z" fill={dev.hair} opacity="0.85" />
  {:else if dev.hairStyle === 'swept'}
    <path d="M67.5 41c-1-12 7-19 17.5-19 9.5 0 15.5 5.5 16 13-6-4.5-14-6-22-3.5-5 1.5-9 5-11.5 9.5z" fill={dev.hair} />
  {:else if dev.hairStyle === 'messy'}
    <path d="M67.5 40c-1.5-11 6.5-18.5 16.5-18.5S102 29 100.5 40l-3.5-6-3 4.5-3.5-6.5-4 5.5-3.5-6.5-4 6.5-3.5-5.5-3.5 5z" fill={dev.hair} />
  {:else if dev.hairStyle === 'long' || dev.hairStyle === 'bun'}
    <path d="M67.5 41a16.5 16.5 0 0 1 33 0c-6-7-11-10-16.5-10S73.5 34 67.5 41z" fill={dev.hair} />
    {#if dev.hairStyle === 'bun' && dev.headwear !== 'beanie' && dev.headwear !== 'cap' && !hood}
      <circle cx="84" cy="20" r="6.5" fill={dev.hair} />
    {/if}
  {:else if dev.hairStyle === 'curly'}
    <circle cx="73" cy="29" r="6" fill={dev.hair} />
    <circle cx="84" cy="25.5" r="7" fill={dev.hair} />
    <circle cx="95" cy="29" r="6" fill={dev.hair} />
  {/if}
{/snippet}

{#snippet headwear()}
  {#if hood}
    <path d="M67 33c3.5-7 10-11 17-11s13.5 4 17 11" fill="none" stroke={fold} stroke-width="4.5" stroke-linecap="round" />
  {:else if dev.headwear === 'beanie'}
    <path d="M66.5 38a17.5 17.5 0 0 1 35 0z" fill={dev.hat} />
    <rect x="65.5" y="34" width="37" height="8" rx="3" fill={shade(dev.hat, 0.78)} />
    <path d="M70 35.5v5M74 35.5v5M78 35.5v5M90 35.5v5M94 35.5v5M98 35.5v5" stroke={dev.hat} stroke-width="1" opacity="0.7" />
    <rect x="81" y="35.5" width="6" height="4.5" rx="1" fill="#d7d3c8" opacity="0.8" />
  {:else if dev.headwear === 'cap'}
    <path d="M67 38.5a17 17 0 0 1 34 0z" fill={dev.hat} />
    <path d="M78.5 38.5a5.5 4.5 0 0 1 11 0z" fill={dev.hair} />
    <path d="M78 38.5h12" stroke={shade(dev.hat, 0.6)} stroke-width="1.6" />
    <circle cx="84" cy="22" r="1.6" fill={shade(dev.hat, 0.6)} />
  {:else if dev.headwear === 'headphones'}
    <path d="M65.5 40a18.5 18.5 0 0 1 37 0" fill="none" stroke="#15171b" stroke-width="3.6" stroke-linecap="round" />
    <rect x="60" y="35" width="9" height="16" rx="4" fill="#15171b" />
    <rect x="99" y="35" width="9" height="16" rx="4" fill="#15171b" />
    <rect x="62" y="38" width="5" height="10" rx="2.5" fill="#2b2f37" />
    <rect x="101" y="38" width="5" height="10" rx="2.5" fill="#2b2f37" />
  {/if}
{/snippet}

{#snippet face()}
  <g class="face" stroke={dev.ink} fill={dev.ink}>
    {#if phase === 'asleep'}
      <path class="line" d="M74.5 44q3 1.6 6 0M87.5 44q3 1.6 6 0" />
      <path class="line thin" d="M82 50.5h4" />
    {:else}
      <!-- The brows carry the mood. -->
      {#if phase === 'needs_you'}
        <path class="line brow" d="M73.5 36.8l6.5-1.2M88 35.6l6.5 1.2" />
      {:else if phase === 'failed'}
        <path class="line brow" d="M73.5 37l6.5 1.6M94.5 37l-6.5 1.6" />
      {:else if phase === 'done'}
        <path class="line brow" d="M73.5 37.5l6.5-0.6M88 36.9l6.5 0.6" />
      {:else}
        <path class="line brow" d="M73.5 38.4l6.5-1M88 37.4l6.5 1" />
      {/if}
      {#if phase === 'done'}
        <path class="line" d="M74.5 43.5q3-2.6 6 0M87.5 43.5q3-2.6 6 0" />
      {:else}
        <g class="lids">
          <ellipse class="white" cx="77.5" cy="43" rx="2.9" ry={alarmed ? 2.3 : 1.8} />
          <ellipse class="white" cx="90.5" cy="43" rx="2.9" ry={alarmed ? 2.3 : 1.8} />
          <g class="eyes">
            <circle class="dot" cx="78" cy="43.2" r="1.55" />
            <circle class="dot" cx="91" cy="43.2" r="1.55" />
          </g>
          <!-- Half shut: busy, not sleepy. -->
          {#if !alarmed}
            <path class="lid" d="M74.3 42.2q3.2-1.7 6.4 0M87.3 42.2q3.2-1.7 6.4 0" />
          {/if}
        </g>
      {/if}
      {#if phase === 'done'}
        <path class="line" d="M80 48.5q4 3.3 8 0" />
      {:else if phase === 'failed'}
        <path class="line" d="M79.5 50.5q2.2-1.8 4.5 0t4.5 0" />
      {:else if phase === 'needs_you'}
        <ellipse class="dot" cx="84.5" cy="50" rx="1.7" ry="1.3" />
      {:else}
        <path class="line thin" d="M81 49.6q3.5 0.9 6.5-0.6" />
      {/if}
    {/if}
    {#if dev.glasses !== 'none' && phase !== 'asleep'}
      {#if dev.glasses === 'square'}
        <rect class="lens" x="72.5" y="39.3" width="10" height="7.4" rx="1.6" />
        <rect class="lens" x="85.5" y="39.3" width="10" height="7.4" rx="1.6" />
      {:else}
        <circle class="lens" cx="77.5" cy="43" r="4.6" />
        <circle class="lens" cx="90.5" cy="43" r="4.6" />
      {/if}
      <path class="line thin" d="M82.5 42.3h3" />
    {/if}
  </g>
{/snippet}

{#snippet head()}
  <g class="head">
    {@render hair()}
    {#if hood}
      <path d="M63.5 47c-2.5-17 8-27 20.5-27s23 10 20.5 27c-1.5 9-6 13-6 13H69.5s-4.5-4-6-13z" fill={dev.cloth} />
    {/if}
    <circle cx="67.5" cy="42.5" r="3" fill={dev.skin} />
    <circle cx="100.5" cy="42.5" r="3" fill={dev.skin} />
    <circle cx="84" cy="40.5" r="16.8" fill={dev.skin} />
    {#if dev.beard !== 'none'}
      <path
        d="M67.8 43c0.8 9.5 7.2 14.5 16.2 14.5S99.4 52.5 100.2 43c-2.2 4.5-5 6.5-7.5 7-2.2 0.4-4.3-1.4-8.7-1.4s-6.5 1.8-8.7 1.4c-2.5-0.5-5.3-2.5-7.5-7z"
        fill={dev.hair}
        opacity={dev.beard === 'full' ? 0.95 : 0.35}
      />
    {/if}
    {@render fringe()}
    {@render headwear()}
    {@render face()}
  </g>
{/snippet}

<svg class="person {phase} pose-{pose}" class:portrait viewBox={portrait ? '52 14 64 64' : '0 0 168 128'} aria-hidden="true" focusable="false">
  {#if !portrait}
    <ellipse class="ground" cx="84" cy="118" rx="74" ry="6" />
    <!-- the light of the screen, falling on the desk -->
    <ellipse class="spill" cx="126" cy="86" rx="44" ry="12" />
    <rect class="chair" x="56.5" y="25" width="55" height="60" rx="12" />
    <path class="chair-edge" d="M64 28.5h40" />
  {/if}

  <g class="torso">
    <rect x="59.5" y="59" width="49" height="42" rx="14" fill={dev.cloth} />
    {#if dev.top === 'jacket'}
      <rect x="78" y="60" width="12" height="40" fill={pale(dev.cloth) ? '#2b2f37' : '#d9d4c8'} />
      <path d="M78 60v40M90 60v40" stroke={fold} stroke-width="2" />
      <path d="M84 62v38" stroke="#8b8f98" stroke-width="0.9" stroke-dasharray="1.4 1.4" />
    {/if}
    <rect x="79.5" y="51" width="9" height="11" rx="3" fill={dev.skin} />
    {#if dev.top === 'hoodie'}
      <path d="M69.5 57.5c3 8 26 8 29 0l2.5 5c-5.5 9-28.5 9-34 0z" fill={fold} />
      <path d="M80.5 64.5v9M87.5 64.5v9" stroke={string} stroke-width="1.3" stroke-linecap="round" />
      <circle cx="80.5" cy="74" r="1.1" fill={string} />
      <circle cx="87.5" cy="74" r="1.1" fill={string} />
    {:else if dev.top === 'tee'}
      <path d="M76 59.5q8 6.5 16 0" stroke={fold} stroke-width="2.2" fill="none" />
    {:else}
      <path d="M71 58.5l8.5 6M97 58.5l-8.5 6" stroke={fold} stroke-width="3" stroke-linecap="round" />
    {/if}
  </g>
  {@render head()}

  {#if !portrait}
    <!-- the desk: matte, with a mat under the keyboard -->
    <rect class="leg" x="15" y="100" width="4.5" height="17" rx="1.5" />
    <rect class="leg" x="148.5" y="100" width="4.5" height="17" rx="1.5" />
    <rect class="desk-edge" x="6" y="94" width="156" height="9" rx="3" />
    <rect class="desk" x="6" y="75" width="156" height="23" rx="4" />
    <rect class="mat" x="52" y="79.5" width="66" height="15" rx="3" />

    <!-- what is on the desk -->
    {#if dev.item === 'can' || dev.item === 'cans'}
      {#if dev.item === 'cans'}
        <g transform="translate(-9 3) rotate(-78 30 80)">
          <rect class="can" x="26" y="74" width="8.5" height="14" rx="2" />
          <rect class="can-band" x="26" y="79" width="8.5" height="4" />
        </g>
      {/if}
      <rect class="can" x="26" y="64" width="9" height="17" rx="2.2" />
      <rect class="can-top" x="26.6" y="64" width="7.8" height="2" rx="1" />
      <rect class="can-band" x="26" y="70.5" width="9" height="5" />
    {:else if dev.item === 'mug'}
      <path class="mug-handle" d="M37 72.5q5 2.6 0 6.4" />
      <rect class="mug" x="25" y="68" width="12.5" height="13" rx="2.6" />
      <path class="print" d="M29 72.7l-1.6 1.7 1.6 1.7M33.5 72.7l1.6 1.7-1.6 1.7M32 72.2l-1.8 4.4" />
      <g class="steam">
        <path d="M29.5 65q-2-2.5 0-5" />
        <path d="M33.5 65.5q2-2.5 0-5" />
      </g>
    {:else if dev.item === 'lamp'}
      <ellipse class="lamp-light" cx="34" cy="83" rx="17" ry="5" />
      <ellipse class="lamp" cx="20" cy="81" rx="7" ry="2.2" />
      <path class="lamp-arm" d="M20 80l5-19 11 5" />
      <path class="lamp" d="M33.5 62.5l9.5 4.5-4.5 5.5z" />
    {:else}
      <path class="leaf" d="M32 71c-6-2-8-8-6-12 5 1.5 7 6 6 12z" />
      <path class="leaf dark" d="M32 71c0-7 3-11 7-12 1.5 5-1.5 10-7 12z" />
      <path class="leaf" d="M32 71c-2.5-6-1.5-10 0-13 2.5 3.5 2.5 8 0 13z" />
      <path class="pot" d="M25 70h14l-1.8 11h-10.4z" />
    {/if}

    <!-- the screen -->
    <rect class="stand" x="134" y="72" width="4" height="9" />
    <rect class="stand" x="126" y="79.5" width="20" height="3" rx="1.5" />
    {#if dev.dual}
      <rect class="bezel" x="100" y="50" width="14" height="24" rx="2" transform="rotate(-8 107 62)" />
    {/if}
    <rect class="bezel" x="111" y="41" width="52" height="33" rx="3" />
    <rect class="screen" x="113.5" y="43.5" width="47" height="28" rx="1.5" />
    {#if phase === 'working'}
      <g class="code">
        <path class="k" d="M117 48.5h6" /><path class="f" d="M125 48.5h11" /><path class="p" d="M138 48.5h3" />
        <path class="p" d="M120 53h4" /><path class="s" d="M126 53h14" />
        <path class="k" d="M120 57.5h7" /><path class="f" d="M129 57.5h9" /><path class="p" d="M140 57.5h8" />
        <path class="p" d="M123 62h10" /><path class="k" d="M135 62h5" />
        <path class="p" d="M117 66.5h3" /><rect class="cursor" x="122" y="64.6" width="2.6" height="3.8" />
      </g>
    {:else if phase === 'needs_you'}
      <rect class="ask" x="113.5" y="43.5" width="47" height="28" rx="1.5" />
      <text class="glyph" x="137" y="62.5">[y/n]</text>
    {:else if phase === 'failed'}
      <rect class="oops" x="113.5" y="43.5" width="47" height="28" rx="1.5" />
      <text class="glyph" x="137" y="62.5">err</text>
    {:else if phase === 'done'}
      <path class="tick" d="M129 57.5l5 5 10.5-11" />
    {:else if phase === 'starting'}
      <g class="boot">
        <circle class="d1" cx="131" cy="57.5" r="1.6" />
        <circle class="d2" cx="137" cy="57.5" r="1.6" />
        <circle class="d3" cx="143" cy="57.5" r="1.6" />
      </g>
    {:else if phase === 'idle' || phase === 'quiet'}
      <path class="prompt" d="M118 64.5l2.5 2-2.5 2" />
      <rect class="cursor dim" x="122" y="64.6" width="2.6" height="3.8" />
    {:else}
      <path class="sheen" d="M120 46l-5 9M128 46l-8 14" />
    {/if}

    <rect class="kbd" x="62" y="84" width="44" height="9.5" rx="2" />
    <path class="caps" d="M64.5 87h39M64.5 90.5h39" />
    {#if pose === 'mouse'}
      <rect class="kbd" x="110" y="85" width="7" height="10" rx="3.5" />
    {/if}

    <g class="arms" fill="none" stroke={dev.cloth} stroke-width="9" stroke-linecap="round" stroke-linejoin="round">
      {#if pose === 'type' || pose === 'rest'}
        <g class="arm-l">
          <path d="M66 71q-5 13 8 19.5" />
          <circle cx="76" cy="91" r="4.8" fill={dev.skin} stroke="none" />
        </g>
        <g class="arm-r">
          <path d="M102 71q5 13-8 19.5" />
          <circle cx="92" cy="91" r="4.8" fill={dev.skin} stroke="none" />
        </g>
      {:else if pose === 'sleep'}
        <path d="M66 71q-9 12 6 19 10 3 24 0 15-7 6-19" />
      {:else if pose === 'mouse'}
        <path d="M66 71q-5 13 8 19.5" />
        <circle cx="76" cy="91" r="4.8" fill={dev.skin} stroke="none" />
        <g class="arm-r">
          <path d="M102 71q11 9 11 19" />
          <circle cx="113" cy="90.5" r="4.8" fill={dev.skin} stroke="none" />
        </g>
      {:else if pose === 'think'}
        <path d="M66 71q-6 15 14 20.5" />
        <path d="M102 71q12 6-4-8" />
        <circle cx="96" cy="60" r="4.8" fill={dev.skin} stroke="none" />
      {:else if pose === 'raise'}
        <path d="M66 71q-5 13 8 19.5" />
        <circle cx="76" cy="91" r="4.8" fill={dev.skin} stroke="none" />
        <g class="arm-r wave">
          <path d="M102 71l13-34" />
          <circle cx="116" cy="33" r="5.2" fill={dev.skin} stroke="none" />
        </g>
      {:else if pose === 'relax'}
        <path d="M66 71l-17-17 16-15" />
        <path d="M102 71l17-17-16-15" />
      {:else if pose === 'ohno'}
        <path d="M66 71l-13-14 16-24" />
        <path d="M102 71l13-14-16-24" />
        <circle cx="70" cy="31" r="4.8" fill={dev.skin} stroke="none" />
        <circle cx="98" cy="31" r="4.8" fill={dev.skin} stroke="none" />
      {/if}
    </g>

    <!-- what they say: a tag, not a cartoon bubble -->
    {#if phase === 'needs_you' || phase === 'failed' || phase === 'starting'}
      <g class="say {phase}">
        <path d="M14 6h30a3 3 0 0 1 3 3v14a3 3 0 0 1-3 3H38l-4.5 5.5L30 26H14a3 3 0 0 1-3-3V9a3 3 0 0 1 3-3z" />
        <text x="29" y="20.5">{phase === 'needs_you' ? '?' : phase === 'failed' ? '!' : '…'}</text>
      </g>
    {:else if phase === 'asleep'}
      <g class="zz">
        <text class="z1" x="110" y="38">z</text>
        <text class="z2" x="117" y="29">z</text>
        <text class="z3" x="123" y="21">z</text>
      </g>
    {/if}
  {/if}
</svg>

<style>
  .person {
    display: block;
    width: 100%;
    height: auto;
    overflow: visible;
    /* What the screen shows lights the desk and the lenses of their glasses. */
    --glow: var(--work);
  }
  .needs_you {
    --glow: var(--needs);
  }
  .done {
    --glow: var(--done);
  }
  .failed {
    --glow: var(--trouble);
  }
  .idle,
  .quiet,
  .starting {
    --glow: var(--quiet);
  }

  .ground {
    fill: var(--shadow);
  }
  .spill {
    fill: var(--glow);
    opacity: 0.16;
    filter: blur(6px);
  }
  .asleep .spill {
    opacity: 0;
  }
  .chair {
    fill: var(--chair);
  }
  .chair-edge {
    stroke: var(--chair-edge);
    stroke-width: 2;
    stroke-linecap: round;
  }
  .desk {
    fill: var(--desk);
  }
  .desk-edge {
    fill: var(--desk-edge);
  }
  .leg {
    fill: var(--desk-leg);
  }
  .mat {
    fill: var(--mat);
  }
  .kbd,
  .stand {
    fill: var(--kbd);
  }
  .caps {
    stroke: var(--keycaps);
    stroke-width: 2.2;
    stroke-dasharray: 2.6 1.1;
  }
  .bezel {
    fill: var(--bezel);
  }
  .screen {
    fill: var(--screen);
  }
  .asleep .screen {
    fill: var(--screen-off);
  }
  .sheen {
    stroke: rgba(255, 255, 255, 0.06);
    stroke-width: 3;
    stroke-linecap: round;
  }

  /* Code, in the colours an editor uses at night. It is code, not status. */
  .code path {
    stroke-width: 2.2;
    stroke-linecap: round;
  }
  .code .k {
    stroke: #c099ff;
  }
  .code .f {
    stroke: #82aaff;
  }
  .code .s {
    stroke: #c3e88d;
  }
  .code .p {
    stroke: #6c7684;
  }
  .cursor {
    fill: #e7eaf0;
  }
  .cursor.dim {
    fill: #6c7684;
  }
  .prompt {
    fill: none;
    stroke: #6c7684;
    stroke-width: 1.4;
  }
  .ask {
    fill: var(--needs);
  }
  .oops {
    fill: var(--trouble);
  }
  .glyph {
    fill: #140e00;
    font-family: var(--mono);
    font-size: 10px;
    font-weight: 700;
    text-anchor: middle;
  }
  .tick {
    fill: none;
    stroke: var(--done);
    stroke-width: 3;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .boot circle {
    fill: var(--quiet);
    opacity: 0.35;
  }

  /* things on the desk */
  .can {
    fill: #d4d6db;
  }
  .can-top {
    fill: #8b9099;
  }
  .can-band {
    fill: #3f325f;
  }
  .mug {
    fill: #121418;
  }
  .mug-handle {
    fill: none;
    stroke: #121418;
    stroke-width: 2.4;
  }
  .print {
    fill: none;
    stroke: #c9ccd3;
    stroke-width: 0.9;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .steam path {
    fill: none;
    stroke: #6c7684;
    stroke-width: 1.3;
    stroke-linecap: round;
    opacity: 0;
  }
  .lamp {
    fill: #4a515c;
  }
  .lamp-arm {
    fill: none;
    stroke: #4a515c;
    stroke-width: 2;
    stroke-linejoin: round;
  }
  .lamp-light {
    fill: #ffe7bf;
    opacity: 0.22;
    filter: blur(4px);
  }
  .leaf {
    fill: #4c6b58;
  }
  .leaf.dark {
    fill: #36503f;
  }
  .pot {
    fill: #6f7277;
  }

  /* the face */
  .face .line {
    fill: none;
    stroke-width: 1.7;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .face .line.brow {
    stroke-width: 2;
  }
  .face .line.thin {
    stroke-width: 1.3;
  }
  .face .dot {
    stroke: none;
  }
  .face .white {
    fill: #f2efe8;
    stroke: none;
  }
  .face .lid {
    fill: none;
    stroke-width: 1.5;
    stroke-linecap: round;
  }
  .face .lens {
    fill: var(--glow);
    fill-opacity: 0.18;
    stroke-width: 1.6;
  }

  /* the tag over someone's head */
  .say {
    transform-origin: 33px 31px;
    animation: pop var(--settle) var(--ease) both;
  }
  @keyframes pop {
    from {
      transform: scale(0.55);
      opacity: 0;
    }
  }
  .say path {
    fill: var(--desk);
    stroke: var(--keycaps);
    stroke-width: 1;
  }
  .say text {
    fill: var(--ink);
    font-family: var(--mono);
    font-size: 15px;
    font-weight: 700;
    text-anchor: middle;
  }
  .say.needs_you path {
    fill: var(--needs);
    stroke: none;
  }
  .say.needs_you text {
    fill: var(--needs-ink);
  }
  .say.failed path {
    fill: var(--trouble);
    stroke: none;
  }
  .say.failed text {
    fill: #1a0503;
  }
  .zz text {
    fill: var(--quiet);
    font-family: var(--mono);
    font-weight: 600;
    opacity: 0;
  }
  .zz .z1 {
    font-size: 11px;
  }
  .zz .z2 {
    font-size: 9.5px;
  }
  .zz .z3 {
    font-size: 8px;
  }

  /* ── a new arrival walks to their chair, once (the desk sets .arriving) ── */
  :global(.arriving) .torso,
  :global(.arriving) .head,
  :global(.arriving) .arms {
    animation: walk-in 720ms var(--ease) both;
  }
  @keyframes walk-in {
    from {
      transform: translate(-58px, 0);
      opacity: 0;
    }
    18% {
      transform: translate(-46px, -3px);
      opacity: 1;
    }
    36% {
      transform: translate(-34px, 0);
    }
    54% {
      transform: translate(-22px, -3px);
    }
    72% {
      transform: translate(-10px, 0);
    }
    86% {
      transform: translate(-3px, -2px);
    }
  }

  /* ── poses that hold still ── */
  .pose-sleep .head {
    transform: translate(2px, 12px) rotate(10deg);
    transform-origin: 84px 58px;
  }
  .pose-relax .head,
  .pose-relax .torso {
    transform: translateY(-2px) rotate(-3deg);
    transform-origin: 84px 90px;
  }
  .pose-think .eyes {
    transform: translate(-0.9px, -1.1px);
  }
  .pose-type .eyes,
  .pose-mouse .eyes {
    transform: translate(1.1px, 0.4px);
  }
  .asleep {
    filter: saturate(0.6) brightness(0.85);
  }
  .wave {
    transform-origin: 102px 71px;
  }

  /* ── loops, as poses on the shared beat ──
     b2, b3 and b4 count 0..1, 0..2 and 0..3; see lib/beat.ts. */
  :global(html[data-b2='0']) .pose-type .arm-l,
  :global(html[data-b2='1']) .pose-type .arm-r {
    transform: translateY(-2px);
  }
  :global(html[data-b2='1']) .wave {
    transform: rotate(13deg);
  }
  :global(html[data-b2='0']) .wave {
    transform: rotate(-7deg);
  }
  :global(html[data-b4='1']) .pose-mouse .eyes {
    transform: translate(1.3px, 0.2px);
  }
  :global(html[data-b4='2']) .pose-mouse .eyes {
    transform: translate(1.3px, 1px);
  }
  :global(html[data-b4='3']) .pose-mouse .eyes {
    transform: translate(0.5px, 1.2px);
  }
  :global(html[data-b4='2']) .pose-mouse .arm-r,
  :global(html[data-b4='3']) .pose-mouse .arm-r {
    transform: translate(1.5px, -1px);
  }
  /* The code scrolls a line now and then, and the cursor blinks. */
  :global(html[data-b4='1']) .working .code,
  :global(html[data-b4='3']) .working .code {
    transform: translateY(-1.6px);
  }
  :global(html[data-b2='1']) .cursor {
    opacity: 0;
  }
  :global(html[data-b3='0']) .boot .d1,
  :global(html[data-b3='1']) .boot .d2,
  :global(html[data-b3='2']) .boot .d3 {
    opacity: 1;
  }
  :global(html[data-b4='0']) .zz .z1,
  :global(html[data-b4='1']) .zz .z1,
  :global(html[data-b4='1']) .zz .z2,
  :global(html[data-b4='2']) .zz .z1,
  :global(html[data-b4='2']) .zz .z2,
  :global(html[data-b4='2']) .zz .z3 {
    opacity: 0.9;
  }
  :global(html[data-b2='0']) .steam path:first-child,
  :global(html[data-b2='1']) .steam path:last-child {
    opacity: 0.7;
  }
  :global(html[data-blink]) .idle .lids,
  :global(html[data-blink]) .working .lids {
    transform-box: fill-box;
    transform-origin: center;
    scale: 1 0.12;
  }
  /* Holding still (a hidden window, or less motion asked for): a cursor stays lit. */
  :global(html.still) .cursor {
    opacity: 1;
  }
</style>

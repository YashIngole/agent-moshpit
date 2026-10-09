<script lang="ts">
  // A developer at a desk, after hours. The same drawing as the office's person
  // (one viewBox, the same eight poses), dressed and lit differently: dark clothes,
  // a matte desk, a mechanical keyboard, and a monitor whose light is the status.
  import { pale, shade, type Dev } from './crew'

  type Phase = 'starting' | 'working' | 'needs_you' | 'done' | 'idle' | 'failed' | 'asleep'

  interface Props {
    dev: Dev
    phase: Phase
    /** The kind of work, for the pose: read, edit, think. */
    tool?: string
    /** Head and shoulders only. */
    portrait?: boolean
  }

  let { dev, phase, tool = '', portrait = false }: Props = $props()

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
        if (tool === 'read' || tool === 'search') return 'mouse'
        if (tool === 'think') return 'think'
        return 'type'
      default:
        return 'rest'
    }
  })

  const fold = $derived(shade(dev.cloth))
  const string = $derived(pale(dev.cloth) ? '#6b675e' : '#c9cbd1')
  const hood = $derived(dev.headwear === 'hood' || phase === 'asleep' && dev.top === 'hoodie')
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
    <path d="M67.5 41c-1-12 7-19 17.5-19 9.500 0 15.500 5.500 16 13-6-4.500-14-6-22-3.500-5 1.500-9 5-11.500 9.500z" fill={dev.hair} />
  {:else if dev.hairStyle === 'messy'}
    <path d="M67.500 40c-1.500-11 6.500-18.500 16.500-18.500S102 29 100.500 40l-3.500-6-3 4.500-3.500-6.500-4 5.500-3.500-6.500-4 6.500-3.500-5.500-3.500 5z" fill={dev.hair} />
  {:else if dev.hairStyle === 'long' || dev.hairStyle === 'bun'}
    <path d="M67.500 41a16.500 16.500 0 0 1 33 0c-6-7-11-10-16.500-10S73.500 34 67.500 41z" fill={dev.hair} />
    {#if dev.hairStyle === 'bun' && dev.headwear !== 'beanie' && dev.headwear !== 'cap'}
      <circle cx="84" cy="20" r="6.500" fill={dev.hair} />
    {/if}
  {:else if dev.hairStyle === 'curly'}
    <circle cx="73" cy="29" r="6" fill={dev.hair} />
    <circle cx="84" cy="25.500" r="7" fill={dev.hair} />
    <circle cx="95" cy="29" r="6" fill={dev.hair} />
  {/if}
{/snippet}

{#snippet headwear()}
  {#if hood}
    <path d="M67 33c3.500-7 10-11 17-11s13.500 4 17 11" fill="none" stroke={fold} stroke-width="4.500" stroke-linecap="round" />
  {:else if dev.headwear === 'beanie'}
    <path d="M66.500 38a17.500 17.500 0 0 1 35 0z" fill={dev.hat} />
    <rect x="65.500" y="34" width="37" height="8" rx="3" fill={shade(dev.hat, 0.78)} />
    <path d="M70 35.500v5M74 35.500v5M78 35.500v5M90 35.500v5M94 35.500v5M98 35.500v5" stroke={dev.hat} stroke-width="1" opacity="0.7" />
    <rect x="81" y="35.500" width="6" height="4.500" rx="1" fill="#d7d3c8" opacity="0.8" />
  {:else if dev.headwear === 'cap'}
    <path d="M67 38.500a17 17 0 0 1 34 0z" fill={dev.hat} />
    <path d="M78.500 38.500a5.500 4.500 0 0 1 11 0z" fill={dev.hair} />
    <path d="M78 38.500h12" stroke={shade(dev.hat, 0.6)} stroke-width="1.600" />
    <circle cx="84" cy="22" r="1.600" fill={shade(dev.hat, 0.6)} />
  {:else if dev.headwear === 'headphones'}
    <path d="M65.500 40a18.500 18.500 0 0 1 37 0" fill="none" stroke="#15171b" stroke-width="3.600" stroke-linecap="round" />
    <rect x="60" y="35" width="9" height="16" rx="4" fill="#15171b" />
    <rect x="99" y="35" width="9" height="16" rx="4" fill="#15171b" />
    <rect x="62" y="38" width="5" height="10" rx="2.500" fill="#2b2f37" />
    <rect x="101" y="38" width="5" height="10" rx="2.500" fill="#2b2f37" />
  {/if}
{/snippet}

{#snippet face()}
  <g class="face" stroke={dev.ink} fill={dev.ink}>
    {#if phase === 'asleep'}
      <path class="line" d="M74.500 44q3 1.600 6 0M87.500 44q3 1.600 6 0" />
      <path class="line thin" d="M82 50.500h4" />
    {:else}
      <!-- Brows carry the mood. -->
      {#if phase === 'needs_you'}
        <path class="line brow" d="M73.500 36.800l6.500-1.200M88 35.600l6.500 1.200" />
      {:else if phase === 'failed'}
        <path class="line brow" d="M73.500 37l6.500 1.600M94.500 37l-6.500 1.600" />
      {:else if phase === 'done'}
        <path class="line brow" d="M73.500 37.500l6.500-0.600M88 36.900l6.500 0.600" />
      {:else}
        <path class="line brow" d="M73.500 38.400l6.500-1M88 37.400l6.500 1" />
      {/if}
      {#if phase === 'done'}
        <path class="line" d="M74.500 43.500q3-2.600 6 0M87.500 43.500q3-2.600 6 0" />
      {:else}
        <g class="lids">
          <ellipse class="white" cx="77.500" cy="43" rx="2.900" ry={phase === 'needs_you' || phase === 'failed' ? 2.300 : 1.800} />
          <ellipse class="white" cx="90.500" cy="43" rx="2.900" ry={phase === 'needs_you' || phase === 'failed' ? 2.300 : 1.800} />
          <g class="eyes">
            <circle class="dot" cx="78" cy="43.200" r="1.550" />
            <circle class="dot" cx="91" cy="43.200" r="1.550" />
          </g>
          <!-- Half shut: busy, not sleepy. -->
          {#if phase !== 'needs_you' && phase !== 'failed'}
            <path class="lid" d="M74.300 42.200q3.200-1.700 6.400 0M87.300 42.200q3.200-1.700 6.400 0" />
          {/if}
        </g>
      {/if}
      {#if phase === 'done'}
        <path class="line" d="M80 48.500q4 3.300 8 0" />
      {:else if phase === 'failed'}
        <path class="line" d="M79.500 50.500q2.200-1.800 4.500 0t4.500 0" />
      {:else if phase === 'needs_you'}
        <ellipse class="dot" cx="84.500" cy="50" rx="1.700" ry="1.300" />
      {:else}
        <path class="line thin" d="M81 49.600q3.500 0.900 6.500-0.600" />
      {/if}
    {/if}
    {#if dev.glasses !== 'none' && phase !== 'asleep'}
      {#if dev.glasses === 'square'}
        <rect class="lens" x="72.500" y="39.300" width="10" height="7.400" rx="1.600" />
        <rect class="lens" x="85.500" y="39.300" width="10" height="7.400" rx="1.600" />
      {:else}
        <circle class="lens" cx="77.500" cy="43" r="4.600" />
        <circle class="lens" cx="90.500" cy="43" r="4.600" />
      {/if}
      <path class="line thin" d="M82.500 42.300h3" />
    {/if}
  </g>
{/snippet}

{#snippet head()}
  <g class="head">
    {@render hair()}
    {#if hood}
      <path d="M63.500 47c-2.500-17 8-27 20.500-27s23 10 20.500 27c-1.500 9-6 13-6 13H69.500s-4.500-4-6-13z" fill={dev.cloth} />
    {/if}
    <circle cx="67.500" cy="42.500" r="3" fill={dev.skin} />
    <circle cx="100.500" cy="42.500" r="3" fill={dev.skin} />
    <circle cx="84" cy="40.500" r="16.800" fill={dev.skin} />
    {#if dev.beard !== 'none'}
      <path
        d="M67.800 43c0.800 9.500 7.200 14.500 16.200 14.500S99.400 52.500 100.200 43c-2.200 4.500-5 6.500-7.500 7-2.200 0.400-4.300-1.400-8.700-1.400s-6.500 1.800-8.700 1.400c-2.500-0.500-5.300-2.500-7.500-7z"
        fill={dev.hair}
        opacity={dev.beard === 'full' ? 0.95 : 0.35}
      />
    {/if}
    {@render fringe()}
    {@render headwear()}
    {@render face()}
  </g>
{/snippet}

<svg
  class="dev {phase} pose-{pose}"
  class:portrait
  viewBox={portrait ? '52 14 64 64' : '0 0 168 128'}
  aria-hidden="true"
  focusable="false"
>
  {#if !portrait}
    <ellipse class="ground" cx="84" cy="118" rx="74" ry="6" />
    <!-- the light of the screen, falling on the desk -->
    <ellipse class="spill" cx="126" cy="86" rx="44" ry="12" />
    <rect class="chair" x="56.500" y="25" width="55" height="60" rx="12" />
    <path class="chair-edge" d="M64 28.500h40" />
  {/if}

  <g class="torso">
    <rect x="59.500" y="59" width="49" height="42" rx="14" fill={dev.cloth} />
    {#if dev.top === 'jacket'}
      <rect x="78" y="60" width="12" height="40" fill={pale(dev.cloth) ? '#2b2f37' : '#d9d4c8'} />
      <path d="M78 60v40M90 60v40" stroke={fold} stroke-width="2" />
      <path d="M84 62v38" stroke="#8b8f98" stroke-width="0.900" stroke-dasharray="1.400 1.400" />
    {/if}
    <rect x="79.500" y="51" width="9" height="11" rx="3" fill={dev.skin} />
    {#if dev.top === 'hoodie'}
      <path d="M69.500 57.500c3 8 26 8 29 0l2.500 5c-5.500 9-28.500 9-34 0z" fill={fold} />
      <path d="M80.500 64.500v9M87.500 64.500v9" stroke={string} stroke-width="1.300" stroke-linecap="round" />
      <circle cx="80.500" cy="74" r="1.100" fill={string} />
      <circle cx="87.500" cy="74" r="1.100" fill={string} />
    {:else if dev.top === 'tee'}
      <path d="M76 59.500q8 6.500 16 0" stroke={fold} stroke-width="2.200" fill="none" />
    {:else}
      <path d="M71 58.500l8.500 6M97 58.500l-8.500 6" stroke={fold} stroke-width="3" stroke-linecap="round" />
    {/if}
  </g>
  {@render head()}

  {#if !portrait}
    <!-- the desk: matte, with a mat under the keyboard -->
    <rect class="leg" x="15" y="100" width="4.500" height="17" rx="1.500" />
    <rect class="leg" x="148.500" y="100" width="4.500" height="17" rx="1.500" />
    <rect class="desk-edge" x="6" y="94" width="156" height="9" rx="3" />
    <rect class="desk" x="6" y="75" width="156" height="23" rx="4" />
    <rect class="mat" x="52" y="79.500" width="66" height="15" rx="3" />

    <!-- what is on the desk -->
    {#if dev.item === 'can' || dev.item === 'cans'}
      {#if dev.item === 'cans'}
        <g transform="translate(-9 3) rotate(-78 30 80)"><rect class="can" x="26" y="74" width="8.500" height="14" rx="2" /><rect class="can-band" x="26" y="79" width="8.500" height="4" /></g>
      {/if}
      <rect class="can" x="26" y="64" width="9" height="17" rx="2.200" />
      <rect class="can-top" x="26.600" y="64" width="7.800" height="2" rx="1" />
      <rect class="can-band" x="26" y="70.500" width="9" height="5" />
    {:else if dev.item === 'mug'}
      <path class="mug-handle" d="M37 72.500q5 2.600 0 6.400" />
      <rect class="mug" x="25" y="68" width="12.500" height="13" rx="2.600" />
      <path class="print" d="M29 72.700l-1.600 1.700 1.600 1.700M33.500 72.700l1.600 1.700-1.600 1.700M32 72.200l-1.800 4.400" />
    {:else if dev.item === 'lamp'}
      <ellipse class="lamp-light" cx="34" cy="83" rx="17" ry="5" />
      <ellipse class="lamp" cx="20" cy="81" rx="7" ry="2.200" />
      <path class="lamp-arm" d="M20 80l5-19 11 5" />
      <path class="lamp" d="M33.500 62.500l9.500 4.500-4.500 5.500z" />
    {:else if dev.item === 'plant'}
      <path class="leaf" d="M32 71c-6-2-8-8-6-12 5 1.500 7 6 6 12z" />
      <path class="leaf dark" d="M32 71c0-7 3-11 7-12 1.500 5-1.500 10-7 12z" />
      <path class="leaf" d="M32 71c-2.500-6-1.500-10 0-13 2.500 3.500 2.500 8 0 13z" />
      <path class="pot" d="M25 70h14l-1.800 11h-10.400z" />
    {/if}

    <!-- the screen -->
    <rect class="metal" x="134" y="72" width="4" height="9" />
    <rect class="metal" x="126" y="79.500" width="20" height="3" rx="1.500" />
    {#if dev.dual}
      <rect class="bezel" x="100" y="50" width="14" height="24" rx="2" transform="rotate(-8 107 62)" />
    {/if}
    <rect class="bezel" x="111" y="41" width="52" height="33" rx="3" />
    <rect class="screen" x="113.500" y="43.500" width="47" height="28" rx="1.500" />
    {#if phase === 'working'}
      <g class="code">
        <path class="k" d="M117 48.500h6" /><path class="f" d="M125 48.500h11" /><path class="p" d="M138 48.500h3" />
        <path class="p" d="M120 53h4" /><path class="s" d="M126 53h14" />
        <path class="k" d="M120 57.500h7" /><path class="f" d="M129 57.500h9" /><path class="p" d="M140 57.500h8" />
        <path class="p" d="M123 62h10" /><path class="k" d="M135 62h5" />
        <path class="p" d="M117 66.500h3" /><rect class="cursor" x="122" y="64.600" width="2.600" height="3.800" />
      </g>
    {:else if phase === 'needs_you'}
      <rect class="ask" x="113.500" y="43.500" width="47" height="28" rx="1.500" />
      <text class="glyph" x="137" y="62.500">[y/n]</text>
    {:else if phase === 'failed'}
      <rect class="oops" x="113.500" y="43.500" width="47" height="28" rx="1.500" />
      <text class="glyph" x="137" y="62.500">err</text>
    {:else if phase === 'done'}
      <path class="tick" d="M129 57.500l5 5 10.500-11" />
    {:else if phase === 'starting'}
      <g class="boot"><circle class="d1" cx="131" cy="57.500" r="1.600" /><circle class="d2" cx="137" cy="57.500" r="1.600" /><circle class="d3" cx="143" cy="57.500" r="1.600" /></g>
    {:else if phase === 'idle'}
      <path class="prompt" d="M118 64.500l2.500 2-2.500 2" /><rect class="cursor dim" x="122" y="64.600" width="2.600" height="3.800" />
    {:else}
      <path class="sheen" d="M120 46l-5 9M128 46l-8 14" />
    {/if}

    <rect class="kb" x="62" y="84" width="44" height="9.500" rx="2" />
    <path class="caps" d="M64.500 87h39M64.500 90.500h39" />
    {#if pose === 'mouse'}
      <rect class="kb" x="110" y="85" width="7" height="10" rx="3.500" />
    {/if}

    <g class="arms" fill="none" stroke={dev.cloth} stroke-width="9" stroke-linecap="round" stroke-linejoin="round">
      {#if pose === 'type' || pose === 'rest'}
        <g class="arm-l"><path d="M66 71q-5 13 8 19.500" /><circle class="hand" cx="76" cy="91" r="4.800" fill={dev.skin} stroke="none" /></g>
        <g class="arm-r"><path d="M102 71q5 13-8 19.500" /><circle class="hand" cx="92" cy="91" r="4.800" fill={dev.skin} stroke="none" /></g>
      {:else if pose === 'sleep'}
        <path d="M66 71q-9 12 6 19 10 3 24 0 15-7 6-19" />
      {:else if pose === 'mouse'}
        <path d="M66 71q-5 13 8 19.500" />
        <circle cx="76" cy="91" r="4.800" fill={dev.skin} stroke="none" />
        <g class="arm-r"><path d="M102 71q11 9 11 19" /><circle cx="113" cy="90.500" r="4.800" fill={dev.skin} stroke="none" /></g>
      {:else if pose === 'think'}
        <path d="M66 71q-6 15 14 20.500" />
        <path d="M102 71q12 6-4-8" />
        <circle cx="96" cy="60" r="4.800" fill={dev.skin} stroke="none" />
      {:else if pose === 'raise'}
        <path d="M66 71q-5 13 8 19.500" />
        <circle cx="76" cy="91" r="4.800" fill={dev.skin} stroke="none" />
        <g class="arm-r wave"><path d="M102 71l13-34" /><circle cx="116" cy="33" r="5.200" fill={dev.skin} stroke="none" /></g>
      {:else if pose === 'relax'}
        <path d="M66 71l-17-17 16-15" />
        <path d="M102 71l17-17-16-15" />
      {:else if pose === 'ohno'}
        <path d="M66 71l-13-14 16-24" />
        <path d="M102 71l13-14-16-24" />
        <circle cx="70" cy="31" r="4.800" fill={dev.skin} stroke="none" />
        <circle cx="98" cy="31" r="4.800" fill={dev.skin} stroke="none" />
      {/if}
    </g>

    <!-- what they say, as a tag rather than a cartoon bubble -->
    {#if phase === 'needs_you' || phase === 'failed' || phase === 'starting'}
      <g class="tag {phase}">
        <path d="M14 6h30a3 3 0 0 1 3 3v14a3 3 0 0 1-3 3H38l-4.500 5.500L30 26H14a3 3 0 0 1-3-3V9a3 3 0 0 1 3-3z" />
        <text x="29" y="20.500">{phase === 'needs_you' ? '?' : phase === 'failed' ? '!' : '…'}</text>
      </g>
    {:else if phase === 'asleep'}
      <text class="zz" x="112" y="34">z</text>
      <text class="zz z2" x="120" y="26">z</text>
    {/if}
  {/if}
</svg>

<style>
  .dev {
    display: block;
    width: 100%;
    height: auto;
    overflow: visible;
    /* What the screen is showing lights the desk and the glasses. */
    --glow: #8fb4ff;
  }
  .needs_you {
    --glow: var(--amber);
  }
  .done {
    --glow: var(--green);
  }
  .failed {
    --glow: var(--red);
  }
  .idle,
  .starting {
    --glow: #6c7a8c;
  }

  .ground {
    fill: rgba(0, 0, 0, 0.45);
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
    fill: #23272e;
  }
  .chair-edge {
    stroke: #343a43;
    stroke-width: 2;
    stroke-linecap: round;
  }
  .desk {
    fill: #2a2f36;
  }
  .desk-edge {
    fill: #1b1f24;
  }
  .leg {
    fill: #15181c;
  }
  .mat {
    fill: #1a1d22;
  }
  .kb {
    fill: #0f1114;
  }
  .caps {
    stroke: #3a414b;
    stroke-width: 2.200;
    stroke-dasharray: 2.600 1.100;
  }
  .metal {
    fill: #0f1114;
  }
  .bezel {
    fill: #0b0d10;
  }
  .screen {
    fill: #10161f;
  }
  .asleep .screen {
    fill: #07090b;
  }
  .sheen {
    stroke: rgba(255, 255, 255, 0.06);
    stroke-width: 3;
    stroke-linecap: round;
  }

  /* code, in the colours an editor uses at night */
  .code path {
    stroke-width: 2.200;
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
    stroke: #6c7a8c;
  }
  .cursor {
    fill: #e6e9ef;
  }
  .cursor.dim {
    fill: #6c7a8c;
  }
  .prompt {
    fill: none;
    stroke: #6c7a8c;
    stroke-width: 1.400;
  }
  .ask {
    fill: var(--amber);
  }
  .oops {
    fill: var(--red);
  }
  .glyph {
    fill: #120d02;
    font-family: 'Geist Mono Variable', monospace;
    font-size: 10px;
    font-weight: 700;
    text-anchor: middle;
  }
  .tick {
    fill: none;
    stroke: var(--green);
    stroke-width: 3;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .boot circle {
    fill: #6c7a8c;
    opacity: 0.4;
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
    stroke-width: 2.400;
  }
  .print {
    fill: none;
    stroke: #c9ccd3;
    stroke-width: 0.900;
    stroke-linecap: round;
    stroke-linejoin: round;
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
  .cans-band {
    fill: none;
    stroke: #121418;
    stroke-width: 2.400;
  }
  .cans-cup {
    fill: #121418;
  }

  /* the face */
  .face .line {
    fill: none;
    stroke-width: 1.700;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .face .line.brow {
    stroke-width: 2;
  }
  .face .line.thin {
    stroke-width: 1.300;
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
    stroke-width: 1.500;
    stroke-linecap: round;
  }
  .face .lens {
    fill: var(--glow);
    fill-opacity: 0.18;
    stroke-width: 1.600;
  }

  /* the tag over someone's head */
  .tag path {
    fill: #2a2f36;
    stroke: #3a414b;
    stroke-width: 1;
  }
  .tag text {
    fill: #e6e9ef;
    font-family: 'Geist Mono Variable', monospace;
    font-size: 15px;
    font-weight: 700;
    text-anchor: middle;
  }
  .tag.needs_you path {
    fill: var(--amber);
    stroke: none;
  }
  .tag.needs_you text {
    fill: #120d02;
  }
  .tag.failed path {
    fill: var(--red);
    stroke: none;
  }
  .tag.failed text {
    fill: #1a0503;
  }
  .zz {
    fill: #6c7a8c;
    font-family: 'Geist Mono Variable', monospace;
    font-size: 11px;
    font-weight: 600;
  }
  .zz.z2 {
    font-size: 9px;
    opacity: 0.7;
  }

  /* poses that hold still */
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
    transform: translate(-0.900px, -1.100px);
  }
  .pose-type .eyes,
  .pose-mouse .eyes {
    transform: translate(1.100px, 0.400px);
  }
  .asleep {
    filter: saturate(0.6) brightness(0.85);
  }
  .wave {
    transform-origin: 102px 71px;
  }
</style>

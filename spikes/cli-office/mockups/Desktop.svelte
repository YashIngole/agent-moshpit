<script lang="ts">
  // What Windows shows while the office runs several terminals: one window, one taskbar
  // button, and each program as a process of its own under it. A diagram, not a copy of
  // Windows: the taskbar and the process list are drawn plainly on purpose.
  import Mock from './Mock.svelte'

  const processes = [
    { name: 'Agent Moshpit', what: 'The window and the tray icon' },
    { name: 'WebView2', what: 'Draws the office and the terminals' },
    { name: 'claude.exe', what: 'Fix the checkout total' },
    { name: 'claude.exe', what: 'Docs pass for the API' },
    { name: 'codex.exe', what: 'Refactor auth middleware' },
    { name: 'codex.exe', what: 'Type the orders API' },
    { name: 'Console Window Host (4)', what: 'One hidden terminal for each of them' }
  ]
</script>

{#snippet moshpit()}
  <svg viewBox="0 0 64 64" aria-hidden="true" focusable="false">
    <rect x="2" y="2" width="60" height="60" rx="14" fill="#15302e" />
    <rect x="19" y="27" width="26" height="18" rx="9" fill="#4c9be8" />
    <circle cx="32" cy="23" r="10.5" fill="#f2c9a1" />
    <path d="M21.5 23a10.5 10.5 0 0 1 21 0c-2.6-4.2-6.1-6.4-10.5-6.4s-7.9 2.2-10.5 6.4z" fill="#2b2b3a" />
    <rect x="10" y="38" width="44" height="15" rx="5" fill="#d0a06e" />
  </svg>
{/snippet}

{#snippet caption(title: string)}
  <div class="caption">
    <span class="name">{title}</span>
    <svg class="controls" viewBox="0 0 96 16" aria-hidden="true">
      <path d="M10 8h10M42 3.5h9v9h-9zM75 3.5l9 9M84 3.5l-9 9" />
    </svg>
  </div>
{/snippet}

<div class="desktop">
  <section class="window office" aria-label="Agent Moshpit">
    <div class="caption">
      <span class="icon">{@render moshpit()}</span>
      <span class="name">Agent Moshpit</span>
      <svg class="controls" viewBox="0 0 96 16" aria-hidden="true">
        <path d="M10 8h10M42 3.5h9v9h-9zM75 3.5l9 9M84 3.5l-9 9" />
      </svg>
    </div>
    <div class="body"><div class="scaled"><Mock screen="split" /></div></div>
  </section>

  <section class="window tasks" aria-label="Task Manager">
    {@render caption('Task Manager')}
    <div class="list">
      <p class="head"><span>Name</span><span>What it is</span></p>
      <p class="group">
        <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M2.5 4.5 6 8l3.5-3.5" /></svg>
        <span class="icon">{@render moshpit()}</span>
        <b>Agent Moshpit ({processes.length + 3})</b>
      </p>
      {#each processes as p, i (i)}
        <p class="row" class:cli={p.name.endsWith('.exe')}><span class="code">{p.name}</span><span>{p.what}</span></p>
      {/each}
    </div>
  </section>

  <p class="note one">Each Claude Code and Codex is a process of its own, listed under Agent Moshpit</p>
  <p class="note two">One button in the taskbar, however many terminals are open</p>

  <footer class="taskbar">
    <div class="apps">
      <span class="app"><svg viewBox="0 0 20 20" aria-hidden="true"><path class="solid" d="M3 3h6v6H3zM11 3h6v6h-6zM3 11h6v6H3zM11 11h6v6h-6z" /></svg></span>
      <span class="app"><svg viewBox="0 0 20 20" aria-hidden="true"><circle cx="9" cy="9" r="5" /><path d="m13 13 4 4" /></svg></span>
      <span class="app"><svg viewBox="0 0 20 20" aria-hidden="true"><path d="M2.5 6V4.5h5l1.5 2h8.500v9h-15z" /></svg></span>
      <span class="app"><svg viewBox="0 0 20 20" aria-hidden="true"><circle cx="10" cy="10" r="7" /><path d="M3 10h14M10 3c-3 3.500-3 10.500 0 14M10 3c3 3.500 3 10.500 0 14" /></svg></span>
      <span class="app"><svg viewBox="0 0 20 20" aria-hidden="true"><path d="m7 6-4 4 4 4M13 6l4 4-4 4" /></svg></span>
      <span class="app running">{@render moshpit()}</span>
    </div>
    <div class="clock"><span>21:42</span><span>08-10-2026</span></div>
  </footer>
</div>

<style>
  .desktop {
    position: relative;
    height: 100%;
    overflow: hidden;
    background: #7f97a3;
    color: #1b2428;
    --edge: #b8c2c8;
  }

  .window {
    position: absolute;
    display: grid;
    grid-template-rows: 34px minmax(0, 1fr);
    overflow: hidden;
    border: 1px solid #5d7480;
    border-radius: 9px;
    background: #f4f6f7;
    box-shadow: 0 2px 4px rgba(20, 35, 45, 0.18), 0 18px 40px -12px rgba(20, 35, 45, 0.5);
  }
  .caption {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 6px 0 12px;
    background: #eef1f3;
    font-size: var(--t-sm);
  }
  .caption .name {
    flex: 1;
  }
  .icon {
    display: block;
    flex: none;
    width: 16px;
    height: 16px;
  }
  .controls {
    width: 96px;
    height: 16px;
    fill: none;
    stroke: #33424a;
    stroke-width: 1.1;
  }

  .office {
    top: 26px;
    left: 34px;
    width: 1030px;
    height: 676px;
  }
  .body {
    min-height: 0;
    overflow: hidden;
  }
  /* The office as it would be at 1430 by 892, seen smaller. */
  .scaled {
    width: 1430px;
    height: 892px;
    zoom: 0.72;
  }

  .tasks {
    top: 392px;
    right: 34px;
    width: 520px;
    height: 372px;
  }
  .list {
    padding: 6px 0 10px;
    background: #fbfcfc;
    font-size: var(--t-sm);
  }
  .list p {
    display: grid;
    grid-template-columns: 244px minmax(0, 1fr);
    align-items: center;
    gap: 12px;
    height: 34px;
    padding: 0 16px;
  }
  .head {
    border-bottom: 1px solid var(--edge);
    font-size: var(--t-xs);
    color: #53646d;
  }
  .list .group {
    display: flex;
    gap: 8px;
    background: #e3ecf1;
  }
  .group svg:first-child {
    width: 12px;
    height: 12px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.5;
  }
  .row .code {
    padding-left: 38px;
    white-space: nowrap;
  }
  .row span:last-child {
    overflow: hidden;
    color: #53646d;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row.cli span:first-child {
    font-weight: 700;
  }

  /* Notes written on the picture, for whoever is reading it. */
  .note {
    position: absolute;
    max-width: 300px;
    padding: 8px 12px;
    border-radius: 8px;
    background: #14232b;
    color: #f4f7f8;
    font-size: var(--t-sm);
    font-weight: 650;
    line-height: 1.35;
  }
  .note::after {
    content: '';
    position: absolute;
    width: 2px;
    background: #14232b;
  }
  .note.one {
    top: 316px;
    right: 150px;
  }
  .note.one::after {
    top: 100%;
    left: 40px;
    height: 20px;
  }
  .note.two {
    bottom: 76px;
    left: 600px;
  }
  .note.two::after {
    top: 100%;
    left: 221px;
    height: 20px;
  }

  .taskbar {
    position: absolute;
    right: 0;
    bottom: 0;
    left: 0;
    display: grid;
    height: 52px;
    place-items: center;
    border-top: 1px solid #c9d2d7;
    background: #e9eef1;
  }
  .apps {
    display: flex;
    gap: 6px;
  }
  .app {
    position: relative;
    display: grid;
    width: 42px;
    height: 42px;
    place-items: center;
    border-radius: 7px;
    color: #4a5c66;
  }
  .app svg {
    width: 22px;
    height: 22px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .app .solid {
    fill: currentColor;
    stroke: none;
  }
  .app.running {
    border: 1px solid #c2ccd2;
    background: #f8fafb;
  }
  .app.running :global(svg) {
    width: 26px;
    height: 26px;
    stroke: none;
  }
  .app.running::after {
    content: '';
    position: absolute;
    bottom: 2px;
    left: 13px;
    width: 16px;
    height: 3px;
    border-radius: 2px;
    background: #2f6f8f;
  }
  .clock {
    position: absolute;
    right: 18px;
    display: grid;
    justify-items: end;
    font-size: var(--t-xs);
    color: #33424a;
  }
</style>

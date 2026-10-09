<script lang="ts">
  // Every key the office answers to, in one list. Everything else typed in a
  // terminal belongs to its program.
  import { onMount } from 'svelte'
  import { office } from '../lib/office.svelte'
  import { voice } from '../lib/voice.svelte'
  import { shortcutLabel } from '../lib/voice'

  let first = $state<HTMLElement>()
  onMount(() => first?.focus())

  const editor = $derived(office.editor?.name)
  const groups: { title: string; keys: [string, string][] }[] = $derived([
    {
      title: 'Anywhere',
      keys: [
        ['Ctrl+`', 'Back to the floor, and back to the terminals'],
        ['Ctrl+Shift+N', 'New agent (n on the floor)'],
        ['Ctrl+Shift+Q', 'Quit, asking first if anyone is busy'],
        ['Ctrl+Q', 'The same, when the keyboard is not in a terminal (there it is the program’s)'],
        ['Ctrl+Shift+/', 'This list'],
        [shortcutLabel(voice.settings.shortcut), 'Start / stop local voice when enabled; configure in More → Voice input']
      ]
    },
    {
      title: 'Terminals',
      keys: [
        ['Alt+1 … Alt+9', 'The keyboard to pane 1 to 9'],
        ['Ctrl+Shift+] / [', 'The keyboard to the next pane, or the one before'],
        ['Ctrl+Shift+Enter', 'Give this pane the room, and put the others back'],
        ['Ctrl+Shift+W', 'Put this pane away; its program keeps running'],
        ['Ctrl+Shift+F', 'Find in what this terminal has shown'],
        ['Ctrl+= / Ctrl+- / Ctrl+0', 'Bigger text, smaller, as usual'],
        ['Ctrl+C', 'Copy, when text is selected; otherwise the program’s'],
        ['Ctrl+V', 'Paste. A picture is pasted as the path of a file holding it'],
        ['Shift+Enter', 'A new line in the program’s prompt'],
        ['Shift+Tab', 'The program’s (Claude Code’s mode switch); the keyboard stays in the terminal']
      ]
    },
    {
      title: 'On the floor',
      keys: [
        ['Arrow keys', 'From desk to desk'],
        ['Enter', 'Open their terminal'],
        ['Ctrl+Enter', 'Open it beside the others, or put it away again'],
        ['F2', 'Rename'],
        ['Delete', 'Take the desk away (it can be brought back for a moment)'],
        ['Shift+F10', 'Everything else that can be done with a desk']
      ]
    },
    {
      title: 'With the mouse',
      keys: [
        ['Ctrl and a click', 'Open a desk beside the others, or put it away again'],
        ['Ctrl and a click on a file path', editor ? `Open it in ${editor}, at its line` : 'Show it in its folder (no editor was found)'],
        ['Ctrl and the wheel', 'Bigger or smaller text, over a terminal'],
        ['Right-click', 'A desk’s menu, or a terminal’s'],
        ['Middle-click', 'Take a desk away'],
        ['Drag a pane’s header', 'Onto another pane, to change places'],
        ['Double-click a pane’s header', 'Give it the room'],
        ['Drop files on a pane', 'Their paths are pasted into it']
      ]
    }
  ])
</script>

<aside class="panel" aria-label="Keyboard shortcuts">
  <header>
    <h2>Keys</h2>
    <button type="button" class="close" aria-label="Close" bind:this={first} onclick={() => office.closePanel()}>
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 3.5l9 9M12.5 3.5l-9 9" /></svg>
    </button>
  </header>
  <div class="body">
    {#each groups as group (group.title)}
      <section>
        <h3>{group.title}</h3>
        <dl>
          {#each group.keys as [key, what] (key)}
            <dt><kbd>{key}</kbd></dt>
            <dd>{what}</dd>
          {/each}
        </dl>
      </section>
    {/each}
  </div>
</aside>

<style>
  .panel {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    width: var(--panel-width);
    min-width: 0;
    min-height: 0;
    border-left: 1px solid var(--line);
    background: var(--panel);
    animation: arrive var(--settle) var(--ease) both;
  }
  @keyframes arrive {
    from {
      transform: translateX(24px);
      opacity: 0;
    }
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--s-4) var(--s-4) var(--s-3);
    border-bottom: 1px solid var(--line);
  }
  h2 {
    font-size: var(--t-xl);
    font-weight: 650;
    letter-spacing: -0.01em;
    line-height: 1.2;
  }
  .close {
    display: grid;
    width: 30px;
    height: 30px;
    margin-right: -6px;
    place-items: center;
    border: 0;
    border-radius: var(--r-1);
    background: transparent;
    color: var(--ink-2);
  }
  .close:hover {
    background: var(--inset);
    color: var(--ink);
  }
  .close svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
  }
  /* One grid for every section, so the keys and what they do line up all the way down. */
  .body {
    display: grid;
    grid-template-columns: fit-content(46%) minmax(0, 1fr);
    align-content: start;
    gap: var(--s-4) var(--s-3);
    padding: var(--s-3) var(--s-4) var(--s-4);
    overflow-y: auto;
  }
  section,
  dl {
    display: grid;
    grid-column: 1 / -1;
    grid-template-columns: subgrid;
  }
  h3 {
    grid-column: 1 / -1;
    margin-bottom: var(--s-2);
    font-family: var(--mono);
    font-size: var(--t-xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--ink-3);
  }
  dl {
    row-gap: 6px;
    margin: 0;
    font-size: var(--t-sm);
    line-height: 1.4;
  }
  dt {
    min-width: 0;
  }
  /* A long key wraps inside its column rather than pushing what it does aside. */
  kbd {
    display: inline-block;
    max-width: 100%;
    white-space: normal;
    overflow-wrap: anywhere;
  }
  dd {
    margin: 0;
    color: var(--ink-2);
  }
  kbd {
    padding: 1px 6px;
    border: 1px solid var(--wall);
    border-radius: 5px;
    font-family: var(--mono);
    font-size: var(--t-xs);
  }
</style>

<script lang="ts">
  // What agents have sent each other, how each agent is told of something new, and how
  // far each thing got. It claims only what the office did or was told: that something
  // was pushed is not that it arrived, and that it was read is not that it was acted on.
  import { onMount } from 'svelte'
  import { bridge } from '../lib/bridge'
  import { office } from '../lib/office.svelte'
  import { DEFAULT_DELIVERY, deliveryWords, linkWords, taskWords, type Mailbox, type MessageDelivery } from '../lib/messages'

  let first = $state<HTMLElement>()
  let box = $state<Mailbox | null>(null)
  let off = $state(false)
  let push = $state<MessageDelivery>({ ...DEFAULT_DELIVERY })

  async function look() {
    const now = await bridge.mailbox().catch(() => null)
    off = now === null
    if (now) {
      box = now
      push = now.push
    }
  }
  onMount(() => {
    first?.focus()
    void look()
    // It changes as agents work: looked at again while the panel is open.
    const again = setInterval(() => void look(), 1500)
    return () => clearInterval(again)
  })

  function set(change: Partial<MessageDelivery>) {
    push = { ...push, ...change }
    bridge.setMessageDelivery($state.snapshot(push))
  }
  const running = $derived(box?.desks.filter(desk => desk.running).length ?? 0)
</script>

<aside class="panel" aria-label="Agent messages">
  <header>
    <h2>Agent messages</h2>
    <button type="button" class="close" aria-label="Close" bind:this={first} onclick={() => office.closePanel()}>
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 3.5l9 9M12.5 3.5l-9 9" /></svg>
    </button>
  </header>

  <div class="body">
    <p>
      Agents in one project can send each other messages and tasks through the office. Nothing is ever typed into a
      terminal: an agent is told by whichever way its program has, and what it has not read is kept for it.
    </p>
    {#if off}
      <p class="note" role="status">The office's connection for agents is off, so there is nothing to show. It is turned off with <code>MOSHPIT_DISABLE_MCP</code>, or by a task history that could not be read.</p>
    {:else}
      <section aria-labelledby="told-heading">
        <h3 id="told-heading">How Claude Code is told</h3>
        <label class="choice">
          <input type="checkbox" checked={push.hooks} onchange={event => set({ hooks: event.currentTarget.checked })} />
          <span><strong>At the next step of its work</strong><small>After a tool call, with your next prompt, and once more as a turn ends, where it keeps the turn going until the message is read. Uses Claude Code's hooks, for the sessions the office starts only.</small></span>
        </label>
        <label class="choice" class:dim={!push.hooks}>
          <input type="checkbox" checked={push.hooks && push.wake} disabled={!push.hooks} onchange={event => set({ wake: event.currentTarget.checked })} />
          <span><strong>Wake it when it is idle</strong><small>An agent sitting at its prompt starts a turn when something comes for it. That turn uses your tokens, and nobody at the keyboard asked for it.</small></span>
        </label>
        <label class="choice">
          <input type="checkbox" checked={push.channels} onchange={event => set({ channels: event.currentTarget.checked })} />
          <span><strong>Push it in as it arrives</strong> <em>research preview</em><small>Uses Claude Code's Channels, which is a preview and may change. Claude Code asks you to confirm in its terminal each time it starts, and says the office's server is a development channel. A pushed message is read, but Claude Code may not record that it was unless the first choice is on as well.</small></span>
        </label>
        <p class="note">These apply to sessions started or carried on from now on; one that is running keeps what it was started with. Codex has no such way: it finds what is new with its next call to the office, or when it checks its inbox.</p>
      </section>

      <section aria-labelledby="reached-heading">
        <h3 id="reached-heading">How each agent is reached</h3>
        {#if !box || box.desks.length === 0}
          <p class="note">No Claude Code or Codex agents yet.</p>
        {:else}
          <ul class="desks">
            {#each box.desks as desk (desk.id)}
              <li>
                <div class="head">
                  <span class="name">{desk.title}</span>
                  <span class="tag">{desk.harness_name}{desk.waiting > 0 ? ` · ${desk.waiting} unread` : ''}</span>
                </div>
                <p class="how">{linkWords(desk)}</p>
                {#if desk.running && desk.link.note}<p class="note">{desk.link.note}</p>{/if}
              </li>
            {/each}
          </ul>
          {#if running === 0}<p class="note">Nobody is running. What is sent to an agent that is not running is kept until it is carried on.</p>{/if}
        {/if}
      </section>

      <section aria-labelledby="sent-heading">
        <h3 id="sent-heading">What was sent</h3>
        {#if !box || box.items.length === 0}
          <p class="note">Nothing yet. Ask an agent to send another one a message, or to hand it a task. In your own words, for example:</p>
          <ul class="ask">
            <li>Start another session in this folder to review your changes, and report what it finds.</li>
            <li>Send these findings to the agent called Fix checkout.</li>
            <li>Check your inbox and tell me what the others said.</li>
          </ul>
        {:else}
          <ul class="items">
            {#each box.items as item (item.id)}
              <li>
                <div class="head">
                  <span class="name">{item.from_title} <span aria-hidden="true">→</span> <span class="aloud">to</span> {item.to_title}</span>
                  <span class="tag">{item.kind}{item.kind === 'task' && taskWords(item.status) ? ` · ${taskWords(item.status)}` : ''}</span>
                </div>
                <p class="text">{item.text}</p>
                <p class="how" class:reached={item.delivery?.state === 'acknowledged' || item.delivery?.state === 'fetched'}>{deliveryWords(item.delivery)}</p>
                {#if item.result}<p class="how">Its result, to {item.from_title}: {deliveryWords(item.result).toLowerCase()}</p>{/if}
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {/if}
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
  .body {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    align-content: start;
    gap: var(--s-3);
    padding: var(--s-3) var(--s-4) var(--s-4);
    overflow-y: auto;
    line-height: 1.45;
  }
  section {
    display: grid;
    gap: var(--s-2);
    padding-top: var(--s-3);
    border-top: 1px solid var(--line);
  }
  h3 {
    font-size: var(--t-md);
    font-weight: 650;
  }
  .note,
  small {
    color: var(--ink-2);
    font-size: var(--t-sm);
  }
  .choice {
    display: flex;
    align-items: flex-start;
    gap: var(--s-2);
  }
  .choice input {
    flex: none;
    width: 16px;
    height: 16px;
    margin: 3px 0 0;
    accent-color: var(--button);
  }
  .choice strong {
    font-weight: 650;
  }
  .choice small {
    display: block;
    margin-top: 2px;
    line-height: 1.45;
  }
  .choice em {
    padding: 0 5px;
    border: 1px solid var(--line);
    border-radius: 4px;
    font-family: var(--mono);
    font-size: var(--t-xs);
    font-style: normal;
    color: var(--ink-2);
    white-space: nowrap;
  }
  /* Off because what it depends on is off: still readable, plainly not available. */
  .choice.dim strong {
    color: var(--ink-2);
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 2px;
    padding: var(--s-2) 0;
    border-bottom: 1px solid var(--line);
  }
  li:last-child {
    border-bottom: 0;
  }
  .head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--s-3);
  }
  .name {
    min-width: 0;
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  .tag {
    flex: none;
    font-family: var(--mono);
    font-size: var(--t-xs);
    color: var(--ink-2);
    white-space: nowrap;
  }
  .text {
    display: -webkit-box;
    overflow: hidden;
    color: var(--ink-2);
    font-size: var(--t-sm);
    overflow-wrap: anywhere;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    user-select: text;
  }
  .how {
    font-size: var(--t-sm);
    color: var(--ink-2);
  }
  /* Things to say to an agent: text to read and copy, not buttons. Nothing here sends anything. */
  .ask li {
    font-size: var(--t-sm);
    user-select: text;
  }
  /* It got there: said in full ink, not in a colour. The four colours mean other things. */
  .how.reached {
    color: var(--ink);
  }
  code {
    font-size: var(--t-xs);
  }
</style>

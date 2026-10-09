<script lang="ts">
  import { onMount, untrack } from 'svelte'
  import { catalogKey, keptCatalog, lines, readCatalog } from '../lib/catalogs'
  import type { Harness, LaunchOptions, ModelCatalog } from '../lib/types'

  let { kind, cwd, options = $bindable({}) }: { kind: Harness; cwd: string; options?: LaunchOptions } = $props()
  let catalog = $state<ModelCatalog | undefined>(untrack(() => keptCatalog(catalogKey(kind.id, cwd, options.profile ?? ''))))
  let loading = $state(false)
  let problem = $state('')
  let customModel = $state(false)
  let advanced = $state(untrack(() => options.permission === 'custom'))
  let serial = 0
  let disposed = false

  const selected = $derived(catalog?.models.find(model => model.id === options.model))
  const efforts = $derived((selected ?? (!options.model ? catalog?.models.find(model => model.id === 'default') : undefined))?.efforts ?? [])
  const permissions = $derived(catalog?.permissions ?? [{ id: '', name: 'Use CLI settings', description: 'Inherits your configured permission settings.' }])
  const permission = $derived(permissions.find(mode => mode.id === (options.permission ?? '')))
  const customId = $derived(Boolean(options.model && !selected))
  const feature = (name: string) => catalog?.features.includes(name) ?? false

  async function load(refresh = false) {
    if (!kind.installed) return
    const ticket = ++serial
    const id = kind.id
    const folder = cwd.trim()
    const profile = options.profile?.trim() ?? ''
    loading = true
    problem = ''
    try {
      const next = await readCatalog(id, folder, profile, refresh)
      if (!disposed && ticket === serial) { catalog = next; problem = next.problem }
    } catch (error) {
      if (!disposed && ticket === serial) problem = typeof error === 'string' ? error : 'The model catalog could not be read. Try Refresh, or enter a model ID.'
    } finally {
      if (!disposed && ticket === serial) loading = false
    }
  }

  $effect(() => {
    const key = catalogKey(kind.id, cwd.trim(), options.profile?.trim() ?? '')
    const installed = kind.installed
    serial += 1
    catalog = keptCatalog(key)
    problem = ''
    const timer = setTimeout(() => { if (installed) void load() }, 350)
    return () => clearTimeout(timer)
  })

  onMount(() => {
    // Long-lived panels pick up new releases too, without polling in the background.
    const timer = setInterval(() => { if (document.visibilityState === 'visible') void load(true) }, 15 * 60_000)
    return () => { disposed = true; serial += 1; clearInterval(timer) }
  })

  function chooseModel(value: string) {
    customModel = value === '__custom__'
    if (customModel) return
    const next = catalog?.models.find(model => model.id === value)
    options = { ...options, model: value, effort: next?.efforts.includes(options.effort ?? '') ? options.effort : '' }
  }

  function choosePermission(value: string) {
    options = { ...options, permission: value, sandbox: value === 'custom' ? options.sandbox : '', approval: value === 'custom' ? options.approval : '' }
    if (value === 'custom') advanced = true
  }

  const setting = (value: string) => value === '' ? null : value === 'on'
</script>

<div class="launch">
  <div class="model-effort">
    <div>
      <div class="label-row">
        <label class="label" for="new-model">Model</label>
        <button type="button" class="refresh" disabled={loading || !kind.installed} aria-label="Refresh available models" onclick={() => void load(true)}>{loading ? 'Loading…' : 'Refresh'}</button>
      </div>
      <select id="new-model" class="field" value={customModel ? '__custom__' : options.model ?? ''} onchange={event => chooseModel(event.currentTarget.value)}>
        <option value="">Use CLI settings</option>
        {#each catalog?.models ?? [] as model (model.id)}<option value={model.id}>{model.name}</option>{/each}
        {#if customId && !customModel}<option value={options.model}>{options.model}</option>{/if}
        <option value="__custom__">Custom model…</option>
      </select>
    </div>
    <div>
      <label class="label" for="new-effort">Effort</label>
      {#if customModel || (customId && !loading)}
        <input id="new-effort" class="field" aria-label="Effort" placeholder="CLI default" value={options.effort ?? ''} maxlength="256" oninput={event => options = { ...options, effort: event.currentTarget.value }} />
      {:else}
        <select id="new-effort" class="field" value={options.effort ?? ''} disabled={!feature('effort') && !options.effort} onchange={event => options = { ...options, effort: event.currentTarget.value }}>
          <option value="">CLI default</option>
          {#each efforts as effort}<option value={effort}>{effort}</option>{/each}
          {#if options.effort && !efforts.includes(options.effort)}<option value={options.effort}>{options.effort} (saved)</option>{/if}
        </select>
      {/if}
    </div>
  </div>
  {#if customModel}
    <div>
      <label class="label" for="new-model-id">Model ID</label>
      <input id="new-model-id" class="field" placeholder="The model ID or alias your CLI accepts" value={options.model ?? ''} maxlength="256" oninput={event => options = { ...options, model: event.currentTarget.value }} />
    </div>
  {/if}
  {#if selected?.description}<p class="hint">{selected.description}</p>{/if}
  {#if problem}<p class="catalog-problem" role="status">{problem}</p>
  {:else}<p class="hint" role="status">{!kind.installed ? 'Model discovery is available after installation. You can enter a model ID now.' : catalog ? 'Models come from your CLI. Refresh to check for new releases.' : 'You can keep your CLI settings or enter a model ID while the catalog loads.'}</p>{/if}

  <div class="permission">
    <label class="label" for="new-permission">Permissions</label>
    <select id="new-permission" class="field" value={options.permission ?? ''} onchange={event => choosePermission(event.currentTarget.value)}>
      {#each permissions as mode (mode.id)}<option value={mode.id} disabled={mode.id === 'auto' && selected?.auto_mode === false}>{mode.name}</option>{/each}
      {#if options.permission && !permission}<option value={options.permission}>{options.permission} (saved)</option>{/if}
    </select>
    {#if options.permission === 'auto' && selected?.auto_mode === false}
      <p class="catalog-problem">Auto is unavailable for this model. Choose another permission mode.</p>
    {:else}<p class="hint">{permission?.description ?? 'This saved setting will be passed to your CLI.'}</p>{/if}
  </div>

  <details bind:open={advanced}>
    <summary>Advanced launch settings</summary>
    <div class="advanced-fields">
      {#if kind.launch === 'codex' && (feature('profile') || options.profile)}
        <div>
          <label class="label" for="new-profile">Codex profile <span>optional</span></label>
          <input id="new-profile" class="field" placeholder="A profile you configured in Codex" value={options.profile ?? ''} oninput={event => options = { ...options, profile: event.currentTarget.value }} />
        </div>
      {/if}
      {#if options.permission === 'custom' && kind.launch === 'codex'}
        <div>
          <label class="label" for="new-sandbox">Sandbox</label>
          <select id="new-sandbox" class="field" value={options.sandbox ?? ''} onchange={event => options = { ...options, sandbox: event.currentTarget.value }}>
            <option value="">Use CLI settings</option><option value="read-only">Read-only</option><option value="workspace-write">Workspace write</option><option value="danger-full-access">Full access</option>
          </select>
        </div>
        <div>
          <label class="label" for="new-approval">Approval policy</label>
          <select id="new-approval" class="field" value={options.approval ?? ''} onchange={event => options = { ...options, approval: event.currentTarget.value }}>
            <option value="">Use CLI settings</option><option value="on-request">Ask when needed</option><option value="never">Never ask; return failures to the agent</option>
          </select>
        </div>
      {/if}
      {#if feature('search') || options.search != null}
        <div>
          <label class="label" for="new-search">Live web search</label>
          <select id="new-search" class="field" value={options.search == null ? '' : options.search ? 'on' : 'off'} onchange={event => options = { ...options, search: setting(event.currentTarget.value) }}>
            <option value="">Use CLI settings</option><option value="on">On</option><option value="off">Off</option>
          </select>
        </div>
      {/if}
      {#if feature('additional_dirs') || options.additional_dirs?.length}
        <div>
          <label class="label" for="new-additional">Additional folders <span>optional</span></label>
          <textarea id="new-additional" class="field" rows="2" placeholder="One folder per line" value={(options.additional_dirs ?? []).join('\n')} onchange={event => options = { ...options, additional_dirs: lines(event.currentTarget.value) }}></textarea>
        </div>
      {/if}
      {#if kind.launch === 'claude'}
        {#if feature('tools') || options.allowed_tools?.length || options.disallowed_tools?.length}
          <div>
            <label class="label" for="new-allowed">Allow without prompting <span>optional</span></label>
            <textarea id="new-allowed" class="field" rows="2" placeholder="One tool rule per line, e.g. Bash(git diff *)" value={(options.allowed_tools ?? []).join('\n')} onchange={event => options = { ...options, allowed_tools: lines(event.currentTarget.value) }}></textarea>
          </div>
          <div>
            <label class="label" for="new-denied">Deny tools <span>optional</span></label>
            <textarea id="new-denied" class="field" rows="2" placeholder="One tool rule per line" value={(options.disallowed_tools ?? []).join('\n')} onchange={event => options = { ...options, disallowed_tools: lines(event.currentTarget.value) }}></textarea>
          </div>
        {/if}
        {#if feature('chrome') || options.chrome != null}
          <div>
            <label class="label" for="new-chrome">Claude in Chrome</label>
            <select id="new-chrome" class="field" value={options.chrome == null ? '' : options.chrome ? 'on' : 'off'} onchange={event => options = { ...options, chrome: setting(event.currentTarget.value) }}>
              <option value="">Use CLI settings</option><option value="on">On</option><option value="off">Off</option>
            </select>
          </div>
        {/if}
        {#if feature('instructions') || options.instructions}
          <div>
            <label class="label" for="new-instructions">Extra instructions <span>optional</span></label>
            <textarea id="new-instructions" class="field" rows="3" maxlength="16000" placeholder="Added to Claude's usual instructions" value={options.instructions ?? ''} oninput={event => options = { ...options, instructions: event.currentTarget.value }}></textarea>
          </div>
        {/if}
      {/if}
      <p class="hint">These settings apply to this desk when it starts or carries on. You can also change settings in its terminal.</p>
      <button type="button" class="button quiet reset" onclick={() => { options = {}; customModel = false }}>Reset to CLI settings</button>
    </div>
  </details>
</div>

<style>
  .launch { display: grid; gap: var(--s-2); min-width: 0; }
  .model-effort { display: grid; grid-template-columns: minmax(0, 1.8fr) minmax(0, 1fr); gap: var(--s-2); align-items: end; }
  .model-effort > div { min-width: 0; }
  .label-row { display: flex; align-items: baseline; justify-content: space-between; gap: var(--s-1); }
  .refresh { padding: 0; border: 0; background: none; color: var(--ink-2); font-size: var(--t-xs); text-decoration: underline; text-underline-offset: 3px; }
  .refresh:hover:not(:disabled) { color: var(--ink); }
  .refresh:disabled { opacity: 0.6; text-decoration: none; }
  select { color-scheme: dark; padding-right: var(--s-3); text-overflow: ellipsis; }
  .permission { margin-top: var(--s-2); }
  .catalog-problem { font-size: var(--t-sm); line-height: 1.45; color: var(--ink-2); user-select: text; }
  details { margin-top: var(--s-1); }
  summary { width: fit-content; font-size: var(--t-sm); color: var(--ink-2); cursor: pointer; }
  summary:hover { color: var(--ink); }
  .advanced-fields { display: grid; gap: var(--s-3); padding-top: var(--s-3); }
  .advanced-fields .label span { font-weight: 400; color: var(--ink-2); }
  .reset { justify-self: start; }
</style>

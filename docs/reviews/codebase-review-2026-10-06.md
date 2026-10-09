# Agent Moshpit — codebase review, 2026-10-06

- **Commit:** none. `main` has no commits yet; every file is untracked.
- **Toolchain:** cargo 1.99.0, vitest 5.0.3, vite 8.3.2, svelte-check 4.
- **Scope:**
  - `src-tauri/src` (~5,000 LOC of Rust)
  - `src/` (Svelte 5, ~4,500 LOC)
  - `tools/` (mock gateway, e2e)
  - `.github/workflows`, Tauri config and capabilities

  Read-only review. One probe test was injected into `office.rs`, run, and removed; the restored file was checked against its sha256.

## Baseline gates

| Gate | Result |
| --- | --- |
| `npm run check` (svelte-check) | 195 files, 0 errors, 0 warnings |
| `npm test` (vitest) | 4 files, 27/27 pass |
| `npm run build` | OK, 130 kB JS (43.8 kB gzip) |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo test --lib` | 37 pass, 1 ignored (opens a real terminal) |
| `npm audit` | 0 vulnerabilities |
| Secrets / personal paths in tree | none found |

The e2e suites (`test:app`, `test:models`, the real-Hermes ones) were not run, because they need a debug Tauri build.

## Verdict

This is a well-built small app.
- **Architecture:** clear and sound. A pure reducer (`office.rs`), one engine task that owns all I/O and timers, a disposable webview fed by snapshots, and a minimal IPC surface (`core:default` only).
- **Hygiene:** blocking HTTP runs on `spawn_blocking` with timeouts, there are no panics on gateway input, and keys never reach logs or error strings.
- **Docs:** honest about what was and wasn't tested.

The bugs cluster in three themes:

1. **Prompts are identified by agent, not by request.** The UI sends "approve for agent X". The core resolves that to *whatever* prompt X has at that instant, and `answered()` clears *whatever* is current. Every finding about approvals and questions comes from this. It matters most because approvals gate shell commands.
2. **The reconnect path is not the first-connect path.** `Relinked` does less than `LinkUp`: it never launches waiting desks and never tombstones dismissed sessions. Several in-flight results also skip the generation check.
3. **The tests cover the happy path and pure helpers.** The engine has 2 unit tests. The frontend tests cover only formatting helpers. The mock gateway never emits multi-question, sudo, secret or expiry flows. Every reproduced bug below sits in that untested gap.

## Findings

### High

**H1. An approval click can resolve a different request than the one on screen.** Reproduced at the reducer level.

- `src/components/PromptCard.svelte:46`: `bridge.approve(agent.id, choice)` sends no request id.
- `src-tauri/src/engine.rs:929-935`: `Cmd::Approve` looks up `self.pending(&agent)` (`:1205-1209`), which returns the current `prompt.request_id`.
- `src-tauri/src/office.rs:654-659`: `approval.request` replaces the current prompt unconditionally.

If a second approval for the same agent arrives between render and click (Hermes sends one per tool call), "once" or "always" approves the new command, which the user may not have read. The card isn't remounted because it is `{#key current.id}` (the agent), and the `$effect` reset clears `busy`, so the button becomes live again.

**Fix:** carry `request_id` from the rendered card through `approve`/`answer`/`secret`. The core rejects a mismatch with "This request changed — check it again". Optionally ignore clicks for about 400 ms after the request id changes.

**H2. Answering a question can wipe a newer approval from the screen while Hermes keeps waiting for it.** Reproduced.

- `engine.rs:1219`: `respond` sends `Msg::Answered { id: agent }` without a request id.
- `office.rs:419-425`: `answered()` takes the *current* prompt's request id and clears it.

Probe sequence: clarify `r10` (two questions), then approval `r11` arrives, then the reply to `r10` lands.

```
PROBE_B prompt_rid=<<<r11>>> queued=0
PROBE_C after_answered prompt=<<<None>>> phase=Idle
```

The desk shows Idle while Hermes blocks on `r11`. The prompt only comes back if a reconcile refetches it, and `approval.pending` is only fetched for adopted sessions (`engine.rs:905`). The agent hangs until the approval times out.

**Fix:** make `Msg::Answered` carry the `request_id`, and have `answered()` act only when it matches.

**H3. The second question of a multi-question clarify request is stuck with disabled buttons.** Reproduced in the core; the UI effect was confirmed by reading.

- `office.rs:289-293`: `next_question` reuses the same `request_id`. Probe output: `q1_rid=<<<r9>>> q2_rid=<<<r9>>> same=true`.
- `PromptCard.svelte:21-29`: the reset `$effect` is keyed on `request_id` only, so it doesn't fire.
- `PromptCard.svelte:34-44`: `act()` clears `busy` only on error.

Question 2 renders with `busy='answer'`, so every choice and Send button is `disabled={!!busy}`. The agent waits until `QUESTION_TIMEOUT_MS`. The mock gateway never sends a `questions` array (`tools/mock-gateway/server.mjs:190`), so the e2e tests can't catch this.

**Fix:** key the reset on `request_id + question_id`, and clear `busy` after a successful call.

**H4. Dismissing a desk that is still starting does not cancel the agent; its first prompt still runs.** Confirmed by reading.

- `engine.rs:983-1000`: Dismiss removes the desk. It only sends `session.close` when a `sid` already exists, and while starting there is none.
- `engine.rs:797-802`: the in-flight launch then sends `Attached`. `office.attach` does nothing for the unknown id (via `change()` at `office.rs:495-497`), but `submit(id, sid, text)` still runs (`engine.rs:1148`, which never checks the office).
- Hermes then executes the task the user cancelled, possibly editing files. The next `apply_live` adopts the session back as a foreign desk (`office.rs:788-801`).

**Fix:** in `Attached`, if `office.get(&id)` is `None`, call `session.close` and skip `then_say`.

**H5. A desk created while reconnecting stays on "Getting set up" forever.** Confirmed by reading.

- `engine.rs:1089-1097`: with no gateway, the desk goes to `waiting`. Hermes is restarted only when the state is `Offline | Resting`, not `Reconnecting`.
- Only `LinkUp` drains `waiting` (`engine.rs:711-713`). `Relinked Ok` (`:762-777`) does not.
- The stuck desk also blocks the idle shutdown, because `waiting` is non-empty.

**Fix:** drain `waiting` in `Relinked Ok`. Better still, share one "went online" function between `LinkUp` and `Relinked`.

**H6. On Linux and macOS, Hermes can outlive the app, which contradicts README:126.** Confirmed by reading.

- `proctree.rs:89-101`: sends `SIGTERM` to the group, and leaves the `SIGKILL` to a detached thread that sleeps 3 s.
- `lib.rs:262-265`: `quit_now` calls `app.exit(0)` immediately after `shutdown()`, so the `SIGKILL` thread dies with the process.
- With `panic = "abort"` (`Cargo.toml`), a crash or a `SIGKILL` of the app runs no clean-up at all, and there is no `PR_SET_PDEATHSIG` or parent-death watchdog.
- Windows is fine: `KILL_ON_JOB_CLOSE` (`proctree.rs:34-46`).

**Fix:**
- Wait for the group to exit (with a deadline) before `exit`.
- Linux: `prctl(PR_SET_PDEATHSIG)` in `pre_exec`.
- Both: hold a stdin pipe to Hermes and treat EOF as "parent gone", if Hermes supports it.
- Until then, soften the README claim.

**H7. `release.yml` targets a retired runner.** Confirmed by reading.

`.github/workflows/release.yml:26` uses `macos-13`, which GitHub retired in Dec 2025. With `fail-fast: false`, the draft release silently ships without an Intel Mac build.

**Fix:** build `x86_64-apple-darwin` on `macos-latest` (line 45 already installs both targets).

### Medium

**M1. An approval request erases a question that is still pending.** Reproduced: `PROBE_B queued=0 qid=<<<>>>` after the approval arrived over an open two-question clarify.

`office.rs:654-658` and `:702-705` clear `queued` and overwrite the single prompt slot. The question is never refetched (reconcile only fetches approvals), so the agent waits until it times out.

**Fix:** keep a per-agent prompt queue keyed by request id.

**M2. Dismissing while reconnecting resurrects the desk.**
- `engine.rs:984-990`: close is sent only when a gateway exists, and its result is discarded (`let _ =`).
- The next `apply_live` adopts the leftover session as a new "elsewhere" desk.

**Fix:** keep a tombstone set of dismissed sids that `apply_live` skips, and retry the close after relinking.

**M3. A crash loop is never capped.**
- `restarts` is reset in `live()` (`engine.rs:889-894`).
- `LinkUp` sets the first reconcile to *now* (`:710`), so it runs within milliseconds of every connect.
- `RESTART_AFTER` is fixed, with no backoff.

A Hermes that dies 5 s after start is therefore restarted forever.

**Fix:** reset `restarts` only after a minimum uptime, and back off exponentially.

**M4. A dead connection may never be noticed.**
- `gateway.rs:184-207`: after missed pings, `keep_alive` only `close()`s the writer.
- `Incoming::Closed` comes only from the reader. A wedged peer with the TCP connection still up leaves the engine `Online` while every request fails.

This is unverified at runtime.

**Fix:** have the heartbeat failure itself emit `Closed`, drain `pending`, and abort the reader.

**M5. A corrupt `desks.json` is silently replaced.**
- `lib.rs:105-107`: a parse error becomes `unwrap_or_default()` (an empty list), and the next save overwrites the bad file.
- Save errors are swallowed (`lib.rs:98-102`).

**Fix:** move the bad file aside and tell the user.

**M6. Saves are armed only on phase transitions.** `engine.rs:1276-1278`. Changes to title, key, model or branch that don't move the phase (`office.rs:805-813`) are lost after a crash.

**Fix:** add a `persist` flag to `Outcome`.

**M7. Hermes chooses what runs in the terminal.**
- `terminal.rs:23-29`: any program allowed by the character list is accepted from Hermes's `cli_command`, including `\\host\share\x.exe`.
- `terminal.rs:31-34,45`: the launcher path is quoted only when it contains a space, so a `&` in a Windows username breaks the `cmd /K` line.

Shell metacharacters in the *command* are correctly rejected.

**Fix:**
- Allow-list the program (`hermes …`, `claude setup-token`) and reject leading `\\`.
- Always quote the launcher path, and refuse paths that contain `&^%!`.

**M8. PATH search can run `hermes.exe` from the current folder.** `backend.rs:66-72`: an empty or relative entry from `split_paths` (a trailing `;` is common) resolves against the current directory. It also never finds `.cmd` shims.

**Fix:** skip non-absolute entries, and consider `PATHEXT`.

**M9. Actions are pinned by tag, with write access.**
- `release.yml` (`contents: write`) uses `tauri-action@v0` and `rust-toolchain@stable`.
- `ci.yml` has no `permissions:` block.
- Release runs no tests and doesn't check that the tag matches `tauri.conf.json`/`Cargo.toml`.

**Fix:** pin to full commit SHAs, use `contents: read` in CI, add `needs: ci` and a version check.

**M10. An older snapshot can replace a newer one.** `src/lib/office.svelte.ts:119-121,139-145`: the invoke reply and the pushed `office:snapshot` are accepted in any order, and `now_ms` is never compared.

**Fix:** drop any snapshot older than the last one taken.

**M11. Room lines arriving while a room opens are dropped.** `office.svelte.ts:173,186`: `#roomEvent` discards events while `room` is `null`, even though the core already streams them (`engine.rs:1239`).

**Fix:** buffer them and replay after the history arrives.

### Low

| Where | Issue |
| --- | --- |
| `engine.rs:797-839` | `Attached`/`Activated`/`Approvals`/`Failed`/`Answered` carry no `gen`; results from a dead link are applied to the new one |
| `engine.rs:699,841` | A stale `LinkUp`/`Relinked` gateway is never `close()`d (it leaks its ping and forward tasks under `MOSHPIT_GATEWAY_URL`) |
| `engine.rs:930` | While reconnecting, approve says "no longer waiting" even though the prompt is kept |
| `office.rs:926-929` | A recovered prompt gets `asked_ms = now`, so the countdown restarts; only the first pending approval is used |
| `office.rs:559-561` | The whole draft (up to 24 KB) is cloned on every `message.delta` |
| `engine.rs:1079,1362` | Synchronous `is_dir` and file write on the engine task |
| `engine.rs:1465`, `Cargo.toml` | `MOSHPIT_GATEWAY_URL` is honoured in release builds and accepts non-loopback hosts; `wss://` is mapped but no TLS crate is compiled in |
| `models.rs:183` vs `lib.rs:400` | `http://` verification URLs are accepted but never opened |
| `lib.rs:444` | A second launch with `--hidden` still opens the window |
| `backend.rs:119-120` | Brief window between spawn and job assignment; use `CREATE_SUSPENDED` |
| `RoomPanel.svelte:86-118` | `remove()` has no busy guard; `stop()` state persists across desk switches |
| `AppMenu.svelte:71-122` | `role="menu"` without menu keyboard behaviour |
| `tools/e2e/ui-demo.mjs:42` | Browser launch is outside `try`, so the preview server leaks on failure |

## Test-suite shape

- **Engine (1,600 LOC):** 2 unit tests, both pure helpers (`engine.rs:1580-1599`). Reconnect, rest, restart, dismiss, waiting and approvals are untested; H1, H2, H4, H5 and M1–M3 live here.
- **Office tests:** happy-path transitions only. Nothing covers events for unknown or mid-attach sessions, overlapping prompts, `answered` racing a new prompt, or remove followed by `apply_live`.
- **Frontend vitest:** `rich`, `words`, `look` and `time` only. There are no tests for `office.svelte.ts` (`#take`, `#roomEvent`) or for the component reset logic.
- **Mock gateway:** never sends `questions[]`, `sudo.request`, `secret.request`, `message.interim` or an expiring prompt, and its `approval.respond` doesn't check the session. Those flows are covered only by real Hermes, which the README says was not exercised for approvals.
- **Positives:** the e2e scripts do exit non-zero on failure, and every instance is isolated with `MOSHPIT_INSTANCE`.

## What's genuinely well done

- **Stale-message guard:** a generation counter drops messages from torn-down connections, and the backend exit watcher is generation-scoped (`engine.rs:291`, `:1489`).
- **No pending-request leaks:** requests are removed on timeout, send failure and close (`gateway.rs:97-100,167,174`).
- **No panics on Hermes input:** every field is read with `.get().and_then().unwrap_or`.
- **IPC surface:** only `core:default`; `open_terminal` takes a provider id, not a command string, and `open_web_page` allows `https` only with control characters rejected (`lib.rs:398-404`).
- **Keys:** `Cmd` has no `Debug` derive, and key errors never echo the key (`engine.rs:57,1405-1431`). The key field is cleared on success and on provider switch (`Models.svelte:90,116`).
- **XSS-safe by design:** `rich.ts` builds no HTML, `Rich.svelte` renders text nodes, and there is a test for it (`rich.test.ts:21`).
- **`types.ts`** mirrors `model.rs` exactly, with no schema drift.
- **Idle shutdown** re-checks its condition when it fires, and new work cancels it (`engine.rs:389-402,1364`).
- **Windows job object** with `KILL_ON_JOB_CLOSE`; `MOSHPIT_INSTANCE` limited to 24 alphanumeric characters; atomic temp-then-rename writes.
- **README** is unusually candid about what was and wasn't tested.

## Suggested order of work

1. **Prompt identity (H1, H2, H3, M1).**
   - Thread `request_id` (and `question_id`) through `approve`/`answer`/`secret`, `Msg::Answered` and `office.answered`, rejecting mismatches.
   - Turn the single prompt slot into a small queue, and fix the PromptCard reset key and `busy`.
   - Add office tests for overlapping prompts, and teach the mock gateway `questions[]`, sudo, secret and expiry.
2. **One "went online" path (H4, H5, M2, M3, Low gen/close items).**
   - Unify `LinkUp` and `Relinked`, add `gen` to in-flight results, tombstone dismissed sids, close orphaned sessions in `Attached`, and back off restarts.
   - Add engine tests with a fake gateway.
3. **Process lifetime on Unix (H6),** or soften the README until it is fixed.
4. **CI/release (H7, M9).**
5. **Persistence (M5, M6)** and **terminal/PATH hardening (M7, M8).**
6. **Frontend races (M10, M11)** and the Low table.

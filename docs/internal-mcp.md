# Agents coordinating through Moshpit

Agent Moshpit connects its launched Claude Code and Codex sessions to a local
MCP server. Their conversations, accounts, tools and permission prompts still
belong to their CLIs. Moshpit provides the operations that belong to the office.

Ask an agent:

> Implement the API here. Start a Codex session in this directory to write the
> tests, give it responsibility for the test files, and bring its result back.

The agent can discover the installed programs, start a second desk with its
task, and read the explicit result. The new session inherits the caller's
directory. `worktree: true` asks the selected CLI to create a worktree instead;
the connection reports its actual directory so later delegation follows it.

## Tools

| Tool | Purpose |
| --- | --- |
| `get_context` | Own desk, directory, name lock, parent, inbox and available programs |
| `rename_session` | Give your own desk a concise title when the user has not named it |
| `set_activity` | Describe current work on the existing desk |
| `list_sessions` | Discover desks in this project and their delegation relationships |
| `start_session` | Launch Claude Code or Codex with a delegated task |
| `send_task` | Queue a task for another connected session in this project |
| `check_inbox` | Read previews of assigned tasks and results; use `get_task` for full text |
| `accept_task` | Mark an assigned inbox task as in progress |
| `get_task` | Read the task and its result, optionally waiting up to 25 seconds |
| `report_result` | Report completion, a blocker, or failure with summary, files and validation |
| `request_attention` | Notify the user at your desk about a blocker |

User names always win, whether supplied at launch or changed manually. Clearing
a name through the normal rename control allows automatic naming again. Agent
names change Moshpit's desk title; they do not rename a CLI conversation.
Activity descriptions supplement the observed process status. They cannot hide
a CLI approval prompt or make a stopped program look alive.

`start_session` and `send_task` require a `request_key`, chosen by the agent for
that operation. Repeating the same request returns the same task and session;
reusing its key for different arguments is rejected. A failed launch produces
a failed task; use a new key when deliberately retrying the launch.

Tasks for existing sessions are delivered through their MCP inbox. They never
enter terminal input, wake a sleeping session, interrupt work, or answer an
approval dialog. The recipient needs to check its inbox between stages of work.
The initial task for a new session is handed to the CLI at launch, so that
workflow starts immediately. Agents retrieve results with `get_task` or
`check_inbox`; this version does not push messages into idle model turns.

Completion requires `report_result`. Terminal silence or a process exit is not
treated as successful work. A blocked task can be accepted again; completed or
failed results are final. Only the assigned session may update a task, and only
its requester and assignee can read its full contents.

## Local connection and storage

The running desktop owns a socket bound to `127.0.0.1` on an ephemeral port.
Each supported child receives a random capability in its environment. The CLI
launches `agent-moshpit --mcp` as its stdio MCP subprocess, which forwards tool
calls to that office. This mode runs before desktop or single-instance setup;
it does not create another window or office. Capabilities are rotated on resume
and revoked on stop, removal, process exit or app shutdown. They are not stored
in task history or put in command-line arguments.

MCP configuration is supplied only for that launch. Global settings, project
configuration, sign-ins and existing permission policies are not edited.
Codex uses its per-launch `-c` settings and environment forwarding. Claude Code
uses `--mcp-config` with environment variable expansion. Both keep their other
MCP servers. A custom replacement for either program must understand that
program's native launch flags. Wrappers that can only be started through a
shell keep working without MCP injection, and MCP delegation to them is rejected.
Other CLI adapters remain usable without this integration.

Discovery and messaging are scoped to the caller's project. Git worktrees and
subfolders share their common Git directory; unrelated folders with the same
name are separate. New sessions can only inherit the caller's directory, with
the optional CLI worktree choice. This is coordination scope, not a replacement
for the CLI's filesystem sandbox or permission policy.

There are limits of four unfinished tasks per requester, eight unfinished
spawned tasks across the office, three delegation levels, and twenty running
desks when starting through MCP. Agree on file ownership before parallel edits
in a shared directory; file claiming and automatic merge coordination are not
implemented in this version.

Tasks and results are saved in `coordination.json` beside `desks.json`, with up
to 256 records. Finished records are pruned first as new tasks are added. Writes
replace the journal atomically after flushing it. Unreadable history is
preserved and shown as a startup problem; MCP stays disabled until it is repaired.
The MCP journal does not include terminal transcripts or session capabilities, but does
include delegated prompts and results, which may contain project information.
Set `MOSHPIT_DISABLE_MCP` before launching the office to disable this integration.

## Verification

Rust tests cover name protection, observed status, inbox ownership, task
transitions, retries, journal recovery, project scope, worktree identity, framed
messages and revoked capabilities. The Windows desktop test starts an isolated
office, replaces both native CLIs with stand-ins, and consumes the real launch
configuration to run the shipped MCP subprocess. It verifies new-session
delegation, the same directory, inbox follow-ups, result handoff, protected names
and reconnection after stopping a desk, without making model calls.
CI runs that desktop test against the Windows release executable after building it.

```powershell
npm ci
npm run check
npm test
npm run tauri build -- --debug --no-bundle
node tools/e2e/mcp.mjs
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

When building alongside other worktrees, use a separate `CARGO_TARGET_DIR`.
The desktop test accepts `MOSHPIT_APP`; its default is this worktree's
`src-tauri/target/debug/agent-moshpit.exe`.

Configuration references: [Codex MCP](https://learn.chatgpt.com/docs/extend/mcp?surface=cli),
[Claude Code MCP](https://code.claude.com/docs/en/mcp), and
[MCP transports](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports).

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
| `get_context` | Own desk, directory, name lock, parent, inbox, available programs, and how new items reach this session |
| `rename_session` | Give your own desk a concise title when the user has not named it |
| `set_activity` | Describe current work on the existing desk |
| `list_sessions` | Discover desks in this project, their delegation relationships, and whether each one's connection is live |
| `start_session` | Launch another session of the caller's own program with a delegated task |
| `send_task` | Queue work to be done for another connected session in this project |
| `send_message` | Send another session something that is not work to be done: a finding, a question, an answer |
| `get_message` | Read a message in full, with how far it got |
| `acknowledge` | Say that a message, an assigned task, or a result has been seen |
| `check_inbox` | Read what is waiting: messages, assigned tasks and results, and how far the messages you sent got |
| `accept_task` | Mark an assigned inbox task as in progress |
| `get_task` | Read the task and its result, optionally waiting up to 25 seconds |
| `report_result` | Report completion, a blocker, or failure with summary, files and validation |
| `request_attention` | Notify the user at your desk about a blocker |

User names always win, whether supplied at launch or changed manually. Clearing
a name through the normal rename control allows automatic naming again. Agent
names change Moshpit's desk title; they do not rename a CLI conversation.
Activity descriptions supplement the observed process status. They cannot hide
a CLI approval prompt or make a stopped program look alive. What an agent said
it was doing lasts for that stretch of work: it is still shown when the work is
done, and is cleared when the next stretch begins.

A desk still called by its program and folder is named without the agent's
help too: after the first prompt it takes what Claude Code or Codex calls the
conversation, once. `/rename` in the program is the user's name, and is kept.

Claude Code is started with the tools that act on its own desk already allowed
(`get_context`, `rename_session`, `set_activity`, `list_sessions`,
`check_inbox`, `accept_task`, `get_task`, `report_result`, `get_message`,
`acknowledge`), so an agent naming its desk, reading its inbox or saying that
a message arrived does not stop to ask. Starting another session, sending a
task or a message and asking for attention are not among them: those are still
the program's, and yours, to allow. `list_sessions` shows other desks' names,
programs and relationships, not how they were launched.

`start_session`, `send_task` and `send_message` require a `request_key`, chosen
by the agent for that operation. Repeating the same request returns the same
task, session or message; reusing its key for different arguments is rejected.
A failed launch produces a failed task; use a new key when deliberately
retrying the launch.

Completion requires `report_result`. Terminal silence or a process exit is not
treated as successful work. A blocked task can be accepted again; completed or
failed results are final. Only the assigned session may update a task, and only
its requester and assignee can read its full contents.

## Messages, and what is known about each

A task is work to be done, and counts against the four a session may have
unfinished. A message is anything else one agent has for another: a finding, a
question, an answer. `send_message` keeps it until it is read; `reply_to` names
the message or task it answers. Only its sender and its recipient can read it.

For every message, every task sent to a running session, and every result, the
office records four things, each a fact about what it did or was told and none a
guess about what a model took in:

| State | What happened |
| --- | --- |
| saved | It is in the journal on disk |
| notified | The recipient's session was told it is there, and by which way |
| fetched | Its whole text was handed to the recipient through a tool call |
| acknowledged | The recipient called `acknowledge` with its id |

The sender sees them in the answer to its own `send_message` or `send_task`, in
`sent_messages` from `check_inbox`, and with `get_message`. You see them under
**More → Agent messages**. Acknowledging is a receipt and nothing more: it
accepts no task and promises no work. A task's own state (queued, in progress,
completed, blocked, failed) is still changed only by `accept_task` and
`report_result`. A result reported again after it was acknowledged is a new
thing to acknowledge.

## How a session is told

Nothing is ever typed into a terminal, and nothing answers an approval prompt.
The initial task for a new session is handed to the CLI at launch. Everything
after that is told in one of these ways, and what has not been read is kept:

| Way | For | What it does |
| --- | --- | --- |
| With a tool's answer | Claude Code and Codex | Any call to the office comes back with `agent_moshpit_inbox`, a note naming what is new. Still unread five minutes later, it is said again with the next call. `check_inbox` lists it. Always on. |
| Hooks | Claude Code 2.1.163 and later | After each batch of tool calls and with each prompt, a hook asks the office what is new and adds it as context. As a turn ends, what is still unread is said once more, which keeps the turn going until it is read. On unless turned off. |
| Waking | Claude Code, with hooks | One more hook waits while the session sits idle at its prompt, and starts a turn when something comes for it. That turn uses the user's tokens. On unless turned off. |
| Pushes | Claude Code 2.1.80 and later | The session is started with Claude Code's Channels research preview, and new items are written into it as they come. **Off unless turned on.** |

They are set under **More → Agent messages**, and apply to sessions started or
carried on afterwards; a running session keeps what it was started with. Codex
has no hooks or pushes the office can use, so a Codex session hears with the
answer to its next call to the office, or when it checks its inbox.

Each way is added to that one launch: the hooks as inline `--settings`, merged
into settings the desk already passes (its own hooks stay, and run first);
pushes as `--dangerously-load-development-channels server:agent_moshpit`. No
settings file is written or edited. A launch is left as it was, and the panel
says why, when the program's version is not known or is too old, when the desk
passes a settings *file* of its own (hooks), or sets channels of its own
(pushes). The hooks are this program started again with `--mcp-hook` or
`--mcp-wait`, without a shell; one that cannot reach the office prints nothing
and succeeds, so it can never hold a session up.

What a note or a push says is another agent's words, not the user's. Each one
says so, and says that it grants no permission. That an agent reads and
acknowledges what reaches it is stated as the office's own rule, in the
server's instructions; what the item asks is left for the agent to weigh
against what the user asked of it.

### What pushes can and cannot do

Pushes are Claude Code's Channels preview, and are kept off by default for that
reason:

- Claude Code asks you to confirm "Loading development channels" in the
  terminal each time such a session starts, and under its notice it may print
  `server:agent_moshpit · no MCP server configured with that name` although
  pushes arrive.
- A push that was written is not a push that arrived. The office records it as
  notified by `channel` and shows "Not known to have arrived" until the item is
  fetched or acknowledged. Unread for 45 seconds, it is said again by a hook or
  with a tool's answer.
- With pushes alone, Claude Code read the pushed message but did not
  acknowledge it in any of four trials: it treats pushed text as coming from
  outside, and takes no instruction from it. With hooks on as well it was
  acknowledged every time. Keep "At the next step of its work" on if you want
  receipts.
- A session stopped on a permission question is not disturbed: the question
  stays as it was, and the item is read after you answer it.

## Limits

There are limits of four unfinished tasks per requester, eight unfinished
spawned tasks across the office, three delegation levels, and twenty running
desks when starting through MCP. One session may have sixteen unacknowledged
messages waiting with another. The office answers sixteen calls at a time and
holds up to ninety-six connections open for watching and waiting. A note names
up to eight items and shows the first 240 characters of each; a push carries
the first 800; `check_inbox` shows the first 512. The whole text is read with
`get_message` or `get_task`.

An idle session with waking turned off is not told until its next prompt. A
desk whose program is not running is told when it is carried on. The hook that
waits is ended by Claude Code after six hours, after which an idle session is
not woken until it has taken another turn. None of the ways is certain, which is
why agents are still told to check their inbox between stages of work.

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
name are separate. A delegated session is always the caller's own program: started
from the same harness row, reading the same CLI configuration, and inheriting the
caller's launch settings, permissions and sandbox included, so delegation does not
loosen them. A session's permission boundary also comes from its harness row's
arguments and its CLI's own configuration, which Moshpit cannot read or translate into
another program's terms, so Codex cannot start Claude Code and Claude Code cannot start
Codex through MCP. Two limits remain: a permission mode changed inside the caller's
session after it started (for example with Shift+Tab) is not visible to Moshpit, so the
new session starts with the caller's launch-time settings; and with `worktree`, project
settings that live only in uncommitted files are whatever the program's worktree
carries. New sessions can only inherit the caller's directory, with
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
messages and revoked capabilities, and for messages: each step of a receipt and
the revision it belongs to, what each way of telling says and when it is due,
what a launch is given for each program and version, and held connections that
end with their session. Two Windows desktop tests start an isolated office,
replace both native CLIs with stand-ins, and consume the real launch
configuration to run the shipped MCP subprocess and hooks. `mcp.mjs` verifies
new-session delegation, the same directory, inbox follow-ups, result handoff,
protected names and reconnection after stopping a desk. `delivery.mjs` verifies
a message said by the hook that waits, by the step hook and as a turn ends, then
fetched and acknowledged; Codex told with a tool's answer; a push written while
another call is pending, with nothing garbled on the line; a message kept for a
stopped session and pushed when it is carried on; every way turned off; and
that nothing was typed into a terminal. Neither makes a model call, and CI runs
both against the Windows debug executable after building it.

What a stand-in cannot show is that a model reads what it is told and
acknowledges it. `node tools/e2e/real-delivery.mjs <how>-<when>` starts your
own Claude Code, on its smallest model, for that; it is run by hand. With
Claude Code 2.1.296 on 12 October 2026, one trial each:

| Started with | Sent while it was | What happened |
| --- | --- | --- |
| hooks, nothing that wakes | busy | told at a step, reminded as the turn ended, acknowledged 12 s after it was sent |
| hooks, nothing that wakes | idle | not woken, as designed; acknowledged 6 s after the next prompt |
| hooks and waking | idle | woken; acknowledged 7 s and 12 s after it was sent, in two trials. In the second it first said it would not read it unasked, and did once reminded as the turn ended |
| hooks and waking | stopped, then carried on | acknowledged 9 s after Carry on |
| pushes alone | idle | pushed, woken and read; not acknowledged, in four trials |
| pushes, hooks and waking | idle, and busy | acknowledged in 9 s, and in 11 s |
| pushes, hooks and waking | stopped on a permission question | the question untouched for 25 s and its file not written; acknowledged 3 s after the question was answered |

Codex 0.162.1, on 13 October 2026, two trials: idle, it was not told for the
15 seconds watched, as designed. Asked by the user to set its activity, it was
told with that call's answer, read the message and acknowledged it, 7 seconds
after the note. Asked the same with "do only what that requires", it read the
note and did nothing about it: what the user asks comes first. That trial is
why an unread note is said again after five minutes.

```powershell
npm ci
npm run check
npm test
npm run tauri build -- --debug --no-bundle
node tools/e2e/mcp.mjs
node tools/e2e/delivery.mjs
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

When building alongside other worktrees, use a separate `CARGO_TARGET_DIR`.
The desktop test accepts `MOSHPIT_APP`; its default is this worktree's
`src-tauri/target/debug/agent-moshpit.exe`.

Configuration references: [Codex MCP](https://learn.chatgpt.com/docs/extend/mcp?surface=cli),
[Claude Code MCP](https://code.claude.com/docs/en/mcp), and
[MCP transports](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports).

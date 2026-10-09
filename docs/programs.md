# The programs Agent Moshpit knows

The office knows very little about each program, on purpose: what it is called, what to type to start it, and how to hand it a task. Everything else is the program's own business and is seen through its terminal.

| Program | Started as | Takes a task as it starts | Carries on after a stop | Separate copy | Installed and updated by the office |
| --- | --- | --- | --- | --- | --- |
| Claude Code | `claude` | yes | yes (`--resume`) | yes | npm `@anthropic-ai/claude-code`; `claude update` |
| Codex | `codex` | yes | yes (`codex resume`), once its own session is verified | yes | npm `@openai/codex`; `codex update` |
| Antigravity CLI | `agy` | yes (`--prompt-interactive`) | yes (`--continue`: its latest conversation in that folder) | no | not installed by the office; `agy update` |
| Hermes | `hermes` | no | yes (`--continue`: its latest conversation, whichever desk that was) | yes | not installed by the office; `hermes update` |
| Gemini CLI | `gemini` | yes (`--prompt-interactive`) | no | no | npm `@google/gemini-cli` |
| OpenCode, Copilot CLI, Amp, Qwen Code, Crush | `opencode`, `copilot`, `amp`, `qwen`, `crush` | no | no | no | npm (`opencode-ai`, `@github/copilot`, `@sourcegraph/amp`, `@qwen-code/qwen-code`, `@charmland/crush`) |
| Cursor CLI, Aider, Goose, Droid | `cursor-agent`, `aider`, `goose`, `droid` | no | no | no | no: install them their own way and they are offered |

Claude Code, Codex, Hermes and Gemini CLI were written from each program's own `--help` on 8 October 2026 (Claude Code 2.1.290, Codex 0.160.0, Hermes 0.21.2, Gemini CLI 0.2.1).

Antigravity CLI is Google's replacement for Gemini CLI, which stopped serving personal Google accounts on 18 June 2026 and now works only with a paid API key or an enterprise licence. Its row was written from [Google's documentation](https://antigravity.google/docs/cli/install) on 9 October 2026, not from the program itself, and has not been run by the makers; it installs with Google's own script (`agy` lands in `~/.local/bin`, or `%LOCALAPPDATA%\agy\bin` on Windows), not from npm.

The rest are known by name only: they start in their folder and you type the task in their terminal, which is right for any program whatever its flags are.

Codex is started with `tui.terminal_title=["session-id"]` as well as terminal notifications. The office resolves the reported ID (or its unique shortened prefix) against `$CODEX_HOME/sessions`, falling back to `~/.codex/sessions`. It never chooses the newest conversation in a folder. This was checked with Codex 0.162.0 on 9 October 2026. An empty conversation has no rollout yet and stays unverified until Codex saves one. A CLI that does not report a usable ID cannot be resumed automatically. IDs saved by older office versions are preserved, but must be chosen explicitly with Codex's `/resume` before relying on that conversation. The setting is documented in the [Codex configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference).

Claude session discovery respects `CLAUDE_CONFIG_DIR`, falling back to `~/.claude`. Folder-trust badges come from the visible terminal dialog; configuration files alone do not prove that a CLI is asking a question. The office does not change CLI permission or approval settings.

## Adding a program of your own

Put a file called `harnesses.json` in the data folder (see [Where things are kept](guide.md#where-things-are-kept)). It is a JSON list of rows. A row whose `id` is one the office already knows replaces that program; any other row is added at the end. The file is read when the office starts.

```json
[
  {
    "id": "mine",
    "name": "My Agent",
    "tag": "Mine",
    "program": "C:\\Tools\\my-agent.exe",
    "task": { "flag": "--ask" },
    "resume": ["--again"]
  }
]
```

| Field | Needed | What it is |
| --- | --- | --- |
| `id` | yes | Letters, digits, `-` and `_`. |
| `name` | yes | The full name: "Claude Code". |
| `tag` | yes | The short word on a desk: "Claude". |
| `program` | yes | What is typed to start it: a plain name found on the `PATH`, or a full path. Never a line for a shell. |
| `args` | | Words passed every time, before anything else. |
| `task` | | How the first task is handed over: `"last"` (as the last word), `{ "flag": "--ask" }` (after that flag), or `"none"` (typed in the terminal; the default). |
| `resume` | | Words that make it carry on an earlier conversation. `{session}` stands for the conversation's id. Empty: it starts afresh. |
| `session` | | How the office learns that id: `"given"` (the office picks one and passes it after `session_arg`), `"codex_rollouts"` (resolve the session ID in that terminal's title against Codex rollout metadata), or `"unknown"` (the default). |
| `session_arg` | | The flag that sets a new conversation's id, when `session` is `"given"`. |
| `name_arg` | | The flag that names a conversation, if the program has one. |
| `worktree` | | Words that make it work on a git worktree of its own. With these the form offers **Work on a separate copy**. |
| `status` | | Where its status is read from: `"activity"` (how its terminal behaves; the default) or `"claude_sessions"` (Claude Code's own session files). |
| `trust` | | Which visible startup trust dialog is recognized: `"claude"`, `"codex"`, or `"none"` (the default). |
| `package` | | Its package on npm, which installs it, updates it when it has no way of its own, and says which version is the newest. |
| `update` | | Words that make the program update itself. |
| `install` | | A command that installs it, program first, for something npm does not install. |

A file that cannot be read, or a row that cannot be used, is reported in a strip at the top of the window when the office starts, with a button that opens the file, at the line when the mistake has one. In JSON a `\` in a Windows path is written `\\`.

A desk whose program is not in the table when the office starts is not shown, but it is kept: it stays in `desks.json`, at its place in the list, and its last screen stays in `screens/`. So if `harnesses.json` is reported as unreadable and your desks for the programs in it are missing from the floor, nothing is lost. Fix the file and start the office again, and they are back where they sat.

The same holds for a row you take out or give another `id` on purpose: its desks are out of sight, not gone, and there is nothing in the window yet that removes them. To be rid of one, put the row back for one start and use **Remove this desk**, or quit the office and delete its entry from `desks.json`.

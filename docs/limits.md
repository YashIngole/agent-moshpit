# Known limits

- Not used by hand on macOS or Linux. CI builds both and runs the automated checks there, and that is all that is known about them.
- A desk says that an agent is working, not on what. The programs do not say.
- Status read from a terminal's behaviour is a reading, not a report. A program that asks a question without ringing its terminal looks idle or done. Answering a raised hand while the window is behind other windows can show "done" for a moment.
- Sessions started outside the office are not shown and cannot be brought in.
- A real click on a real Windows notification has not been tried by hand, nor has the taskbar flash been seen. What was checked is that the notification carries the desk's address, and that opening that address opens the desk in the running office.
- Whether Claude Code and Codex take a pasted picture's path as the picture has not been checked with the real programs.
- **Work on a separate copy** has not been run end to end. The office passes the program its own flag; the worktree is the program's doing.
- Installing a real package with npm from inside the app has not been run; a stand-in installer was.
- Claude Code's status has not been watched through real work in a test, because that means giving it a real task.
- Programs known only by name, and Gemini CLI, always start afresh. Codex carries on only after the office has read which session it began. Antigravity CLI and Hermes carry on their latest conversation, not a particular desk's.
- Antigravity CLI's row was written from Google's documentation and has not been run by the makers: how it takes a task (`--prompt-interactive`) and carries on (`--continue`) may need correcting.
- A desk's last screen is written when the office quits, not while it runs, so after a crash an away desk shows an older screen or none.
- On Windows, a program that can only be started through `cmd.exe` cannot be handed a task containing characters `cmd.exe` would read as commands. The office says so and asks you to type the task in the terminal.
- Updating a program while agents are running it can fail on Windows until they stop.
- The tray icon and its menu are not covered by any automated test.
- One window, no sounds.
- Installers are not signed with a certificate (the macOS app is signed ad hoc only), so Windows and macOS warn the first time. The app has no updater of its own.

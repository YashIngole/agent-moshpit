# Using Agent Moshpit

Everything the window does, in the order you meet it. For installing it, see the [README](../README.md); for the programs it can run and adding your own, [programs.md](programs.md).

![The floor: two rooms of agents at desks, each tagged with its program. One has a hand up, and an amber band across the top names them and offers to open their terminal](office.png)

| Terminals beside the floor | The floor as a strip | New agent |
| --- | --- | --- |
| ![Three terminals in a grid to the right of the floor, each headed by its agent's face, name, status and program](terminals.png) | ![The floor narrowed to a list of names, with a "waiting" group on top, beside four terminals. One agent is away and its pane offers Carry on](strip.png) | ![The New agent panel: a choice of program, a task, a folder, a tick box for a separate copy, and a name](new-agent.png) |

The pictures show the built-in demo data, not a real session. The people and their status are drawn by the app; what is in the terminals is typed by hand to stand in for a program.

## What it is, and what it is not

Agent Moshpit starts the agent programs you already have, each in a pseudo-terminal of its own, as child processes of the app. Everything under a pane's header is that program's own screen: its prompt, its slash commands, its approvals, its settings. The office draws none of it. What the office adds is the floor, where one look tells you the state of every agent and whoever needs you is brought to the top.

- **It runs your own programs with their own sign-ins.** It keeps no account, API key or agent model setting. Optional local voice has its own recognition model. Whatever `claude` or `codex` does when you type it in a terminal, it does here.
- **It is not a harness.** There is no conversation view, no diff view and no Approve button of the office's own. A question is answered where the program asks it.
- **It sends nothing about you or your work.** It asks two questions about versions, when it starts and every 12 hours, unless `MOSHPIT_NO_UPDATE_CHECK` is set: npm (`npm view <package> version`), for the newest version of each installed program that is published there, and its own releases on GitHub, for a newer version of the office. There is no telemetry and no remote font or script. The programs it starts talk to their own services as they always do. An install or an update runs only when you press its button, in a terminal where you see every line.
- **Nothing is a separate window.** There is one Agent Moshpit button in the taskbar. Each program is a process of its own under the app, and is ended when the app quits.
- **It shows the agents you start from it.** A session you started in another terminal, or in a desktop app, does not appear.
- **A link in a terminal opens in your browser**, and only if it is an `https` address or an `http` one on this computer. The window itself cannot be sent anywhere. A file path is only ever handed to your editor: nothing a program prints is run.
- It is not affiliated with Anthropic, OpenAI, Google, Nous Research or the makers of any other program it can run, nor with Gather.

## Start an agent

Press **+ new agent** (`n` on the floor, `Ctrl+Shift+N` anywhere, or **New agent…** in the tray menu). An empty desk in a room, or the **+** on a room's sign, opens the same form on that room's folder.

- **Who should take it?** The programs found on this computer. Ones the office can install sit behind **+N to install**. Choosing one shows the exact command, and the button becomes **Install … and start**: the install runs in a pane where you can watch it, and the agent then starts in that pane.
- **What should they do?** Optional, and offered only for programs that take a task as they start (Claude Code, Codex, Antigravity CLI, Gemini CLI). Leave it empty and say it in their terminal. Hermes, and the programs the office knows only by name, start in the folder and wait for you there.
- **Folder.** Type it, browse for it, or pick one used before.
- **Work on a separate copy**, where the program can (Claude Code, Codex, Hermes). The program is asked to make a git worktree of its own (`--worktree`), so several agents can change one project without colliding.
- **Name.** Optional. Left empty it is the start of the task, or the program and folder: "Claude in web", then "Claude in web 2".

`Ctrl+Enter` starts the agent from anywhere in the form. Closing the form keeps what you typed in it. The program and the separate-copy box are remembered for the next agent, because people start several alike.

The new agent walks to a desk in the room of their project, and their terminal opens in a pane of its own.

## The floor

One room per project (the git repository's name, or the folder's), a desk per agent. A desk shows the person, a name, a status with how long it has lasted, a tag saying which program it is, and one line about what is going on.

| Status | How they look | What it means |
| --- | --- | --- |
| Starting | sitting, three dots on the screen | The program has been started and has not settled yet. |
| Working | typing, code on the screen | It is at work. |
| Needs you | a hand up, an amber tag, an amber screen | It has stopped on a question only you can answer, in its terminal. |
| Done | leaning back, a green tick on the screen | It finished a stretch of work that you have not looked at yet. |
| Idle | sitting at a prompt | Running, with nothing to do. |
| Trouble | hands on their head, a red tag, a red screen | The program could not be started, or stopped with an error as soon as it started. |
| Away | asleep, the screen dark | Not running. The desk shows where they left off and offers to start them again. |

A status is always a word and a pose as well as a colour. Four colours carry meaning and are used for nothing else: amber for needs you, green for done, red for trouble, and blue for the light of a screen at work.

- A small dot before a name means their terminal has printed something since you last had it in front of you.
- The counts in the top bar (`1 needs you`, `2 working`, `1 done`, `1 in trouble`) are buttons: a click shows the next desk in that state.
- With more than ten agents and no terminals open, desks are drawn smaller so a crowd fits.
- The floor is one stop for `Tab`. The arrow keys walk from desk to desk and `Enter` opens one.

## Terminals

Click a desk and their terminal opens to the right of the floor, with the keyboard in it. A click on another desk shows that one in the same pane. Hold `Ctrl` as you click, or press `Ctrl+Enter` on a desk, to open it beside the others instead: two sit side by side, more make a grid, as many as you like. Do the same on a desk that is already open and its pane is put away again, with its program still running.

- **Each pane is headed by its agent**: face, name, status, project and branch, and program.
- **The grid stays as you made it.** A new pane goes beside its neighbour, and a closed one gives its room to a neighbour; nothing else moves. Drag any edge between two panes, or double-click one to make them all even. Drag a pane by its header onto another to swap the two.
- **Give one pane the room** with `Ctrl+Shift+Enter`, the button in its header, or a double-click on the header. A **+N** chip lists the ones waiting behind it and puts them back.
- **Put a pane away** with its × or `Ctrl+Shift+W`. The program keeps running at its desk.
- **Back to the floor, and back again,** with `Ctrl` + `` ` ``. The same panes return. Pick a desk while you are on the floor and they return with that desk among them. `Esc` is not used for this, because the programs use it themselves.
- **The floor beside the terminals** keeps the width you drag it to. Below 300 pixels it becomes a strip: a list of names, each with its status and program, with whoever needs you, is in trouble or is done gathered on top under **waiting**. Double-click that edge to switch between the strip and the usual width.
- In a window narrower than 760 pixels the terminals take the whole window and the floor waits behind them.

## In a terminal

Whatever you type goes to the program, apart from the office's own keys, which are listed under [Keys](#keys).

- **Copy.** `Ctrl+C` copies when text is selected; with nothing selected it goes to the program. **⋯ → Copy on select** copies as soon as you select.
- **Paste.** `Ctrl+V`, or right-click and **Paste**.
- **A picture on the clipboard.** `Ctrl+V` saves it as a file under `pasted/` in the data folder and pastes that file's path, so the program is given the picture as a file.
- **Files dropped on a pane.** Their paths are pasted into that pane, in quotes where a path has a space. The pane lights up before you let go.
- **A file path** such as `src/app.ts:42`. `Ctrl` and a click opens it in your editor at that line. The office looks for Cursor, VS Code, Windsurf, Antigravity and Zed and uses the first it finds, until you choose one under **⋯ → Agent programs → Files open in**. With none of them here, the file is shown in its folder.
- **A web address.** A click opens it in your browser.
- **Find** in what a terminal has shown with `Ctrl+Shift+F`.
- **Text size.** `Ctrl+=`, `Ctrl+-` and `Ctrl+0`, or `Ctrl` and the mouse wheel. It is remembered.
- **Right-click** for Copy, Paste, Select all, Find and Clear the scrollback, followed by everything in the desk's own menu.

Each terminal keeps 5,000 lines of scrollback, and the core keeps the last half megabyte each program printed, so a pane that was put away, or a window that was closed, comes back showing the screen as it stands.

## Voice input

Voice starts off. Open **⋯ → Voice input**, enable **local voice**, and explicitly download a model. The panel identifies **Hugging Face** as the model host before download; no account or payment is needed. Downloads go into the app's data folder, separate from the installer, and are checked for their exact size and SHA-256 before becoming usable. An interrupted or failed download is never used. **Remove** deletes a downloaded model.

- **Small Q5_1**, recommended: 190,085,487 bytes (190.1 MB).
- **Base Q5_1**, lighter: 59,707,625 bytes (59.7 MB).

Select a terminal with a running program and press the **mic** in the top bar. The system's default microphone is used. Listening is **blue**, with the original terminal's name and elapsed time. **Stop and insert** stops the microphone and transcribes locally on the CPU. **Cancel voice input** discards the recording or pending result. Recording stops automatically at 60 seconds; very short and silent audio is rejected. No mic is opened by enabling voice or downloading a model.

The default in-app toggle is **Ctrl+Shift+Space**. Choose **Ctrl+Alt+Shift+Space** or **No shortcut** in the panel if your program uses that key. The shortcut is reserved only while voice is enabled; plain Space and the office's existing keys retain their behavior. Holding the shortcut does not repeatedly toggle capture.

Changing keyboard focus does not change the destination. Text goes only into the pane and live terminal session selected at recording start. Closing/putting away that pane, hiding it behind a zoomed pane, stopping/restarting its program, cancelling, changing voice settings, minimising or closing the window discards the pending recording/result. Closing to tray never keeps a hidden microphone running. Terminal controls and line breaks are stripped. **Voice never presses Enter or runs/submits the text.** Review it in the terminal and send it yourself.

Choose automatic language detection, English or Hindi. Multilingual models do not establish usable Hindi/Hinglish, accent or technical-vocabulary accuracy. Try shorter recordings or Base if memory is constrained. On microphone failure, check OS microphone access and the default input device; the app stops capture and explains how to retry. Linux needs a working ALSA input; macOS asks for system consent.

Audio and transcripts are kept only in memory and discarded after use or cancellation. Voice leaves the clipboard alone and makes no recognition or rewriting network request. To avoid retaining dictated text through terminal echo, a desk that receives voice text skips the app's saved-screen file for that app run; its conversation is still owned and potentially saved by its CLI. Other desks retain their usual screen snapshots. Voice's Windows recorded-WAV tests do not establish real microphone behavior. [Validation and remaining checks](reviews/voice-validation-2026-10-09.md).

## When someone needs you

Amber means one thing here: someone needs you.

- Their hand goes up at their desk, with a line saying what for when the office knows.
- A band across the top names whoever has waited longest and has one button, **Open their terminal**, which opens it beside whatever is already open. Anyone else waiting is listed on the band. The band is only there for people whose terminal is not already in front of you.
- The top bar counts them, the window's title becomes `Agent Moshpit (2 need you)`, the taskbar button flashes if the window is behind, and the tray icon changes.
- You get a notification, unless you are looking at that terminal.

The hand stays up until the question is answered in their terminal. Going back to the floor, or opening another pane, does not lower it.

How the office knows, without ever reading the words on a program's screen:

- **Claude Code says so itself.** It keeps a small file about each running session (`~/.claude/sessions/<pid>.json`), and the office reads from it whether Claude is busy, idle or waiting.
- **Every other program is read from how its terminal behaves.** One that keeps printing is working. One that goes quiet is idle, or done if it had been working. One that rings the terminal's bell or sends a terminal notice wants you, or has finished if the notice says so. Codex is started with its terminal notices turned on for this.
- **A folder nobody has trusted yet.** Claude Code and Codex first ask whether to trust a folder they have not been told about. The office reads their own settings before starting one, and shows **needs you**, "Asks whether to trust this folder", until you press a key in that terminal.
- **A task that stops at once.** A program handed a task that goes quiet almost immediately, before anyone has typed, shows **needs you**, "Asked something before starting".

A green tick means they finished something you have not seen. Bring their terminal in front of you and it comes down.

## Notifications

The office tells you when an agent needs you, is done, or hit a problem, unless that agent's terminal is already in front of you. On Windows a click on the notification opens the office with that desk's terminal beside the others, and starts the office first if it had quit; a newer notification about a desk takes the place of the older one.

## A desk's menu: rename, restart, stop, remove

Right-click a desk on the floor, in the strip or on a pane's header, press `Shift+F10` on it, or use the ⋯ on its pane.

- **Open their terminal**, or **Open beside the others**.
- **Start another like this.** The New agent form, on the same program and folder.
- **Rename** (`F2`). An emptied name goes back to the one the desk was given. Claude Code's own name for a session is used until you name the desk yourself.
- **Restart their program** and **Stop their program.** Stop ends the program and keeps the desk, away. Both ask first if the agent is working, starting or waiting for you, and the menu says whether the conversation will be carried on or started afresh.
- **Carry on** or **Start again**, for a desk whose program is not running.
- **Open the folder in** your editor, **Show their folder**, **Copy the folder path**.
- **Remove this desk.** Also the × that appears on a desk under the pointer, a middle-click, or `Delete`. The desk leaves the floor at once and can be brought back with **Undo** for eight seconds; after that its program is ended and the desk is gone. If the agent is busy you are asked first. The program keeps the conversation in its own history, and nothing of yours is deleted: the folder, and a worktree a program made for a separate copy, are left as they are.

## Closing the window, and quitting

Closing the window does not quit. The window goes and the office stays in the tray: agents keep working, the tray icon shows when someone needs you, and notifications still arrive. The first time, a notification says so. Click the tray icon, or start the app again, and the window comes back with the panes it had.

- **To quit**, use **Quit Agent Moshpit** in the tray menu or the ⋯ menu, or press `Ctrl+Shift+Q` (`Ctrl+Q` works too when the keyboard is not in a terminal). Quitting ends every agent's program. If anyone is busy you are asked first, and told who will carry on later and who will start afresh.
- **⋯ → Closing the window quits** makes the window's × quit instead, asking first if anyone is busy. It is off by default.
- On a desktop with no tray, closing the window quits.

## What comes back after a restart

- Every desk is back, away. Nothing is started until you ask.
- Opening a desk shows its last screen as it was drawn when the office quit, with up to 500 lines above it, and a **Carry on** or **Start again** button.
- **Carry on** resumes the conversation where the program can: Claude Code, Codex once the office has read which session it began, and Hermes. The others start afresh in the same folder.
- The window has the size and place it had, the same floor width and text size, and `Ctrl` + `` ` `` brings back the panes that were open.

What does not come back is a terminal's scrollback beyond that last screen.

## The ⋯ menu

- **Agent programs.** Every program the office knows, with its version, whether a newer one is out, and a button to install or update it. The command that will run is written beside the button, and it runs in a pane. This panel also chooses the editor that file paths open in.
- **Voice input.** Enable local voice, download/remove models, choose language and the in-app toggle shortcut.
- **Make the terminals even**, when more than one is open.
- **Copy on select**, off by default.
- **Closing the window quits**, off by default.
- **Keys.** Every shortcut in one list.
- **Quit Agent Moshpit.**
- **Update to …**, at the top, when a newer version of the office is out (a dot on ⋯ says so). Choosing it fetches the update, checks it against the app's own key, puts it in place and starts the office again. If anyone is busy you are asked first, and told who will carry on. Nothing is fetched before you choose it. If it cannot be had, the office says why and offers the download page instead.

## Keys

| | |
| --- | --- |
| **Anywhere** | |
| `Ctrl` + `` ` `` | Back to the floor, and back to the terminals |
| `Ctrl+Shift+N` | New agent (`n` on the floor) |
| `Ctrl+Shift+Q` | Quit, asking first if anyone is busy |
| `Ctrl+Q` | The same, when the keyboard is not in a terminal (there it is the program's) |
| `Ctrl+Shift+/` | The list of keys (`?` on the floor) |
| `Esc` | Close a side panel or a menu, when the keyboard is not in a terminal |
| **Terminals** | |
| `Ctrl+Shift+Space` | Start / stop voice when enabled (configurable in Voice input) |
| `Alt+1` … `Alt+9` | The keyboard to pane 1 to 9 |
| `Ctrl+Shift+]` / `Ctrl+Shift+[` | The keyboard to the next pane, or the one before |
| `Ctrl+Shift+Enter` | Give this pane the room, and put the others back |
| `Ctrl+Shift+W` | Put this pane away; its program keeps running |
| `Ctrl+Shift+F` | Find in what this terminal has shown |
| `Ctrl+=` / `Ctrl+-` / `Ctrl+0` | Bigger text, smaller, the usual size |
| `Ctrl+C` | Copy, when text is selected; otherwise the program's |
| `Ctrl+V` | Paste. A picture is pasted as the path of a file holding it |
| `Shift+Enter` | A new line in the program's prompt |
| `Shift+Tab` | The program's (Claude Code's mode switch); the keyboard stays in the terminal |
| **On the floor** | |
| Arrow keys | From desk to desk |
| `Enter` | Open their terminal |
| `Ctrl+Enter` | Open it beside the others, or put it away again |
| `F2` | Rename |
| `Delete` | Take the desk away (it can be brought back for a moment) |
| `Shift+F10` | The desk's menu |
| **With the mouse** | |
| `Ctrl` and a click | Open a desk beside the others, or put it away again |
| `Ctrl` and a click on a file path | Open it in your editor, at its line |
| `Ctrl` and the wheel | Bigger or smaller text, over a terminal |
| Right-click | A desk's menu, or a terminal's |
| Middle-click | Take a desk away |
| Drag a pane's header | Onto another pane, to change places |
| Double-click a pane's header | Give it the room |
| Drop files on a pane | Their paths are pasted into it |

## Settings from outside

| Variable | What it does |
| --- | --- |
| `MOSHPIT_NO_UPDATE_CHECK` | Set to anything, the office never asks for newer versions: not npm about the agent programs, and not GitHub about itself. |
| `MOSHPIT_UPDATE_URL` | Ask this address about newer versions of the office instead of its releases on GitHub. Used by the tests. |
| `MOSHPIT_UPDATE_ONLY_FETCH` | Set to anything, an update is fetched and checked and then left alone. Used by the tests. |
| `MOSHPIT_DATA_DIR` | Keep the office's files in this folder instead of the app's data folder. |
| `MOSHPIT_INSTANCE` | A name (letters and digits) that makes this a separate office next to the one already open: its own window, data folder, window title ("Agent Moshpit (name)") and notification address. `--instance name` on the command line does the same. Used by the tests. |
| `MOSHPIT_DEVTOOLS` | Set to anything, the webview's browser keys are left on (F5 reloads the window; in a debug build F12 opens the developer tools), for working on the window itself. Windows. |
| `MOSHPIT_APP` | Read by the app tests only: the path of the program to test. |

The office also reads `CLAUDE_CONFIG_DIR` and `CODEX_HOME`, the programs' own variables, to find where each keeps the folders it trusts.

On the command line: `--hidden` starts in the tray with no window, and `--instance name` is described above.

## Where things are kept

The data folder is `%APPDATA%\io.github.yashingole.agentmoshpit` on Windows, `~/Library/Application Support/io.github.yashingole.agentmoshpit` on macOS and `~/.local/share/io.github.yashingole.agentmoshpit` on Linux, unless `MOSHPIT_DATA_DIR` says otherwise.

| File | What is in it |
| --- | --- |
| `desks.json` | The desks: which program, the name, the folder, and the id of the conversation when it is known. |
| `window.json` | The window's size and place. |
| `settings.json` | Whether closing the window quits; voice enabled/model/language/shortcut preferences. Written when changed. |
| `voice-models/` | Verified optional model weights. No recordings or transcript files. Partial downloads are not used. |
| `screens/` | Each desk's last screen, written when the office quits. |
| `pasted/` | Pictures pasted into terminals. Cleared after a week. |
| `harnesses.json` | Your own programs, if you wrote any. |
| `told-about-tray` | An empty marker, so the notice about the tray is given only once. |

The layout of the panes, the floor's width, the text size, the editor you chose, copy on select and the folders used before are kept by the window itself, in the webview's storage (on Windows under `%LOCALAPPDATA%\io.github.yashingole.agentmoshpit`).

Conversations are not kept by the office. Each program keeps its own history where it always does.

On Windows the office also writes one entry under `HKEY_CURRENT_USER\Software\Classes\agent-moshpit` each time it starts. That is what lets a clicked notification open a desk: the notification opens an `agent-moshpit://desk/…` address, and Windows hands it to the app.

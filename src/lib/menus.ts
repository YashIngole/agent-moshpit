// What the small menus offer: a desk's (on the floor, in the strip, on its pane's
// header) and a terminal's (a right-click in it).
import { bridge } from './bridge'
import { office, type MenuItem } from './office.svelte'
import { terms } from './terms'
import type { Agent } from './types'
import { resumeAction, resumeHint } from './words'

/** Everything that can be done with a desk, in one place. */
export function deskItems(agent: Agent): MenuItem[] {
  const items: MenuItem[] = []
  const shown = office.open.has(agent.id)
  if (!shown) items.push({ label: 'Open their terminal', hint: 'Enter, or a click', run: () => office.show(agent.id) })
  if (!shown && (office.terminals || office.stowed.rows.length > 0)) {
    items.push({ label: 'Open beside the others', hint: 'Ctrl+Enter, or Ctrl and a click', run: () => office.show(agent.id, true) })
  }
  items.push({ label: 'Start another like this', hint: `${agent.harness_name} in ${agent.repo || agent.cwd}, with a task of its own`, run: () => office.openNew(agent.cwd, agent.harness, agent.launch) })
  items.push({ label: 'Rename', hint: 'F2', run: () => office.startRename(agent.id) })
  if (agent.running) {
    // A right-click away from Paste: someone in the middle of something is asked about first.
    const busy = office.busy(agent.id) ? `${agent.title} is ${agent.phase === 'needs_you' ? 'waiting for you' : agent.phase === 'starting' ? 'starting' : agent.phase === 'quiet' ? 'quiet and may still be working' : 'working'}.` : ''
    items.push({
      label: 'Restart their program',
      hint: resumeHint(agent),
      confirm: busy ? { text: `${busy} Restarting ends its current program. ${resumeHint(agent)}`, yes: 'Restart it' } : undefined,
      run: () => void office.restart(agent.id)
    })
    items.push({
      label: 'Stop their program',
      hint: agent.resumable ? 'The conversation is kept' : 'The desk stays; they start afresh next time',
      confirm: busy ? { text: `${busy} Stopping ends what it is doing. ${agent.resumable ? 'The conversation is kept.' : 'The desk stays, and they start afresh next time.'}`, yes: 'Stop it' } : undefined,
      run: () => office.stop(agent.id)
    })
  } else {
    items.push({
      label: resumeAction(agent),
      hint: resumeHint(agent),
      run: () => {
        office.show(agent.id)
        void office.wake(agent.id)
      }
    })
  }
  if (office.editor) {
    const editor = office.editor.name
    items.push({ label: `Open the folder in ${editor}`, hint: agent.cwd, divided: true, run: () => office.openFolder(agent.id) })
  }
  items.push({
    label: 'Show their folder',
    divided: !office.editor,
    hint: agent.cwd,
    run: () => void bridge.showFolder(agent.id).catch(error => (office.problem = typeof error === 'string' ? error : 'That folder could not be shown.'))
  })
  items.push({
    label: 'Copy the folder path',
    run: () =>
      void navigator.clipboard.writeText(agent.cwd).then(
        () => office.say('Folder path copied.'),
        () => (office.problem = 'The folder path could not be copied.')
      )
  })
  items.push({
    label: 'Remove this desk',
    hint: 'Ends their program; Undo for a moment. Delete',
    danger: true,
    divided: true,
    // Someone in the middle of something is asked about first; anyone else can be brought back for a moment.
    confirm: office.busy(agent.id)
      ? { text: `${agent.title} is ${agent.phase === 'needs_you' ? 'waiting for you' : agent.phase === 'quiet' ? 'quiet and may still be working' : agent.phase === 'starting' ? 'starting' : 'working'}. Their program is ended and the desk is taken away. ${agent.harness_name} keeps the conversation in its own history.`, yes: 'Remove the desk' }
      : undefined,
    run: () => office.remove(agent.id)
  })
  return items
}

/** Open a desk's menu at a place in the window. */
export function openDeskMenu(id: string, x: number, y: number) {
  const agent = office.agents.find(a => a.id === id)
  if (agent) office.openMenu({ x, y, items: deskItems(agent) })
}

/** Ask before taking a desk away when they are busy; otherwise take it away, with a moment to undo. */
export function askToRemove(id: string, x?: number, y?: number) {
  const agent = office.agents.find(a => a.id === id)
  if (!agent) return
  if (!office.busy(id)) return office.remove(id)
  const items = deskItems(agent)
  const remove = items.find(item => item.danger)!
  const place = x !== undefined && y !== undefined ? { x, y } : office.placeOf(id)
  office.openMenu({ ...place, items, asking: remove })
}

/** What a right-click in a terminal offers. */
export function termItems(id: string): MenuItem[] {
  const term = terms.get(id)
  if (!term) return []
  const items: MenuItem[] = []
  if (term.hasSelection()) items.push({ label: 'Copy', hint: 'Ctrl+C', run: () => void term.copy() })
  items.push({
    label: 'Paste',
    hint: 'Ctrl+V',
    run: () =>
      void term.pasteClipboard().then(
        () => term.focus(),
        () => {
          office.say('Press Ctrl+V in the terminal to paste.')
          term.focus()
        }
      )
  })
  items.push({ label: 'Select all', run: () => term.selectAll() })
  items.push({ label: 'Find…', hint: 'Ctrl+Shift+F', run: () => (office.finding = id) })
  items.push({ label: 'Clear the scrollback', hint: 'The program’s own screen stays', run: () => term.clear() })
  const agent = office.agents.find(a => a.id === id)
  if (agent) {
    const desk = deskItems(agent).filter(item => item.label !== 'Open their terminal')
    desk[0] = { ...desk[0]!, divided: true }
    items.push(...desk)
  }
  return items
}

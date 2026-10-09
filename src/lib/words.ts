// The words the office uses for each status. One place, so they stay consistent.
import type { Agent, Phase } from './types'

export const STATUS_WORD: Record<Phase, string> = {
  needs_you: 'Needs you',
  working: 'Working',
  quiet: 'Quiet',
  done: 'Done',
  idle: 'Idle',
  failed: 'Trouble',
  starting: 'Starting',
  asleep: 'Away'
}

/** One line under the nameplate: what is going on at this desk right now. */
export function doing(agent: Agent): string {
  switch (agent.phase) {
    case 'needs_you':
      return agent.activity || 'Waiting for you in their terminal'
    case 'working':
      return agent.activity || 'Working'
    case 'quiet':
      return 'No recent output. It may still be working.'
    case 'starting':
      return agent.activity || 'Getting set up'
    case 'done':
      return agent.activity || 'Finished. Open their terminal to see what they did.'
    case 'failed':
      return agent.activity || 'Something went wrong'
    case 'asleep':
      // Opening the desk shows where they left off; starting them again is a button there.
      return 'Not running. Open to see where they left off.'
    case 'idle':
      return agent.activity || 'Waiting for something to do'
  }
}

/** A sentence for screen readers: the whole desk in one go. */
export function describe(agent: Agent): string {
  const where = agent.repo ? `, in ${agent.repo}` : ''
  return `${agent.title}, ${agent.harness_name}${where}. ${STATUS_WORD[agent.phase]}. ${doing(agent)}`
}

export function resumeAction(agent: Agent): string {
  if (!agent.resumable) return 'Start again'
  if (agent.resume_scope === 'folder') return 'Continue in folder'
  if (agent.resume_scope === 'latest') return 'Continue latest'
  return 'Carry on'
}

export function resumeHint(agent: Agent): string {
  if (!agent.resumable) return 'Starts their program afresh.'
  if (agent.resume_scope === 'folder') return 'Continues the latest conversation in this folder; it may belong to another desk.'
  if (agent.resume_scope === 'latest') return 'Continues the latest conversation selected by the program; it may belong to another desk.'
  return 'Resumes this desk’s saved conversation.'
}

/** Words a name does not end on: "Fix the total when a" reads as cut off. As in `office.rs`. */
const JOINERS = new Set('a an the and or but so if when while to of in on at by for from with as that is it its'.split(' '))

/** First part of a text on one line, cut on a character with an ellipsis. As `shorten` in `harness.rs`. */
export function shorten(text: string, most: number): string {
  const flat = text.split(/\s+/).filter(Boolean).join(' ')
  const chars = [...flat]
  return chars.length <= most ? flat : `${chars.slice(0, most - 1).join('')}…`
}

/** A name for a desk when the user gave none: the start of its task. As `title_from` in `office.rs`. */
export function titleFrom(task: string): string {
  const words = (task.split('\n')[0] ?? '').split(/\s+/).filter(Boolean).slice(0, 6)
  // Never shorter than two words for it.
  while (words.length > 2 && JOINERS.has(words.at(-1)!.toLowerCase().replace(/[.,:;]+$/, ''))) words.pop()
  return shorten(words.join(' '), 40).replace(/[.,:;]+$/, '')
}

/** A name no other desk has: "Claude in web", then "Claude in web 2". As `unique_title` in `office.rs`. */
export function uniqueTitle(title: string, taken: string[]): string {
  if (!taken.includes(title)) return title
  for (let n = 2; ; n++) if (!taken.includes(`${title} ${n}`)) return `${title} ${n}`
}

/** Where an agent is working: the project, and the branch when there is one. */
export function place(agent: Agent): string {
  return [agent.repo, agent.branch].filter(Boolean).join(' · ')
}

import type { Agent } from './types'

export interface RoomGroup {
  key: string
  repo: string
  label: string
  agents: Agent[]
}

/** Names are labels; only the core's canonical project path identifies a room. */
export function groupProjects(agents: Agent[]): RoomGroup[] {
  const groups = new Map<string, RoomGroup>()
  for (const agent of agents) {
    const key = agent.project || agent.cwd || agent.id
    const group = groups.get(key)
    if (group) group.agents.push(agent)
    else groups.set(key, { key, repo: agent.repo || 'No folder', label: agent.repo || 'No folder', agents: [agent] })
  }
  const rooms = [...groups.values()]
  for (const room of rooms) {
    if (rooms.some(other => other !== room && other.repo === room.repo)) room.label = room.key
  }
  return rooms
}

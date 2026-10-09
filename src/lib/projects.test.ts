import { expect, test } from 'vitest'
import { groupProjects } from './projects'
import type { Agent } from './types'

const agent = (id: string, project: string, cwd = project) => ({ id, project, cwd, repo: 'shop' }) as Agent

test('same names in different folders have distinct rooms and add-agent folders', () => {
  const rooms = groupProjects([agent('a', '/client/shop'), agent('b', '/personal/shop')])
  expect(rooms.map(r => [r.key, r.label, r.agents[0]?.cwd])).toEqual([
    ['/client/shop', '/client/shop', '/client/shop'], ['/personal/shop', '/personal/shop', '/personal/shop']
  ])
})

test('worktrees share their canonical repository room', () => {
  const rooms = groupProjects([agent('a', '/client/shop'), agent('b', '/client/shop', '/branches/fix')])
  expect(rooms).toHaveLength(1)
  expect(rooms[0]?.label).toBe('shop')
  expect(rooms[0]?.agents.map(a => a.id)).toEqual(['a', 'b'])
})

test('case-sensitive project identities stay distinct', () => {
  expect(groupProjects([agent('a', '/code/Shop'), agent('b', '/code/shop')])).toHaveLength(2)
})

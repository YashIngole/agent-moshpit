import { describe, expect, it } from 'vitest'
import { EMPTY, arrange, close, even, has, ids, moveColumnEdge, moveRowEdge, only, open, parse, shape, swap, trade, type Layout } from './layout'

const lengths = (layout: Layout) => layout.rows.map(row => row.panes.length)
const sizes = (layout: Layout, row = 0) => layout.rows[row]!.panes.map(pane => Number(pane.size.toFixed(3)))

describe('laying out terminals', () => {
  it('finds a shape for any number of panes', () => {
    expect([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10].map(shape)).toEqual([[], [1], [2], [2, 1], [2, 2], [3, 2], [3, 3], [3, 2, 2], [3, 3, 2], [3, 3, 3], [4, 3, 3]])
  })

  it('opens panes one after another into a grid close to square', () => {
    let layout = EMPTY
    const seen: number[][] = []
    for (const id of ['a', 'b', 'c', 'd', 'e', 'f']) {
      layout = open(layout, id)
      seen.push(lengths(layout))
    }
    expect(seen).toEqual([[1], [2], [2, 1], [2, 2], [2, 3], [3, 3]])
    expect(new Set(ids(layout))).toEqual(new Set(['a', 'b', 'c', 'd', 'e', 'f']))
  })

  it('opens a pane without moving the others', () => {
    const before = arrange(['a', 'b', 'c'])
    const after = open(before, 'x', 'c')
    // c's row had room: x sits beside c, and a and b stay where they were.
    expect(after.rows.map(row => row.panes.map(pane => pane.id))).toEqual([['a', 'b'], ['c', 'x']])
  })

  it('keeps the sizes that were dragged when a pane is opened', () => {
    const dragged = moveColumnEdge(arrange(['a', 'b']), 0, 0, 0.7)
    // The row is full: the newcomer gets a row of its own and the dragged row is untouched.
    const three = open(dragged, 'c', 'b')
    expect(sizes(three)).toEqual(sizes(dragged))
    expect(lengths(three)).toEqual([2, 1])
    // In a dragged row with room, the neighbour gives half of its share.
    const four = moveColumnEdge(arrange(['a', 'b', 'c', 'd']), 0, 0, 0.7)
    const [a, b] = sizes(four)
    const after = sizes(open(four, 'x', 'b'))
    expect(after[0]).toBeCloseTo(a!)
    expect(after[1]).toBeCloseTo(b! / 2)
    expect(after[2]).toBeCloseTo(b! / 2)
  })

  it('does not open the same desk twice', () => {
    const layout = arrange(['a', 'b'])
    expect(open(layout, 'a')).toBe(layout)
    expect(ids(arrange(['a', 'b', 'a']))).toEqual(['a', 'b'])
  })

  it('opens a pane beside the one it was asked from, when its row has room', () => {
    expect(ids(open(arrange(['a', 'b', 'c', 'd']), 'x', 'a'))).toEqual(['a', 'x', 'b', 'c', 'd'])
    // A full row: the shortest row with room takes it.
    expect(ids(open(arrange(['a', 'b', 'c']), 'x', 'a'))).toEqual(['a', 'b', 'c', 'x'])
    // Beside a pane that is not open is the same as beside the last.
    expect(ids(open(arrange(['a', 'b']), 'x', 'nobody'))).toEqual(['a', 'b', 'x'])
  })

  it('shows a different desk in the same pane, keeping its size', () => {
    const dragged = moveColumnEdge(arrange(['a', 'b']), 0, 0, 0.7)
    const swapped = swap(dragged, 'b', 'c')
    expect(ids(swapped)).toEqual(['a', 'c'])
    expect(sizes(swapped)).toEqual(sizes(dragged))
    // A desk that is already open moves to the pane rather than showing twice.
    expect(ids(swap(arrange(['a', 'b', 'c']), 'a', 'c'))).toEqual(['c', 'b'])
    expect(swap(dragged, 'nobody', 'c')).toBe(dragged)
  })

  it('closes down to one pane, and to none, without moving the others', () => {
    let layout = arrange(['a', 'b', 'c', 'd'])
    layout = close(layout, 'b')
    // a takes the top row; c and d stay below, where they were.
    expect(layout.rows.map(row => row.panes.map(pane => pane.id))).toEqual([['a'], ['c', 'd']])
    layout = close(close(layout, 'a'), 'c')
    expect(lengths(layout)).toEqual([1])
    expect(close(layout, 'd')).toEqual(EMPTY)
    expect(close(layout, 'nobody')).toBe(layout)
  })

  it('keeps the sizes that were dragged when a pane closes', () => {
    // Three in two rows; the lone pane of the second row goes, and the first row is as it was.
    const dragged = moveColumnEdge(arrange(['a', 'b', 'c']), 0, 0, 0.25)
    expect(sizes(close(dragged, 'c'))).toEqual(sizes(dragged))
    // In a dragged row, the pane beside the one that closed takes its share.
    const three = moveColumnEdge(moveColumnEdge(arrange(['a', 'b', 'c', 'd', 'e', 'f']), 0, 0, 0.5), 0, 1, 0.75)
    const [a, b, c] = sizes(three)
    const closed = sizes(close(three, 'b'))
    expect(closed[0]).toBeCloseTo(a! + b!)
    expect(closed[1]).toBeCloseTo(c!)
  })

  it('lets two panes change places, each keeping the place it moves into', () => {
    const dragged = moveColumnEdge(arrange(['a', 'b', 'c']), 0, 0, 0.7)
    const traded = trade(dragged, 'a', 'c')
    expect(traded.rows.map(row => row.panes.map(pane => pane.id))).toEqual([['c', 'b'], ['a']])
    expect(sizes(traded)).toEqual(sizes(dragged))
    expect(trade(dragged, 'a', 'nobody')).toBe(dragged)
  })

  it('drags the edge between two panes without touching the others', () => {
    const three = arrange(['a', 'b', 'c', 'd', 'e', 'f'])
    const moved = moveColumnEdge(three, 0, 0, 0.5)
    // Three even panes; the first edge dragged to the middle: a grows, b shrinks, c stays.
    expect(sizes(moved)).toEqual([1.5, 0.5, 1])
    expect(sizes(moved, 1)).toEqual([1, 1, 1])
    // There is no edge after the last pane.
    expect(moveColumnEdge(three, 0, 2, 0.5)).toBe(three)
  })

  it('never lets a pane or a row be dragged away to nothing', () => {
    const squeezed = moveColumnEdge(arrange(['a', 'b']), 0, 0, 0)
    expect(sizes(squeezed)[0]).toBeGreaterThan(0.1)
    expect(sizes(squeezed)[0]! + sizes(squeezed)[1]!).toBeCloseTo(2)
    const rows = moveRowEdge(arrange(['a', 'b', 'c']), 0, 1)
    expect(rows.rows[1]!.size).toBeGreaterThan(0.1)
    expect(rows.rows[0]!.size + rows.rows[1]!.size).toBeCloseTo(2)
  })

  it('makes everything even again', () => {
    const messy = moveRowEdge(moveColumnEdge(arrange(['a', 'b', 'c']), 0, 0, 0.8), 0, 0.3)
    expect(even(messy)).toEqual(arrange(['a', 'b', 'c']))
  })

  it('reads a layout back only if it is one', () => {
    const layout = moveColumnEdge(arrange(['a', 'b', 'c']), 0, 0, 0.6)
    expect(parse(JSON.parse(JSON.stringify(layout)))).toEqual(layout)
    for (const bad of [null, 'x', {}, { rows: 'x' }, { rows: [{ panes: [{ id: '' }] }] }, { rows: [{ panes: [{ id: 'a' }, { id: 'a' }] }] }]) {
      expect(parse(bad)).toEqual(EMPTY)
    }
    // A size that is not a share becomes an even one.
    expect(parse({ rows: [{ size: -3, panes: [{ id: 'a', size: 'wide' }] }] })).toEqual(arrange(['a']))
  })

  it('drops the panes of desks that have gone', () => {
    const layout = only(arrange(['a', 'b', 'c', 'd']), id => id !== 'b' && id !== 'd')
    expect(ids(layout)).toEqual(['a', 'c'])
    expect(has(layout, 'b')).toBe(false)
  })
})

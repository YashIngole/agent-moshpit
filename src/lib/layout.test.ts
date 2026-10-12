import { describe, expect, it } from 'vitest'
import {
  EMPTY,
  aim,
  arrange,
  asRows,
  close,
  count,
  even,
  has,
  ids,
  land,
  move,
  moveEdge,
  neighbour,
  only,
  open,
  parse,
  plan,
  putAtEdge,
  putBeside,
  samePicture,
  shape,
  sideBySide,
  swap,
  trade,
  type Layout,
  type Node,
  type Shape,
  type Side
} from './layout'

// A grid, read as rows of panes: how many in each, who, and their shares.
const lengths = (layout: Layout) => asRows(layout)!.map(row => row.panes.length)
const seats = (layout: Layout) => asRows(layout)!.map(row => row.panes.map(pane => pane.id))
const sizes = (layout: Layout, row = 0) => asRows(layout)![row]!.panes.map(pane => Number(pane.size.toFixed(3)))
const rowSizes = (layout: Layout) => asRows(layout)!.map(row => Number(row.size.toFixed(3)))
/** The edge after pane `index` of row `row` of a grid, and the edge under row `index`. */
const moveColumnEdge = (layout: Layout, row: number, index: number, at: number) => moveEdge(layout, asRows(layout)!.length > 1 ? [row] : [], index, at)
const moveRowEdge = (layout: Layout, index: number, at: number) => moveEdge(layout, [], index, at)

/** Where a pane is, in hundredths of the room: left, top, width, height. */
const where = (layout: Layout, id: string) => {
  const pane = plan(layout).panes.find(p => p.id === id)!
  return [pane.left, pane.top, pane.width, pane.height].map(n => Math.round(n * 100))
}
const picture = (layout: Layout) => Object.fromEntries(ids(layout).map(id => [id, where(layout, id)]))

const row = (...parts: (string | Node)[]): Node => ({ dir: 'row', parts: parts.map(p => ({ size: 1, node: typeof p === 'string' ? { id: p } : p })) })
const column = (...parts: (string | Node)[]): Node => ({ dir: 'column', parts: parts.map(p => ({ size: 1, node: typeof p === 'string' ? { id: p } : p })) })
const of = (root: Node): Layout => ({ root })

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
    expect(count(layout)).toBe(6)
  })

  it('opens a pane without moving the others', () => {
    const before = arrange(['a', 'b', 'c'])
    const after = open(before, 'x', 'c')
    // c's row had room: x sits beside c, and a and b stay where they were.
    expect(seats(after)).toEqual([['a', 'b'], ['c', 'x']])
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
    expect(seats(layout)).toEqual([['a'], ['c', 'd']])
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
    expect(seats(traded)).toEqual([['c', 'b'], ['a']])
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
    // An edge left where it stands changes nothing.
    expect(moveColumnEdge(three, 0, 0, 1 / 3)).toBe(three)
  })

  it('never lets a pane or a row be dragged away to nothing', () => {
    const squeezed = moveColumnEdge(arrange(['a', 'b']), 0, 0, 0)
    expect(sizes(squeezed)[0]).toBeGreaterThan(0.1)
    expect(sizes(squeezed)[0]! + sizes(squeezed)[1]!).toBeCloseTo(2)
    const rows = moveRowEdge(arrange(['a', 'b', 'c']), 0, 1)
    expect(rowSizes(rows)[1]).toBeGreaterThan(0.1)
    expect(rowSizes(rows)[0]! + rowSizes(rows)[1]!).toBeCloseTo(2)
  })

  it('keeps every pane its least when an edge is dragged across a room within a room', () => {
    // a beside a column of b over a row of c and d: the right half needs room for c and d both.
    const nested = of(row('a', column('b', row('c', 'd'))))
    const dragged = moveEdge(nested, [], 0, 1, { row: 0.1, column: 0.1 })
    expect(where(dragged, 'c')[2]).toBe(10)
    expect(where(dragged, 'd')[2]).toBe(10)
    expect(where(dragged, 'a')[2]).toBe(80)
    // In a room too small for that, neither side is dragged away to nothing.
    const tiny = moveEdge(nested, [], 0, 1, { row: 0.4, column: 0.4 })
    expect(where(tiny, 'a')[2]).toBeLessThan(100)
    expect(where(tiny, 'c')[2]).toBeGreaterThan(0)
  })

  it('keeps a narrow pane inside an uneven split its least when an edge outside it is dragged', () => {
    // A room 1600 by 900, where no pane is narrower than 160 or shorter than 110.
    const least = { row: 160 / 1600, column: 110 / 900 }
    const widths = (layout: Layout) => Object.fromEntries(plan(layout).panes.map(pane => [pane.id, Math.round(pane.width * 1600)]))
    const heights = (layout: Layout) => Object.fromEntries(plan(layout).panes.map(pane => [pane.id, Math.round(pane.height * 900)]))
    // a large, beside a column of b, c and d; e put beside b, and b made the narrow one of the two.
    let room = putBeside(arrange(['a', 'b', 'c', 'd'], 'main-beside', 'a'), 'e', 'b', 'right')
    room = moveEdge(room, [1, 0], 0, 0.3, least)
    expect(widths(room)).toEqual({ a: 960, b: 192, e: 448, c: 640, d: 640 })
    // Dragging a's edge across moves no edge inside the column: b keeps its three tenths of
    // it, so the column stops while b still has its least. It used to go on to 96.
    const squeezed = moveEdge(room, [], 0, 0.8, least)
    expect(widths(squeezed)).toEqual({ a: 1067, b: 160, e: 373, c: 533, d: 533 })
    // Dragged the other way there is nothing in a to keep, but a itself.
    expect(widths(moveEdge(room, [], 0, 0, least)).a).toBe(160)

    // The same the other way up: a above a row of b, c and d; e under b, and b the short one.
    let tall = putBeside(arrange(['a', 'b', 'c', 'd'], 'main-below', 'a'), 'e', 'b', 'bottom')
    tall = moveEdge(tall, [1, 0], 0, 0.4, least)
    const pressed = moveEdge(tall, [], 0, 0.95, least)
    expect(heights(pressed).b).toBe(110)
    expect(Object.values(heights(pressed)).every(height => height >= 110)).toBe(true)
  })

  it('makes everything even again', () => {
    const messy = moveRowEdge(moveColumnEdge(arrange(['a', 'b', 'c']), 0, 0, 0.8), 0, 0.3)
    expect(even(messy)).toEqual(arrange(['a', 'b', 'c']))
    expect(even(EMPTY)).toBe(EMPTY)
  })

  it('reads a layout back only if it is one', () => {
    const layout = moveColumnEdge(arrange(['a', 'b', 'c']), 0, 0, 0.6)
    expect(parse(JSON.parse(JSON.stringify(layout)))).toEqual(layout)
    for (const bad of [null, 'x', {}, { rows: 'x' }, { rows: [{ panes: [{ id: '' }] }] }, { rows: [{ panes: [{ id: 'a' }, { id: 'a' }] }] }, { root: 'x' }, { root: { id: '' } }, { root: { dir: 'sideways', parts: [] } }, { root: row('a', 'a') }, { root: { dir: 'row', parts: [{ size: 1 }] } }]) {
      expect(parse(bad)).toEqual(EMPTY)
    }
    expect(parse({ root: null })).toEqual(EMPTY)
    // A size that is not a share becomes an even one.
    expect(parse({ rows: [{ size: -3, panes: [{ id: 'a', size: 'wide' }] }] })).toEqual(arrange(['a']))
    expect(parse({ root: { dir: 'row', parts: [{ size: 'wide', node: { id: 'a' } }, { size: -1, node: { id: 'b' } }] } })).toEqual(arrange(['a', 'b']))
  })

  it('still reads the rows of panes an earlier office kept', () => {
    const kept = { rows: [{ size: 1, panes: [{ id: 'a', size: 1.4 }, { id: 'b', size: 0.6 }] }, { size: 2, panes: [{ id: 'c', size: 1 }] }] }
    const layout = parse(kept)
    expect(seats(layout)).toEqual([['a', 'b'], ['c']])
    expect(sizes(layout)).toEqual([1.4, 0.6])
    expect(rowSizes(layout)).toEqual([1, 2])
    expect(picture(layout)).toEqual({ a: [0, 0, 70, 33], b: [70, 0, 30, 33], c: [0, 33, 100, 67] })
  })

  it('reads any tree back in the one form the same picture is kept in', () => {
    // A split of one is that one; a row inside a row is one row, everyone where they were.
    expect(parse({ root: { dir: 'row', parts: [{ size: 1, node: { id: 'a' } }] } })).toEqual(arrange(['a']))
    const nested = { root: { dir: 'row', parts: [{ size: 1, node: { id: 'a' } }, { size: 3, node: row('b', 'c', 'd') }] } }
    const flat = parse(nested)
    expect(sideBySide(flat)).toBe(4)
    expect(picture(flat)).toEqual({ a: [0, 0, 25, 100], b: [25, 0, 25, 100], c: [50, 0, 25, 100], d: [75, 0, 25, 100] })
    // Nothing the office builds is this deep.
    let deep: Node = { id: 'z' }
    for (let i = 0; i < 40; i++) deep = i % 2 ? row(`r${i}`, deep) : column(`c${i}`, deep)
    expect(parse({ root: deep })).toEqual(EMPTY)
  })

  it('drops the panes of desks that have gone', () => {
    const before = arrange(['a', 'b', 'c', 'd'])
    const layout = only(before, id => id !== 'b' && id !== 'd')
    expect(ids(layout)).toEqual(['a', 'c'])
    expect(has(layout, 'b')).toBe(false)
    // With everyone still here it is the very same layout: nothing to lay out again.
    expect(only(before, () => true)).toBe(before)
  })

  it('reads the panes as the eye does, however the room was built', () => {
    // Two columns of two are read across, like two rows of two.
    expect(ids(of(row(column('a', 'b'), column('c', 'd'))))).toEqual(['a', 'c', 'b', 'd'])
    expect(ids(arrange(['a', 'b', 'c'], 'main-beside', 'b'))).toEqual(['b', 'a', 'c'])
    expect(samePicture(of(row(column('a', 'b'), column('c', 'd'))), of(column(row('a', 'c'), row('b', 'd'))))).toBe(true)
    expect(samePicture(arrange(['a', 'b']), arrange(['b', 'a']))).toBe(false)
  })

  it('says when the room is nothing but panes side by side', () => {
    expect([EMPTY, arrange(['a']), arrange(['a', 'b']), arrange(['a', 'b', 'c']), arrange(['a', 'b', 'c'], 'side'), arrange(['a', 'b'], 'stacked')].map(sideBySide)).toEqual([0, 1, 2, 0, 3, 0])
  })
})

describe('putting a pane on a side of another', () => {
  it('stacks two that were side by side', () => {
    const layout = putBeside(arrange(['a', 'b']), 'a', 'b', 'bottom')
    expect(picture(layout)).toEqual({ b: [0, 0, 100, 50], a: [0, 50, 100, 50] })
    expect(picture(putBeside(arrange(['a', 'b']), 'a', 'b', 'top'))).toEqual({ a: [0, 0, 100, 50], b: [0, 50, 100, 50] })
  })

  it('changes its place in a line without changing anyone\'s size', () => {
    const dragged = moveColumnEdge(arrange(['a', 'b', 'c'], 'side'), 0, 0, 0.5)
    const moved = putBeside(dragged, 'a', 'c', 'right')
    expect(ids(moved)).toEqual(['b', 'c', 'a'])
    expect(where(moved, 'a')[2]).toBe(where(dragged, 'a')[2])
    expect(where(moved, 'b')[2]).toBe(where(dragged, 'b')[2])
    expect(ids(putBeside(arrange(['a', 'b', 'c'], 'side'), 'c', 'a', 'left'))).toEqual(['c', 'a', 'b'])
  })

  it('changes nothing where it already stands there', () => {
    const two = arrange(['a', 'b'])
    expect(putBeside(two, 'a', 'b', 'left')).toBe(two)
    expect(putBeside(two, 'b', 'a', 'right')).toBe(two)
    expect(putBeside(two, 'a', 'a', 'left')).toBe(two)
    expect(putBeside(two, 'a', 'nobody', 'left')).toBe(two)
  })

  it('shares the room the other had, when their line runs the other way', () => {
    // Above b in a row of two: a and b share b's half; nothing else is there to move.
    const grid = arrange(['a', 'b', 'c', 'd'])
    const moved = putBeside(grid, 'a', 'd', 'right')
    expect(picture(moved)).toEqual({ b: [0, 0, 100, 50], c: [0, 50, 33, 50], d: [33, 50, 33, 50], a: [67, 50, 33, 50] })
    const under = putBeside(grid, 'a', 'd', 'bottom')
    expect(picture(under)).toEqual({ b: [0, 0, 100, 50], c: [0, 50, 50, 50], d: [50, 50, 50, 25], a: [50, 75, 50, 25] })
  })

  it('opens a pane that was not open, on that side', () => {
    const layout = putBeside(arrange(['a', 'b']), 'x', 'a', 'bottom')
    expect(picture(layout)).toEqual({ a: [0, 0, 50, 50], b: [50, 0, 50, 100], x: [0, 50, 50, 50] })
  })

  it('takes half of a neighbour whose line was dragged, and an even share of one that was not', () => {
    const dragged = moveColumnEdge(arrange(['a', 'b']), 0, 0, 0.7)
    expect(picture(putBeside(dragged, 'x', 'a', 'right'))).toEqual({ a: [0, 0, 35, 100], x: [35, 0, 35, 100], b: [70, 0, 30, 100] })
    expect(picture(putBeside(arrange(['a', 'b']), 'x', 'a', 'right'))).toEqual({ a: [0, 0, 33, 100], x: [33, 0, 33, 100], b: [67, 0, 33, 100] })
  })

  it('finds the other again when the two were all there was of a line', () => {
    // a and b side by side inside a column: moving a under b folds their row away first.
    const layout = of(column(row('a', 'b'), 'c'))
    expect(picture(putBeside(layout, 'a', 'b', 'bottom'))).toEqual({ b: [0, 0, 100, 33], a: [0, 33, 100, 33], c: [0, 67, 100, 33] })
  })
})

describe('giving a pane a whole side of the room', () => {
  it('stands it along that side, with the others in what is left', () => {
    const grid = arrange(['a', 'b', 'c', 'd'])
    expect(picture(putAtEdge(grid, 'a', 'left'))).toEqual({ a: [0, 0, 33, 100], b: [33, 0, 67, 50], c: [33, 50, 33, 50], d: [67, 50, 33, 50] })
    expect(picture(putAtEdge(arrange(['a', 'b']), 'a', 'bottom'))).toEqual({ b: [0, 0, 100, 50], a: [0, 50, 100, 50] })
  })

  it('gives it as much as one more in the line would have, and never less than a quarter', () => {
    expect(where(putAtEdge(arrange(['a', 'b', 'c'], 'stacked'), 'x', 'bottom'), 'x')).toEqual([0, 75, 100, 25])
    expect(where(putAtEdge(arrange(['a', 'b', 'c', 'd', 'e'], 'side'), 'x', 'left'), 'x')).toEqual([0, 0, 25, 100])
    expect(where(putAtEdge(arrange(['a', 'b', 'c'], 'side'), 'x', 'bottom'), 'x')).toEqual([0, 50, 100, 50])
  })

  it('changes nothing where it already has that side', () => {
    const two = arrange(['a', 'b'])
    expect(putAtEdge(two, 'a', 'left')).toBe(two)
    expect(putAtEdge(two, 'b', 'right')).toBe(two)
    const one = arrange(['a'])
    expect(putAtEdge(one, 'a', 'top')).toBe(one)
    expect(putAtEdge(EMPTY, 'a', 'top')).toEqual(one)
  })
})

describe('moving between panes', () => {
  it('knows the pane next to another on each side', () => {
    const grid = arrange(['a', 'b', 'c', 'd'])
    expect((['left', 'right', 'top', 'bottom'] as Side[]).map(side => neighbour(grid, 'a', side))).toEqual([null, 'b', null, 'c'])
    expect((['left', 'right', 'top', 'bottom'] as Side[]).map(side => neighbour(grid, 'd', side))).toEqual(['c', null, 'b', null])
    // Of two that are as near, the higher (or the one further left).
    const large = arrange(['m', 's', 't'], 'main-beside')
    expect(neighbour(large, 'm', 'right')).toBe('s')
    expect(neighbour(large, 't', 'left')).toBe('m')
    expect(neighbour(arrange(['m', 's', 't'], 'main-below'), 'm', 'bottom')).toBe('s')
    // Two over one: above the wide one, both are as near and overlap it as much, so the one further left.
    expect(neighbour(arrange(['a', 'b', 'c']), 'c', 'top')).toBe('a')
    // Three over two: above each of the two is the one that overlaps it more.
    const five = arrange(['a', 'b', 'c', 'd', 'e'])
    expect(neighbour(five, 'd', 'top')).toBe('a')
    expect(neighbour(five, 'e', 'top')).toBe('c')
    expect(neighbour(grid, 'nobody', 'left')).toBeNull()
  })

  it('moves a pane by changing places with its neighbour, or to the whole side when there is none', () => {
    const two = arrange(['a', 'b'])
    expect(ids(move(two, 'a', 'right'))).toEqual(['b', 'a'])
    expect(picture(move(two, 'a', 'bottom'))).toEqual({ b: [0, 0, 100, 50], a: [0, 50, 100, 50] })
    expect(move(two, 'a', 'left')).toBe(two)
    expect(move(two, 'nobody', 'left')).toBe(two)
    const grid = arrange(['a', 'b', 'c', 'd'])
    expect(seats(move(grid, 'a', 'bottom'))).toEqual([['c', 'b'], ['a', 'd']])
    expect(where(move(grid, 'a', 'left'), 'a')).toEqual([0, 0, 33, 100])
  })
})

describe('aiming a dragged pane', () => {
  const room = { width: 1000, height: 800 }
  const two = arrange(['a', 'b'])

  it('goes nowhere over itself, or where it already is', () => {
    expect(aim(two, 'a', 100, 400, room)).toBeNull()
    // On the near side of its neighbour: it is there already.
    expect(aim(two, 'a', 520, 400, room)).toBeNull()
    // Past the side of the room it already has.
    expect(aim(two, 'a', -10, 400, room)).toBeNull()
    expect(aim(two, 'nobody', 100, 400, room)).toBeNull()
  })

  it('changes places on a header, or in the middle of a pane', () => {
    expect(aim(two, 'a', 750, 20, room)).toEqual({ kind: 'trade', target: 'b' })
    expect(aim(two, 'a', 750, 400, room)).toEqual({ kind: 'trade', target: 'b' })
  })

  it('goes to the side of a pane it is near', () => {
    expect(aim(two, 'a', 900, 400, room)).toEqual({ kind: 'beside', target: 'b', side: 'right' })
    expect(aim(two, 'a', 750, 700, room)).toEqual({ kind: 'beside', target: 'b', side: 'bottom' })
    expect(aim(two, 'a', 750, 100, room)).toEqual({ kind: 'beside', target: 'b', side: 'top' })
    // In a corner: the side it is nearer, by how far that side's band reaches.
    expect(aim(two, 'a', 960, 500, room)).toEqual({ kind: 'beside', target: 'b', side: 'right' })
    expect(aim(two, 'a', 880, 700, room)).toEqual({ kind: 'beside', target: 'b', side: 'bottom' })
  })

  it('takes a whole side at the edge of the room, and past it', () => {
    expect(aim(two, 'a', 990, 400, room)).toEqual({ kind: 'edge', side: 'right' })
    expect(aim(two, 'a', 1040, 400, room)).toEqual({ kind: 'edge', side: 'right' })
    expect(aim(two, 'a', 750, 790, room)).toEqual({ kind: 'edge', side: 'bottom' })
    // The top of the room is headers, where two change places; only past it is the top side.
    expect(aim(two, 'a', 750, 5, room)).toEqual({ kind: 'trade', target: 'b' })
    expect(aim(two, 'a', 750, -5, room)).toEqual({ kind: 'edge', side: 'top' })
  })

  it('offers no sides on a pane too small to aim at one', () => {
    const small = { width: 500, height: 200 }
    expect(aim(two, 'a', 480, 100, small)).toEqual({ kind: 'edge', side: 'right' })
    expect(aim(two, 'a', 440, 100, small)).toEqual({ kind: 'trade', target: 'b' })
    expect(aim(two, 'a', 300, 190, small)).toEqual({ kind: 'edge', side: 'bottom' })
    expect(aim(two, 'a', 300, 150, small)).toEqual({ kind: 'trade', target: 'b' })
  })

  it('lands where it was aimed', () => {
    expect(picture(land(two, 'a', { kind: 'beside', target: 'b', side: 'bottom' }))).toEqual({ b: [0, 0, 100, 50], a: [0, 50, 100, 50] })
    expect(ids(land(two, 'a', { kind: 'trade', target: 'b' }))).toEqual(['b', 'a'])
    expect(ids(land(two, 'a', { kind: 'edge', side: 'right' }))).toEqual(['b', 'a'])
  })
})

describe('arranging every pane at once', () => {
  it('puts them in each shape', () => {
    const list = ['a', 'b', 'c']
    expect(picture(arrange(list, 'side'))).toEqual({ a: [0, 0, 33, 100], b: [33, 0, 33, 100], c: [67, 0, 33, 100] })
    expect(picture(arrange(list, 'stacked'))).toEqual({ a: [0, 0, 100, 33], b: [0, 33, 100, 33], c: [0, 67, 100, 33] })
    expect(picture(arrange(list, 'main-beside', 'b'))).toEqual({ b: [0, 0, 60, 100], a: [60, 0, 40, 50], c: [60, 50, 40, 50] })
    expect(picture(arrange(list, 'main-below', 'c'))).toEqual({ c: [0, 0, 100, 60], a: [0, 60, 50, 40], b: [50, 60, 50, 40] })
    expect(picture(arrange(list, 'grid'))).toEqual({ a: [0, 0, 50, 50], b: [50, 0, 50, 50], c: [0, 50, 100, 50] })
  })

  it('makes do with one or two', () => {
    for (const to of ['grid', 'side', 'stacked', 'main-beside', 'main-below'] as Shape[]) {
      expect(arrange(['a'], to)).toEqual({ root: { id: 'a' } })
      expect(arrange([], to)).toEqual(EMPTY)
    }
    expect(picture(arrange(['a', 'b'], 'main-beside'))).toEqual({ a: [0, 0, 60, 100], b: [60, 0, 40, 100] })
    expect(picture(arrange(['a', 'b'], 'main-below', 'nobody'))).toEqual({ a: [0, 0, 100, 60], b: [0, 60, 100, 40] })
  })

  it('opens the next pane in a room built by hand beside the one it was asked from', () => {
    const large = arrange(['m', 's', 't'], 'main-beside')
    // The line of small ones has room for one more in a tall room, so it joins them.
    expect(picture(open(large, 'x', 't', { width: 1600, height: 1100 }))).toEqual({ m: [0, 0, 60, 100], s: [60, 0, 40, 33], t: [60, 33, 40, 33], x: [60, 67, 40, 33] })
    // In a short one it does not: it shares the large pane's width, which is wide enough for two.
    expect(picture(open(large, 'x', 'm', { width: 2000, height: 600 }))).toEqual({ m: [0, 0, 30, 100], x: [30, 0, 30, 100], s: [60, 0, 40, 50], t: [60, 50, 40, 50] })
    // Too narrow to stand beside: below, where there is the height for it.
    expect(picture(open(large, 'x', 'm', { width: 1200, height: 900 }))).toEqual({ m: [0, 0, 60, 50], x: [0, 50, 60, 50], s: [60, 0, 40, 50], t: [60, 50, 40, 50] })
  })
})

// ── the grid as it was laid out before a pane could be put anywhere ─────────
// Kept here, word for word, as what `open` and `close` are held to in a grid.

interface OldPane { id: string; size: number }
interface OldRow { size: number; panes: OldPane[] }
interface OldLayout { rows: OldRow[] }

const oldIds = (layout: OldLayout) => layout.rows.flatMap(r => r.panes.map(pane => pane.id))
const oldMean = (values: number[]) => (values.length > 0 ? values.reduce((total, value) => total + value, 0) / values.length : 1)
const oldEvenRow = (panes: OldPane[]) => panes.every(pane => Math.abs(pane.size - panes[0]!.size) < 1e-6)

function oldOpen(layout: OldLayout, id: string, beside?: string): OldLayout {
  if (oldIds(layout).includes(id)) return layout
  const list = oldIds(layout)
  if (list.length === 0) return { rows: [{ size: 1, panes: [{ id, size: 1 }] }] }
  const widest = Math.ceil(Math.sqrt(list.length + 1))
  const anchor = beside !== undefined && list.includes(beside) ? beside : list.at(-1)!
  const anchorRow = layout.rows.findIndex(r => r.panes.some(pane => pane.id === anchor))
  let target = anchorRow
  if (layout.rows[target]!.panes.length >= widest) {
    const roomy = layout.rows.map((r, at) => ({ at, length: r.panes.length })).filter(r => r.length < widest)
    target = roomy.length > 0 ? roomy.reduce((best, r) => (r.length < best.length ? r : best)).at : -1
  }
  if (target < 0) return { rows: [...layout.rows, { size: oldMean(layout.rows.map(r => r.size)), panes: [{ id, size: 1 }] }] }
  return {
    rows: layout.rows.map((r, at) => {
      if (at !== target) return r
      const near = at === anchorRow ? r.panes.findIndex(pane => pane.id === anchor) : r.panes.length - 1
      if (oldEvenRow(r.panes)) {
        const panes = r.panes.map(pane => ({ ...pane, size: 1 }))
        panes.splice(near + 1, 0, { id, size: 1 })
        return { ...r, panes }
      }
      const half = r.panes[near]!.size / 2
      const panes = r.panes.map((pane, i) => (i === near ? { ...pane, size: half } : pane))
      panes.splice(near + 1, 0, { id, size: half })
      return { ...r, panes }
    })
  }
}

function oldClose(layout: OldLayout, id: string): OldLayout {
  if (!oldIds(layout).includes(id)) return layout
  const rows = layout.rows
    .map(r => {
      const at = r.panes.findIndex(pane => pane.id === id)
      if (at < 0) return r
      const left = r.panes.filter(pane => pane.id !== id)
      if (left.length === 0 || oldEvenRow(r.panes)) return { ...r, panes: left.map(pane => ({ ...pane, size: 1 })) }
      const heir = Math.max(0, at - 1)
      const share = r.panes[at]!.size
      return { ...r, panes: left.map((pane, i) => (i === heir ? { ...pane, size: pane.size + share } : pane)) }
    })
    .filter(r => r.panes.length > 0)
  return { rows }
}

/** A sequence of numbers from 0 up to 1 that is the same every run. */
function dice(seed: number) {
  let state = seed >>> 0
  return () => {
    state = (state + 0x6d2b79f5) >>> 0
    let t = state
    t = Math.imul(t ^ (t >>> 15), t | 1)
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

/** Shares of a whole, so layouts that keep them as different numbers can be compared. */
const shares = (values: number[]) => values.map(value => Number((value / values.reduce((total, v) => total + v, 0)).toFixed(6)))

describe('a grid nobody has rearranged', () => {
  it('opens and closes exactly as it always did', () => {
    for (let seed = 1; seed <= 40; seed++) {
      const roll = dice(seed)
      const pick = <T,>(list: T[]) => list[Math.floor(roll() * list.length)]!
      let was: OldLayout = { rows: [] }
      let now: Layout = EMPTY
      let made = 0
      for (let step = 0; step < 60; step++) {
        const here = oldIds(was)
        if (here.length === 0 || (here.length < 9 && roll() < 0.6)) {
          const id = `p${made++}`
          const beside = here.length > 0 && roll() < 0.7 ? pick(here) : undefined
          was = oldOpen(was, id, beside)
          now = open(now, id, beside)
        } else {
          const id = pick(here)
          was = oldClose(was, id)
          now = close(now, id)
        }
        const rows = asRows(now)
        expect(rows, `seed ${seed}, step ${step}: still a grid`).not.toBeNull()
        expect(rows!.map(r => r.panes.map(pane => pane.id))).toEqual(was.rows.map(r => r.panes.map(pane => pane.id)))
        expect(ids(now)).toEqual(oldIds(was))
        expect(shares(rows!.map(r => r.size))).toEqual(shares(was.rows.map(r => r.size)))
        rows!.forEach((r, i) => expect(shares(r.panes.map(pane => pane.size))).toEqual(shares(was.rows[i]!.panes.map(pane => pane.size))))
      }
    }
  })
})

describe('whatever is done to a room', () => {
  const sides: Side[] = ['left', 'right', 'top', 'bottom']
  const shapes: Shape[] = ['grid', 'side', 'stacked', 'main-beside', 'main-below']

  /** The one form a room is kept in: no split of one, none inside one running the same way, every share a share. */
  function wellFormed(node: Node, inside?: 'row' | 'column'): string | null {
    if ('id' in node) return node.id ? null : 'a pane with no desk'
    if (node.parts.length < 2) return 'a split of fewer than two'
    if (node.dir === inside) return 'a split inside one that runs the same way'
    for (const part of node.parts) {
      if (!(part.size > 0) || !Number.isFinite(part.size)) return `a share of ${part.size}`
      const wrong = wellFormed(part.node, node.dir)
      if (wrong) return wrong
    }
    return null
  }

  it('is still one room, every pane in it once and none over another', () => {
    for (let seed = 1; seed <= 60; seed++) {
      const roll = dice(seed * 7919)
      const pick = <T,>(list: T[]) => list[Math.floor(roll() * list.length)]!
      let layout: Layout = EMPTY
      const open_ = new Set<string>()
      let made = 0
      for (let step = 0; step < 80; step++) {
        const here = [...open_]
        const what = roll()
        const room = { width: 500 + roll() * 2000, height: 300 + roll() * 1100 }
        let did = ''
        if (here.length === 0 || (here.length < 8 && what < 0.2)) {
          const id = `p${made++}`
          layout = open(layout, id, here.length > 0 && roll() < 0.6 ? pick(here) : undefined, room)
          open_.add(id)
          did = `open ${id}`
        } else if (what < 0.3) {
          const id = pick(here)
          layout = close(layout, id)
          open_.delete(id)
          did = `close ${id}`
        } else if (what < 0.5) {
          // A pane that is open moves; one that is not, opens there.
          const fresh = here.length < 8 && roll() < 0.3
          const id = fresh ? `p${made++}` : pick(here)
          const target = pick(here)
          const side = pick(sides)
          const next = putBeside(layout, id, target, side)
          if (id !== target) open_.add(id)
          did = `put ${id} ${side} of ${target}`
          if (id !== target) {
            // It ends up on that side of the other, touching it.
            const mine = plan(next).panes.find(p => p.id === id)!
            const other = plan(next).panes.find(p => p.id === target)!
            const touching = side === 'left' ? mine.left + mine.width - other.left : side === 'right' ? other.left + other.width - mine.left : side === 'top' ? mine.top + mine.height - other.top : other.top + other.height - mine.top
            expect(Math.abs(touching), `seed ${seed}, step ${step}: ${did}`).toBeLessThan(1e-6)
          }
          layout = next
        } else if (what < 0.6) {
          const fresh = here.length < 8 && roll() < 0.3
          const id = fresh ? `p${made++}` : pick(here)
          const side = pick(sides)
          layout = putAtEdge(layout, id, side)
          open_.add(id)
          did = `put ${id} along the ${side}`
          const mine = plan(layout).panes.find(p => p.id === id)!
          const along = side === 'left' || side === 'right' ? mine.height : mine.width
          const against = side === 'left' ? mine.left : side === 'top' ? mine.top : side === 'right' ? 1 - mine.left - mine.width : 1 - mine.top - mine.height
          expect(along, `seed ${seed}, step ${step}: ${did} takes the whole side`).toBeCloseTo(1)
          expect(against, `seed ${seed}, step ${step}: ${did} stands at it`).toBeCloseTo(0)
        } else if (what < 0.7 && here.length > 1) {
          const [a, b] = [pick(here), pick(here)]
          const before = plan(layout).panes
          layout = trade(layout, a, b)
          did = `trade ${a} ${b}`
          const after = plan(layout).panes
          const at = (list: typeof before, id: string) => { const p = list.find(pane => pane.id === id)!; return [p.left, p.top, p.width, p.height] }
          expect(at(after, a)).toEqual(at(before, b))
          expect(at(after, b)).toEqual(at(before, a))
        } else if (what < 0.8) {
          const id = pick(here)
          const side = pick(sides)
          layout = move(layout, id, side)
          did = `move ${id} ${side}`
        } else if (what < 0.92) {
          const edges = plan(layout).edges
          if (edges.length > 0) {
            const edge = pick(edges)
            const least = { row: 0.02 + roll() * 0.1, column: 0.02 + roll() * 0.1 }
            // The way the edge's split runs, and how much of that way each pane has.
            const dir = edge.kind === 'column' ? 'row' : 'column'
            const along = (pane: { width: number; height: number }) => (dir === 'row' ? pane.width : pane.height)
            const roomy = plan(layout).panes.every(pane => along(pane) >= least[dir] - 1e-9)
            layout = moveEdge(layout, edge.path, edge.index, roll() * 1.4 - 0.2, least)
            did = `drag edge ${edge.key}`
            // With room for everyone before, nobody is under the least after, however deep
            // in the dragged split they sit and whatever share of it they have.
            if (roomy) for (const pane of plan(layout).panes) expect(along(pane), `seed ${seed}, step ${step}: ${did} keeps ${pane.id} its least`).toBeGreaterThanOrEqual(least[dir] - 1e-9)
          }
        } else if (what < 0.96) {
          layout = arrange(here, pick(shapes), pick(here))
          did = 'arrange'
        } else {
          layout = even(layout)
          did = 'even'
        }

        const said = `seed ${seed}, step ${step}: ${did}`
        if (layout.root) expect(wellFormed(layout.root), said).toBeNull()
        const panes = plan(layout).panes
        expect(new Set(panes.map(p => p.id)), said).toEqual(open_)
        expect(panes.length, said).toBe(open_.size)
        expect(count(layout), said).toBe(open_.size)
        // They fill the room between them, each inside it, and no two share any of it.
        expect(panes.reduce((area, p) => area + p.width * p.height, 0), said).toBeCloseTo(open_.size === 0 ? 0 : 1, 6)
        for (const p of panes) {
          expect(p.left >= -1e-9 && p.top >= -1e-9 && p.left + p.width <= 1 + 1e-9 && p.top + p.height <= 1 + 1e-9 && p.width > 0 && p.height > 0, said).toBe(true)
          for (const q of panes) {
            if (p === q) continue
            const shared = Math.max(0, Math.min(p.left + p.width, q.left + q.width) - Math.max(p.left, q.left)) * Math.max(0, Math.min(p.top + p.height, q.top + q.height) - Math.max(p.top, q.top))
            expect(shared, said).toBeLessThan(1e-9)
          }
        }
        // Every edge stands inside its own split.
        for (const edge of plan(layout).edges) expect(edge.at > 0 && edge.at < 1, said).toBe(true)
        // Kept and read back, it is the same room; and with nobody gone, the very same one.
        expect(parse(JSON.parse(JSON.stringify(layout))), said).toEqual(layout)
        expect(only(layout, () => true), said).toBe(layout)
      }
    }
  })
})

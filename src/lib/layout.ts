// How the open terminals share the window. One pane fills it; two sit side by
// side; more make a grid. Any edge between two can be dragged, and a pane can be
// put on any side of another, or along a whole side of the room.
//
// The room is a tree: a pane, or panes and smaller rooms side by side (a row) or
// one above another (a column). It is kept in one form, so that the same picture
// is always the same tree: a split has two parts or more, and a split inside a
// split runs the other way.
//
// Everything here is a plain function of the layout it is given, so it can be
// tested without a window. A function that changes nothing returns the layout it
// was given, the very same one.

export interface Pane {
  /** The desk whose terminal this is. */
  id: string
}

/** Which way a split runs: its parts side by side, or one above another. */
export type Dir = 'row' | 'column'

export interface Part {
  /** Its share of the split, against the other parts. */
  size: number
  node: Node
}

export interface Split {
  dir: Dir
  parts: Part[]
}

export type Node = Pane | Split

export interface Layout {
  root: Node | null
}

export type Side = 'left' | 'right' | 'top' | 'bottom'

/** A shape all the open panes can be put into at once. */
export type Shape = 'grid' | 'side' | 'stacked' | 'main-beside' | 'main-below'

/** Where a pane is, as shares of the room. */
export interface Placed {
  id: string
  left: number
  top: number
  width: number
  height: number
}

/** An edge between two parts of a split, which can be dragged. */
export interface Edge {
  key: string
  /** `column`: an upright line between two side by side. `row`: a level line between two stacked. */
  kind: Dir
  /** The way from the root to its split, and which of that split's edges it is. */
  path: number[]
  index: number
  /** Where the line is, and how long, as shares of the room. */
  left: number
  top: number
  length: number
  /** Where its split starts and how far it runs along its own way, as shares of the room. */
  from: number
  span: number
  /** Where the edge stands within its split, from 0 to 1. */
  at: number
}

/** The least a pane may be dragged down to, as shares of the room's width (`row`) and height (`column`). */
export interface Least {
  row: number
  column: number
}

/** The room's size in pixels, for the rules that depend on how big a pane really is. */
export interface Room {
  width: number
  height: number
}

/** Where a pane being dragged would go if it were let go here. */
export type Aim = { kind: 'edge'; side: Side } | { kind: 'beside'; target: string; side: Side } | { kind: 'trade'; target: string }

export const EMPTY: Layout = { root: null }

const LEAST: Least = { row: 0.075, column: 0.075 }
/** A room to reckon with when none is given: a laptop's worth of terminals. */
const USUAL_ROOM: Room = { width: 1400, height: 800 }
/** In a room built by hand, a pane is put beside another only where each stays this wide, or below it where each stays this tall. */
const WIDE_ENOUGH = 560
const TALL_ENOUGH = 260
/** How close to the room's own edge a pane is let go to take that whole side. */
const ROOM_EDGE = 24
/** The height of a pane's header: let go on one, two panes change places. */
const HEADER = 46
/** How much of a pane, from each of its edges, puts another on that side of it. */
const BAND = 0.3
/** A pane narrower or shorter than this is too small to aim at a side of. */
const AIM_WIDTH = 320
const AIM_HEIGHT = 220
const EPS = 1e-6

const isPane = (node: Node): node is Pane => 'id' in node
const sum = (values: number[]) => values.reduce((total, value) => total + value, 0)
const total = (split: Split) => sum(split.parts.map(part => part.size))
const axisOf = (side: Side): Dir => (side === 'left' || side === 'right' ? 'row' : 'column')
const before = (side: Side) => side === 'left' || side === 'top'
/** Whether parts all have the same share: nobody has dragged their edges. */
const evenParts = (parts: Part[]) => parts.every(part => Math.abs(part.size - parts[0]!.size) <= EPS * Math.max(part.size, parts[0]!.size))

// ── the tree ───────────────────────────────────────────────────────────────

/** Every pane under a node, in the order the tree holds them. */
function leaves(node: Node | null, out: string[] = []): string[] {
  if (!node) return out
  if (isPane(node)) out.push(node.id)
  else for (const part of node.parts) leaves(part.node, out)
  return out
}

/** The way from a node down to a pane: which part to take at each split. */
function find(node: Node | null, id: string, path: number[] = []): number[] | null {
  if (!node) return null
  if (isPane(node)) return node.id === id ? path : null
  for (let i = 0; i < node.parts.length; i++) {
    const found = find(node.parts[i]!.node, id, [...path, i])
    if (found) return found
  }
  return null
}

function nodeAt(node: Node | null, path: number[]): Node | null {
  let here = node
  for (const step of path) {
    if (!here || isPane(here)) return null
    here = here.parts[step]?.node ?? null
  }
  return here
}

/** The tree with the node at the end of a path changed, and nothing else. */
function change(node: Node, path: number[], to: (node: Node) => Node): Node {
  if (path.length === 0) return to(node)
  if (isPane(node)) return node
  const [step, ...rest] = path
  return { dir: node.dir, parts: node.parts.map((part, i) => (i === step ? { size: part.size, node: change(part.node, rest, to) } : part)) }
}

/**
 * The same picture in the one form it is kept in: a split left with one part is
 * that part, and a split inside one that runs the same way is folded into it, its
 * parts keeping exactly the room they had.
 */
function tidy(node: Node): Node | null {
  if (isPane(node)) return node
  const parts: Part[] = []
  for (const part of node.parts) {
    const child = tidy(part.node)
    if (!child) continue
    if (!isPane(child) && child.dir === node.dir) {
      const all = total(child)
      for (const inner of child.parts) parts.push({ size: (inner.size * part.size) / all, node: inner.node })
    } else parts.push({ size: part.size, node: child })
  }
  if (parts.length === 0) return null
  if (parts.length === 1) return parts[0]!.node
  return { dir: node.dir, parts }
}

const made = (node: Node | null): Layout => {
  const root = node ? tidy(node) : null
  return root ? { root } : EMPTY
}

// ── where everything is ────────────────────────────────────────────────────

interface Box {
  left: number
  top: number
  width: number
  height: number
}

const WHOLE: Box = { left: 0, top: 0, width: 1, height: 1 }

/** The room one part of a split has, given the room its split has. */
function partBox(split: Split, index: number, box: Box): Box {
  const all = total(split) || 1
  const offset = sum(split.parts.slice(0, index).map(part => part.size)) / all
  const share = split.parts[index]!.size / all
  return split.dir === 'row'
    ? { left: box.left + offset * box.width, top: box.top, width: share * box.width, height: box.height }
    : { left: box.left, top: box.top + offset * box.height, width: box.width, height: share * box.height }
}

function boxAt(root: Node, path: number[]): Box {
  let box = WHOLE
  let here = root
  for (const step of path) {
    if (isPane(here)) break
    box = partBox(here, step, box)
    here = here.parts[step]!.node
  }
  return box
}

function walk(node: Node, box: Box, path: number[], panes: Placed[], edges: Edge[]) {
  if (isPane(node)) {
    panes.push({ id: node.id, ...box })
    return
  }
  const across = node.dir === 'row'
  node.parts.forEach((part, i) => {
    const mine = partBox(node, i, box)
    walk(part.node, mine, [...path, i], panes, edges)
    if (i === node.parts.length - 1) return
    const from = across ? box.left : box.top
    const span = across ? box.width : box.height
    const edge = across ? mine.left + mine.width : mine.top + mine.height
    edges.push({
      key: `${path.join('.')}:${i}`,
      kind: across ? 'column' : 'row',
      path,
      index: i,
      left: across ? edge : box.left,
      top: across ? box.top : edge,
      length: across ? box.height : box.width,
      from,
      span,
      at: span > 0 ? (edge - from) / span : 0
    })
  })
}

/** As the eye reads them: the higher first, and of two as high as each other, the one to the left. */
const reading = (a: Placed, b: Placed) => (Math.abs(a.top - b.top) > EPS ? a.top - b.top : a.left - b.left)

/** Where each pane and each edge is, as shares of the room. Panes come in reading order. */
export function plan(layout: Layout): { panes: Placed[]; edges: Edge[] } {
  const panes: Placed[] = []
  const edges: Edge[] = []
  if (layout.root) walk(layout.root, WHOLE, [], panes, edges)
  panes.sort(reading)
  return { panes, edges }
}

/** The open panes, in reading order. */
export function ids(layout: Layout): string[] {
  return plan(layout).panes.map(pane => pane.id)
}

export function has(layout: Layout, id: string): boolean {
  return find(layout.root, id) !== null
}

export function count(layout: Layout): number {
  return leaves(layout.root).length
}

/** How many panes stand side by side, when that is all the room is: one, or a single row of them. Otherwise none. */
export function sideBySide(layout: Layout): number {
  const root = layout.root
  if (!root) return 0
  if (isPane(root)) return 1
  return root.dir === 'row' && root.parts.every(part => isPane(part.node)) ? root.parts.length : 0
}

/** Whether two layouts are the same picture: every pane in the same place. */
export function samePicture(a: Layout, b: Layout): boolean {
  if (a === b) return true
  const was = new Map(plan(a).panes.map(pane => [pane.id, pane]))
  const now = plan(b).panes
  if (was.size !== now.length) return false
  return now.every(pane => {
    const old = was.get(pane.id)
    return old !== undefined && Math.abs(old.left - pane.left) < EPS && Math.abs(old.top - pane.top) < EPS && Math.abs(old.width - pane.width) < EPS && Math.abs(old.height - pane.height) < EPS
  })
}

// ── a grid: rows of panes ──────────────────────────────────────────────────

interface Seat {
  id: string
  size: number
}

interface Row {
  size: number
  panes: Seat[]
}

/**
 * The layout as rows of panes, when that is what it is: one pane, one row, or rows
 * one above another. Null for a room someone has built some other way.
 */
export function asRows(layout: Layout): Row[] | null {
  const root = layout.root
  if (!root) return []
  if (isPane(root)) return [{ size: 1, panes: [{ id: root.id, size: 1 }] }]
  const row = (node: Node): Seat[] | null => {
    if (isPane(node)) return [{ id: node.id, size: 1 }]
    if (node.dir !== 'row') return null
    const seats: Seat[] = []
    for (const part of node.parts) {
      if (!isPane(part.node)) return null
      seats.push({ id: part.node.id, size: part.size })
    }
    return seats
  }
  if (root.dir === 'row') {
    const panes = row(root)
    return panes ? [{ size: 1, panes }] : null
  }
  const rows: Row[] = []
  for (const part of root.parts) {
    const panes = row(part.node)
    if (!panes) return null
    rows.push({ size: part.size, panes })
  }
  return rows
}

function fromRows(rows: Row[]): Layout {
  const nodes: Part[] = rows
    .filter(row => row.panes.length > 0)
    .map(row => ({
      size: row.size,
      node: row.panes.length === 1 ? { id: row.panes[0]!.id } : { dir: 'row' as const, parts: row.panes.map(seat => ({ size: seat.size, node: { id: seat.id } })) }
    }))
  if (nodes.length === 0) return EMPTY
  if (nodes.length === 1) return { root: nodes[0]!.node }
  return { root: { dir: 'column', parts: nodes } }
}

/**
 * The shape for this many panes: as close to square as it gets, wider than tall,
 * with any longer rows first. 1 → [1], 2 → [2], 3 → [2, 1], 5 → [3, 2], 7 → [3, 2, 2].
 */
export function shape(count: number): number[] {
  if (count <= 0) return []
  const columns = Math.ceil(Math.sqrt(count))
  const rows = Math.ceil(count / columns)
  const each = Math.floor(count / rows)
  const longer = count % rows
  return Array.from({ length: rows }, (_, i) => each + (i < longer ? 1 : 0))
}

const mean = (values: number[]) => (values.length > 0 ? sum(values) / values.length : 1)
const evenRow = (panes: Seat[]) => panes.every(pane => Math.abs(pane.size - panes[0]!.size) <= EPS * Math.max(pane.size, panes[0]!.size))

/**
 * One more pane in a grid, beside `anchor`, and nothing else moves. It joins its
 * neighbour's row when that row has room (a row may be as wide as the grid is tall,
 * so the room stays close to square); otherwise the shortest row with room, or a new
 * row at the bottom. A row nobody dragged stays even; in one that was dragged, the
 * neighbour gives the newcomer half its share.
 */
function openInGrid(rows: Row[], id: string, anchor: string): Row[] {
  const seated = rows.reduce((n, row) => n + row.panes.length, 0)
  const widest = Math.ceil(Math.sqrt(seated + 1))
  const anchorRow = rows.findIndex(row => row.panes.some(pane => pane.id === anchor))
  let target = anchorRow
  if (rows[target]!.panes.length >= widest) {
    const roomy = rows.map((row, at) => ({ at, length: row.panes.length })).filter(row => row.length < widest)
    target = roomy.length > 0 ? roomy.reduce((best, row) => (row.length < best.length ? row : best)).at : -1
  }
  if (target < 0) return [...rows, { size: mean(rows.map(row => row.size)), panes: [{ id, size: 1 }] }]
  return rows.map((row, at) => {
    if (at !== target) return row
    const near = at === anchorRow ? row.panes.findIndex(pane => pane.id === anchor) : row.panes.length - 1
    if (evenRow(row.panes)) {
      const panes = row.panes.map(pane => ({ ...pane, size: 1 }))
      panes.splice(near + 1, 0, { id, size: 1 })
      return { ...row, panes }
    }
    const half = row.panes[near]!.size / 2
    const panes = row.panes.map((pane, i) => (i === near ? { ...pane, size: half } : pane))
    panes.splice(near + 1, 0, { id, size: half })
    return { ...row, panes }
  })
}

// ── opening and closing ────────────────────────────────────────────────────

/**
 * One more pane, beside `beside` (else beside the last), and nothing else moves.
 *
 * In a grid it goes where a grid would have it (see `openInGrid`). In a room built
 * by hand it carries on the line its neighbour stands in while everyone in that line
 * stays a usable size, and otherwise takes half of its neighbour: beside it where
 * both stay wide enough, below it where both stay tall enough.
 */
export function open(layout: Layout, id: string, beside?: string, room: Room = USUAL_ROOM): Layout {
  if (has(layout, id)) return layout
  if (!layout.root) return { root: { id } }
  const list = ids(layout)
  const anchor = beside !== undefined && list.includes(beside) ? beside : list.at(-1)!
  const rows = asRows(layout)
  if (rows) return fromRows(openInGrid(rows, id, anchor))

  const path = find(layout.root, anchor)!
  const line = nodeAt(layout.root, path.slice(0, -1)) as Split
  const lineBox = boxAt(layout.root, path.slice(0, -1))
  const own = boxAt(layout.root, path)
  const px = (box: Box) => ({ width: box.width * room.width, height: box.height * room.height })
  if (line.parts.every(part => isPane(part.node))) {
    const each = line.dir === 'row' ? px(lineBox).width / (line.parts.length + 1) : px(lineBox).height / (line.parts.length + 1)
    if (each >= (line.dir === 'row' ? WIDE_ENOUGH : TALL_ENOUGH)) return putBeside(layout, id, anchor, line.dir === 'row' ? 'right' : 'bottom')
  }
  if (px(own).width / 2 >= WIDE_ENOUGH) return putBeside(layout, id, anchor, 'right')
  if (px(own).height / 2 >= TALL_ENOUGH) return putBeside(layout, id, anchor, 'bottom')
  return putBeside(layout, id, anchor, 'right')
}

/** The same pane showing a different desk. Sizes stay as they were dragged. */
export function swap(layout: Layout, from: string, to: string): Layout {
  if (from === to || !has(layout, from)) return layout
  // Already open elsewhere: nothing is shown twice, so the pane that had it goes.
  const base = has(layout, to) ? close(layout, to) : layout
  return { root: rename(base.root!, id => (id === from ? to : id)) }
}

function rename(node: Node, name: (id: string) => string): Node {
  if (isPane(node)) return { id: name(node.id) }
  return { dir: node.dir, parts: node.parts.map(part => ({ size: part.size, node: rename(part.node, name) })) }
}

/**
 * Without this pane, and nothing else moves: its share goes to the one before it
 * (after it, for the first), and where nobody had dragged an edge, they stay even.
 * Two left with one of them gone are the one that stays, in the room both had.
 */
export function close(layout: Layout, id: string): Layout {
  const path = find(layout.root, id)
  if (!path || !layout.root) return layout
  if (path.length === 0) return EMPTY
  const at = path.at(-1)!
  return made(
    change(layout.root, path.slice(0, -1), node => {
      const split = node as Split
      const left = split.parts.filter((_, i) => i !== at)
      if (evenParts(split.parts)) return { dir: split.dir, parts: left.map(part => ({ size: 1, node: part.node })) }
      const heir = Math.max(0, at - 1)
      const share = split.parts[at]!.size
      return { dir: split.dir, parts: left.map((part, i) => (i === heir ? { size: part.size + share, node: part.node } : part)) }
    })
  )
}

/** Two open panes change places, each taking the other's place and size. */
export function trade(layout: Layout, a: string, b: string): Layout {
  if (a === b || !layout.root || !has(layout, a) || !has(layout, b)) return layout
  return { root: rename(layout.root, id => (id === a ? b : id === b ? a : id)) }
}

/** Without the panes of desks that are no longer there. */
export function only(layout: Layout, alive: (id: string) => boolean): Layout {
  return leaves(layout.root).reduce((current, id) => (alive(id) ? current : close(current, id)), layout)
}

// ── putting a pane somewhere ───────────────────────────────────────────────

/**
 * Put a pane on one side of another: left or right of it, above or below. The pane
 * may be open already (it moves) or not (it opens there).
 *
 * In a line that runs that way the pane just takes its place in the line, and an
 * even line stays even. Otherwise the two share the room the other one had, half
 * each. Where it already stands there, nothing changes.
 */
export function putBeside(layout: Layout, id: string, target: string, side: Side): Layout {
  if (id === target || !layout.root) return layout
  const there = find(layout.root, target)
  if (!there) return layout
  const axis = axisOf(side)
  const here = find(layout.root, id)

  // In the same line, running the same way: it only changes its place in the line.
  if (here && here.length === there.length && here.length > 0 && here.slice(0, -1).every((step, i) => step === there[i])) {
    const line = nodeAt(layout.root, here.slice(0, -1)) as Split
    if (line.dir === axis) {
      const from = here.at(-1)!
      const to = there.at(-1)!
      if (from === (before(side) ? to - 1 : to + 1)) return layout
      const moved = line.parts.filter((_, i) => i !== from)
      const where = moved.findIndex(part => isPane(part.node) && part.node.id === target)
      moved.splice(before(side) ? where : where + 1, 0, line.parts[from]!)
      return { root: change(layout.root, here.slice(0, -1), () => ({ dir: line.dir, parts: moved })) }
    }
  }

  const without = here ? close(layout, id) : layout
  const root = without.root!
  const path = find(root, target)!
  const pair = (other: Node): Part[] => (before(side) ? [{ size: 1, node: { id } }, { size: 1, node: other }] : [{ size: 1, node: other }, { size: 1, node: { id } }])
  const line = path.length > 0 ? (nodeAt(root, path.slice(0, -1)) as Split) : null
  if (line && line.dir === axis) {
    const at = path.at(-1)!
    const neighbour = line.parts[at]!
    const parts = evenParts(line.parts) ? [...line.parts] : line.parts.map((part, i) => (i === at ? { size: part.size / 2, node: part.node } : part))
    parts.splice(before(side) ? at : at + 1, 0, { size: evenParts(line.parts) ? neighbour.size : neighbour.size / 2, node: { id } })
    return made(change(root, path.slice(0, -1), () => ({ dir: line.dir, parts })))
  }
  return made(change(root, path, node => ({ dir: axis, parts: pair(node) })))
}

/** How many panes stand in a line along one way, at the most: what a new one along that side would stand beside. */
function tracks(node: Node, axis: Dir): number {
  if (isPane(node)) return 1
  const each = node.parts.map(part => tracks(part.node, axis))
  return node.dir === axis ? sum(each) : Math.max(...each)
}

/**
 * Give a pane one whole side of the room: its full height on the left or right, its
 * full width along the top or bottom. It takes as much as one more in the line would,
 * and never less than a quarter; everyone else keeps their share of what is left.
 */
export function putAtEdge(layout: Layout, id: string, side: Side): Layout {
  const root = layout.root
  if (!root) return { root: { id } }
  const axis = axisOf(side)
  const here = find(root, id)
  if (here && here.length === 0) return layout
  if (here && here.length === 1 && !isPane(root) && root.dir === axis && here[0] === (before(side) ? 0 : root.parts.length - 1)) return layout

  const rest = (here ? close(layout, id) : layout).root!
  const mine: Pane = { id }
  if (!isPane(rest) && rest.dir === axis) {
    const part: Part = { size: total(rest) / Math.min(3, tracks(rest, axis)), node: mine }
    return made({ dir: axis, parts: before(side) ? [part, ...rest.parts] : [...rest.parts, part] })
  }
  const others: Part = { size: Math.min(3, tracks(rest, axis)), node: rest }
  return made({ dir: axis, parts: before(side) ? [{ size: 1, node: mine }, others] : [others, { size: 1, node: mine }] })
}

/**
 * The pane next to this one on one side: the nearest that stands there and overlaps
 * it; of two as near, the one that overlaps more; then the higher, or the one further
 * left. None at the room's own edge.
 */
export function neighbour(layout: Layout, id: string, side: Side): string | null {
  const panes = plan(layout).panes
  const me = panes.find(pane => pane.id === id)
  if (!me) return null
  const across = axisOf(side) === 'row'
  let best: { id: string; gap: number; overlap: number; order: number } | null = null
  for (const other of panes) {
    if (other.id === id) continue
    const gap = side === 'left' ? me.left - (other.left + other.width) : side === 'right' ? other.left - (me.left + me.width) : side === 'top' ? me.top - (other.top + other.height) : other.top - (me.top + me.height)
    if (gap < -EPS) continue
    const overlap = across ? Math.min(me.top + me.height, other.top + other.height) - Math.max(me.top, other.top) : Math.min(me.left + me.width, other.left + other.width) - Math.max(me.left, other.left)
    if (overlap <= EPS) continue
    const order = across ? other.top : other.left
    const nearer = !best || gap < best.gap - EPS || (Math.abs(gap - best.gap) <= EPS && (overlap > best.overlap + EPS || (Math.abs(overlap - best.overlap) <= EPS && order < best.order)))
    if (nearer) best = { id: other.id, gap, overlap, order }
  }
  return best?.id ?? null
}

/** Move a pane one step: it changes places with its neighbour on that side, and with none there, takes that whole side of the room. */
export function move(layout: Layout, id: string, side: Side): Layout {
  if (!has(layout, id)) return layout
  const next = neighbour(layout, id, side)
  return next ? trade(layout, id, next) : putAtEdge(layout, id, side)
}

// ── dragging a pane ────────────────────────────────────────────────────────

/**
 * Where a pane held at this point in the room would go if it were let go: null where
 * that is nowhere new. The point is in pixels from the room's top left corner.
 *
 * Over itself: nowhere. At the room's own edge, or past it: that whole side (the top
 * only from past it, since the top of the room is the panes' headers). On another
 * pane's header: the two change places. Near one of its sides: on that side of it.
 * Anywhere else on it: the two change places.
 */
export function aim(layout: Layout, moving: string, x: number, y: number, room: Room): Aim | null {
  if (!has(layout, moving) || room.width <= 0 || room.height <= 0) return null
  const found = aimAt(layout, moving, x, y, room)
  return found && !samePicture(layout, land(layout, moving, found)) ? found : null
}

function aimAt(layout: Layout, moving: string, x: number, y: number, room: Room): Aim | null {
  const past: [Side, number][] = [
    ['left', -x],
    ['right', x - room.width],
    ['top', -y],
    ['bottom', y - room.height]
  ]
  const beyond = past.filter(([, far]) => far > 0)
  // Past two sides at once, by a corner: the one it is further past. Left and right first, among equals.
  if (beyond.length > 0) return { kind: 'edge', side: beyond.reduce((most, next) => (next[1] > most[1] ? next : most))[0] }

  const boxes = plan(layout).panes.map(pane => ({ id: pane.id, left: pane.left * room.width, top: pane.top * room.height, width: pane.width * room.width, height: pane.height * room.height }))
  const over = boxes.find(box => x >= box.left && x < box.left + box.width && y >= box.top && y < box.top + box.height)
  if (!over || over.id === moving) return null

  const near: [Side, number][] = [
    ['left', x],
    ['right', room.width - x],
    ['bottom', room.height - y]
  ]
  const atEdge = near.filter(([, far]) => far <= ROOM_EDGE)
  if (atEdge.length > 0) return { kind: 'edge', side: atEdge.reduce((least, next) => (next[1] < least[1] ? next : least))[0] }

  const dx = x - over.left
  const dy = y - over.top
  if (dy < HEADER) return { kind: 'trade', target: over.id }
  // How far in from each side, against how far that side's band reaches. Under one: in the band.
  const bands: [Side, number][] = []
  if (over.width >= AIM_WIDTH) bands.push(['left', dx / (BAND * over.width)], ['right', (over.width - dx) / (BAND * over.width)])
  if (over.height >= AIM_HEIGHT) bands.push(['top', (dy - HEADER) / Math.max(BAND * over.height - HEADER, 40)], ['bottom', (over.height - dy) / (BAND * over.height)])
  const inside = bands.filter(([, far]) => far < 1)
  if (inside.length === 0) return { kind: 'trade', target: over.id }
  return { kind: 'beside', target: over.id, side: inside.reduce((least, next) => (next[1] < least[1] ? next : least))[0] }
}

/** The layout once a dragged pane is let go where it was aimed. */
export function land(layout: Layout, moving: string, to: Aim): Layout {
  if (to.kind === 'edge') return putAtEdge(layout, moving, to.side)
  if (to.kind === 'beside') return putBeside(layout, moving, to.target, to.side)
  return trade(layout, moving, to.target)
}

// ── arranging everything at once ───────────────────────────────────────────

/**
 * These panes, in this order, in one shape: a grid as close to square as it gets; all
 * side by side; all one above another; or one of them large with the others in a line
 * beside it, or below it. `main` is the large one (else the first).
 */
export function arrange(list: string[], to: Shape = 'grid', main?: string): Layout {
  const unique = list.filter((id, i) => id && list.indexOf(id) === i)
  if (unique.length === 0) return EMPTY
  const line = (dir: Dir, members: string[]): Node => (members.length === 1 ? { id: members[0]! } : { dir, parts: members.map(id => ({ size: 1, node: { id } })) })
  if (to === 'side') return { root: line('row', unique) }
  if (to === 'stacked') return { root: line('column', unique) }
  if (to === 'main-beside' || to === 'main-below') {
    const first = main !== undefined && unique.includes(main) ? main : unique[0]!
    const others = unique.filter(id => id !== first)
    if (others.length === 0) return { root: { id: first } }
    const outer: Dir = to === 'main-beside' ? 'row' : 'column'
    return made({ dir: outer, parts: [{ size: 1.5, node: { id: first } }, { size: 1, node: line(outer === 'row' ? 'column' : 'row', others) }] })
  }
  let next = 0
  return fromRows(shape(unique.length).map(length => ({ size: 1, panes: unique.slice(next, (next += length)).map(id => ({ id, size: 1 })) })))
}

// ── the edges ──────────────────────────────────────────────────────────────

/** The least room a node needs along one way, as a share of the room: a pane its least, a line of them their sum. */
function need(node: Node, dir: Dir, least: Least): number {
  if (isPane(node)) return least[dir]
  const each = node.parts.map(part => need(part.node, dir, least))
  return node.dir === dir ? sum(each) : Math.max(...each)
}

/**
 * Move one edge of the split at the end of `path`, the one after its part `index`, to
 * `at`: how far along that split it should now stand, from 0 to 1. Only the two parts
 * on either side change, and no pane in either goes under the least.
 */
export function moveEdge(layout: Layout, path: number[], index: number, at: number, least: Least = LEAST): Layout {
  const split = nodeAt(layout.root, path)
  if (!layout.root || !split || isPane(split) || index < 0 || index >= split.parts.length - 1 || !Number.isFinite(at)) return layout
  const sizes = split.parts.map(part => part.size)
  const all = sum(sizes)
  const box = boxAt(layout.root, path)
  const extent = split.dir === 'row' ? box.width : box.height
  if (all <= 0 || extent <= 0) return layout
  // What one unit of this split's shares is worth, as a share of the room.
  const unit = extent / all
  const pair = sizes[index]! + sizes[index + 1]!
  let low = need(split.parts[index]!.node, split.dir, least) / unit
  let high = pair - need(split.parts[index + 1]!.node, split.dir, least) / unit
  // A room too small for both to keep their least: neither is dragged away to nothing.
  if (low > high) {
    low = pair * 0.15
    high = pair - low
  }
  const first = Math.min(high, Math.max(low, at * all - sum(sizes.slice(0, index))))
  if (Math.abs(first - sizes[index]!) < 1e-9) return layout
  const parts = split.parts.map((part, i) => (i === index ? { size: first, node: part.node } : i === index + 1 ? { size: pair - first, node: part.node } : part))
  return { root: change(layout.root, path, () => ({ dir: split.dir, parts })) }
}

/** Every pane and row an even share again. */
export function even(layout: Layout): Layout {
  const level = (node: Node): Node => (isPane(node) ? node : { dir: node.dir, parts: node.parts.map(part => ({ size: 1, node: level(part.node) })) })
  return layout.root ? { root: level(layout.root) } : layout
}

// ── kept between runs ──────────────────────────────────────────────────────

/** No room is built deeper than this by hand; anything deeper was not written by the office. */
const DEEPEST = 12

/**
 * A layout read back from storage, if it is one. Anything else is no layout. Rows of
 * panes, as the office kept them before a pane could be put anywhere, are still read.
 */
export function parse(saved: unknown): Layout {
  if (!saved || typeof saved !== 'object') return EMPTY
  const seen = new Set<string>()
  const share = (value: unknown) => (typeof value === 'number' && Number.isFinite(value) && value > 0 ? value : 1)
  const name = (value: unknown): string | null => {
    if (typeof value !== 'string' || !value || seen.has(value)) return null
    seen.add(value)
    return value
  }

  if ('root' in saved) {
    const read = (value: unknown, depth: number): Node | null => {
      if (!value || typeof value !== 'object' || depth > DEEPEST) return null
      if ('id' in value) {
        const id = name(value.id)
        return id === null ? null : { id }
      }
      const { dir, parts } = value as { dir?: unknown; parts?: unknown }
      if ((dir !== 'row' && dir !== 'column') || !Array.isArray(parts) || parts.length === 0) return null
      const read_: Part[] = []
      for (const part of parts) {
        if (!part || typeof part !== 'object') return null
        const node = read((part as { node?: unknown }).node, depth + 1)
        if (!node) return null
        read_.push({ size: share((part as { size?: unknown }).size), node })
      }
      return { dir, parts: read_ }
    }
    if (saved.root === null) return EMPTY
    const root = read(saved.root, 0)
    return root ? made(root) : EMPTY
  }

  if (!('rows' in saved) || !Array.isArray(saved.rows)) return EMPTY
  const rows: Row[] = []
  for (const row of saved.rows as { size?: unknown; panes?: unknown }[]) {
    if (!row || !Array.isArray(row.panes)) return EMPTY
    const panes: Seat[] = []
    for (const pane of row.panes as { id?: unknown; size?: unknown }[]) {
      const id = pane ? name(pane.id) : null
      if (id === null) return EMPTY
      panes.push({ id, size: share(pane.size) })
    }
    if (panes.length > 0) rows.push({ size: share(row.size), panes })
  }
  return fromRows(rows)
}

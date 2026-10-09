// Pictures of the office over Claude Code and Codex, before any of it is built.
//
//   npx vite --port 4174, then /spikes/cli-office/mockups/index.html?screen=floor
//   screens: floor, terminal, side, split, new, elsewhere, desktop; looks: plan, wall, night-floor, night-split, night-cast
//
// The people, furniture and colours are the app's own; everything else is pretend.
import { mount } from 'svelte'
import '../../../src/styles/base.css'
import Desktop from './Desktop.svelte'
import Mock, { type Screen } from './Mock.svelte'
import Night from './night/Night.svelte'
import Vibes from './Vibes.svelte'

const SCREENS: Screen[] = ['floor', 'terminal', 'side', 'split', 'new', 'elsewhere']
const asked = new URLSearchParams(location.search).get('screen') ?? ''
const target = document.getElementById('app')!

if (asked === 'desktop') {
  mount(Desktop, { target })
} else if (asked === 'night-floor' || asked === 'night-split' || asked === 'night-cast') {
  mount(Night, { target, props: { screen: asked.slice(6) as 'floor' | 'split' | 'cast' } })
} else if (asked === 'plan' || asked === 'wall') {
  mount(Vibes, { target, props: { screen: asked } })
} else {
  mount(Mock, { target, props: { screen: SCREENS.find(s => s === asked) ?? 'floor' } })
}

// Shared helpers for driving the real desktop app in tests (Windows).
//
// The app is started with WebView2's remote debugging port open, so the same
// Playwright API used for web pages can read and click the real window.
import { execFileSync, spawn } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright-core'

const here = path.dirname(fileURLToPath(import.meta.url))
export const root = path.resolve(here, '..', '..')
export const sleep = ms => new Promise(resolve => setTimeout(resolve, ms))

/** The app to test: `MOSHPIT_APP`, else the debug build in cargo's target folder. */
export function appPath() {
  if (process.env.MOSHPIT_APP) return process.env.MOSHPIT_APP
  const target = process.env.CARGO_TARGET_DIR || path.join(root, 'src-tauri', 'target')
  return path.join(target, 'debug', 'agent-moshpit.exe')
}

/**
 * Start the app. `env` is added to the environment.
 *
 * It runs as its own named instance, so a test never hands over to an office the
 * person at this computer has open, and never shares its window storage.
 */
export function launch({ port = 9223, env = {}, args = [] } = {}) {
  const child = spawn(appPath(), args, {
    env: {
      ...process.env,
      MOSHPIT_INSTANCE: process.env.MOSHPIT_INSTANCE || 'e2e',
      // Tests never ask npm for versions: no network, and no processes the test did not start.
      MOSHPIT_NO_UPDATE_CHECK: '1',
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
      ...env
    },
    stdio: 'ignore',
    windowsHide: false
  })
  return { child, port }
}

/** Connect to the app's window. Retries while the webview is still starting. */
export async function attach(port = 9223, timeoutMs = 30000) {
  const deadline = Date.now() + timeoutMs
  let last
  while (Date.now() < deadline) {
    try {
      const browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`)
      const pages = browser.contexts().flatMap(context => context.pages())
      const page = pages.find(p => !p.url().startsWith('devtools://'))
      if (page) return { browser, page }
      await browser.close()
    } catch (error) {
      last = error
    }
    await sleep(300)
  }
  throw new Error(`could not attach to the app window: ${last?.message ?? 'no page found'}`)
}

function tree(pid) {
  const raw = execFileSync(
    'powershell',
    ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', path.join(here, 'tree.ps1'), '-RootPid', String(pid)],
    { encoding: 'utf8' }
  )
  return JSON.parse(raw.trim() || '[]')
}

/** Every process the app started, itself included: pid, name, memory, CPU seconds. */
export function processes(pid) {
  return tree(pid)
}

/** Close the app's window the way a person does, by its close button. */
export function closeWindow(pid) {
  return execFileSync(
    'powershell',
    ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', path.join(here, 'close-window.ps1'), '-ProcessId', String(pid)],
    { encoding: 'utf8' }
  ).trim()
}

/**
 * Whether the office window exists. The app always owns a few invisible helper
 * windows (tray, single-instance), so this looks for the titled one.
 */
export function hasWindow(pid) {
  const out = execFileSync(
    'powershell',
    ['-NoProfile', '-Command', `(Get-Process -Id ${pid}).MainWindowTitle`],
    { encoding: 'utf8' }
  ).trim()
  // "Agent Moshpit", or "Agent Moshpit (2 need you)".
  return out.startsWith('Agent Moshpit')
}

/** The title of the app's window, as the taskbar shows it. */
export function windowTitle(pid) {
  return execFileSync('powershell', ['-NoProfile', '-Command', `(Get-Process -Id ${pid}).MainWindowTitle`], { encoding: 'utf8' }).trim()
}

/**
 * A named office tells Windows that it opens `agent-moshpit-<name>://` (what a click on
 * one of its notifications opens). These read that entry, and take it away again when a
 * test is over, so nothing of the test is left in the registry.
 */
const addressKey = instance => `HKCU\\Software\\Classes\\agent-moshpit-${instance}`

export function addressCommand(instance = process.env.MOSHPIT_INSTANCE || 'e2e') {
  try {
    return execFileSync('reg', ['query', `${addressKey(instance)}\\shell\\open\\command`, '/ve'], { encoding: 'utf8' })
  } catch {
    return ''
  }
}

export function forgetAddress(instance = process.env.MOSHPIT_INSTANCE || 'e2e') {
  try {
    execFileSync('reg', ['delete', addressKey(instance), '/f'], { stdio: 'ignore' })
  } catch {
    // never written, or gone already
  }
  forgetNotices(instance)
}

/** The app id Windows shows a build's notifications under, when it was not installed (see toast.rs). */
const UNINSTALLED = String.raw`{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\WindowsPowerShell\v1.0\powershell.exe`

/**
 * Take a named office's notifications out of the notification centre: each is filed
 * under its address as its group, so only the test's own go, and none is left to be
 * clicked after its address is gone.
 */
export function forgetNotices(instance = process.env.MOSHPIT_INSTANCE || 'e2e') {
  const script = [
    '[void][Windows.UI.Notifications.ToastNotificationManager,Windows.UI.Notifications,ContentType=WindowsRuntime]',
    `[Windows.UI.Notifications.ToastNotificationManager]::History.RemoveGroup('agent-moshpit-${instance}', '${UNINSTALLED}')`
  ].join('; ')
  try {
    execFileSync('powershell', ['-NoProfile', '-Command', script], { stdio: 'ignore' })
  } catch {
    // none there
  }
}

/** What a named office's notifications in the notification centre say, each as Windows was given it. */
export function notices(instance = process.env.MOSHPIT_INSTANCE || 'e2e') {
  const script = [
    '[void][Windows.UI.Notifications.ToastNotificationManager,Windows.UI.Notifications,ContentType=WindowsRuntime]',
    `@([Windows.UI.Notifications.ToastNotificationManager]::History.GetHistory('${UNINSTALLED}') | Where-Object { $_.Group -eq 'agent-moshpit-${instance}' } | ForEach-Object { $_.Content.GetXml() }) -join [char]10`
  ].join('; ')
  try {
    return execFileSync('powershell', ['-NoProfile', '-Command', script], { encoding: 'utf8' }).split(/\r?\n/).map(line => line.trim()).filter(Boolean)
  } catch {
    return []
  }
}

/** Open an address the way Windows does when a notification is clicked. */
export function openAddress(address) {
  if (!/^[a-z0-9-]+:\/\/[a-z0-9/]*$/i.test(address)) throw new Error(`not an address this test opens: ${address}`)
  execFileSync('powershell', ['-NoProfile', '-Command', `Start-Process '${address}'`], { stdio: 'ignore' })
}

export function isRunning(pid) {
  try {
    const out = execFileSync('tasklist', ['/FI', `PID eq ${pid}`, '/NH'], { encoding: 'utf8' })
    return out.includes(String(pid))
  } catch {
    return false
  }
}

export function killTree(pid) {
  try {
    execFileSync('taskkill', ['/PID', String(pid), '/T', '/F'], { stdio: 'ignore' })
  } catch {
    // already gone
  }
}

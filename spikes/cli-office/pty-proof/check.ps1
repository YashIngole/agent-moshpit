# Runs pty-proof and looks at what Windows sees while it holds Claude Code and Codex.
#
#   powershell -File check.ps1 -Proof <pty-proof.exe> -Out <dir> -Cwd <dir> -Commands "claude","cmd.exe /d /c codex"
#
# Prints the process tree under the proof, and every visible top-level window that any
# process in that tree owns. A window in that list would be a second taskbar button.
param(
  [Parameter(Mandatory)] [string] $Proof,
  [Parameter(Mandatory)] [string] $Out,
  [Parameter(Mandatory)] [string] $Cwd,
  [Parameter(Mandatory)] [string[]] $Commands,
  [int] $Hold = 14
)

Add-Type @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class Windows {
  delegate bool Each(IntPtr hwnd, IntPtr lParam);
  [DllImport("user32.dll")] static extern bool EnumWindows(Each each, IntPtr lParam);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr hwnd);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowText(IntPtr hwnd, StringBuilder text, int max);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassName(IntPtr hwnd, StringBuilder text, int max);
  public static List<string> Visible() {
    var found = new List<string>();
    EnumWindows((hwnd, l) => {
      if (!IsWindowVisible(hwnd)) return true;
      uint pid; GetWindowThreadProcessId(hwnd, out pid);
      var title = new StringBuilder(256); GetWindowText(hwnd, title, 256);
      var kind = new StringBuilder(256); GetClassName(hwnd, kind, 256);
      found.Add(pid + "\t" + hwnd.ToInt64() + "\t" + kind + "\t" + title);
      return true;
    }, IntPtr.Zero);
    return found;
  }
}
'@

New-Item -ItemType Directory -Force $Out | Out-Null
Remove-Item (Join-Path $Out '*') -Force -ErrorAction SilentlyContinue
$before = [Windows]::Visible() | ForEach-Object { ($_ -split "`t")[1] }

$quoted = @("`"$Out`"", "`"$Cwd`"", $Hold) + ($Commands | ForEach-Object { "`"$_`"" })
$app = Start-Process -FilePath $Proof -ArgumentList $quoted -PassThru
$pids = Join-Path $Out 'pids.txt'
for ($i = 0; $i -lt 40 -and -not (Test-Path $pids); $i++) { Start-Sleep -Milliseconds 250 }
Start-Sleep -Seconds ([Math]::Max(4, $Hold - 6))

"--- what the proof started"
Get-Content $pids

# Everything under the proof, however deep.
$all = Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, Name
$tree = @($app.Id)
do {
  $grew = $false
  foreach ($p in $all) {
    if ($tree -contains $p.ParentProcessId -and $tree -notcontains $p.ProcessId) { $tree += $p.ProcessId; $grew = $true }
  }
} while ($grew)

"--- processes under the proof (pid, parent, name)"
$all | Where-Object { $tree -contains $_.ProcessId } | Sort-Object ParentProcessId, ProcessId |
  ForEach-Object { "{0,6} {1,6}  {2}" -f $_.ProcessId, $_.ParentProcessId, $_.Name }

$now = [Windows]::Visible()
"--- visible windows owned by any of them"
$owned = $now | Where-Object { $tree -contains [int]($_ -split "`t")[0] }
if ($owned) { $owned } else { '(none)' }
"--- visible windows that were not there before, from anyone"
$new = $now | Where-Object { $before -notcontains ($_ -split "`t")[1] }
if ($new) { $new } else { '(none)' }

for ($i = 0; $i -lt 80 -and -not (Test-Path (Join-Path $Out 'done.txt')); $i++) { Start-Sleep -Milliseconds 250 }
"--- left running afterwards"
$left = Get-CimInstance Win32_Process | Where-Object { $tree -contains $_.ProcessId } | ForEach-Object { "{0,6}  {1}" -f $_.ProcessId, $_.Name }
if ($left) { $left } else { '(none)' }

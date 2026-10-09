# Agent Moshpit: install it, or bring it up to date, on Windows.
#
#   irm https://agentmoshpit.com/install.ps1 | iex
#
# It fetches the newest installer from github.com/YashIngole/agent-moshpit, runs it
# without questions (for you only: no administrator is needed), and starts the app.
# Nothing else on the computer is changed.
$ErrorActionPreference = 'Stop'

$file = 'agent-moshpit_windows_x64-setup.exe'
$from = "https://github.com/YashIngole/agent-moshpit/releases/latest/download/$file"
$to = Join-Path ([IO.Path]::GetTempPath()) $file

if (-not [Environment]::Is64BitOperatingSystem) {
    throw 'Agent Moshpit was not installed: there is a build for 64-bit Windows only.'
}
# A copy that is running has agents in it: it is for you to end them, not for this.
if (Get-Process -Name 'agent-moshpit' -ErrorAction SilentlyContinue) {
    throw "Agent Moshpit was not installed: it is running. Quit it first (that ends its agents' programs), then run this again."
}

Write-Host "Getting $file"
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
Invoke-WebRequest -Uri $from -OutFile $to -UseBasicParsing

if ($env:MOSHPIT_INSTALL_ONLY_FETCH) {
    Write-Host "Fetched $to ($((Get-Item $to).Length) bytes). It was not installed: this was asked only to fetch it."
    return
}

Write-Host 'Installing it.'
$installer = Start-Process -FilePath $to -ArgumentList '/S' -Wait -PassThru
Remove-Item $to -ErrorAction SilentlyContinue
if ($installer.ExitCode -ne 0) {
    throw "Agent Moshpit was not installed: its installer stopped with code $($installer.ExitCode)."
}

$entry = Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Agent Moshpit' -ErrorAction SilentlyContinue
$app = if ($entry -and $entry.InstallLocation) { Join-Path $entry.InstallLocation.Trim('"') 'agent-moshpit.exe' }
if ($app -and (Test-Path $app)) {
    Write-Host 'Agent Moshpit is installed. Starting it.'
    Start-Process $app
} else {
    Write-Host 'Agent Moshpit is installed. It is in the Start menu.'
}

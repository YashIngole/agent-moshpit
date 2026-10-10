# Agent Moshpit: install it, or bring it up to date, on Windows.
#
#   irm https://agentmoshpit.com/install.ps1 | iex
#
# It fetches the newest installer from github.com/YashIngole/agent-moshpit, runs it
# without questions (for you only: no administrator is needed), and starts the app.
# Nothing else on the computer is changed.
#
# Nothing is run unchecked. The release's list of installer hashes must carry a signature
# from the key below (tools/install-signers in the repository), which ssh-keygen from
# Windows' OpenSSH Client checks, and the installer must have the hash that list gives it.
#
# Run as a script block of its own: iex runs a script in the caller's session, and its
# preferences (stop on any error, no progress bar) must not stay behind in that window.
& {
    $ErrorActionPreference = 'Stop'

    $repo = 'https://github.com/YashIngole/agent-moshpit/releases'
    $signer = 'installers@agentmoshpit.com'
    $signers = 'installers@agentmoshpit.com namespaces="agentmoshpit-install" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIO+PcdRuFfWQEXF3m4TryI+MxJ0ZyAlrPf88PavyArDQ'
    $file = 'agent-moshpit_windows_x64-setup.exe'
    function Stop-Install([string]$why) { throw "Agent Moshpit was not installed: $why" }

    if (-not [Environment]::Is64BitOperatingSystem) {
        Stop-Install 'there is a build for 64-bit Windows only.'
    }
    $onlyFetch = [bool]$env:MOSHPIT_INSTALL_ONLY_FETCH
    # A copy that is running has agents in it: it is for you to end them, not for this.
    if (-not $onlyFetch -and (Get-Process -Name 'agent-moshpit' -ErrorAction SilentlyContinue)) {
        Stop-Install "it is running. Quit it first (that ends its agents' programs), then run this again."
    }
    $keygen = Get-Command ssh-keygen -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $keygen) {
        Stop-Install "checking the download's signature needs ssh-keygen, from the OpenSSH Client that Windows includes. Add it under Settings, System, Optional features, then run this again."
    }

    $work = Join-Path ([IO.Path]::GetTempPath()) ('agent-moshpit-' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $work | Out-Null
    try {
        $ProgressPreference = 'SilentlyContinue'
        [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
        function Get-File([string]$from, [string]$to, [string]$what) {
            try { Invoke-WebRequest -Uri $from -OutFile $to -UseBasicParsing }
            catch { Stop-Install "$what could not be downloaded ($($_.Exception.Message))." }
        }

        # First the signed list of the newest release's installers, then the one installer, checked.
        $sums = Join-Path $work 'SHA256SUMS'
        Get-File "$repo/latest/download/SHA256SUMS" $sums "the release's signed list of installers"
        Get-File "$repo/latest/download/SHA256SUMS.sig" "$sums.sig" "the signature of the release's list of installers"
        $allowed = Join-Path $work 'signers'
        [IO.File]::WriteAllText($allowed, "$signers`n")
        # ssh-keygen reads the signed bytes on its input; they are written exactly as downloaded.
        $start = New-Object System.Diagnostics.ProcessStartInfo
        $start.FileName = $keygen.Source
        $start.Arguments = '-Y verify -f "{0}" -I {1} -n agentmoshpit-install -s "{2}"' -f $allowed, $signer, "$sums.sig"
        $start.UseShellExecute = $false
        $start.RedirectStandardInput = $true
        $start.RedirectStandardOutput = $true
        $start.RedirectStandardError = $true
        $check = [Diagnostics.Process]::Start($start)
        $signed = [IO.File]::ReadAllBytes($sums)
        $check.StandardInput.BaseStream.Write($signed, 0, $signed.Length)
        $check.StandardInput.Close()
        $null = $check.StandardOutput.ReadToEnd()
        $said = $check.StandardError.ReadToEnd()
        $check.WaitForExit()
        if ($check.ExitCode -ne 0) {
            if ($said -match 'option') {
                Stop-Install "this Windows' OpenSSH Client is too old to check signatures (8.1 or newer is needed). Update Windows, then run this again."
            }
            Stop-Install "the release's list of installers is not signed with Agent Moshpit's key."
        }
        $lines = [IO.File]::ReadAllLines($sums)
        if ($lines.Count -eq 0 -or -not ($lines[0] -match '^# Agent Moshpit (v\d+\.\d+\.\d+(-[0-9A-Za-z.]+)?)$')) {
            Stop-Install 'the signed list of installers names no release.'
        }
        $tag = $Matches[1]
        $want = $null
        foreach ($line in $lines) {
            if ($line -match '^([0-9a-f]{64})  ([\w.-]+)$' -and $Matches[2] -eq $file) { $want = $Matches[1] }
        }
        if (-not $want) { Stop-Install "$file is not in the signed list of installers for $tag." }

        Write-Host "Getting $file ($tag)"
        $to = Join-Path $work $file
        Get-File "$repo/download/$tag/$file" $to $file
        $stream = [IO.File]::OpenRead($to)
        try { $have = -join ([Security.Cryptography.SHA256]::Create().ComputeHash($stream) | ForEach-Object { $_.ToString('x2') }) }
        finally { $stream.Dispose() }
        if ($have -ne $want) {
            Stop-Install "$file is not the file the signed list names. Nothing was run."
        }
        if ($onlyFetch) {
            Write-Host "Fetched and checked $file ($tag, $((New-Object IO.FileInfo $to).Length) bytes). It was not installed: this was asked only to fetch it."
            return
        }

        Write-Host 'Installing it.'
        $installer = Start-Process -FilePath $to -ArgumentList '/S' -Wait -PassThru
        if ($installer.ExitCode -ne 0) {
            Stop-Install "its installer stopped with code $($installer.ExitCode)."
        }
    } finally {
        Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
    }

    $entry = Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Agent Moshpit' -ErrorAction SilentlyContinue
    $app = if ($entry -and $entry.InstallLocation) { Join-Path $entry.InstallLocation.Trim('"') 'agent-moshpit.exe' }
    if ($app -and (Test-Path $app)) {
        Write-Host 'Agent Moshpit is installed. Starting it.'
        Start-Process $app
    } else {
        Write-Host 'Agent Moshpit is installed. It is in the Start menu.'
    }
}

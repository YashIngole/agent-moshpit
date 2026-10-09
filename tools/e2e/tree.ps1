param([int]$RootPid)
# Prints the process tree rooted at $RootPid as JSON: pid, name, working set, private bytes, CPU seconds.
$all = Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, Name
function Get-Kids([int]$id) {
  foreach ($c in ($all | Where-Object { $_.ParentProcessId -eq $id })) {
    $c
    Get-Kids $c.ProcessId
  }
}
$tree = @($all | Where-Object { $_.ProcessId -eq $RootPid }) + @(Get-Kids $RootPid)
$rows = foreach ($t in $tree) {
  $p = Get-Process -Id $t.ProcessId -ErrorAction SilentlyContinue
  if ($p) {
    [pscustomobject]@{
      pid  = $p.Id
      name = $p.ProcessName
      ws   = $p.WorkingSet64
      priv = $p.PrivateMemorySize64
      cpu  = $p.TotalProcessorTime.TotalSeconds
    }
  }
}
ConvertTo-Json -InputObject @($rows) -Compress

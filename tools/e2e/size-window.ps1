param([int]$ProcessId, [int]$Width, [int]$Height)
# Resize only the test process's window; keep its position, focus and stacking order.
Add-Type -Namespace Win -Name Api -MemberDefinition '[DllImport("user32.dll", SetLastError=true)] public static extern bool SetWindowPos(System.IntPtr hWnd, System.IntPtr after, int x, int y, int cx, int cy, uint flags);'
$process = Get-Process -Id $ProcessId -ErrorAction Stop
if ($process.MainWindowHandle -eq [System.IntPtr]::Zero) { throw 'The test process has no window.' }
if ($Width -lt 340 -or $Height -lt 420) { throw 'The test window size is too small.' }
if (-not [Win.Api]::SetWindowPos($process.MainWindowHandle, [System.IntPtr]::Zero, 0, 0, $Width, $Height, 0x0016)) {
  throw 'The test window could not be resized.'
}

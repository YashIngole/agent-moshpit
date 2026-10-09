param([int]$ProcessId)
# Closes a process's main window the way clicking its X does (WM_CLOSE).
Add-Type -Namespace Win -Name Api -MemberDefinition '[DllImport("user32.dll")] public static extern bool PostMessage(System.IntPtr hWnd, uint Msg, System.IntPtr wParam, System.IntPtr lParam);'
$process = Get-Process -Id $ProcessId -ErrorAction Stop
if ($process.MainWindowHandle -eq [System.IntPtr]::Zero) {
  Write-Output 'no window'
  exit 0
}
[void][Win.Api]::PostMessage($process.MainWindowHandle, 0x0010, [System.IntPtr]::Zero, [System.IntPtr]::Zero)
Write-Output 'closed'

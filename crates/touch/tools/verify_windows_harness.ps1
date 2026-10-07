# Exercise only the window created by this test process. No global mouse/keyboard
# injection, no other app handles, no game assets, no system changes.
$ErrorActionPreference = 'Stop'
$touchCrateRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$touchHarnessExe = Join-Path $touchCrateRoot 'target\debug\examples\harness.exe'
$touchRecordPath = Join-Path $touchCrateRoot 'target\windows-mouse.jsonl'
if (-not (Test-Path -LiteralPath $touchHarnessExe)) { throw 'Build the harness example first.' }
Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class TouchHarnessProbe {
    [DllImport("user32.dll", SetLastError=true)]
    public static extern bool PostMessageW(IntPtr hwnd, uint msg, UIntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")]
    public static extern uint GetDpiForWindow(IntPtr hwnd);
    [DllImport("user32.dll")]
    public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
    private delegate bool EnumCallback(IntPtr hwnd, IntPtr data);
    [DllImport("user32.dll")]
    private static extern bool EnumWindows(EnumCallback callback, IntPtr data);
    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)]
    private static extern int GetWindowTextW(IntPtr hwnd, StringBuilder text, int count);
    public static IntPtr FindHarnessWindow(uint expectedPid) {
        IntPtr result = IntPtr.Zero;
        EnumWindows((hwnd, data) => {
            uint pid;
            GetWindowThreadProcessId(hwnd, out pid);
            if (pid != expectedPid) return true;
            var title = new StringBuilder(512);
            GetWindowTextW(hwnd, title, title.Capacity);
            if (!title.ToString().StartsWith("Touch harness")) return true;
            result = hwnd;
            return false;
        }, IntPtr.Zero);
        return result;
    }
}
'@
# Keep this probe thread in the same per-monitor DPI domain as winit. Otherwise
# Windows virtualizes posted mouse coordinates between sender and receiver.
$touchPreviousDpi = [TouchHarnessProbe]::SetThreadDpiAwarenessContext([IntPtr](-4))
$touchProbeProcess = Start-Process -FilePath $touchHarnessExe -ArgumentList @('--record', ('"' + $touchRecordPath + '"')) -PassThru -WindowStyle Hidden
try {
    for ($touchWait = 0; $touchWait -lt 50; $touchWait++) {
        $touchProbeProcess.Refresh()
        if ($touchProbeProcess.HasExited) { throw 'Harness exited before window creation.' }
        $touchProbeWindow = [TouchHarnessProbe]::FindHarnessWindow([uint32]$touchProbeProcess.Id)
        if ($touchProbeWindow -ne [IntPtr]::Zero) { break }
        Start-Sleep -Milliseconds 100
    }
    if ($touchProbeWindow -eq [IntPtr]::Zero) { throw 'Harness window was not created.' }
    $touchProbeScale = [TouchHarnessProbe]::GetDpiForWindow($touchProbeWindow) / 96.0
    function Send-TouchProbeMouse([double]$pointX, [double]$pointY, [bool]$isDown) {
        $touchPhysicalX = [int][Math]::Round($pointX * $touchProbeScale)
        $touchPhysicalY = [int][Math]::Round($pointY * $touchProbeScale)
        $touchPointParam = [IntPtr](($touchPhysicalY -shl 16) -bor ($touchPhysicalX -band 65535))
        if (-not [TouchHarnessProbe]::PostMessageW($touchProbeWindow, 0x0200, [UIntPtr]::Zero, $touchPointParam)) { throw 'WM_MOUSEMOVE failed.' }
        $touchMouseMsg = if ($isDown) { 0x0201 } else { 0x0202 }
        $touchMouseFlags = if ($isDown) { [UIntPtr]::new([uint32]1) } else { [UIntPtr]::Zero }
        if (-not [TouchHarnessProbe]::PostMessageW($touchProbeWindow, $touchMouseMsg, $touchMouseFlags, $touchPointParam)) { throw 'Mouse button message failed.' }
    }
    # Tap action.
    Send-TouchProbeMouse 650 200 $true
    Start-Sleep -Milliseconds 60
    Send-TouchProbeMouse 650 200 $false
    Start-Sleep -Milliseconds 120
    # Hold to aim, release latch, then tap attack.
    Send-TouchProbeMouse 650 200 $true
    Start-Sleep -Milliseconds 420
    Send-TouchProbeMouse 650 200 $false
    Start-Sleep -Milliseconds 120
    Send-TouchProbeMouse 650 200 $true
    Start-Sleep -Milliseconds 60
    Send-TouchProbeMouse 650 200 $false
    Start-Sleep -Milliseconds 120
    # Two header taps: exploring -> aiming -> menu, then direct menu activation.
    for ($touchModeTap = 0; $touchModeTap -lt 2; $touchModeTap++) {
        Send-TouchProbeMouse 100 18 $true
        Start-Sleep -Milliseconds 30
        Send-TouchProbeMouse 100 18 $false
        Start-Sleep -Milliseconds 80
    }
    Send-TouchProbeMouse 110 160 $true
    Start-Sleep -Milliseconds 40
    Send-TouchProbeMouse 110 160 $false
    Start-Sleep -Milliseconds 100
    if (-not [TouchHarnessProbe]::PostMessageW($touchProbeWindow, 0x0010, [UIntPtr]::Zero, [IntPtr]::Zero)) { throw 'WM_CLOSE failed.' }
    if (-not $touchProbeProcess.WaitForExit(5000)) { throw 'Harness did not exit after its own WM_CLOSE.' }
    if ($touchProbeProcess.ExitCode -ne 0) { throw ('Harness exit ' + $touchProbeProcess.ExitCode) }
    $touchProbeRecords = @(Get-Content -LiteralPath $touchRecordPath | ForEach-Object { $_ | ConvertFrom-Json })
    $touchProbeFrames = @($touchProbeRecords | Where-Object { $_.type -eq 'frame' })
    foreach ($touchExpectedBits in @(16384, 512, 16896)) {
        if (-not ($touchProbeFrames | Where-Object { $_.expect.buttons -eq $touchExpectedBits })) { throw ('Missing mouse-driven pad state ' + $touchExpectedBits) }
    }
    $touchMenuFrames = @($touchProbeFrames | Where-Object { $_.expect.ui_actions | Where-Object { $_.action -eq 'activate' -and $_.id -eq 0 } })
    if ($touchMenuFrames.Count -ne 1 -or $touchMenuFrames[0].expect.buttons -ne 0) { throw 'Direct mouse menu activation was missing, duplicated, or also sent pad input.' }
    Write-Output ('WINDOWS_MOUSE_PASS: action, held aim, latched attack, direct menu; ' + $touchProbeFrames.Count + ' recorded frames; DPI scale ' + $touchProbeScale)
} finally {
    $touchProbeProcess.Refresh()
    if (-not $touchProbeProcess.HasExited -and $touchProbeProcess.MainWindowHandle -ne [IntPtr]::Zero) {
        [void][TouchHarnessProbe]::PostMessageW($touchProbeProcess.MainWindowHandle, 0x0010, [UIntPtr]::Zero, [IntPtr]::Zero)
        [void]$touchProbeProcess.WaitForExit(3000)
    }
    if ($touchPreviousDpi -ne [IntPtr]::Zero) { [void][TouchHarnessProbe]::SetThreadDpiAwarenessContext($touchPreviousDpi) }
}

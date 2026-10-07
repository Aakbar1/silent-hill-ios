param([Parameter(Mandatory=$true)][string]$Exe, [Parameter(Mandatory=$true)][string]$Disc)
# Post messages ONLY to the process created here. No global cursor/input changes.
$ErrorActionPreference = 'Stop'
$touchRoot = (Resolve-Path (Join-Path $PSScriptRoot '../../..')).Path
$touchPrivate = [IO.Path]::GetFullPath((Join-Path $touchRoot '../../private/work/touchwire'))
$touchStamp = [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfffffffZ')
$touchLog = Join-Path $touchPrivate "mouse-$touchStamp.log"
$touchErr = Join-Path $touchPrivate "mouse-$touchStamp.err"
$touchPng = Join-Path $touchPrivate "mouse-$touchStamp.png"
Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class NativeTouchProbe {
    private delegate bool EnumCallback(IntPtr hwnd, IntPtr data);
    [DllImport("user32.dll")] private static extern bool EnumWindows(EnumCallback callback, IntPtr data);
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] private static extern int GetWindowTextW(IntPtr hwnd, StringBuilder text, int count);
    [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr hwnd, uint msg, UIntPtr wp, IntPtr lp);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int left, top, right, bottom; }
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hwnd, out Rect rect);
    public static IntPtr Find(uint expected) {
        IntPtr result = IntPtr.Zero;
        EnumWindows((hwnd, data) => { uint pid; GetWindowThreadProcessId(hwnd, out pid);
            if (pid != expected) return true;
            var title = new StringBuilder(512); GetWindowTextW(hwnd, title, title.Capacity);
            if (!title.ToString().StartsWith("Silent Hill native C")) return true;
            Rect r; if (!GetClientRect(hwnd, out r) || r.right < 200 || r.bottom < 200) return true;
            result = hwnd; return false; }, IntPtr.Zero);
        return result;
    }
}
'@
$touchPreviousDpi = [NativeTouchProbe]::SetThreadDpiAwarenessContext([IntPtr](-4))
$touchArgs = @('--disc', ('"' + (Resolve-Path $Disc).Path + '"'), '--input', 'touch', '--frames', '2400',
    '--screenshot', ('"' + $touchPng + '"'), '--expect-state', '18', '--expect-step', '1',
    '--expect-option-entry', '0', '--min-lit-pixels', '1000')
$touchProcess = Start-Process -FilePath (Resolve-Path $Exe).Path -ArgumentList $touchArgs -PassThru -WindowStyle Hidden -RedirectStandardOutput $touchLog -RedirectStandardError $touchErr
$touchHandle = $touchProcess.Handle # Retain the process handle for reliable ExitCode.
try {
    $touchWindow = [IntPtr]::Zero
    for ($touchWait = 0; $touchWait -lt 100; $touchWait++) {
        $touchProcess.Refresh()
        if ($touchProcess.HasExited) { throw 'Native process exited before its window appeared.' }
        $touchWindow = [NativeTouchProbe]::Find([uint32]$touchProcess.Id)
        if ($touchWindow -ne [IntPtr]::Zero) { break }
        Start-Sleep -Milliseconds 100
    }
    if ($touchWindow -eq [IntPtr]::Zero) { throw 'Native window was not created.' }
    $touchScale = [NativeTouchProbe]::GetDpiForWindow($touchWindow) / 96.0
    $touchRect = New-Object NativeTouchProbe+Rect
    if (-not [NativeTouchProbe]::GetClientRect($touchWindow, [ref]$touchRect)) { throw 'GetClientRect failed.' }
    Write-Output "Native mouse window PID=$($touchProcess.Id) client=$($touchRect.right)x$($touchRect.bottom) DPI=$touchScale"
    function Send-NativeTap([double]$x, [double]$y) {
        $touchPoint = [IntPtr](([int][Math]::Round($y) -shl 16) -bor ([int][Math]::Round($x) -band 65535))
        foreach ($touchMsg in @(0x0200, 0x0201, 0x0202)) {
            $touchFlags = if ($touchMsg -eq 0x0201) { [UIntPtr]::new([uint32]1) } else { [UIntPtr]::Zero }
            if (-not [NativeTouchProbe]::PostMessageW($touchWindow, $touchMsg, $touchFlags, $touchPoint)) { throw 'Own-window mouse message failed.' }
            Start-Sleep -Milliseconds 60
        }
    }
    # Release build maintains the host's 60 Hz pacing. Verify actual scenes below;
    # timing is an input schedule, never treated as evidence of a transition.
    Start-Sleep -Milliseconds 24000
    Send-NativeTap ($touchRect.right * 0.5) ($touchRect.bottom * 0.4)
    Start-Sleep -Milliseconds 2500
    Send-NativeTap ($touchRect.right - 88 * $touchScale) ($touchRect.bottom - 32 * $touchScale)
    Start-Sleep -Milliseconds 1500
    Send-NativeTap ($touchRect.right - 32 * $touchScale) ($touchRect.bottom - 32 * $touchScale)
    Start-Sleep -Milliseconds 700
    Send-NativeTap (108 * $touchScale) ($touchRect.bottom - 40 * $touchScale)
    Start-Sleep -Milliseconds 500
    Send-NativeTap ($touchRect.right - 88 * $touchScale) ($touchRect.bottom - 32 * $touchScale)
    if (-not $touchProcess.WaitForExit(30000)) { throw 'Native mouse probe timed out.' }
    if ($touchProcess.ExitCode -ne 0) { throw "Native mouse exit $($touchProcess.ExitCode): $(Get-Content $touchErr -Raw)" }
    $touchEvidence = Get-Content -LiteralPath $touchLog -Raw
    if ($touchEvidence -notmatch 'TOUCH_SCENE tick=\d+ state=7 step=1 menu=3' -or
        $touchEvidence -notmatch 'TOUCH_RESULT PASS' -or -not (Test-Path -LiteralPath $touchPng)) {
        throw 'Actual difficulty/options state or private screenshot evidence missing.'
    }
    Write-Output "NATIVE_MOUSE_PASS: difficulty and options through winit mouse events; DPI=$touchScale; evidence=$touchLog"
} finally {
    $touchProcess.Refresh()
    if (-not $touchProcess.HasExited -and $touchWindow -ne [IntPtr]::Zero) {
        [void][NativeTouchProbe]::PostMessageW($touchWindow, 0x0010, [UIntPtr]::Zero, [IntPtr]::Zero)
        [void]$touchProcess.WaitForExit(3000)
    }
    if ($touchPreviousDpi -ne [IntPtr]::Zero) { [void][NativeTouchProbe]::SetThreadDpiAwarenessContext($touchPreviousDpi) }
}

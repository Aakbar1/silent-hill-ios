$ErrorActionPreference = 'Stop'
$iosRoot = Split-Path $PSScriptRoot -Parent
$smokeOutput = Join-Path $iosRoot 'out/windows-smoke'
New-Item -ItemType Directory -Force -Path $smokeOutput | Out-Null
$logPath = Join-Path $smokeOutput 'shell.log'
if (Test-Path -LiteralPath $logPath) { Remove-Item -LiteralPath $logPath }
$env:SHELL_SMOKE_FRAMES = '8'
$env:SHELL_DOCUMENTS = $smokeOutput
try {
    $executable = Join-Path $iosRoot 'shell/target/debug/silenthill-shell.exe'
    $process = Start-Process -FilePath $executable -WindowStyle Hidden -PassThru
    if (-not $process.WaitForExit(60000)) {
        Stop-Process -Id $process.Id
        throw 'Window/GPU smoke timed out after 60 seconds.'
    }
    if ($process.ExitCode -ne 0) { throw "Shell exited with $($process.ExitCode)" }
    $log = Get-Content -LiteralPath $logPath -Raw
    if ($log -notmatch 'first frame presented' -or $log -notmatch 'smoke complete; frames=8') {
        throw "Shell did not present 8 frames. Log: $log"
    }
    Write-Output 'PASS: native Windows window + GPU + shader, 8 presented frames, Documents log, clean exit.'
} finally {
    Remove-Item Env:SHELL_SMOKE_FRAMES -ErrorAction SilentlyContinue
    Remove-Item Env:SHELL_DOCUMENTS -ErrorAction SilentlyContinue
}

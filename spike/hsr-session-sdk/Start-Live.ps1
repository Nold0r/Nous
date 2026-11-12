param(
    [ValidateRange(1, 600)][int]$Seconds = 120,
    [string]$OutputPath = 'D:\HSR-Lab\protocol\4.6.0\session-sdk-live.json'
)
$ErrorActionPreference = 'Stop'
$captureExe = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\hsr-sniff\target\release\hsr-session-sdk.exe'))
$captureReport = [IO.Path]::GetFullPath($OutputPath)
if (-not (Test-Path -LiteralPath $captureExe -PathType Leaf)) {
    throw 'Build hsr-session-sdk first; see README.md.'
}
if (-not (Test-Path -LiteralPath ([IO.Path]::GetDirectoryName($captureReport)) -PathType Container)) {
    throw 'The report directory must already exist.'
}
$captureIdentity = [Security.Principal.WindowsIdentity]::GetCurrent()
$capturePrincipal = [Security.Principal.WindowsPrincipal]::new($captureIdentity)
if ($capturePrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    & $captureExe --live --seconds $Seconds --out $captureReport
    exit $LASTEXITCODE
}
$captureArguments = @('--live', '--seconds', "$Seconds", '--out', ('"{0}"' -f $captureReport))
$captureProcess = Start-Process -FilePath $captureExe -Verb RunAs -WindowStyle Hidden -ArgumentList $captureArguments -PassThru
Write-Output "Capture process started: $($captureProcess.Id); duration: $Seconds seconds."
Write-Output "Report after completion: $captureReport"

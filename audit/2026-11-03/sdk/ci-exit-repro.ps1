$ErrorActionPreference = 'Stop'
& cmd.exe /d /c 'exit 7'
$firstExit = $LASTEXITCODE
& cmd.exe /d /c 'exit 0'
$lastExit = $LASTEXITCODE
Write-Output "firstExit=$firstExit; lastExit=$lastExit"
exit $LASTEXITCODE

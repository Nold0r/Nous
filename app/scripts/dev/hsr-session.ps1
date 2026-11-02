<#
Dev script: logs into Honkai: Star Rail or back out to the login screen by
emulating clicks, so the sniffer sees a fresh login (data tests, recordings)
without doing it by hand. Not part of the app.

  npm run game:login     # start the game if needed, then click through to the world
  npm run game:logout    # Esc -> power button -> confirm: back to the login screen
  npm run game:relogin   # logout, then login

Run from an ADMINISTRATOR terminal: the game runs elevated, and Windows drops
input sent to it from a normal process. Click points are fractions of the game
window's client area, measured on game 4.6 (RU UI) from screenshots; see
docs/Руководства/Вход в игру скриптом.md.

Safety: input is only sent while the game window is in the foreground, and the
script refuses to start if the mouse/keyboard were used in the last few seconds
(someone is at the PC). -Force skips that check.
#>
param(
    [Parameter(Mandatory)][ValidateSet('login', 'logout', 'relogin')][string]$Action,
    [switch]$Force,
    # extra wait multiplier for slow PCs
    [double]$Slow = 1.0
)

$ErrorActionPreference = 'Stop'

Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class Hsr {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
    [StructLayout(LayoutKind.Sequential)] struct LASTINPUTINFO { public uint cbSize; public uint dwTime; }
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, int x, int y, uint d, UIntPtr e);
    [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint f, UIntPtr e);
    [DllImport("user32.dll")] static extern bool GetLastInputInfo(ref LASTINPUTINFO i);
    public static double IdleSeconds() {
        var i = new LASTINPUTINFO(); i.cbSize = (uint)Marshal.SizeOf(i);
        GetLastInputInfo(ref i);
        return ((uint)Environment.TickCount - i.dwTime) / 1000.0;
    }
}
'@
# physical pixels, so client rect and cursor agree under display scaling
[void][Hsr]::SetProcessDPIAware()

# --- click points: fractions of the client area (game 4.6) ---
$P = @{
    TitleFree    = @(0.50, 0.55)   # empty area of the title screen (avoid server button and right column)
    Forward      = @(0.50, 0.937)  # "ВПЕРЁД" bar at the bottom
    PhonePower   = @(0.973, 0.905) # power button at the bottom of the phone menu's right strip
    ConfirmLeave = @(0.608, 0.621) # "Подтвердить" in "Вы хотите вернуться на страницу входа?"
}

function Wait([double]$sec) { Start-Sleep -Milliseconds ([int]($sec * 1000 * $Slow)) }

function Get-GameWindow {
    $p = Get-Process StarRail -ErrorAction SilentlyContinue | Where-Object MainWindowHandle -ne 0 | Select-Object -First 1
    if ($p) { return $p.MainWindowHandle }
    return [IntPtr]::Zero
}

function Find-GameExe {
    foreach ($root in 'HKCU:\Software\Cognosphere\HYP', 'HKCU:\Software\miHoYo\HYP') {
        if (-not (Test-Path $root)) { continue }
        $key = Get-ChildItem $root -Recurse -ErrorAction SilentlyContinue |
            Where-Object { $_.PSChildName -like 'hkrpg_*' -and $_.GetValue('GameInstallPath') } | Select-Object -First 1
        if ($key) {
            $exe = Join-Path $key.GetValue('GameInstallPath') 'StarRail.exe'
            if (Test-Path $exe) { return $exe }
        }
    }
    throw 'StarRail.exe not found in the HoYoPlay registry keys'
}

function Focus-Game {
    $h = Get-GameWindow
    if ($h -eq [IntPtr]::Zero) { throw 'game window not found' }
    [void][Hsr]::ShowWindow($h, 9) # SW_RESTORE
    [void][Hsr]::SetForegroundWindow($h)
    Start-Sleep -Milliseconds 400
    if ([Hsr]::GetForegroundWindow() -ne $h) { throw 'could not bring the game to the foreground' }
    return $h
}

function Click([string]$name) {
    $h = Focus-Game
    $r = New-Object Hsr+RECT; [void][Hsr]::GetClientRect($h, [ref]$r)
    $pt = New-Object Hsr+POINT
    $pt.X = [int]($r.R * $P[$name][0]); $pt.Y = [int]($r.B * $P[$name][1])
    [void][Hsr]::ClientToScreen($h, [ref]$pt)
    [void][Hsr]::SetCursorPos($pt.X, $pt.Y)
    Start-Sleep -Milliseconds 150
    [Hsr]::mouse_event(0x02, 0, 0, 0, [UIntPtr]::Zero) # left down
    Start-Sleep -Milliseconds 60
    [Hsr]::mouse_event(0x04, 0, 0, 0, [UIntPtr]::Zero) # left up
    Write-Host "  click $name ($($pt.X), $($pt.Y))"
}

function Key([byte]$vk, [string]$name) {
    [void](Focus-Game)
    [Hsr]::keybd_event($vk, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 60
    [Hsr]::keybd_event($vk, 0, 2, [UIntPtr]::Zero)
    Write-Host "  key $name"
}

function Do-Login {
    if ((Get-GameWindow) -eq [IntPtr]::Zero) {
        $exe = Find-GameExe
        Write-Host "starting $exe"
        Start-Process -FilePath $exe -WorkingDirectory (Split-Path $exe)
        $deadline = (Get-Date).AddMinutes(3)
        while ((Get-GameWindow) -eq [IntPtr]::Zero) {
            if ((Get-Date) -gt $deadline) { throw 'game window did not appear' }
            Start-Sleep 2
        }
        Write-Host 'title screen: waiting for it to load'
        Wait 25
    }
    Write-Host 'title screen -> click'
    Click TitleFree
    Wait 20
    Write-Host '"Forward" screen -> click'
    Click Forward
    Wait 30
    Write-Host 'done: should be in the world (announcement pop-ups, if any, are left for you)'
}

function Do-Logout {
    Write-Host 'world -> Esc (phone menu)'
    Key 0x1B 'Esc'
    Wait 2.5
    Write-Host 'phone -> power button'
    Click PhonePower
    Wait 2.5
    Write-Host 'dialog -> confirm'
    Click ConfirmLeave
    Wait 15
    Write-Host 'done: back on the login screen'
}

$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
    [Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) { throw 'run from an administrator terminal: the game ignores input from normal processes' }

$idle = [Hsr]::IdleSeconds()
if (-not $Force -and $idle -lt 5) {
    throw "mouse/keyboard used $([math]::Round($idle, 1)) s ago: hands off and retry (or -Force)"
}

switch ($Action) {
    'login' { Do-Login }
    'logout' { Do-Logout }
    'relogin' { Do-Logout; Do-Login }
}

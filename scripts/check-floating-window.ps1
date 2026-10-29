param([Parameter(Mandatory)][string]$Executable)
$ErrorActionPreference = 'Stop'

# Run on the Windows Actions runner against the actual packaged application.
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class FloatingWindowCheck {
    public delegate bool EnumProc(IntPtr hwnd, IntPtr data);
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc callback, IntPtr data);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint processId);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern int GetWindowRgn(IntPtr hwnd, IntPtr region);
    [DllImport("gdi32.dll")] public static extern IntPtr CreateRectRgn(int left, int top, int right, int bottom);
    [DllImport("gdi32.dll")] public static extern int GetRgnBox(IntPtr region, out Rect rect);
    [DllImport("gdi32.dll")] public static extern bool PtInRegion(IntPtr region, int x, int y);
    [DllImport("gdi32.dll")] public static extern bool DeleteObject(IntPtr obj);
}
'@

$appProcess = Start-Process -FilePath (Resolve-Path -LiteralPath $Executable) -PassThru -WindowStyle Hidden
try {
    $script:ball = [IntPtr]::Zero
    $deadline = (Get-Date).AddSeconds(45)
    while ($script:ball -eq [IntPtr]::Zero -and (Get-Date) -lt $deadline) {
        if ($appProcess.HasExited) { throw "Application exited before showing the floating window." }
        [FloatingWindowCheck]::EnumWindows({
            param($hwnd, $data)
            $ownerId = [uint32]0
            [void][FloatingWindowCheck]::GetWindowThreadProcessId($hwnd, [ref]$ownerId)
            if ($ownerId -eq $appProcess.Id -and [FloatingWindowCheck]::IsWindowVisible($hwnd)) {
                $client = New-Object FloatingWindowCheck+Rect
                [void][FloatingWindowCheck]::GetClientRect($hwnd, [ref]$client)
                $expectedSize = [Math]::Round(76 * [FloatingWindowCheck]::GetDpiForWindow($hwnd) / 96)
                if ([Math]::Abs($client.Right - $expectedSize) -le 1 -and [Math]::Abs($client.Bottom - $expectedSize) -le 1) {
                    $script:ball = $hwnd
                }
            }
            return $true
        }, [IntPtr]::Zero) | Out-Null
        if ($script:ball -eq [IntPtr]::Zero) { Start-Sleep -Milliseconds 500 }
    }
    if ($script:ball -eq [IntPtr]::Zero) { throw 'The mounted frontend did not show the 76px floating window.' }

    # Capture the real HWND and its surrounding corners on the Windows runner.
    Add-Type -AssemblyName System.Drawing
    New-Item -ItemType Directory -Force -Path '.artifacts/floating' | Out-Null
    $frame = New-Object FloatingWindowCheck+Rect
    [void][FloatingWindowCheck]::GetWindowRect($script:ball, [ref]$frame)
    $bitmap = [System.Drawing.Bitmap]::new(($frame.Right - $frame.Left + 32), ($frame.Bottom - $frame.Top + 32))
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    try {
        Start-Sleep -Milliseconds 1000
        $graphics.CopyFromScreen(($frame.Left - 16), ($frame.Top - 16), 0, 0, $bitmap.Size)
        $bitmap.Save((Join-Path (Get-Location) '.artifacts/floating/desktop-orb.png'))
    } catch {
        Write-Warning "Desktop capture unavailable: $_"
    } finally {
        $graphics.Dispose()
        $bitmap.Dispose()
    }

    $region = [FloatingWindowCheck]::CreateRectRgn(0, 0, 0, 0)
    try {
        if ([FloatingWindowCheck]::GetWindowRgn($script:ball, $region) -le 0) { throw 'The actual floating HWND has no native region.' }
        $bounds = New-Object FloatingWindowCheck+Rect
        [void][FloatingWindowCheck]::GetRgnBox($region, [ref]$bounds)
        $width = $bounds.Right - $bounds.Left
        $height = $bounds.Bottom - $bounds.Top
        if ([Math]::Abs($width - $height) -gt 1) { throw "Region is not circular: $width x $height" }
        $cx = [int](($bounds.Left + $bounds.Right) / 2)
        $cy = [int](($bounds.Top + $bounds.Bottom) / 2)
        if (-not [FloatingWindowCheck]::PtInRegion($region, $cx, $cy)) { throw 'Circle center is excluded.' }
        foreach ($x in @($bounds.Left, ($bounds.Right - 1))) {
            foreach ($y in @($bounds.Top, ($bounds.Bottom - 1))) {
                if ([FloatingWindowCheck]::PtInRegion($region, $x, $y)) { throw 'A rectangular corner is still in the native window.' }
            }
        }
        Write-Output "PASS: mounted floating window is visible, native region is $width x $height, center included, all four corners excluded."
    } finally {
        [void][FloatingWindowCheck]::DeleteObject($region)
    }
} finally {
    [FloatingWindowCheck]::EnumWindows({
        param($hwnd, $data)
        $ownerId = [uint32]0
        [void][FloatingWindowCheck]::GetWindowThreadProcessId($hwnd, [ref]$ownerId)
        if ($ownerId -eq $appProcess.Id) {
            $client = New-Object FloatingWindowCheck+Rect
            [void][FloatingWindowCheck]::GetClientRect($hwnd, [ref]$client)
            Write-Host "Window $hwnd visible=$([FloatingWindowCheck]::IsWindowVisible($hwnd)) client=$($client.Right)x$($client.Bottom) dpi=$([FloatingWindowCheck]::GetDpiForWindow($hwnd))"
        }
        return $true
    }, [IntPtr]::Zero) | Out-Null
    if (-not $appProcess.HasExited) { Stop-Process -Id $appProcess.Id -Force }
}

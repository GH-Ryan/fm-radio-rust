param(
  [Parameter(Mandatory = $true)][string]$Out,
  [int]$Width = 1366,
  [int]$Height = 768
)

# Captures the app window at an exact size, off-screen if necessary.
#
# Two things make screen capture the wrong tool here. The Store wants
# screenshots of at least 1366x768, and a maximized window on this display only
# yields a 1366x720 client area, so the height falls short. And a full-screen
# grab would carry the taskbar and whatever else happens to be behind the
# window, which is not something to put in a listing.
#
# So the window is resized to the exact target and drawn straight out of the
# window's own surface. That works even when the window hangs below the bottom
# of the screen, which is what makes the height reachable at all.
#
# PW_RENDERFULLCONTENT is not optional here: WebView2 composites through
# DirectComposition, and a plain BitBlt of such a window returns an empty or
# stale frame.
#
# One trap worth knowing: the captured surface is only as fresh as the window's
# last presented frame. WebView2 stops compositing once the window loses the
# foreground, so a capture taken after clicking something else can come back
# byte-for-byte identical to the previous one even though the app has moved on.
# When that happens, bring the app window forward and capture again rather than
# trusting the image.
#
# Comments are deliberately ASCII: Windows PowerShell 5.1 reads a .ps1 file as
# ANSI unless it carries a BOM.

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$src = @'
using System;
using System.Drawing;
using System.Runtime.InteropServices;

public static class WindowShot {
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int n);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);

  [StructLayout(LayoutKind.Sequential)]
  public struct RECT { public int Left, Top, Right, Bottom; }

  [DllImport("gdi32.dll")] public static extern IntPtr CreateCompatibleDC(IntPtr dc);
  [DllImport("gdi32.dll")] public static extern IntPtr CreateCompatibleBitmap(IntPtr dc, int w, int h);
  [DllImport("gdi32.dll")] public static extern IntPtr SelectObject(IntPtr dc, IntPtr o);
  [DllImport("gdi32.dll")] public static extern bool DeleteObject(IntPtr o);
  [DllImport("gdi32.dll")] public static extern bool DeleteDC(IntPtr dc);

  const uint PW_RENDERFULLCONTENT = 0x00000002;

  public static bool Capture(IntPtr hwnd, string path, int width, int height) {
    RECT r;
    if (!GetWindowRect(hwnd, out r)) return false;

    IntPtr screen = GetDC(IntPtr.Zero);
    IntPtr mem = CreateCompatibleDC(screen);
    IntPtr bmp = CreateCompatibleBitmap(screen, width, height);
    IntPtr old = SelectObject(mem, bmp);

    // White, because the app is dark and a transparent or black surround would
    // read as a rendering bug rather than as a margin.
    using (var g = Graphics.FromHdc(mem)) {
      g.Clear(Color.White);
    }

    bool ok = PrintWindow(hwnd, mem, PW_RENDERFULLCONTENT);
    if (!ok) PrintWindow(hwnd, mem, 0);

    using (var img = Image.FromHbitmap(bmp)) {
      img.Save(path, System.Drawing.Imaging.ImageFormat.Png);
    }

    SelectObject(mem, old);
    DeleteObject(bmp);
    DeleteDC(mem);
    ReleaseDC(IntPtr.Zero, screen);
    return ok;
  }

  [DllImport("user32.dll")] public static extern IntPtr GetDC(IntPtr h);
  [DllImport("user32.dll")] public static extern int ReleaseDC(IntPtr h, IntPtr dc);

  public static void Resize(IntPtr hwnd, int w, int h) {
    ShowWindow(hwnd, 9); // SW_RESTORE
    SetWindowPos(hwnd, IntPtr.Zero, 0, 0, w, h, 0x0004 | 0x0010); // NOZORDER|NOACTIVATE
  }
}
'@

Add-Type -TypeDefinition $src -ReferencedAssemblies System.Drawing, System.Windows.Forms

$proc = Get-Process -Name 'fm-radio' -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $proc) { throw 'fm-radio window not found' }

[WindowShot]::Resize($proc.MainWindowHandle, $Width, $Height)
Start-Sleep -Seconds 2
[WindowShot]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
Start-Sleep -Seconds 3

New-Item -ItemType Directory -Path (Split-Path $Out) -Force | Out-Null
$ok = [WindowShot]::Capture($proc.MainWindowHandle, $Out, $Width, $Height)

$img = [System.Drawing.Image]::FromFile($Out)
Write-Host ("{0}  {1}x{2}  PrintWindow={3}  {4:N0} bytes" -f $Out, $img.Width, $img.Height, $ok, (Get-Item $Out).Length)
$img.Dispose()
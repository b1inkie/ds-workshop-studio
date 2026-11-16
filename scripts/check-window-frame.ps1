param([string]$Executable = 'src-tauri/target/i686-pc-windows-msvc/debug/ds-workshop-tool.exe')
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class FrameCheck {
[StructLayout(LayoutKind.Sequential)] public struct RECT {public int l,t,r,b;}
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h,out RECT r);
[DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h,out RECT r);
[DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h,IntPtr dc,uint flags);
[DllImport("user32.dll")] public static extern int GetWindowRgn(IntPtr h,IntPtr r);
[DllImport("gdi32.dll")] public static extern IntPtr CreateRectRgn(int a,int b,int c,int d);
[DllImport("gdi32.dll")] public static extern bool PtInRegion(IntPtr r,int x,int y);
[DllImport("gdi32.dll")] public static extern bool DeleteObject(IntPtr r);
[DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h,int cmd);
[DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h,uint msg,IntPtr w,IntPtr l);
}
'@
$p = Start-Process (Resolve-Path $Executable) -PassThru
$region = [FrameCheck]::CreateRectRgn(0,0,0,0)
try {
    Start-Sleep -Seconds 2
    $p.Refresh()
    $h = $p.MainWindowHandle
    $rect = New-Object FrameCheck+RECT
    $client = New-Object FrameCheck+RECT
    [void][FrameCheck]::GetWindowRect($h,[ref]$rect)
    [void][FrameCheck]::GetClientRect($h,[ref]$client)
    $width = $rect.r-$rect.l; $height = $rect.b-$rect.t
    if ($width -ne $client.r -or $height -ne $client.b) { throw 'Unexpected native non-client border' }
    if ([FrameCheck]::GetWindowRgn($h,$region) -ne 0) { throw 'Binary native clipping region must not be used for antialiased corners' }
    $bmp = New-Object Drawing.Bitmap $width,$height
    $graphics = [Drawing.Graphics]::FromImage($bmp)
    $dc = $graphics.GetHdc()
    try { [void][FrameCheck]::PrintWindow($h,$dc,2) } finally { $graphics.ReleaseHdc($dc) }
    New-Item '.local' -ItemType Directory -Force | Out-Null
    $bmp.Save("$pwd/.local/native-frame-verified.png")
    # Sample straight edges away from rounded corners and content; native white strips fail.
    $samples = @(@(0,100),@(($width-1),100),@(100,0),@(100,($height-1)))
    foreach ($point in $samples) {
        $pixel = $bmp.GetPixel($point[0],$point[1])
        if ([Math]::Max($pixel.R,[Math]::Max($pixel.G,$pixel.B)) -gt 80) { throw "Bright native edge at $point : $pixel" }
    }
    $graphics.Dispose(); $bmp.Dispose()
    $x = $rect.l+1; $y = $rect.t+[int]($height/2)
    $hit = [FrameCheck]::SendMessage($h,0x84,[IntPtr]::Zero,[IntPtr](($y -shl 16) -bor ($x -band 65535))).ToInt32()
    if ($hit -ne 10) { throw "Left edge is not resizable: HT=$hit" }
    [void][FrameCheck]::ShowWindow($h,3)
    Start-Sleep -Milliseconds 600
    if ([FrameCheck]::GetWindowRgn($h,$region) -ne 0) { throw 'Maximized region was not cleared' }
    [void][FrameCheck]::ShowWindow($h,9)
    Start-Sleep -Milliseconds 600
    if ([FrameCheck]::GetWindowRgn($h,$region) -ne 0) { throw 'Restored window uses binary native clipping' }
    # The desktop smoke test captures actual WebView alpha, not a CSS approximation.
    $alphaImage = Join-Path $pwd '.local/desktop-smoke.png'
    if (-not (Test-Path $alphaImage)) { throw 'Run npm run test:desktop to generate the alpha screenshot first' }
    $image = [Drawing.Bitmap]::FromFile($alphaImage)
    $fractional = 0
    for ($y=0; $y -lt 14; $y++) { for ($x=0; $x -lt 14; $x++) {
        $alpha = $image.GetPixel($x,$y).A
        if ($alpha -gt 0 -and $alpha -lt 255) { $fractional++ }
    }}
    $transparentCorner = $image.GetPixel(0,0).A -eq 0
    $image.Dispose()
    if (-not $transparentCorner -or $fractional -lt 5) { throw 'Corner has no fractional alpha antialiasing' }
    Write-Host "Native frame passed: no white edges/non-client border/hard clipping, edge resize, maximize/restore. Alpha screenshot: $fractional antialiased corner pixels."
} finally {
    [void][FrameCheck]::DeleteObject($region)
    Stop-Process -Id $p.Id -ErrorAction SilentlyContinue
}

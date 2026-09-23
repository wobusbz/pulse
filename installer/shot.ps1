param([string]$Out, [string]$TitleLike)
Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class W {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  public struct RECT { public int left, top, right, bottom; }
}
"@
$script:hit = [IntPtr]::Zero
$cb = [W+EnumProc]{
  param($h,$l)
  if ([W]::IsWindowVisible($h)) {
    $cn = New-Object System.Text.StringBuilder 256
    [W]::GetClassName($h,$cn,256) | Out-Null
    if ($cn.ToString() -eq '#32770') {
      $sb = New-Object System.Text.StringBuilder 256
      [W]::GetWindowText($h,$sb,256) | Out-Null
      if ($sb.ToString() -like $TitleLike) { $script:hit = $h }
    }
  }
  return $true
}
[W]::EnumWindows($cb,[IntPtr]::Zero) | Out-Null
if ($script:hit -eq [IntPtr]::Zero) { Write-Output 'window not found'; exit 1 }
$r = New-Object W+RECT
[W]::GetWindowRect($script:hit, [ref]$r) | Out-Null
Add-Type -AssemblyName System.Drawing
$w = $r.right - $r.left
$hh = $r.bottom - $r.top
$bmp = New-Object System.Drawing.Bitmap($w,$hh)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($r.left,$r.top,0,0,(New-Object System.Drawing.Size($w,$hh)))
# upscale 2x for a clearer image
$big = New-Object System.Drawing.Bitmap(($w*2),($hh*2))
$g2 = [System.Drawing.Graphics]::FromImage($big)
$g2.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$g2.DrawImage($bmp,0,0,$big.Width,$big.Height)
$big.Save($Out,[System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose();$bmp.Dispose();$big.Dispose();$g2.Dispose()
Write-Output ('captured ' + $w + 'x' + $hh)

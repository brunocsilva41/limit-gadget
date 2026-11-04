<#
  Gera docs/social-preview.png (1280x640), a imagem que o GitHub mostra quando o link do
  repositório é compartilhado. O GitHub não permite enviá-la por API: faça o upload manual em
  Settings > General > Social preview.
#>
$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing
$root = Split-Path -Parent $PSScriptRoot
$iconPath = Join-Path $root "icons\icon.png"
$out = Join-Path $root "docs\social-preview.png"

$W = 1280; $H = 640
$bmp = New-Object System.Drawing.Bitmap $W, $H
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = 'AntiAlias'; $g.TextRenderingHint = 'AntiAliasGridFit'; $g.InterpolationMode = 'HighQualityBicubic'

# fundo: gradiente grafite com brilhos suaves (mesma paleta das capturas)
$rect = New-Object System.Drawing.Rectangle 0, 0, $W, $H
$lg = New-Object System.Drawing.Drawing2D.LinearGradientBrush $rect, ([System.Drawing.Color]::FromArgb(16, 18, 30)), ([System.Drawing.Color]::FromArgb(40, 24, 62)), 30
$g.FillRectangle($lg, $rect)
foreach ($glow in @(@(0.86, 0.15, 640, 232, 128, 92, 110), @(0.05, 0.95, 560, 66, 133, 244, 80))) {
  $cx = [int]($W * $glow[0]); $cy = [int]($H * $glow[1]); $rad = $glow[2] / 2
  $path = New-Object System.Drawing.Drawing2D.GraphicsPath
  $path.AddEllipse($cx - $rad, $cy - $rad, 2 * $rad, 2 * $rad)
  $pg = New-Object System.Drawing.Drawing2D.PathGradientBrush $path
  $pg.CenterColor = [System.Drawing.Color]::FromArgb($glow[6], $glow[3], $glow[4], $glow[5])
  $pg.SurroundColors = @([System.Drawing.Color]::FromArgb(0, $glow[3], $glow[4], $glow[5]))
  $g.FillPath($pg, $path)
}

# ícone do app
$icon = [System.Drawing.Image]::FromFile($iconPath)
$g.DrawImage($icon, 110, 150, 340, 340)

# textos
$white = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(245, 245, 247))
$muted = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(170, 245, 245, 247))
$title = New-Object System.Drawing.Font "Segoe UI Semibold", 76, ([System.Drawing.FontStyle]::Regular), ([System.Drawing.GraphicsUnit]::Pixel)
$sub = New-Object System.Drawing.Font "Segoe UI", 30, ([System.Drawing.FontStyle]::Regular), ([System.Drawing.GraphicsUnit]::Pixel)
$chipFont = New-Object System.Drawing.Font "Segoe UI Semibold", 24, ([System.Drawing.FontStyle]::Regular), ([System.Drawing.GraphicsUnit]::Pixel)
$g.DrawString("LimitGadget", $title, $white, 500, 175)
$g.DrawString("Os limites de uso das suas IAs", $sub, $muted, 506, 282)
$g.DrawString("na barra de tarefas do Windows", $sub, $muted, 506, 322)

# provedores
$chips = @(@("Claude", 217, 119, 87), @("ChatGPT", 16, 163, 127), @("Copilot", 163, 113, 247), @("Antigravity", 66, 133, 244))
$x = 506; $y = 400
foreach ($c in $chips) {
  $size = $g.MeasureString($c[0], $chipFont)
  $w = [int]$size.Width + 36
  $r = New-Object System.Drawing.Rectangle $x, $y, $w, 52
  $gp = New-Object System.Drawing.Drawing2D.GraphicsPath
  $d = 14
  $gp.AddArc($r.X, $r.Y, $d, $d, 180, 90); $gp.AddArc($r.Right - $d, $r.Y, $d, $d, 270, 90)
  $gp.AddArc($r.Right - $d, $r.Bottom - $d, $d, $d, 0, 90); $gp.AddArc($r.X, $r.Bottom - $d, $d, $d, 90, 90); $gp.CloseFigure()
  $g.FillPath((New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(36, $c[1], $c[2], $c[3]))), $gp)
  $g.DrawPath((New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(120, $c[1], $c[2], $c[3])), 1.5), $gp)
  $g.DrawString($c[0], $chipFont, (New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb($c[1], $c[2], $c[3]))), $x + 18, $y + 11)
  $x += $w + 14
}
New-Item -ItemType Directory -Force (Split-Path $out) | Out-Null
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
"Gerado: $out"

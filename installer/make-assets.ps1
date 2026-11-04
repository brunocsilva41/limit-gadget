<#
  Gera os recursos gráficos do instalador (header.bmp e sidebar.bmp).
  Só usa System.Drawing, sem dependências externas.
  Uso: powershell -NoProfile -ExecutionPolicy Bypass -File installer\make-assets.ps1
#>
param(
  [string]$OutDir = $PSScriptRoot
)

Add-Type -AssemblyName System.Drawing
. (Join-Path $PSScriptRoot '..\icons\icon-draw.ps1')

# Paleta do app
$Grafite1  = [System.Drawing.Color]::FromArgb(0x1c, 0x1c, 0x1e)
$Grafite2  = [System.Drawing.Color]::FromArgb(0x2c, 0x2c, 0x2e)
$Trilho    = [System.Drawing.Color]::FromArgb(0x48, 0x48, 0x4a)
$Texto     = [System.Drawing.Color]::FromArgb(0xf5, 0xf5, 0xf7)
$TextoSub  = [System.Drawing.Color]::FromArgb(0x98, 0x98, 0x9d)
$Terracota = [System.Drawing.Color]::FromArgb(0xe8, 0x80, 0x5c)

function Get-FontFamilyName {
  param([string[]]$Candidates)
  $installed = (New-Object System.Drawing.Text.InstalledFontCollection).Families | ForEach-Object { $_.Name }
  foreach ($c in $Candidates) { if ($installed -contains $c) { return $c } }
  return 'Segoe UI'
}

# Segoe UI Variable existe no Windows 11. No Windows 10 cai para Segoe UI Semibold.
$FontTitulo = Get-FontFamilyName @('Segoe UI Variable Display Semib', 'Segoe UI Variable Display', 'Segoe UI Semibold', 'Segoe UI')
$FontTexto  = Get-FontFamilyName @('Segoe UI Variable Text', 'Segoe UI')

function New-Canvas {
  param([int]$W, [int]$H)
  $bmp = New-Object System.Drawing.Bitmap($W, $H, [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
  $bmp.SetResolution(96, 96)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.SmoothingMode      = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
  $g.InterpolationMode  = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
  $g.PixelOffsetMode    = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
  $g.TextRenderingHint  = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
  # Gradiente vertical muito sutil, de grafite escuro para grafite um pouco mais claro
  $rect = New-Object System.Drawing.Rectangle(0, 0, $W, $H)
  $br = New-Object System.Drawing.Drawing2D.LinearGradientBrush($rect, $Grafite1, $Grafite2, 90.0)
  $g.FillRectangle($br, $rect)
  $br.Dispose()
  return @{ Bmp = $bmp; G = $g }
}

function Add-Ring {
  param($G, [single]$Cx, [single]$Cy, [single]$Diametro, [single]$Espessura, [double]$Fracao)
  $r = $Diametro / 2
  $rect = New-Object System.Drawing.RectangleF(($Cx - $r), ($Cy - $r), $Diametro, $Diametro)
  $penTrilho = New-Object System.Drawing.Pen($Trilho, $Espessura)
  $G.DrawEllipse($penTrilho, $rect)
  $penArco = New-Object System.Drawing.Pen($Terracota, $Espessura)
  $penArco.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
  $penArco.EndCap   = [System.Drawing.Drawing2D.LineCap]::Round
  $G.DrawArc($penArco, $rect, -90.0, [single](360.0 * $Fracao))
  $penTrilho.Dispose(); $penArco.Dispose()
}

function Add-CenteredText {
  param($G, [string]$Texto, [string]$Familia, [single]$Tamanho, [System.Drawing.FontStyle]$Estilo, $Cor, [single]$X, [single]$Y, [single]$W, [single]$H)
  $font = New-Object System.Drawing.Font($Familia, $Tamanho, $Estilo, [System.Drawing.GraphicsUnit]::Pixel)
  $sf = New-Object System.Drawing.StringFormat
  $sf.Alignment = [System.Drawing.StringAlignment]::Center
  $sf.LineAlignment = [System.Drawing.StringAlignment]::Center
  $brush = New-Object System.Drawing.SolidBrush($Cor)
  $G.DrawString($Texto, $font, $brush, (New-Object System.Drawing.RectangleF($X, $Y, $W, $H)), $sf)
  $brush.Dispose(); $font.Dispose(); $sf.Dispose()
}

function Save-Bmp {
  param($Canvas, [string]$Path)
  $Canvas.G.Dispose()
  $Canvas.Bmp.Save($Path, [System.Drawing.Imaging.ImageFormat]::Bmp)
  $Canvas.Bmp.Dispose()
}

function Add-Icon {
  param($G, [int]$Tam, [single]$X, [single]$Y)
  $ic = New-IconBitmap $Tam $true
  $G.DrawImage($ic, $X, $Y, $Tam, $Tam)
  $ic.Dispose()
}

# ---- sidebar.bmp: 164 x 314 ----
$c = New-Canvas 164 314
Add-Icon $c.G 88 38 70
Add-CenteredText $c.G 'LimitGadget' $FontTitulo 21 ([System.Drawing.FontStyle]::Regular) $Texto 0 196 164 30
Add-CenteredText $c.G 'Limites de IA na' $FontTexto 12 ([System.Drawing.FontStyle]::Regular) $TextoSub 0 230 164 18
Add-CenteredText $c.G 'barra de tarefas' $FontTexto 12 ([System.Drawing.FontStyle]::Regular) $TextoSub 0 247 164 18
Save-Bmp $c (Join-Path $OutDir 'sidebar.bmp')

# ---- header.bmp: 150 x 57 ----
$c = New-Canvas 150 57
Add-Icon $c.G 40 10 8.5
Add-CenteredText $c.G 'LimitGadget' $FontTitulo 14 ([System.Drawing.FontStyle]::Regular) $Texto 50 12 98 33
Save-Bmp $c (Join-Path $OutDir 'header.bmp')

Write-Output ("Gerado: " + (Join-Path $OutDir 'sidebar.bmp') + " e " + (Join-Path $OutDir 'header.bmp') + " (fonte: $FontTitulo / $FontTexto)")

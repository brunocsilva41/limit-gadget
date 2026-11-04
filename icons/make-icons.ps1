<#
  Gera todos os ícones do LimitGadget (PNGs, ICO multi-resolução). Só usa System.Drawing.
  Uso: powershell -NoProfile -ExecutionPolicy Bypass -File icons\make-icons.ps1
#>
param([string]$OutDir = $PSScriptRoot)

. (Join-Path $PSScriptRoot 'icon-draw.ps1')

$sizesDir = Join-Path $OutDir 'sizes'
New-Item -ItemType Directory -Force $sizesDir | Out-Null

function Save-Png([int]$size, [string]$path, [bool]$bg = $true) {
  $b = New-IconBitmap $size $bg
  $b.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
  $b.Dispose()
}

# PNGs principais (os que o tauri.conf.json usa)
Save-Png 32  (Join-Path $OutDir '32x32.png')
Save-Png 128 (Join-Path $OutDir '128x128.png')
Save-Png 256 (Join-Path $OutDir '128x128@2x.png')
Save-Png 512 (Join-Path $OutDir 'icon.png')

# Tamanhos auxiliares e versão sem fundo (bandeja)
foreach ($s in 16, 24, 32, 48, 64, 128, 256) { Save-Png $s (Join-Path $sizesDir "$s.png") }
foreach ($s in 16, 24, 32, 48, 64) { Save-Png $s (Join-Path $sizesDir "tray-$s.png") $false }

# ICO multi-resolução com PNG dentro de cada entrada
$icoSizes = 16, 24, 32, 48, 64, 128, 256
$blobs = @()
foreach ($s in $icoSizes) { $blobs += ,([System.IO.File]::ReadAllBytes((Join-Path $sizesDir "$s.png"))) }

$ms = New-Object System.IO.MemoryStream
$bw = New-Object System.IO.BinaryWriter($ms)
$bw.Write([uint16]0); $bw.Write([uint16]1); $bw.Write([uint16]$icoSizes.Count)
$offset = 6 + 16 * $icoSizes.Count
for ($i = 0; $i -lt $icoSizes.Count; $i++) {
  $s = $icoSizes[$i]
  $dim = if ($s -ge 256) { 0 } else { $s }
  $bw.Write([byte]$dim); $bw.Write([byte]$dim); $bw.Write([byte]0); $bw.Write([byte]0)
  $bw.Write([uint16]1); $bw.Write([uint16]32)
  $bw.Write([uint32]$blobs[$i].Length); $bw.Write([uint32]$offset)
  $offset += $blobs[$i].Length
}
foreach ($b in $blobs) { $bw.Write($b) }
$bw.Flush()
[System.IO.File]::WriteAllBytes((Join-Path $OutDir 'icon.ico'), $ms.ToArray())
$bw.Dispose(); $ms.Dispose()

Write-Output "Ícones gerados em $OutDir"

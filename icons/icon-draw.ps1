# Desenho do ícone do LimitGadget (medidor de anel). Usado por make-icons.ps1 e installer\make-assets.ps1.
# Todas as medidas são frações do lado do ícone.
Add-Type -AssemblyName System.Drawing

function New-RoundedRectPath {
  param([single]$X, [single]$Y, [single]$W, [single]$H, [single]$R)
  $p = New-Object System.Drawing.Drawing2D.GraphicsPath
  $d = $R * 2
  $p.AddArc($X, $Y, $d, $d, 180, 90)
  $p.AddArc(($X + $W - $d), $Y, $d, $d, 270, 90)
  $p.AddArc(($X + $W - $d), ($Y + $H - $d), $d, $d, 0, 90)
  $p.AddArc($X, ($Y + $H - $d), $d, $d, 90, 90)
  $p.CloseFigure()
  return $p
}

# Desenha o ícone em um Bitmap ARGB de $Size x $Size (super-amostragem 4x e redução).
# $ComFundo = $false gera a versão só com anel e ponto (bandeja), em tamanho cheio.
function New-IconBitmap {
  param([int]$Size, [bool]$ComFundo = $true)
  $ss = 4
  $S = $Size * $ss
  $big = New-Object System.Drawing.Bitmap($S, $S, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $g = [System.Drawing.Graphics]::FromImage($big)
  $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
  $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
  $g.Clear([System.Drawing.Color]::Transparent)

  if ($ComFundo) {
    $path = New-RoundedRectPath 0 0 $S $S ($S * 0.22)
    $rect = New-Object System.Drawing.RectangleF(0, 0, $S, $S)
    $bg = New-Object System.Drawing.Drawing2D.LinearGradientBrush($rect,
      [System.Drawing.Color]::FromArgb(0x2d, 0x2d, 0x32), [System.Drawing.Color]::FromArgb(0x15, 0x15, 0x18), 90.0)
    $g.FillPath($bg, $path)
    # borda interna de 1 px (no tamanho final), branco a 8%
    $bw = [single]($ss * 1.0)
    $inner = New-RoundedRectPath ($bw / 2) ($bw / 2) ($S - $bw) ($S - $bw) ($S * 0.22 - $bw / 2)
    $pb = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(20, 255, 255, 255), $bw)
    $g.DrawPath($pb, $inner)
    $bg.Dispose(); $pb.Dispose(); $path.Dispose(); $inner.Dispose()
    $rr = 0.30; $sw = 0.11; $dot = 0.07
  } else {
    $rr = 0.385; $sw = 0.14; $dot = 0.09
  }

  $c = $S / 2.0
  $r = $S * $rr
  $w = [single]($S * $sw)
  $ring = New-Object System.Drawing.RectangleF(($c - $r), ($c - $r), (2 * $r), (2 * $r))

  # trilho translúcido
  $trackA = if ($ComFundo) { 41 } else { 70 }
  $pt = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb($trackA, 255, 255, 255), $w)
  $g.DrawEllipse($pt, $ring)

  # arco de 70% (252 graus), começa no topo e segue no sentido horário
  $gr = New-Object System.Drawing.RectangleF(($c - $r - $w / 2), ($c - $r - $w / 2), (2 * $r + $w), (2 * $r + $w))
  $ab = New-Object System.Drawing.Drawing2D.LinearGradientBrush($gr,
    [System.Drawing.Color]::FromArgb(0xff, 0x9f, 0x78), [System.Drawing.Color]::FromArgb(0xe8, 0x80, 0x5c), 90.0)
  $pa = New-Object System.Drawing.Pen($ab, $w)
  $pa.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
  $pa.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
  $g.DrawArc($pa, $ring, -90.0, 252.0)

  # ponto central claro
  $dr = $S * $dot
  $db = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(0xf5, 0xf1, 0xec))
  $g.FillEllipse($db, [single]($c - $dr), [single]($c - $dr), [single](2 * $dr), [single](2 * $dr))
  $pt.Dispose(); $pa.Dispose(); $ab.Dispose(); $db.Dispose(); $g.Dispose()

  if ($ss -eq 1) { return $big }
  $out = New-Object System.Drawing.Bitmap($Size, $Size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $go = [System.Drawing.Graphics]::FromImage($out)
  $go.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
  $go.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
  $go.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceCopy
  $go.DrawImage($big, 0, 0, $Size, $Size)
  $go.Dispose(); $big.Dispose()
  return $out
}

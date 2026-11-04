<#
  Gera as capturas de tela da documentação (docs/screenshots) com DADOS DE EXEMPLO.

  Como funciona:
    - abre um fundo neutro em tela cheia (para as janelas translúcidas ficarem bonitas e sem
      mostrar nada pessoal da sua área de trabalho);
    - roda uma instância do app com LIMITGADGET_DEMO=1 e uma pasta de dados temporária, então
      suas configurações e seu histórico reais não são lidos nem alterados;
    - captura cada modo e cada aba de configurações.

  Uso (com o app já compilado):
    .\scripts\capture-docs.ps1
  Requer Windows, resolução 100% de escala e que o app real esteja fechado (o atalho Ctrl+Alt+L
  e a bandeja são compartilhados).
#>
param(
  [string]$Exe = "",
  [string]$Out = ""
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
if (-not $Exe) { $Exe = Join-Path $root "src-tauri\target\release\limit-gadget.exe" }
if (-not $Out) { $Out = Join-Path $root "docs\screenshots" }
if (-not (Test-Path $Exe)) { throw "Executável não encontrado: $Exe (rode npm run tauri build antes)" }
New-Item -ItemType Directory -Force $Out | Out-Null

Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Drawing, System.Windows.Forms

$demoData = Join-Path $env:TEMP "limitgadget-demo-appdata"
$demoCfgDir = Join-Path $demoData "LimitGadget"
$realAppData = $env:APPDATA

# ---------------------------------------------------------------- fundo neutro em tela cheia
$backdropScript = Join-Path $env:TEMP "limitgadget-backdrop.ps1"
@'
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
Add-Type -Namespace Win -Name Native -MemberDefinition '[System.Runtime.InteropServices.DllImport("user32.dll")] public static extern bool ShowWindow(System.IntPtr h, int c);'
$f = New-Object System.Windows.Forms.Form
$f.Text = 'LimitGadget-Fundo'
$f.FormBorderStyle = 'None'; $f.StartPosition = 'Manual'; $f.Bounds = $b
$f.ShowInTaskbar = $false; $f.BackColor = [System.Drawing.Color]::FromArgb(18, 20, 31)
$f.GetType().GetProperty('DoubleBuffered', [System.Reflection.BindingFlags]'Instance,NonPublic').SetValue($f, $true, $null)
$f.Add_Paint({
  param($s, $e)
  $g = $e.Graphics; $g.SmoothingMode = 'AntiAlias'
  $r = New-Object System.Drawing.Rectangle 0, 0, $s.Width, $s.Height
  $lg = New-Object System.Drawing.Drawing2D.LinearGradientBrush $r, ([System.Drawing.Color]::FromArgb(16, 18, 30)), ([System.Drawing.Color]::FromArgb(44, 24, 66)), 60
  $g.FillRectangle($lg, $r)
  foreach ($glow in @(@(0.82, 0.18, 0.55, 232, 128, 92, 120), @(0.15, 0.85, 0.6, 66, 133, 244, 100), @(0.5, 0.5, 0.45, 120, 90, 200, 55))) {
    $cx = [int]($s.Width * $glow[0]); $cy = [int]($s.Height * $glow[1]); $rad = [int]($s.Width * $glow[2] / 2)
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $path.AddEllipse($cx - $rad, $cy - $rad, 2 * $rad, 2 * $rad)
    $pg = New-Object System.Drawing.Drawing2D.PathGradientBrush $path
    $pg.CenterColor = [System.Drawing.Color]::FromArgb($glow[6], $glow[3], $glow[4], $glow[5])
    $pg.SurroundColors = @([System.Drawing.Color]::FromArgb(0, $glow[3], $glow[4], $glow[5]))
    $g.FillPath($pg, $path)
  }
})
$f.Show(); [void][Win.Native]::ShowWindow($f.Handle, 5)
[System.Windows.Forms.Application]::Run()
'@ | Set-Content $backdropScript -Encoding UTF8

# ---------------------------------------------------------------- utilidades
function Stop-App { Stop-Process -Name limit-gadget -Force -ErrorAction SilentlyContinue; Start-Sleep -Milliseconds 800 }

function Start-Demo($mode, $extraArgs = @()) {
  Stop-App
  New-Item -ItemType Directory -Force $demoCfgDir | Out-Null
  Remove-Item (Join-Path $demoCfgDir "*") -Force -ErrorAction SilentlyContinue
  $cfg = @{ mode = $mode; theme = "dark"; bar_auto = $true; autostart = $false; remember_pos = $false;
            providers = @("claude", "codex", "copilot", "antigravity") } | ConvertTo-Json
  Set-Content (Join-Path $demoCfgDir "settings.json") $cfg -Encoding UTF8
  $env:LIMITGADGET_DEMO = "1"; $env:APPDATA = $demoData
  try { Start-Process $Exe -ArgumentList $extraArgs } finally { $env:APPDATA = $realAppData; Remove-Item Env:\LIMITGADGET_DEMO -ErrorAction SilentlyContinue }
  Start-Sleep -Seconds 11
}

function Find-Window($name) {
  $c = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::NameProperty), $name
  return [System.Windows.Automation.AutomationElement]::RootElement.FindFirst([System.Windows.Automation.TreeScope]::Children, $c)
}

function Rect-Of($el) {
  $r = $el.Current.BoundingRectangle
  return [System.Drawing.Rectangle]::FromLTRB([int]$r.Left, [int]$r.Top, [int]$r.Right, [int]$r.Bottom)
}

function Save-Region([System.Drawing.Rectangle]$rect, $file, [bool]$withTaskbar = $false) {
  if (-not (Find-Window 'LimitGadget-Fundo')) { throw 'Fundo neutro ausente: captura cancelada para não expor a área de trabalho.' }
  $b = if ($withTaskbar) { [System.Windows.Forms.Screen]::PrimaryScreen.Bounds } else { [System.Windows.Forms.Screen]::PrimaryScreen.WorkingArea }
  $rect.Intersect($b)
  Clear-Hover
  $bmp = New-Object System.Drawing.Bitmap $rect.Width, $rect.Height
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($rect.Location, [System.Drawing.Point]::Empty, $rect.Size)
  $bmp.Save((Join-Path $Out $file), [System.Drawing.Imaging.ImageFormat]::Png)
  "{0}: {1}x{2}" -f $file, $rect.Width, $rect.Height
}

# tira o ponteiro e o foco do caminho: sem dicas flutuantes nem anel de foco nas capturas
function Clear-Hover {
  [System.Windows.Forms.Cursor]::Position = New-Object System.Drawing.Point 2, 2
  Start-Sleep -Milliseconds 700
}
function Unfocus($win) { try { $win.SetFocus() } catch {} ; Clear-Hover }

function Pad([System.Drawing.Rectangle]$r, $n) { return [System.Drawing.Rectangle]::FromLTRB($r.Left - $n, $r.Top - $n, $r.Right + $n, $r.Bottom + $n) }

Add-Type -Namespace Win -Name Mouse -MemberDefinition '[System.Runtime.InteropServices.DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y); [System.Runtime.InteropServices.DllImport("user32.dll")] public static extern void mouse_event(int f, int dx, int dy, int d, int e);'
function Click-Element($el) {
  $r = $el.Current.BoundingRectangle
  [void][Win.Mouse]::SetCursorPos([int]($r.Left + $r.Width / 2), [int]($r.Top + $r.Height / 2))
  Start-Sleep -Milliseconds 250
  [Win.Mouse]::mouse_event(2, 0, 0, 0, 0); Start-Sleep -Milliseconds 60; [Win.Mouse]::mouse_event(4, 0, 0, 0, 0)
  Start-Sleep -Milliseconds 900
}

function Find-Button($win, $like) {
  $all = $win.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
  foreach ($el in $all) { if ($el.Current.ControlType.ProgrammaticName -eq 'ControlType.Button' -and $el.Current.Name -like $like) { return $el } }
}
function Expand-Provider($win, $like) {
  $el = Find-Button $win $like; $p = $null
  if ($el -and $el.TryGetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern, [ref]$p)) {
    if ($p.Current.ExpandCollapseState -ne 'Expanded') { Click-Element $el } }
}
function Collapse-Provider($win, $like) {
  $el = Find-Button $win $like; $p = $null
  if ($el -and $el.TryGetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern, [ref]$p)) {
    if ($p.Current.ExpandCollapseState -eq 'Expanded') { Click-Element $el } }
}
function Select-Tab($win, $name) {
  $c = New-Object System.Windows.Automation.PropertyCondition ([System.Windows.Automation.AutomationElement]::NameProperty), $name
  foreach ($el in $win.FindAll([System.Windows.Automation.TreeScope]::Descendants, $c)) {
    if ($el.Current.ControlType.ProgrammaticName -match 'TabItem') { Click-Element $el; return }
  }
}

# ---------------------------------------------------------------- execução
$backdrop = Start-Process powershell -ArgumentList "-NoProfile", "-STA", "-File", $backdropScript -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 2
try {
  # 1) barra sobre a barra de tarefas
  Start-Demo "bar"
  $w = Find-Window "LimitGadget"
  Save-Region (Pad (Rect-Of $w) 4) "barra.png" $true

  # 2) mini
  Start-Demo "mini"
  $w = Find-Window "LimitGadget"
  Save-Region (Pad (Rect-Of $w) 30) "mini.png"

  # 3) normal + configurações (cena principal)
  Start-Demo "normal" @("--settings")
  $g = Find-Window "LimitGadget"
  $s = Find-Window ([string]::Format("Configura{0}{1}es · LimitGadget", [char]0x00E7, [char]0x00F5))
  if (-not $s) { throw "Janela de configurações não encontrada" }
  $rg = Rect-Of $g; $rs = Rect-Of $s
  $tp = $s.GetCurrentPattern([System.Windows.Automation.TransformPattern]::Pattern)
  $tp.Move([double]($rg.Left - $rs.Width - 56), [double]([Math]::Max(24, $rg.Top + ($rg.Height - $rs.Height) / 2)))
  Start-Sleep -Milliseconds 900
  $rs = Rect-Of $s
  Unfocus $g
  $scene = [System.Drawing.Rectangle]::Union($rg, $rs)
  Save-Region (Pad $scene 40) "cena.png"
  Save-Region (Pad $rg 30) "normal-claude.png"

  Collapse-Provider $g "Claude*"; Start-Sleep -Milliseconds 700
  Expand-Provider $g "Copilot*"; Start-Sleep -Milliseconds 1300
  $g = Find-Window "LimitGadget"; Unfocus $g
  Save-Region (Pad (Rect-Of $g) 30) "normal-copilot.png"

  Collapse-Provider $g "Copilot*"; Expand-Provider $g "ChatGPT*"; Start-Sleep -Milliseconds 1300
  $g = Find-Window "LimitGadget"; Unfocus $g
  Save-Region (Pad (Rect-Of $g) 30) "normal-chatgpt.png"

  $tabs = @(@("Aparência", "config-aparencia.png"), @("Provedores", "config-provedores.png"), @("Barra", "config-barra.png"), @("Avisos", "config-avisos.png"), @("Sistema", "config-sistema.png"))
  foreach ($t in $tabs) {
    $name = $t[0] -replace "Aparência", ("Apar" + [char]0x00EA + "ncia")
    Select-Tab $s $name; Start-Sleep -Milliseconds 1100; Unfocus $s
    Save-Region (Pad (Rect-Of $s) 24) $t[1]
  }
}
finally {
  Stop-App
  if ($backdrop -and -not $backdrop.HasExited) { Stop-Process -Id $backdrop.Id -Force }
  Remove-Item -Recurse -Force $demoData -ErrorAction SilentlyContinue
  $env:APPDATA = $realAppData
}
"Capturas em $Out"

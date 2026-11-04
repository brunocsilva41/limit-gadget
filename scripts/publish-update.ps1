<#
  Gera uma versão assinada do LimitGadget e o latest.json que o atualizador do app lê.

  Uso:
    .\scripts\publish-update.ps1 -BaseUrl "https://exemplo.com/limitgadget" -Notes "O que mudou"

  Resultado (pasta dist-update\):
    LimitGadget_<versão>_x64-setup.exe      instalador
    LimitGadget_<versão>_x64-setup.exe.sig  assinatura do atualizador
    latest.json                             manifesto (envie junto com o instalador)

  Publique os três arquivos em -BaseUrl e cole "<BaseUrl>/latest.json" em
  Configurações > Atualizações > Endereço das atualizações.

  A chave privada fica FORA do projeto (~\.tauri\limitgadget.key). Não a envie a ninguém
  e faça backup: sem ela, as versões já instaladas não aceitam atualização nova.
#>
param(
  [Parameter(Mandatory = $true)][string]$BaseUrl,
  [string]$Notes = "",
  [string]$KeyPath = "$env:USERPROFILE\.tauri\limitgadget.key"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

if (-not (Test-Path $KeyPath)) { throw "Chave privada não encontrada em $KeyPath" }

$conf = Get-Content "src-tauri\tauri.conf.json" -Raw | ConvertFrom-Json
$version = $conf.version

$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content $KeyPath -Raw)
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
if (-not $env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR = Join-Path $root "src-tauri\target" }

npm run build
npx tauri build
if ($LASTEXITCODE -ne 0) { throw "Falha no build" }

$nsis = Join-Path $env:CARGO_TARGET_DIR "release\bundle\nsis"
$setup = "LimitGadget_${version}_x64-setup.exe"
$sig = Get-Content (Join-Path $nsis "$setup.sig") -Raw

$out = Join-Path $root "dist-update"
New-Item -ItemType Directory -Force $out | Out-Null
Copy-Item (Join-Path $nsis $setup) $out -Force
Copy-Item (Join-Path $nsis "$setup.sig") $out -Force

$manifest = [ordered]@{
  version   = $version
  notes     = $Notes
  pub_date  = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
  platforms = [ordered]@{
    "windows-x86_64" = [ordered]@{
      signature = $sig.Trim()
      url       = ($BaseUrl.TrimEnd('/') + "/" + $setup)
    }
  }
}
$manifest | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $out "latest.json") -Encoding UTF8

Write-Host "Pronto: $out"
Write-Host "Envie os 3 arquivos para $BaseUrl"

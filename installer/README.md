# Instalador do LimitGadget

Instalador NSIS gerado pelo Tauri (`npx tauri build`). A configuração fica em `bundle.windows.nsis`, em `src-tauri/tauri.conf.json`. O modelo do Tauri não é substituído: só há imagens, textos, o idioma e o arquivo de hooks.

Instalação por usuário (sem administrador), em pt-BR.

## Arquivos

- `make-assets.ps1`: gera `header.bmp` (150x57) e `sidebar.bmp` (164x314), 24 bits.
- `header.bmp`, `sidebar.bmp`: imagens já geradas. Ficam versionadas.
- `hooks.nsh`: textos das telas e ações extras.
- `PortugueseBR.nsh`: textos em pt-BR do Tauri (cópia ajustada) e remoção do texto padrão do NSIS no rodapé das janelas (`LangString ^Branding`).

## Regenerar as imagens

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File installer\make-assets.ps1
```

Usa só o System.Drawing. Usa Segoe UI Variable se existir e, se não, Segoe UI Semibold.

## Telas

1. Boas-vindas, com a opção "Iniciar com o Windows".
2. Pasta de instalação.
3. Progresso.
4. Conclusão, com "Abrir o LimitGadget" (marcada) e "Criar atalho na área de trabalho" (desmarcada, por `MUI_FINISHPAGE_SHOWREADME_NOTCHECKED` em `hooks.nsh`).

Se já houver uma versão instalada, o Tauri mostra antes uma tela de reinstalação.

## O que cada hook faz

- Boas-vindas: acrescenta a caixa "Iniciar com o Windows". Vem marcada se o valor já existir.
- `NSIS_HOOK_POSTINSTALL`: se a caixa foi marcada, grava `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, valor `LimitGadget`, com o caminho do exe entre aspas.
- `NSIS_HOOK_POSTUNINSTALL` (fora de atualização): remove o valor `Run` e a chave `HKCU\Software\bruno\LimitGadget`. Se a caixa de apagar dados da tela de desinstalação estiver marcada, apaga `%APPDATA%\LimitGadget`. O padrão é manter.

Modo silencioso (`/S`), passivo (`/P`) e atualização (`/UPDATE`) não mostram telas. Nesses modos nada é alterado em "Iniciar com o Windows" e nenhum dado é apagado. O atalho na área de trabalho e o relançamento do app seguem o comportamento padrão do Tauri (`/NS` evita o atalho, `/R` reabre o app).

## Como testar

Não compile dentro da pasta do projeto. Copie para uma pasta de trabalho e use outro `CARGO_TARGET_DIR`:

```powershell
$env:CARGO_TARGET_DIR = 'C:\lgtmp\installer-target'
$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content "$env:USERPROFILE\.tauri\limitgadget.key" -Raw)
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ''
npx vite build
npx tauri build
```

O instalador sai em `bundle\nsis\LimitGadget_<versão>_x64-setup.exe`, dentro do diretório de destino.

Teste silencioso, em pasta temporária:

```powershell
.\LimitGadget_<versão>_x64-setup.exe /S /D=C:\lgtmp\inst-test
C:\lgtmp\inst-test\uninstall.exe /S
```

Depois confira que não sobraram atalhos, chaves do registro nem processos. Atenção: o instalador usa as mesmas chaves e atalhos de uma instalação real. Faça backup antes, se houver uma.

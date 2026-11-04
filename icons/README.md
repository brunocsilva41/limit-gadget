# Ícones do LimitGadget

Tudo é gerado por script, só com PowerShell e System.Drawing (sem dependências).

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File icons\make-icons.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File installer\make-assets.ps1
```

- `icon-draw.ps1`: o desenho (função `New-IconBitmap`), usado pelos dois scripts.
- `make-icons.ps1`: gera `32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.png`, `sizes/` (16 a 256, e `tray-*` sem fundo) e o `icon.ico` multi-resolução (16, 24, 32, 48, 64, 128, 256, PNG dentro de cada entrada).
- `icon.svg`: versão vetorial escrita à mão, com as mesmas proporções.
- `installer/make-assets.ps1`: gera `header.bmp` e `sidebar.bmp` usando o mesmo ícone.
- `icon.icns` (macOS) não é regenerado.

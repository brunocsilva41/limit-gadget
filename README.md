# LimitGadget

Um gadget leve para a barra de tarefas do Windows que mostra seus limites de uso nas ferramentas de IA:

- **Claude Code** — cota semanal via `api.anthropic.com/api/oauth/usage`
- **ChatGPT / Codex** — `chatgpt.com/backend-api/wham/usage`
- **Antigravity** — lido localmente quando a IDE está aberta

## Requisitos

- **Rust 1.80+** (`rustup update`)
- **Node.js 20+** e **npm** (`nvm` recomendado)
- **WebView2 Runtime** (já incluso no Windows 10/11 moderno)
- **Tauri CLI**: `cargo install tauri-cli --version "^2"`

## Configuração inicial

```bash
cd limit-gadget
npm install
```

### Tokens de ambiente (não comite!)

Crie um arquivo `.env` na raiz ou defina as variáveis no sistema:

```bash
# .env (NÃO commitar)
CLAUDE_CODE_TOKEN=seu_access_token_aqui
CHATGPT_TOKEN=seu_access_token_aqui
CHATGPT_ACCOUNT_ID=seu_account_id_aqui
```

> **Como obter os tokens:**
> - **Claude Code**: `~/.claude/.credentials.json` → `claudeAiOauth.accessToken`
> - **ChatGPT**: `~/.codex/auth.json` → `tokens.access_token` e `tokens.account_id`

O app **não salva tokens** no código — eles ficam apenas nas variáveis de ambiente do processo.

## Desenvolvimento

```bash
npm run tauri:dev
```

Isso abre a janela do painel + ícone na bandeja. O loop de 60 s roda em background.

## Build de produção

```bash
npm run tauri:build
```

Gera um `.msi` e `.exe` em `src-tauri/target/release/bundle/`.

## Releases e atualizações

O app se atualiza pelas releases do GitHub: o plugin updater do Tauri lê
`https://github.com/<dono>/<repo>/releases/latest/download/latest.json`.

### Como publicar

1. Atualize a versão (X.Y.Z) nos 3 arquivos: `package.json`, `src-tauri/Cargo.toml` e `src-tauri/tauri.conf.json`.
2. Em `CHANGELOG.md`, mova o conteúdo de `[Unreleased]` para uma seção `[X.Y.Z]`.
3. Faça merge na `main` e crie a tag na `main`: `git tag vX.Y.Z` e `git push origin vX.Y.Z`.

A tag dispara `.github/workflows/release.yml`, que:

1. Roda todo o CI (consistência, frontend, Rust, segurança e pacote). Se qualquer etapa falhar, nada é publicado.
2. Confirma que o commit da tag está na `main` e que a tag bate com a versão.
3. Extrai as notas da seção do `CHANGELOG.md` (falha se não existir).
4. Compila, assina e publica a Release com o instalador NSIS, o `.sig` e o `latest.json`.

Comandos úteis: `npm run check:versions`, `npm run typecheck`, `npm test`.

### Segredos necessários

Em GitHub > Settings > Secrets and variables > Actions > New repository secret:

- `TAURI_SIGNING_PRIVATE_KEY`: conteúdo do arquivo da chave privada do updater.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: senha da chave (se a chave não tem senha, não cadastre este segredo).

### Proteção de branch

Recomendado: em Settings > Branches, proteja a `main` exigindo pull request e as etapas do CI
(Consistência, Frontend, Rust, Segurança e Pacote) como checagens obrigatórias.

## Estrutura

```
limit-gadget/
├── src/                    # Frontend React + Tailwind
│   ├── App.tsx             # Painel compacto (280×170)
│   ├── main.tsx
│   ├── index.css
│   ├── adapters/
│   │   ├── api.ts          # HTTP fetchers (front-end helpers)
│   │   └── storage.ts      # Cache local
│   └── components/
│       └── Icons.tsx       # SVGs inline
├── src-tauri/              # Backend Rust
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   └── src/
│       ├── lib.rs          # Adapters + comandos Tauri + loop 60s
│       └── main.rs
├── icons/                  # Ícones multi-res (PNG + SVG)
├── package.json
├── vite.config.ts
├── tailwind.config.cjs
└── tsconfig.json
```

## Comportamento

- **Ícone na bandeja**: anel colorido (verde/âmbar/vermelho) = pior % entre as 3 fontes
- **Clique no ícone**: abre/fecha o painel **colado à barra de tarefas**, acima do relógio
- **Painel**: 3 linhas — ícone minúsculo + nome + % grande + barra fina + "reseta em Xh Ym"
- **Atualização**: a cada 60 s (paralelo, timeout 10 s, backoff exponencial em falha)
- **Notificação nativa**: dispara uma vez ao atingir **≥ 90%**, re-arma ao cair **< 85%** ou no reset
- **Cache**: último snapshot salvo em `%APPDATA%\LimitGadget\store.json` (sobrevive a reinício)
- **Consumo**: ~30–40 MB RAM, ~0% CPU ocioso, ~1 MB rede/dia

## Roadmap

- [ ] Adapter Antigravity completo (CSRF → language_server local)
- [ ] Ícone animado (SVG dinâmico via `tauri::tray`)
- [ ] Modo compacto (só 3 argolas no painel)
- [ ] Auto-start no Windows (entrada em `Run`)
- [ ] Tema claro/escuro automático

## Licença

MIT
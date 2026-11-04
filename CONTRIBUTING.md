# Contribuindo com o LimitGadget

Obrigado pelo interesse! Este guia explica como rodar o projeto, o que a CI verifica e como uma release é publicada.

## Antes de começar

Procure nas [issues](https://github.com/brunocsilva41/limit-gadget/issues) se o assunto já existe. Para mudanças grandes, abra uma issue primeiro, para alinharmos a ideia antes do trabalho.

**Nunca inclua em issues, PRs ou capturas de tela:** tokens, o conteúdo de `~/.claude`, `~/.codex` ou `~/.copilot`, nomes de projetos ou títulos de conversas reais. Para capturas de tela, use o modo demonstração (`LIMITGADGET_DEMO=1`, veja o README).

## Rodando localmente

Pré-requisitos: Rust estável, Node.js 20.19 ou mais e o WebView2 Runtime.

```bash
npm ci
npm run tauri:dev
```

## O que a CI verifica

Toda PR e todo push na `main` passam por cinco etapas, em ordem:

1. **Consistência:** versão igual em `package.json`, `Cargo.toml` e `tauri.conf.json`, e nenhuma expressão proibida nos arquivos.
2. **Frontend:** `npm run typecheck`, `npm test` e `npm run build`.
3. **Rust:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` e `cargo test`.
4. **Segurança:** `npm audit` e `cargo audit`.
5. **Pacote:** compila o instalador completo.

Rode o equivalente antes de abrir a PR:

```bash
npm run check:versions && npm run typecheck && npm test
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

## Convenções

- **Commits** no formato [Conventional Commits](https://www.conventionalcommits.org/pt-br/), em português: `feat(ui): ...`, `fix(backend): ...`, `docs: ...`, `ci: ...`, `build(deps): ...`.
- **Uma PR, um assunto.** Commits pequenos e com mensagens que expliquem o porquê.
- **CHANGELOG:** toda mudança visível ao usuário entra em `[Unreleased]` no [CHANGELOG.md](CHANGELOG.md).
- **Testes:** correções e funções novas vêm com teste quando fizer sentido.
- **Texto da interface** em português, curto e sem jargão.

## Publicar uma release

Só quem mantém o repositório publica. Uma release nasce de uma tag na `main`:

1. Atualize a versão (X.Y.Z) nos três arquivos: `package.json`, `src-tauri/Cargo.toml` e `src-tauri/tauri.conf.json` (e rode `cargo check` para atualizar o `Cargo.lock`).
2. No `CHANGELOG.md`, mova o conteúdo de `[Unreleased]` para uma seção `[X.Y.Z] - AAAA-MM-DD`.
3. Faça a PR, espere a CI passar e mescle na `main`.
4. Crie e envie a tag: `git tag -a vX.Y.Z -m "LimitGadget X.Y.Z"` e `git push origin vX.Y.Z`.

A tag dispara o workflow de release, que roda **toda** a CI de novo. Só se tudo passar ele confirma que o commit está na `main`, extrai as notas do CHANGELOG, compila, assina e publica a release com o instalador, a assinatura e o `latest.json` que o atualizador do app lê.

### Segredos do repositório

Em Settings > Secrets and variables > Actions:

- `TAURI_SIGNING_PRIVATE_KEY`: conteúdo da chave privada do atualizador. Guarde um backup fora do repositório: sem ela, as versões já instaladas não aceitam atualização nova.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: só se a chave tiver senha.

### Proteção de branch (recomendado)

Em Settings > Branches, proteja a `main` exigindo pull request e as cinco etapas da CI como checagens obrigatórias.

# Política de segurança

## Versões com suporte

Recebe correções de segurança a **última versão publicada**. O app se atualiza sozinho, então manter-se nela é o suficiente.

## Como reportar uma vulnerabilidade

**Não abra uma issue pública.** Use o relato privado do GitHub: em [Security > Report a vulnerability](https://github.com/brunocsilva41/limit-gadget/security/advisories/new). Descreva o problema, como reproduzi-lo e o impacto que você enxerga. Você receberá uma resposta assim que possível.

## O que o app faz com credenciais

Isso ajuda a avaliar um relato:

- O LimitGadget **lê** os arquivos de login que você já tem (Claude Code, Codex e GitHub CLI) e usa os tokens apenas para consultar a **API do próprio provedor**.
- Ele **não envia** tokens, nem dados de uso, para nenhum outro servidor e não tem telemetria.
- Ele guarda em disco somente configurações, histórico de uso e o último valor lido, em `%APPDATA%\LimitGadget`.
- As atualizações são baixadas das releases deste repositório e só são instaladas se a assinatura bater com a chave pública embutida no app.

## Fora do escopo

- Avisos do SmartScreen sobre o instalador (ele ainda não tem certificado de assinatura de código).
- Problemas que exijam acesso prévio e total à conta do Windows do usuário.

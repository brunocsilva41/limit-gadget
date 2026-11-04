# Changelog

Todas as mudanças relevantes deste projeto são registradas aqui.
O formato segue o [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/) e o projeto usa [Versionamento Semântico](https://semver.org/lang/pt-BR/).

## [Unreleased]

### Alterado

- Dependências atualizadas: React 19, Vite 8, TypeScript 7, vitest 5 e windows-sys 0.61.
- Ações do GitHub atualizadas, e a publicação usa o tauri-action v1 (`uploadUpdaterJson`).
- O Dependabot agrupa React e Vite com suas peças relacionadas, para não abrir PRs que quebram a instalação.

### Removido

- Dependências sem uso (plugins JS do Tauri, Tailwind, PostCSS e Autoprefixer).

## [0.5.0] - 2026-11-04

### Adicionado

- Anel de porcentagem em todos os lugares que mostram uso: o arco em volta do número indica quanto do limite já foi consumido.
- Uso do Claude Code por projeto nos últimos 7 dias, lido dos logs locais.
- Meta de consumo por dia com aviso, e "hoje +X%" nos limites de semana e mês.
- Resumo semanal na segunda-feira com o pico de cada provedor.
- Escolha do monitor onde o gadget e a barra aparecem.
- Ícones sem cor (opcional).
- Backup e restauração das configurações e do histórico.
- Barra compacta com setas para alternar entre os provedores.
- Configurações em abas por categoria, com janela translúcida e animações.
- Endereço de atualização padrão apontando para as releases do GitHub.
- Últimas sessões dos 7 dias do ChatGPT/Codex (projeto e mensagens) e do Copilot (nome, projeto e tokens), em seções que abrem e fecham.
- Novo ícone do aplicativo, do instalador e da bandeja (anel de progresso com pontas arredondadas).

### Alterado

- O modo Mini agora é uma lista compacta; só a Barra continua em formato longo.
- O X do gadget passa a manter os limites na barra de tarefas em vez de encerrar o app.

### Corrigido

- O menu de um provedor expandido não fechava no modo flutuante: ele piscava e reabria sozinho.
- No Mini, os botões ficavam sobre o limite do primeiro provedor; agora têm um cabeçalho próprio.
- Na Barra completa, o botão de configurações era cortado; a largura agora comporta todos os provedores.
- O ícone de configurações parecia o de alternar tema; agora é uma engrenagem.
- A janela de configurações abria minimizada ou atrás de outras janelas.
- Projetos do Claude Code com o mesmo nome apareciam duplicados.

## [0.4.0]

### Adicionado

- Três modos de exibição: Normal, Mini e Barra.
- Provedores Claude, ChatGPT/Codex, GitHub Copilot e Antigravity (este mostra a última leitura quando a IDE está fechada).
- Avisos de limite configuráveis (aviso e crítico) com notificações.
- Previsão de esgotamento de cada limite.
- Histórico de uso por provedor.
- Atalho global Ctrl+Alt+L para mostrar e esconder o gadget.
- Barra fixa ou solta, com pino.
- Cor da barra acompanhando a da barra de tarefas.
- Atualizador automático pelas releases do GitHub.

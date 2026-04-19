---
description: >
  Mostra o progresso atual do port com estatisticas.
allowed-tools: Read, Glob, Grep, Bash
---

Levante o status atual do port:

1. Contar quantos modulos foram analisados (docs/modules/*.md)
2. Contar quantos modulos foram portados (src/**/mod.rs)
3. Rodar `cargo test` e reportar resultados
4. Rodar `wc -l src/**/*.rs` para contar linhas de codigo Rust
5. Ler CLAUDE.md e mostrar o checklist de fases
6. Listar proximos modulos a portar por ordem de dependencia

Apresentar um resumo visual com porcentagens de progresso.

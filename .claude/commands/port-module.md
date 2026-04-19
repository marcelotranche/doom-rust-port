---
description: >
  Porta um modulo C do DOOM para Rust. Requer que a analise
  ja tenha sido feita. Uso: /port-module wad
allowed-tools: Read, Glob, Grep, Write, Edit, Bash, Agent
---

Porte o modulo "$ARGUMENTS" de C para Rust.

## Workflow

1. **Verificar pre-requisitos**
   - Confirmar que `docs/modules/$ARGUMENTS.md` existe (analise feita)
   - Se nao existir, rodar /analyze-module primeiro

2. **Projetar** — Use @rust-architect para:
   - Ler a analise do modulo
   - Projetar structs, enums, traits e API Rust
   - Produzir codigo-esqueleto em `src/$ARGUMENTS/mod.rs`

3. **Implementar**
   - Preencher a implementacao completa
   - Seguir as convencoes da skill `didactic-code`
   - Adicionar testes unitarios
   - Cada funcao portada deve referenciar a original em C

4. **Validar**
   - Rodar `cargo clippy -- -D warnings`
   - Rodar `cargo test`
   - Corrigir qualquer warning ou erro

5. **Review** — Use @rust-reviewer para:
   - Verificar qualidade do codigo
   - Verificar valor didatico
   - Aplicar sugestoes de melhoria

6. **Atualizar progresso**
   - Marcar modulo como concluido no CLAUDE.md

---
description: >
  Escreve ou atualiza documentacao de um modulo do DOOM.
  Uso: /doc-module wad
allowed-tools: Read, Glob, Grep, Write, Edit, Agent
---

Documente o modulo "$ARGUMENTS" do port DOOM-Rust.

1. Use o subagente @doc-writer para:
   - Ler o codigo Rust em `src/$ARGUMENTS/`
   - Ler a analise C em `docs/modules/$ARGUMENTS.md` (se existir)
   - Produzir/atualizar documentacao completa

2. A documentacao deve cobrir:
   - O que o modulo faz no contexto do engine DOOM
   - Como foi portado de C para Rust
   - Decisoes de design e trade-offs
   - Diagramas ASCII quando relevante
   - Referencias cruzadas com outros modulos

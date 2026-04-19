---
description: >
  Analisa um modulo C do DOOM original e produz documentacao.
  Uso: /analyze-module wad (analisa w_wad.c/h)
allowed-tools: Read, Glob, Grep, Bash, Agent
---

Analise o modulo "$ARGUMENTS" do DOOM original.

1. Use o subagente @c-analyst para:
   - Localizar os arquivos .c e .h correspondentes em
     `references/DOOM-master/linuxdoom-1.10/` e `references/chocolate-doom-master/src/`
   - Produzir um relatorio completo em `docs/modules/$ARGUMENTS.md`

2. O relatorio deve incluir:
   - Proposito do modulo no engine
   - Structs e tipos definidos
   - Funcoes publicas e internas
   - Variaveis globais
   - Dependencias com outros modulos
   - Complexidade estimada do port

3. Apos o relatorio, resumir os pontos principais aqui na conversa.

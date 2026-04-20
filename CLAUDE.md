# DOOM Rust Port — Contexto do Projeto

## O Que Este Projeto
Port educacional do engine DOOM (1993) de C para Rust idiomatico.
O objetivo e produzir codigo didatico que ensine tanto a arquitetura
do DOOM quanto a linguagem Rust simultaneamente.

## Fontes de Referencia
- `references/DOOM-master/` — Linuxdoom 1.10 original (C). Fonte primaria.
  - Codigo-fonte em `references/DOOM-master/linuxdoom-1.10/`
- `references/chocolate-doom-master/` — Chocolate Doom. Consultar para
  entender decisoes de portabilidade e correcoes de bugs.
  - Codigo-fonte em `references/chocolate-doom-master/src/`
- `references/freedoom-master/` — Assets livres. Usar para testes.

## Convencoes de Codigo

### Rust
- Rust edition 2021, MSRV 1.75+
- Usar `cargo clippy` antes de cada commit
- Preferir tipos seguros: sem `unsafe` exceto quando justificado
  com comentario `// SAFETY: <razao>`
- Nomes em ingles para codigo, comentarios em portugues (BR)
- Cada funcao publica tem docstring `///` em portugues
- Modulos seguem a separacao do DOOM original quando possivel

### Estilo Didatico
- Cada arquivo comeca com um bloco `//!` explicando o que o modulo
  faz no contexto do engine DOOM
- Structs importantes tem comentario sobre o equivalente em C
- Algoritmos complexos (BSP traversal, rendering) tem comentarios
  passo-a-passo referenciando o codigo C original
- Usar `todo!()` com descricao para funcionalidades pendentes

### Mapeamento C -> Rust
| Conceito C (DOOM)         | Equivalente Rust            |
|---------------------------|-----------------------------|
| `fixed_t` (16.16)        | `struct Fixed(i32)`         |
| `void*` generico          | `enum` ou trait objects     |
| Arrays globais            | Structs com ownership claro |
| `thinker_t` linked list   | `Vec<Box<dyn Thinker>>`    |
| `#define` constantes      | `const` ou `enum`           |
| `typedef struct`          | `struct` com `impl`         |

### Dependencias Permitidas
- `sdl2` — janela, input, audio
- `bitflags` — flags de linedefs/sectors
- `byteorder` — leitura de WAD (little-endian)
- `thiserror` — erros tipados
- `log` + `env_logger` — logging

### Comandos
- `cargo build` — compilar
- `cargo test` — rodar testes
- `cargo clippy` — lint
- `cargo run -- --iwad assets/freedoom.wad` — executar

### Git Workflow (OneFlow)
Baseado no modelo OneFlow: uma unica branch principal (`main`) com
branches de feature/fase criadas a partir dela.

- Branch principal: `main` — unica branch de longa duracao
- Cada fase do port tem sua propria branch: `fase-N-descricao`
  (ex: `fase-2-matematica-tabelas`)
- Branches de fase sao criadas a partir de `main` e merged de volta
- Ao concluir uma fase, criar PR via `gh pr create --base main`
- NAO fazer merge automaticamente — aguardar aprovacao do usuario
- So iniciar a proxima fase apos o PR ser aprovado/merged
- Commits na branch devem passar em `cargo clippy` e `cargo test`
- Sem branches develop, release ou hotfix — simplicidade do OneFlow

## Progresso do Port
<!-- Claude atualiza esta secao automaticamente -->
- [x] Fase 0: Setup e analise
- [x] Fase 1: WAD loader
- [x] Fase 2: Matematica e tipos base
- [x] Fase 3: Map loader (BSP)
- [x] Fase 4: Renderer
- [x] Fase 5: Game loop e input
- [x] Fase 6: Coisas e colisao
- [x] Fase 7: Audio
- [x] Fase 8: Menus e HUD
- [x] Fase 9: Networking
- [x] Fase 10: Polish e freedoom jogavel
- [ ] Fase 11: Integracao SDL2 — janela, rendering e input

## Compact Instructions
Ao compactar a conversa, preservar:
- Estado atual da fase de port
- Lista de modulos ja portados e pendentes
- Decisoes arquiteturais tomadas (especialmente desvios do C)
- Bugs conhecidos e workarounds

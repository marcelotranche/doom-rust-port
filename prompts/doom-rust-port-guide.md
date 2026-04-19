# 🔥 DOOM em Rust — Guia Completo de Port com Claude Code

## Visão Geral do Projeto

**Objetivo:** Portar o engine do DOOM (1993, id Software) de C para Rust idiomático, usando Claude Code como ferramenta principal de desenvolvimento. O projeto é concebido como **material didático** para aprendizado simultâneo de arquitetura de game engines clássicos e da linguagem Rust.

**Fontes de referência disponíveis em `references/`:**
- `doom/` — Código-fonte original do DOOM (Linuxdoom, GPL)
- `freedoom/` — Assets livres compatíveis com o engine DOOM
- `chocolate_doom/` — Port fiel ao original com melhorias de portabilidade

**Resultado esperado:** Um executável Rust capaz de carregar WADs do Freedoom e rodar o jogo completo, com código extensivamente comentado em português, servindo como guia de estudo.

---

## Parte 1 — Estrutura do Projeto Claude Code

### 1.1 Estrutura de Diretórios

```
doom-rust/
├── CLAUDE.md                          # Memória do projeto
├── .claude/
│   ├── settings.json                  # Hooks e permissões
│   ├── agents/
│   │   ├── c-analyst.md               # Agente de análise de código C
│   │   ├── rust-architect.md          # Agente de arquitetura Rust
│   │   ├── rust-reviewer.md           # Agente de code review
│   │   └── doc-writer.md              # Agente de documentação
│   ├── skills/
│   │   ├── doom-conventions/
│   │   │   └── SKILL.md               # Convenções do engine DOOM
│   │   ├── rust-patterns/
│   │   │   └── SKILL.md               # Padrões Rust para o port
│   │   └── didactic-code/
│   │       └── SKILL.md               # Regras de código didático
│   └── commands/
│       ├── analyze-module.md          # /analyze-module <nome>
│       ├── port-module.md             # /port-module <nome>
│       ├── doc-module.md              # /doc-module <nome>
│       └── progress.md                # /progress
├── references/
│   ├── doom/                          # Linuxdoom original
│   ├── freedoom/                      # Assets livres
│   └── chocolate_doom/                # Port de referência
├── docs/
│   ├── architecture.md                # Arquitetura geral do engine
│   ├── modules/                       # Documentação por módulo
│   │   ├── wad.md
│   │   ├── renderer.md
│   │   ├── game-logic.md
│   │   └── ...
│   └── glossary.md                    # Glossário de termos DOOM/Rust
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── wad/                           # Carregamento de WAD files
│   ├── renderer/                      # Renderização por software
│   ├── map/                           # BSP, geometry, sectors
│   ├── game/                          # Game loop, thinkers, player
│   ├── sound/                         # Subsistema de áudio
│   ├── net/                           # Networking (ipx/udp)
│   ├── menu/                          # Menus e HUD
│   ├── video/                         # Framebuffer e paletas
│   └── utils/                         # Fixed-point math, tabelas
├── assets/
│   └── freedoom.wad                   # WAD do Freedoom para testes
├── tests/
│   ├── wad_tests.rs
│   ├── map_tests.rs
│   └── fixed_point_tests.rs
└── Cargo.toml
```

---

### 1.2 CLAUDE.md — Memória do Projeto

Criar o arquivo `CLAUDE.md` na raiz do projeto com o seguinte conteúdo:

```markdown
# DOOM Rust Port — Contexto do Projeto

## O Que É Este Projeto
Port educacional do engine DOOM (1993) de C para Rust idiomático.
O objetivo é produzir código didático que ensine tanto a arquitetura
do DOOM quanto a linguagem Rust simultaneamente.

## Fontes de Referência
- `references/DOOM-master/` — Linuxdoom original (C). Fonte primária.
- `references/chocolate-doom-master/` — Chocolate Doom. Consultar para
  entender decisões de portabilidade e correções de bugs.
- `references/freedoom-master/` — Assets livres. Usar para testes.

## Convenções de Código

### Rust
- Rust edition 2021, MSRV 1.75+
- Usar `cargo clippy` antes de cada commit
- Preferir tipos seguros: sem `unsafe` exceto quando justificado
  com comentário `// SAFETY: <razão>`
- Nomes em inglês para código, comentários em português (BR)
- Cada função pública tem docstring `///` em português
- Módulos seguem a separação do DOOM original quando possível

### Estilo Didático
- Cada arquivo começa com um bloco `//!` explicando o que o módulo
  faz no contexto do engine DOOM
- Structs importantes têm comentário sobre o equivalente em C
- Algoritmos complexos (BSP traversal, rendering) têm comentários
  passo-a-passo referenciando o código C original
- Usar `todo!()` com descrição para funcionalidades pendentes

### Mapeamento C → Rust
| Conceito C (DOOM)         | Equivalente Rust            |
|---------------------------|-----------------------------|
| `fixed_t` (16.16)        | `struct Fixed(i32)`         |
| `void*` genérico          | `enum` ou trait objects     |
| Arrays globais            | Structs com ownership claro |
| `thinker_t` linked list   | `Vec<Box<dyn Thinker>>`    |
| `#define` constantes      | `const` ou `enum`           |
| `typedef struct`          | `struct` com `impl`         |

### Dependências Permitidas
- `sdl2` — janela, input, áudio
- `bitflags` — flags de linedefs/sectors
- `byteorder` — leitura de WAD (little-endian)
- `thiserror` — erros tipados
- `log` + `env_logger` — logging

### Comandos
- `cargo build` — compilar
- `cargo test` — rodar testes
- `cargo clippy` — lint
- `cargo run -- --iwad assets/freedoom.wad` — executar

## Progresso do Port
<!-- Claude atualiza esta seção automaticamente -->
- [x] Fase 0: Setup e análise
- [ ] Fase 1: WAD loader
- [ ] Fase 2: Matemática e tipos base
- [ ] Fase 3: Map loader (BSP)
- [ ] Fase 4: Renderer
- [ ] Fase 5: Game loop e input
- [ ] Fase 6: Coisas e colisão
- [ ] Fase 7: Áudio
- [ ] Fase 8: Menus e HUD
- [ ] Fase 9: Networking
- [ ] Fase 10: Polish e freedoom jogável

## Compact Instructions
Ao compactar a conversa, preservar:
- Estado atual da fase de port
- Lista de módulos já portados e pendentes
- Decisões arquiteturais tomadas (especialmente desvios do C)
- Bugs conhecidos e workarounds
```

---

### 1.3 Subagentes (`.claude/agents/`)

#### `c-analyst.md` — Analista de Código C

```markdown
---
name: c-analyst
description: >
  Analisa código C do DOOM original e Chocolate Doom. Use quando
  precisar entender um módulo C, mapear dependências entre arquivos,
  identificar variáveis globais, ou documentar o fluxo de um
  subsistema. Trigger: análise de código C, entendimento de módulo,
  mapeamento de dependências, "o que faz este arquivo C".
tools: Read, Glob, Grep, Bash
model: sonnet
maxTurns: 30
---

Você é um especialista em C e em arquitetura do engine DOOM (1993).

## Sua Função
Analisar código-fonte C do DOOM para preparar o port para Rust.
Você trabalha exclusivamente com leitura — nunca modifica arquivos.

## Como Analisar um Módulo
1. Ler o arquivo .c e seu .h correspondente
2. Identificar:
   - Structs e typedefs definidos
   - Variáveis globais (extern e static)
   - Funções públicas vs internas (static)
   - Dependências: quais outros módulos este inclui
   - Macros e #defines relevantes
3. Produzir um relatório em `docs/modules/<nome>.md` com:
   - Propósito do módulo no engine
   - Lista de structs com campos explicados
   - Lista de funções com assinatura e propósito
   - Dependências (quais .h inclui)
   - Estado global que mantém
   - Complexidade estimada do port (baixa/média/alta)
   - Sugestões de como mapear para Rust idiomático

## Referências
- Código original: `references/DOOM-master/linuxdoom-1.10/`
- Chocolate Doom: `references/chocolate-doom-master/src/`
- Comparar ambos quando houver diferenças relevantes

## Formato de Saída
Sempre produzir um arquivo .md em `docs/modules/` com o relatório.
Ser conciso mas completo. Priorizar informação útil para o port.
```

#### `rust-architect.md` — Arquiteto Rust

```markdown
---
name: rust-architect
description: >
  Projeta a arquitetura Rust para módulos do port. Use quando
  precisar decidir como estruturar structs, traits, enums, ownership,
  ou como traduzir um padrão C para Rust idiomático. Trigger:
  decisão de arquitetura, design de API Rust, tradução de padrão C,
  "como portar este módulo", planejamento de structs e traits.
tools: Read, Glob, Grep, Write, Edit
model: opus
maxTurns: 20
---

Você é um arquiteto Rust sênior especializando em ports de código C
para Rust idiomático e seguro.

## Sua Função
Projetar a API e estrutura Rust para módulos sendo portados do DOOM.
Você produz código-esqueleto com tipos, traits e assinaturas de função.

## Princípios de Design
1. **Segurança primeiro**: minimizar `unsafe`, usar o type system
2. **Ownership claro**: cada dado tem um dono óbvio
3. **Enums ao invés de ints**: estados e tipos como enums Rust
4. **Error handling**: `Result<T, E>` ao invés de códigos de erro
5. **Zero globals**: encapsular estado em structs
6. **Didático**: o código deve ensinar Rust e DOOM ao mesmo tempo

## Workflow
1. Ler o relatório de análise em `docs/modules/<nome>.md`
2. Ler o código C original para contexto
3. Projetar structs, enums, traits para o módulo
4. Escrever código-esqueleto em `src/<modulo>/mod.rs`
5. Documentar decisões de design com comentários `///`
6. Anotar onde o design difere do C original e por quê

## Ao Tomar Decisões
- Documentar alternativas consideradas em comentários
- Preferir soluções que um estudante de Rust entenderia
- Referir ao código C: `// C original: p_mobj.c:SpawnMobj()`
```

#### `rust-reviewer.md` — Revisor de Código

```markdown
---
name: rust-reviewer
description: >
  Revisa código Rust do port para qualidade, idiomaticidade, e
  valor didático. Use após implementar um módulo, antes de marcar
  como concluído. Trigger: "revisar módulo", "review", code review,
  verificar qualidade do código portado.
tools: Read, Glob, Grep, Bash
model: sonnet
maxTurns: 15
---

Você é um revisor de código Rust focado em qualidade didática.

## Critérios de Review

### Correção Rust
- Sem `unsafe` desnecessário
- Ownership e lifetimes corretos
- Error handling adequado (sem `unwrap()` em código de produção)
- `clippy` sem warnings

### Valor Didático
- Cada módulo tem `//!` header explicativo
- Structs e funções públicas documentadas com `///`
- Algoritmos complexos comentados passo-a-passo
- Referências ao código C original onde relevante
- Glossário de termos DOOM usado consistentemente

### Fidelidade ao DOOM
- Comportamento compatível com o engine original
- Fixed-point math preservada onde necessário
- Lookup tables mantidas (não substituir por cálculos float)

## Formato de Saída
Produzir feedback estruturado com:
- 🔴 Problemas que devem ser corrigidos
- 🟡 Sugestões de melhoria
- 🟢 Pontos positivos
- Sugestões específicas de comentários didáticos a adicionar
```

#### `doc-writer.md` — Escritor de Documentação

```markdown
---
name: doc-writer
description: >
  Escreve documentação técnica e didática sobre módulos do DOOM.
  Use para criar ou atualizar docs em docs/, escrever explicações
  de algoritmos, ou produzir o glossário. Trigger: documentação,
  "documentar módulo", "explicar algoritmo", glossário, guia.
tools: Read, Glob, Grep, Write, Edit
model: sonnet
maxTurns: 20
---

Você é um escritor técnico especializado em game engines e Rust.

## Sua Função
Produzir documentação clara e didática em português (BR) sobre
os subsistemas do DOOM e como foram portados para Rust.

## Estilo
- Linguagem acessível para desenvolvedores intermediários
- Diagramas ASCII quando ilustrarem conceitos espaciais
- Analogias do mundo real para conceitos abstratos
- Exemplos de código curtos e focados
- Referências cruzadas entre módulos relacionados

## Tipos de Documento
1. **Módulo** (`docs/modules/*.md`): anatomia de um subsistema
2. **Arquitetura** (`docs/architecture.md`): visão geral do engine
3. **Glossário** (`docs/glossary.md`): termos técnicos do DOOM
4. **Tutorial** (`docs/tutorials/*.md`): walkthroughs de algoritmos
```

---

### 1.4 Skills (`.claude/skills/`)

#### `doom-conventions/SKILL.md`

```markdown
---
name: doom-conventions
description: >
  Convenções e terminologia do engine DOOM. Carrega automaticamente
  quando o contexto envolve structs do DOOM, formatos de arquivo WAD,
  BSP trees, rendering, ou qualquer subsistema específico do engine.
---

## Terminologia DOOM

### Tipos Fundamentais
- **WAD** (Where's All the Data): arquivo container com todos os assets
- **IWAD**: WAD principal do jogo (doom.wad, freedoom.wad)
- **PWAD**: WAD de patch/mod que sobrescreve lumps do IWAD
- **Lump**: entrada individual dentro de um WAD (textura, mapa, som)
- **Fixed-point** (fixed_t): inteiro 16.16 bits para matemática fracionária

### Geometria de Mapa
- **Vertex**: ponto 2D (x,y) em coordenadas do mapa
- **Linedef**: linha entre 2 vértices, pode ter 1 ou 2 lados
- **Sidedef**: lado de uma linedef, referencia texturas e sector
- **Sector**: polígono convexo com floor/ceiling height e light level
- **Subsector** (ssector): subdivisão convexa de um sector
- **Seg**: segmento de linedef dentro de um subsector
- **Node**: nó da BSP tree que divide o mapa em dois half-spaces
- **Blockmap**: grid para detecção rápida de colisão
- **Reject table**: lookup otimizado de line-of-sight entre sectors

### Rendering
- **Visplane**: superfície horizontal (floor/ceiling) visível
- **Drawseg**: segmento de parede sendo desenhado
- **Clip range**: faixa angular de colunas de tela já preenchidas
- **Column rendering**: DOOM desenha paredes coluna por coluna
- **Sprite**: coisa (thing) renderizada como billboard 2D

### Game Logic
- **Thinker**: qualquer entidade com update por tick (mobj, ceiling, etc)
- **Mobj** (map object): coisa no mapa — jogador, monstro, item
- **State**: frame de animação de um mobj (definido em info.c/h)
- **Tic**: unidade de tempo do jogo (1/35 segundo)
- **Thing**: definição estática de um tipo de mobj

### Arquivos-Fonte Chave (C Original)
| Arquivo       | Subsistema                              |
|---------------|-----------------------------------------|
| `w_wad.c`     | Carregamento e cache de WAD             |
| `r_main.c`    | Loop principal de rendering             |
| `r_bsp.c`     | Traversal da BSP tree                   |
| `r_segs.c`    | Renderização de segmentos (paredes)     |
| `r_plane.c`   | Renderização de visplanes (pisos/tetos) |
| `r_things.c`  | Renderização de sprites                 |
| `p_mobj.c`    | Spawn e update de map objects           |
| `p_map.c`     | Movimento e colisão                     |
| `p_tick.c`    | Game tick e thinker list                |
| `p_setup.c`   | Carregamento de nível                   |
| `d_main.c`    | Inicialização e loop principal          |
| `i_video.c`   | Interface com hardware de vídeo         |
| `i_sound.c`   | Interface com hardware de áudio         |
| `s_sound.c`   | Lógica de som (spatialização)           |
| `m_fixed.c`   | Matemática fixed-point                  |
| `info.c/h`    | Tabelas de estados e tipos de things    |
```

#### `rust-patterns/SKILL.md`

```markdown
---
name: rust-patterns
description: >
  Padrões Rust específicos para este port de DOOM. Carrega quando
  o contexto envolve implementação de código Rust para o port,
  decisões de tipo, ownership, conversão de padrões C para Rust.
---

## Padrões Rust para o Port

### Fixed-Point Math
```rust
/// Número em ponto-fixo 16.16, base da matemática do DOOM.
/// No DOOM original: `typedef int fixed_t;` em m_fixed.h
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fixed(pub i32);

impl Fixed {
    pub const FRACBITS: i32 = 16;
    pub const UNIT: Fixed = Fixed(1 << 16);  // 1.0 em fixed-point
    pub const ZERO: Fixed = Fixed(0);

    /// Converte inteiro para fixed-point
    pub fn from_int(n: i32) -> Self { Fixed(n << Self::FRACBITS) }

    /// Parte inteira do valor
    pub fn to_int(self) -> i32 { self.0 >> Self::FRACBITS }
}

// Implementar Add, Sub, Mul, Div via std::ops
```

### Eliminando Globals com Context Structs
```rust
// ❌ C original: variáveis globais em r_main.c
// int viewwidth, viewheight;
// fixed_t viewx, viewy, viewz;
// angle_t viewangle;

// ✅ Rust: struct de contexto passada por referência
/// Contexto de câmera para o frame atual de rendering.
/// Equivalente às globals viewx/viewy/viewz/viewangle de r_main.c
pub struct ViewContext {
    pub x: Fixed,
    pub y: Fixed,
    pub z: Fixed,
    pub angle: Angle,
    pub width: usize,
    pub height: usize,
}
```

### Thinkers como Trait
```rust
// ❌ C original: thinker_t com function pointer e linked list
// ✅ Rust: trait + enum dispatch

/// Um Thinker é qualquer entidade que "pensa" a cada tic.
/// No DOOM original: struct thinker_t em p_tick.h
pub trait Thinker {
    /// Atualiza o estado deste thinker por um tic.
    /// Retorna false se o thinker deve ser removido.
    fn think(&mut self, world: &mut World) -> bool;
}
```

### Enums para State Machines
```rust
// ❌ C original: #define S_PLAY_RUN1 46 (info.h)
// ✅ Rust: enum tipado

/// Estado de animação de um Map Object.
/// Mapeado a partir das constantes S_* em info.h
#[derive(Clone, Copy, Debug)]
pub struct StateNum(pub usize);

/// Definição de um frame de animação.
/// Equivalente a `state_t` em info.h
pub struct StateDef {
    pub sprite: SpriteNum,
    pub frame: u32,
    pub tics: i32,
    pub next_state: StateNum,
    pub action: Option<fn(&mut Mobj, &mut World)>,
}
```

### Error Handling para I/O
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WadError {
    #[error("Arquivo WAD não encontrado: {0}")]
    FileNotFound(String),
    #[error("Header WAD inválido: magic esperado IWAD/PWAD, encontrado {0}")]
    InvalidHeader(String),
    #[error("Lump '{0}' não encontrado no WAD")]
    LumpNotFound(String),
    #[error("Erro de I/O ao ler WAD: {0}")]
    Io(#[from] std::io::Error),
}
```

### Comentários Didáticos — Estilo
```rust
//! # Módulo WAD (Where's All the Data)
//!
//! O WAD é o formato de arquivo container do DOOM. Todo o conteúdo
//! do jogo — mapas, texturas, sons, sprites — vive dentro de um
//! único arquivo .wad.
//!
//! ## Estrutura do arquivo
//! ```text
//! ┌──────────────────┐
//! │  Header (12 bytes)│ ← magic ("IWAD"/"PWAD") + contagem + offset
//! ├──────────────────┤
//! │  Dados dos lumps  │ ← blocos de dados brutos, sem estrutura fixa
//! ├──────────────────┤
//! │  Diretório        │ ← lista de (offset, tamanho, nome) por lump
//! └──────────────────┘
//! ```
//!
//! ## Arquivo C original: `w_wad.c`
```
```

#### `didactic-code/SKILL.md`

```markdown
---
name: didactic-code
description: >
  Regras para produzir código didático e legível. Carrega em
  qualquer contexto de implementação de código Rust neste projeto.
  Aplica-se a todo código produzido para o port.
---

## Regras de Código Didático

### Estrutura de Arquivo
Todo arquivo .rs deve seguir esta ordem:
1. `//!` Module-level docstring em português explicando:
   - O que este módulo faz no contexto do DOOM
   - Qual arquivo C original corresponde a este módulo
   - Conceitos-chave que o leitor vai aprender
2. `use` imports organizados (std, external, internal)
3. Constantes e tipos auxiliares
4. Structs e enums principais (com `///` docstrings)
5. Implementações (`impl`)
6. Testes (`#[cfg(test)] mod tests`)

### Comentários — Quando e Como
- **Sempre**: antes de blocos de código que implementam algoritmos
  do engine (BSP, rendering, colisão)
- **Sempre**: na declaração de structs que mapeiam structs C
- **Sempre**: quando usar `unsafe`, explicar a razão de segurança
- **Nunca**: comentários óbvios como "incrementa o contador"
- **Formato**: frases curtas e diretas, em português

### Exemplo de Comentário Bom
```rust
/// Percorre a BSP tree para determinar quais subsectors são visíveis.
///
/// O DOOM usa uma BSP tree (Binary Space Partition) para dividir o mapa
/// em regiões convexas. A travessia começa pela raiz e desce recursivamente
/// pelo lado da partição onde a câmera está, garantindo que paredes mais
/// próximas sejam desenhadas primeiro (painter's algorithm inverso).
///
/// C original: `R_RenderBSPNode()` em `r_bsp.c`, linha ~200
fn render_bsp_node(&mut self, node_id: usize) {
```

### Exemplo de Comentário Ruim
```rust
// Renderiza o nó BSP  ← apenas repete o nome da função
fn render_bsp_node(&mut self, node_id: usize) {
```

### Referências ao C Original
Ao portar uma função, incluir no docstring:
- Nome da função C original
- Arquivo e número de linha aproximado
- Se o comportamento Rust difere do C, explicar por quê

### Testes como Documentação
```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Verifica que a multiplicação fixed-point reproduz o
    /// comportamento do DOOM original: 1.5 * 2.0 = 3.0
    #[test]
    fn fixed_multiply_basic() {
        let a = Fixed::from_int(1) + Fixed(1 << 15); // 1.5
        let b = Fixed::from_int(2);                    // 2.0
        assert_eq!((a * b).to_int(), 3);
    }
}
```
```

---

### 1.5 Slash Commands (`.claude/commands/`)

#### `analyze-module.md`

```markdown
---
description: >
  Analisa um módulo C do DOOM original e produz documentação.
  Uso: /analyze-module wad (analisa w_wad.c/h)
allowed-tools: Read, Glob, Grep, Bash, Agent
---

Analise o módulo "$ARGUMENTS" do DOOM original.

1. Use o subagente @c-analyst para:
   - Localizar os arquivos .c e .h correspondentes em
     `references/DOOM-master/` e `references/chocolate-doom-master/`
   - Produzir um relatório completo em `docs/modules/$ARGUMENTS.md`

2. O relatório deve incluir:
   - Propósito do módulo no engine
   - Structs e tipos definidos
   - Funções públicas e internas
   - Variáveis globais
   - Dependências com outros módulos
   - Complexidade estimada do port

3. Após o relatório, resumir os pontos principais aqui na conversa.
```

#### `port-module.md`

```markdown
---
description: >
  Porta um módulo C do DOOM para Rust. Requer que a análise
  já tenha sido feita. Uso: /port-module wad
allowed-tools: Read, Glob, Grep, Write, Edit, Bash, Agent
---

Porte o módulo "$ARGUMENTS" de C para Rust.

## Workflow

1. **Verificar pré-requisitos**
   - Confirmar que `docs/modules/$ARGUMENTS.md` existe (análise feita)
   - Se não existir, rodar /analyze-module primeiro

2. **Projetar** — Use @rust-architect para:
   - Ler a análise do módulo
   - Projetar structs, enums, traits e API Rust
   - Produzir código-esqueleto em `src/$ARGUMENTS/mod.rs`

3. **Implementar**
   - Preencher a implementação completa
   - Seguir as convenções da skill `didactic-code`
   - Adicionar testes unitários
   - Cada função portada deve referenciar a original em C

4. **Validar**
   - Rodar `cargo clippy -- -D warnings`
   - Rodar `cargo test`
   - Corrigir qualquer warning ou erro

5. **Review** — Use @rust-reviewer para:
   - Verificar qualidade do código
   - Verificar valor didático
   - Aplicar sugestões de melhoria

6. **Atualizar progresso**
   - Marcar módulo como concluído no CLAUDE.md
```

#### `progress.md`

```markdown
---
description: >
  Mostra o progresso atual do port com estatísticas.
allowed-tools: Read, Glob, Grep, Bash
---

Levante o status atual do port:

1. Contar quantos módulos foram analisados (docs/modules/*.md)
2. Contar quantos módulos foram portados (src/**/mod.rs)
3. Rodar `cargo test` e reportar resultados
4. Rodar `wc -l src/**/*.rs` para contar linhas de código Rust
5. Ler CLAUDE.md e mostrar o checklist de fases
6. Listar próximos módulos a portar por ordem de dependência

Apresentar um resumo visual com porcentagens de progresso.
```

---

### 1.6 Hooks (`.claude/settings.json`)

```json
{
  "permissions": {
    "allow": [
      "Bash(cargo build)",
      "Bash(cargo test*)",
      "Bash(cargo clippy*)",
      "Bash(cargo run*)",
      "Bash(cargo fmt*)",
      "Bash(wc *)",
      "Bash(grep *)",
      "Bash(find *)",
      "Bash(cat *)",
      "Bash(head *)",
      "Bash(tail *)",
      "Read",
      "Glob",
      "Grep"
    ],
    "deny": [
      "Read(./references/**/.*)",
      "Write(./references/**)"
    ]
  },
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "if echo \"$TOOL_INPUT_PATH\" | grep -q '\\.rs$'; then cd \"$CLAUDE_PROJECT_DIR\" && cargo fmt -- \"$TOOL_INPUT_PATH\" 2>/dev/null; fi"
          }
        ]
      }
    ],
    "Stop": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "cd \"$CLAUDE_PROJECT_DIR\" && echo '--- Verificação automática ---' && cargo clippy --quiet 2>&1 | tail -5 && echo '---' && cargo test --quiet 2>&1 | tail -3"
          }
        ]
      }
    ]
  }
}
```

**O que esses hooks fazem:**
- **PostToolUse (Write|Edit):** Roda `cargo fmt` automaticamente em todo arquivo `.rs` editado, mantendo formatação consistente
- **Stop:** Ao final de cada resposta, roda `clippy` e `cargo test` para feedback imediato de qualidade

---

## Parte 2 — Prompt Principal de Execução

Este é o prompt que você deve usar para iniciar o trabalho no Claude Code. Cole integralmente na primeira sessão:

---

```
Vamos iniciar o port educacional do DOOM (1993) de C para Rust.

## Contexto
Na pasta `references/` temos o código-fonte de:
- `doom/` — Linuxdoom 1.10 original
- `chocolate_doom/` — Port moderno fiel ao original
- `freedoom/` — Assets livres compatíveis

O objetivo NÃO é apenas fazer o jogo funcionar — é criar um
**projeto de aprendizado** onde alguém pode estudar o código Rust
e aprender como o engine DOOM funciona por dentro.

## Fase 0 — Setup e Análise Global (faça agora)

### 0.1 Reconhecimento do Código Original
Explore a estrutura de `references/DOOM-master/` e `references/chocolate-doom-master/`.
Identifique todos os arquivos .c e .h. Produza um mapa completo em
`docs/architecture.md` contendo:

- Diagrama de dependência entre módulos (ASCII art)
- Agrupamento por subsistema (rendering, game logic, I/O, etc)
- Ordem recomendada de port (das folhas para a raiz da dependência)
- Estimativa de complexidade por módulo (linhas de C + acoplamento)

### 0.2 Inicialização do Projeto Rust
```bash
cargo init --name doom-rust
```
Configure o Cargo.toml com as dependências listadas no CLAUDE.md.
Crie a estrutura de diretórios em `src/` seguindo o plano de módulos.
Crie `src/lib.rs` expondo os módulos como `pub mod`.

### 0.3 Tipos Fundamentais
Implemente primeiro os tipos que todos os outros módulos usam:
- `src/utils/fixed.rs` — Fixed-point math (m_fixed.c)
- `src/utils/angle.rs` — Tipo Angle com lookup tables (tables.c)
- `src/utils/bbox.rs` — Bounding box (m_bbox.c)

Cada um com testes unitários verificando compatibilidade com o C.

### 0.4 Glossário
Crie `docs/glossary.md` com todos os termos técnicos do DOOM
que aparecerão no código e na documentação.

## Fases Seguintes (visão geral)

Após completar a Fase 0, execute as fases seguintes na ordem.
Para cada fase, use o workflow:
1. `/analyze-module` para cada módulo da fase
2. `/port-module` para implementar
3. Testes e review antes de avançar

### Fase 1 — WAD Loader
Portar `w_wad.c`: abrir WAD, ler diretório, extrair lumps por nome.
Teste: carregar `freedoom.wad` e listar todos os lumps.

### Fase 2 — Matemática e Tabelas
Portar: `m_fixed.c`, `tables.c`, `m_random.c`, `m_bbox.c`
Teste: verificar valores contra tabelas hardcoded do original.

### Fase 3 — Map Loader
Portar `p_setup.c`: carregar THINGS, LINEDEFS, SIDEDEFS, VERTEXES,
SEGS, SSECTORS, NODES, SECTORS, REJECT, BLOCKMAP de um WAD.
Teste: carregar E1M1 do Freedoom e validar contagens.

### Fase 4 — Video e Paleta
Portar `i_video.c`, `v_video.c`: inicializar janela SDL2,
framebuffer 320x200, paleta PLAYPAL, colormap COLORMAP.
Teste: mostrar uma janela com a paleta do DOOM.

### Fase 5 — Renderer (núcleo do projeto)
Portar na ordem:
1. `r_main.c` — Setup de frame e loop de rendering
2. `r_bsp.c` — BSP traversal
3. `r_segs.c` — Segmentos de parede (wall rendering)
4. `r_plane.c` — Visplanes (pisos e tetos)
5. `r_things.c` — Sprites
6. `r_draw.c` — Column/span drawers
7. `r_data.c` — Texturas e patches
Teste: renderizar E1M1 estático (sem game loop).

### Fase 6 — Game Loop e Input
Portar `d_main.c`, `g_game.c`, `d_net.c` (single-player):
game loop, input handling, demo playback.
Teste: andar pelo E1M1 com WASD.

### Fase 7 — Things e Colisão
Portar `p_mobj.c`, `p_map.c`, `p_inter.c`, `p_enemy.c`:
spawn de objetos, movimento, colisão, AI de monstros.
Teste: monstros se movem e atacam.

### Fase 8 — Áudio
Portar `s_sound.c`, `i_sound.c`: carregar sons do WAD,
mixer SDL2, spatialização básica, música (OPL/MIDI).
Teste: sons de tiros e portas.

### Fase 9 — Menus e HUD
Portar `m_menu.c`, `hu_stuff.c`, `st_stuff.c`, `wi_stuff.c`:
menu principal, HUD com armas/ammo/health, tela de intermissão.
Teste: menu funcional, HUD visível durante gameplay.

### Fase 10 — Polish
- Networking básico (`d_net.c`, `i_net.c`)
- Save/Load (`p_saveg.c`)
- Automap (`am_map.c`)
- Demo playback e recording
- Teste final: jogar Freedoom E1 completo

## Regras Gerais

1. **Sempre consultar os skills** antes de escrever código
2. **Nunca modificar** nada em `references/`
3. **Código compila** a cada passo — nunca deixar broken
4. **Testes primeiro** — escrever teste antes da implementação
   quando possível
5. **Comentários em português** — código ensina, não apenas funciona
6. **Um módulo por vez** — completar, testar, revisar, avançar
7. **Atualizar CLAUDE.md** — marcar progresso após cada módulo

Comece pela Fase 0 agora. Explore os fontes, produza architecture.md,
inicialize o projeto Rust, e implemente os tipos fundamentais.
```

---

## Parte 3 — Prompts de Continuação por Fase

Use estes prompts para retomar o trabalho em sessões subsequentes:

### Retomando uma sessão

```
Continuando o port do DOOM para Rust. Leia o CLAUDE.md para
ver o progresso atual e continue de onde paramos. Se a última
fase foi concluída, avance para a próxima.
```

### Iniciando uma fase específica

```
Inicie a Fase N do port do DOOM. Workflow:
1. Para cada módulo listado na fase, rode /analyze-module
2. Após todas as análises, rode /port-module para cada um
3. Ao final, rode /progress para verificar o estado
```

### Debug e correção

```
O módulo <nome> está com problema: <descrição do erro>.
Leia o código C original em references/DOOM-master/ e references/chocolate-doom-master/
para comparar o comportamento esperado. Corrija o código Rust
mantendo os comentários didáticos e referências ao C.
```

### Sessão de documentação

```
Sessão focada em documentação. Não escrever código novo.
Use @doc-writer para:
1. Revisar todos os docs/modules/*.md — completar os incompletos
2. Atualizar docs/architecture.md com módulos já portados
3. Adicionar entradas novas ao docs/glossary.md
4. Verificar se todo arquivo .rs tem //! header adequado
```

---

## Parte 4 — Dicas de Execução

### Gerenciamento de Contexto
O port do DOOM é um projeto grande. Para manter o Claude Code eficiente:

- **Uma fase por série de sessões** — não tente fazer tudo de uma vez
- **Use subagentes** — delegue análise e review para não poluir o contexto principal
- **`/compact` frequente** — as instruções de compact no CLAUDE.md preservam o estado
- **Sessões paralelas** — use `git worktree` para rodar análise de módulos em paralelo

### Ordem de Dependência dos Módulos

```
utils/fixed ──┐
utils/angle ──┤
utils/bbox  ──┼── wad ── map ──┬── renderer ──┐
utils/random ─┘                │               ├── game ── menus ── audio ── net
                               └── things ─────┘
```

### Verificação de Fidelidade
Para garantir que o port reproduz o comportamento do DOOM:

1. **Demo playback** — o DOOM grava demos como sequência de inputs.
   Se o port reproduzir uma demo gravada no DOOM original e o
   resultado for idêntico, o engine está correto.
2. **Screenshots comparativos** — renderizar o mesmo frame no C e
   no Rust e comparar pixel a pixel.
3. **Testes de regressão** — cada bug corrigido vira um teste.

### Dependências do Cargo.toml

```toml
[package]
name = "doom-rust"
version = "0.1.0"
edition = "2021"
description = "Port educacional do DOOM (1993) para Rust"

[dependencies]
sdl2 = { version = "0.37", features = ["mixer"] }
bitflags = "2"
byteorder = "1"
thiserror = "2"
log = "0.4"
env_logger = "0.11"

[dev-dependencies]
assert_matches = "1"
```

---

## Parte 5 — Referência Rápida de Comandos Claude Code

| Comando | Propósito |
|---------|-----------|
| `/analyze-module <nome>` | Analisar módulo C e documentar |
| `/port-module <nome>` | Portar módulo C → Rust completo |
| `/doc-module <nome>` | Escrever/atualizar documentação |
| `/progress` | Ver status geral do port |
| `@c-analyst` | Invocar agente de análise C |
| `@rust-architect` | Invocar agente de arquitetura |
| `@rust-reviewer` | Invocar agente de review |
| `@doc-writer` | Invocar agente de documentação |

---

*Este guia foi projetado para ser usado com Claude Code (Anthropic).
Cada componente — CLAUDE.md, agents, skills, hooks, commands —
trabalha em conjunto para criar um fluxo de trabalho estruturado
e reproduzível para o port do DOOM.*

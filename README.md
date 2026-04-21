# DOOM Rust

Port educacional do engine **DOOM (1993)** de C para **Rust idiomático**.

## ⚠️ Aviso: Projeto Experimental

Este é um **projeto experimental e educacional**, desenvolvido como
estudo prático de port C → Rust assistido por IA. **Não é um
software de produção** e não tem vínculo com a id Software,
Bethesda, ZeniMax ou qualquer detentor de direitos do DOOM.

**Quem optar por compilar, executar ou modificar este software o
faz por sua própria conta e risco.** Os autores e colaboradores
não se responsabilizam por:

- Perda ou corrupção de dados
- Danos a hardware (GPU, periféricos, etc.)
- Instabilidade do sistema operacional
- Incompatibilidades com WADs de terceiros
- Eventuais bugs, crashes ou comportamentos inesperados
- Quaisquer consequências diretas ou indiretas do uso

O código é fornecido **"COMO ESTÁ"**, sem garantias de qualquer
natureza, expressas ou implícitas, incluindo mas não se limitando
a garantias de comercialização, adequação a um propósito específico
ou não-violação. Consulte a seção [Licença](#licença) para detalhes.

## Proposta do Projeto

Este projeto reimplementa o clássico DOOM da id Software em Rust, com
o objetivo duplo de:

1. **Ensinar a arquitetura interna do DOOM** — BSP trees, raycasting,
   fixed-point math, sistema de thinkers, formato WAD, etc.
2. **Ensinar Rust idiomático** — traduzindo padrões C (ponteiros,
   arrays globais, `void*`, linked lists) para construtos seguros
   de Rust (ownership, enums, trait objects, `Vec<Box<dyn T>>`).

O código é deliberadamente **didático**: comentários em português
referenciam o arquivo C original, structs importantes têm
equivalência com o C documentada, e algoritmos complexos têm
explicação passo-a-passo.

### Fontes de referência

- `references/DOOM-master/` — Linuxdoom 1.10 (código original)
- `references/chocolate-doom-master/` — Chocolate Doom (correções)
- `references/freedoom-master/` — Assets livres para testes

## Sobre o DOOM (1993)

DOOM é um dos jogos mais influentes da história. Lançado pela
**id Software** em **dezembro de 1993**, foi pioneiro em várias
tecnologias:

- **Renderer 2.5D** via Binary Space Partitioning (BSP) — paredes
  verticais, sem geometria 3D real, mas com ilusão convincente
- **Setores com alturas independentes** de piso e teto
- **Raycasting por coluna** (320 colunas de 200 pixels)
- **Fixed-point math** (16.16) — sem FPU disponível na época
- **Formato WAD** para assets (mapas, texturas, sprites, áudio)
- **Deathmatch em rede** — 4 jogadores via IPX
- **Modding pela comunidade** — WADs customizados

O código-fonte foi liberado em **1997** sob licença GPL, permitindo
estudos e ports como este.

## Sobre a Linguagem Rust

**Rust** é uma linguagem de programação de sistemas moderna
desenvolvida pela Mozilla (2010) e mantida pela Rust Foundation.
Oferece:

- **Memory safety sem garbage collector** — ownership e borrow
  checker em tempo de compilação
- **Zero-cost abstractions** — performance comparável a C/C++
- **Concorrência segura** — tipos `Send`/`Sync` impedem data races
- **Sistema de tipos expressivo** — enums algébricos, traits,
  pattern matching exaustivo
- **Cargo** — gerenciador de pacotes e build system integrados

É particularmente adequada para engines de jogo: performance
previsível, controle sobre alocação, e segurança sem overhead.

### Rust no Kernel Linux

Desde a versão **6.1 do kernel Linux (dezembro de 2022)**, Rust
passou a ser **oficialmente aceita** como segunda linguagem de
sistema do kernel, ao lado de C. Linus Torvalds aprovou a inclusão
do suporte a Rust após anos de discussão na comunidade.

A partir da versão **6.13 (janeiro de 2025)**, partes reais do
kernel já são escritas em Rust:

- **Drivers experimentais** — primeiros drivers em Rust (Nova GPU
  para chips NVIDIA RTX, Apple AGX, Asahi Linux).
- **Abstrações de subsistemas** — bindings seguros para módulos,
  timers, kmalloc, sync primitives, etc.
- **DRM (Direct Rendering Manager)** — partes da infraestrutura
  gráfica ganharam APIs Rust.
- **Android Binder** — o Google contribuiu uma implementação Rust
  do driver Binder para o Android 15.

A motivação principal é **reduzir bugs de segurança de memória**,
que historicamente representam ~70% das CVEs em software de sistema.
O borrow checker elimina classes inteiras de vulnerabilidades
(use-after-free, data races, buffer overflows) em tempo de
compilação, sem custo de runtime.

Outros projetos de sistema que adotaram Rust:

- **Windows** — Microsoft migrou partes do kernel NT, incluindo
  GDI e código de boot, para Rust.
- **Firefox** — engine CSS Stylo e componente Servo.
- **Chromium** — bibliotecas de parsing e codec Rust.
- **AWS Firecracker** — hypervisor de microVMs usado pelo AWS
  Lambda, escrito inteiramente em Rust.

Este contexto reforça por que **portar DOOM para Rust tem valor
didático**: mostra na prática os mesmos padrões de migração que
a indústria está aplicando em sistemas reais.

## Principais Desafios

Reimplementar um engine C de 1993 em Rust moderno expôs vários
conflitos de paradigma:

### 1. Globals mutáveis vs. Ownership
O DOOM original usa dezenas de arrays globais mutáveis
(`sectors[]`, `lines[]`, `mobjs[]`). Em Rust, isso quebra o
borrow checker. **Solução:** agrupar estado em `MapData` e
passar `&mut` explicitamente.

### 2. Ponteiros `void*` e cast dinâmico
Thinkers em C são `thinker_t*` com `void (*function)()`. Em Rust,
usamos **`trait Thinker`** e `Vec<Box<dyn Thinker>>`. Porém,
thinkers precisam modificar sectors — resolvido passando
`&mut [Sector]` para `think()`.

### 3. Fixed-point 16.16
O C usa `typedef int fixed_t` com macros `FixedMul`/`FixedDiv`.
Em Rust, criamos `struct Fixed(i32)` com `impl Mul/Div` e
proteção contra overflow (`abs(a)>>14 >= abs(b)`).

### 4. Renderer BSP e colunas de pixels
O código de `r_draw.c` é altamente otimizado em C. Portar
mantendo legibilidade **e** performance exigiu fast-paths
com bitmask (potência de 2) e cache de lookup por nome.

### 5. Formato WAD endian-specific
Arquivos WAD são little-endian, com structs `repr(C)` que em
C são lidos via `read()`. Em Rust usamos `byteorder::LittleEndian`
com parsing explícito e seguro.

### 6. Angles em 32-bit
DOOM usa `angle_t` (u32) onde 0x00000000 = 0 graus e 0xFFFFFFFF
= ~360 graus, aproveitando overflow natural. Replicamos em
`struct Angle(u32)` com `Wrapping<u32>` quando necessário.

### 7. Rendering 3D correto
Vários bugs sutis no port: `DBITS` errado (19 vs 5), `centery`
errado (100 vs 84 para viewheight/2), filtro de trigger lines
two-sided, iluminação `DISTMAP`. Corrigidos comparando
cuidadosamente com `r_main.c`.

## Fases Executadas

| Fase | Módulo                          | Status     |
|------|---------------------------------|------------|
| 0    | Setup e análise                 | Concluída  |
| 1    | WAD loader                      | Concluída  |
| 2    | Matemática e tipos base         | Concluída  |
| 3    | Map loader (BSP)                | Concluída  |
| 4    | Renderer                        | Concluída  |
| 5    | Game loop e input               | Concluída  |
| 6    | Coisas e colisão                | Concluída  |
| 7    | Áudio                           | Concluída  |
| 8    | Menus e HUD                     | Concluída  |
| 9    | Networking                      | Concluída  |
| 10   | Polish e freedoom jogável       | Concluída  |
| 11   | Integração SDL2                 | Em revisão |

Cada fase teve sua própria branch `fase-N-descricao` e foi
mesclada em `main` via Pull Request (OneFlow workflow).

## Tempo de Migração e Estatísticas

### Cronologia

- **Início:** 19 de abril de 2026
- **Última atividade:** 21 de abril de 2026
- **Duração:** ~3 dias de desenvolvimento intensivo

### Código

| Métrica                    | Valor          |
|----------------------------|----------------|
| Arquivos Rust              | 54             |
| Linhas de código Rust      | ~26.100        |
| Testes unitários           | 361 (100% ok)  |
| Commits                    | 40+            |
| Módulos principais         | 9 (wad, map, renderer, game, menu, sound, net, video, utils) |

### Comparação com o original

| Métrica                    | DOOM C (1993)  | DOOM Rust      |
|----------------------------|----------------|----------------|
| Arquivos-fonte principais  | ~80 (.c/.h)    | 54 (.rs)       |
| Linhas aproximadas         | ~36.000        | ~26.100        |
| Testes automatizados       | 0              | 361            |
| Safety checks              | Manuais        | Compilador     |

## Dependências

### Rust

- **Rust 1.75+** (edition 2021)
- Instale via [rustup.rs](https://rustup.rs/):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

### SDL2 (janela, input, áudio)

Linux (Debian/Ubuntu):
```bash
sudo apt install libsdl2-dev libsdl2-mixer-dev
```

macOS (Homebrew):
```bash
brew install sdl2 sdl2_mixer
```

Windows (MSYS2):
```bash
pacman -S mingw-w64-x86_64-SDL2 mingw-w64-x86_64-SDL2_mixer
```

### WAD de assets

Necessário um IWAD do DOOM. Opções:

1. **Freedoom** (gratuito, livre) — baixado automaticamente por
   `make freedoom`:
   ```bash
   make freedoom
   ```
2. **DOOM original** — copie `doom.wad` ou `doom2.wad` para
   `assets/`.

### Crates Rust (automaticamente via cargo)

| Crate        | Versão | Uso                              |
|--------------|--------|----------------------------------|
| sdl2         | 0.37   | Janela, input, áudio (opcional)  |
| bitflags     | 2      | Flags de linedefs/sectors        |
| byteorder    | 1      | Parsing de WAD little-endian     |
| thiserror    | 2      | Tipos de erro ergonômicos        |
| log          | 0.4    | Logging                          |
| env_logger   | 0.11   | Backend de logging               |

## Como Rodar

> ⚠️ **Lembrete:** Conforme o [aviso de projeto experimental](#️-aviso-projeto-experimental),
> a execução é por sua conta e risco.

### Método 1 — Cargo nativo (recomendado)

```bash
# Baixar Freedoom (uma vez)
make freedoom

# Compilar e rodar
cargo run --release -- --iwad assets/freedoom1.wad

# Ou via Makefile
make run

# Com mapa e dificuldade customizados
make run WARP="2 3" SKILL=5
```

### Método 2 — Docker

Sem instalar Rust ou SDL2 na máquina:

```bash
# Build da imagem
make docker

# Baixar Freedoom e rodar
make freedoom docker-run
```

Veja [`docs/docker-guide.md`](docs/docker-guide.md) para mais
detalhes.

### Método 3 — Manualmente

```bash
cargo build --release --features sdl
./target/release/doom-rust --iwad assets/freedoom1.wad
```

### Opções de linha de comando

| Flag             | Descrição                              |
|------------------|----------------------------------------|
| `--iwad PATH`    | Caminho do arquivo WAD (obrigatório)   |
| `--warp E M`     | Episódio E, mapa M (ex: `--warp 1 1`)  |
| `--skill N`      | Dificuldade 1-5 (1=Baby, 5=Nightmare)  |
| `--nomusic`      | Desabilitar música                     |
| `--nosound`      | Desabilitar efeitos sonoros            |

### Controles

| Tecla             | Ação                        |
|-------------------|-----------------------------|
| Setas             | Movimento / rotação         |
| Ctrl              | Atirar                      |
| Espaço / Enter    | Usar (abrir portas, switches) |
| Shift             | Correr                      |
| Tab               | Automap                     |
| Esc               | Menu                        |

## Desenvolvimento

### Comandos úteis

```bash
cargo build              # compilar
cargo test               # rodar 361 testes
cargo clippy -- -D warnings  # lint estrito
make test                # atalho para testes
make clippy              # atalho para lint
```

### Estrutura do projeto

```
src/
  wad/        # Carregamento de arquivos WAD
  map/        # Parsing de mapas e BSP
  renderer/   # Renderer 2.5D por BSP + colunas
  game/       # Game loop, thinkers, portas, armas
  menu/       # Menus, HUD, status bar
  sound/      # Áudio (SDL2 mixer)
  net/        # Networking (placeholders)
  video/      # Framebuffer e SDL2
  utils/      # Fixed-point, angles, tabelas
```

### Workflow Git (OneFlow)

- Branch principal: `main`
- Cada fase: `fase-N-descricao` → PR para `main`
- `cargo clippy` e `cargo test` devem passar antes do merge

## Documentação

A pasta [`docs/`](docs/) reúne a documentação técnica e didática
produzida ao longo do port. Os arquivos são complementares ao
código-fonte e ao `CLAUDE.md` (contexto de IA do projeto):

| Arquivo | Conteúdo |
|---------|----------|
| [`docs/architecture.md`](docs/architecture.md) | Visão geral da arquitetura do DOOM original — estatísticas por subsistema, diagrama de dependências entre os módulos C (`p_*`, `r_*`, `g_*`, `s_*`, `w_*`, `m_*`, `z_*`) e LOC aproximado de cada um. Use como mapa mental antes de navegar o código. |
| [`docs/glossary.md`](docs/glossary.md) | Glossário dos termos técnicos do engine DOOM (WAD, IWAD, PWAD, lump, patch, BSP, sector, linedef, thinker, fixed-point, angle_t...) e da linguagem Rust aplicada ao port (ownership, trait objects, `Vec<Box<dyn T>>`, etc.). Referência rápida ao encontrar jargão. |
| [`docs/docker-guide.md`](docs/docker-guide.md) | Guia passo-a-passo para compilar e executar o port em contêiner Docker, sem instalar Rust ou SDL2 na máquina host. Inclui pré-requisitos, obtenção de IWAD e exemplos de execução. |
| [`docs/modules/`](docs/modules/) | Pasta reservada para documentação por módulo individual (a ser populada conforme cada subsistema Rust for aprofundado). |

Documentos complementares na raiz do repositório:

- [`CLAUDE.md`](CLAUDE.md) — Contexto do projeto usado pelo
  assistente de IA: convenções de código, mapeamento C → Rust,
  dependências permitidas, workflow Git e progresso das fases.
- [`README.md`](README.md) — Este arquivo: visão geral, instruções
  de build/execução e histórico de desafios.

## Desenvolvimento Assistido por Claude Code

Todo o desenvolvimento deste port foi executado com o auxílio do
**[Claude Code](https://claude.com/claude-code)** — o assistente
de desenvolvimento de IA da Anthropic. Desde a análise do código C
original até a implementação, revisão e documentação do código Rust,
o fluxo combinou decisões arquiteturais do autor com a geração e
refatoração guiadas pelo assistente.

A colaboração seguiu um modelo **estruturado**: em vez de prompts
soltos, o repositório configura o Claude Code com instruções de
projeto, agentes especializados, slash-commands reutilizáveis,
skills contextuais e hooks de verificação automática. Esses
recursos vivem em [`.claude/`](.claude/) e [`CLAUDE.md`](CLAUDE.md),
e são versionados junto com o código.

### Estrutura dos recursos do Claude Code utilizados

```
doom-rust/
├── CLAUDE.md                    # Contexto raiz do projeto (lido em toda sessão)
└── .claude/
    ├── settings.json            # Permissões e hooks compartilhados
    ├── settings.local.json      # Overrides locais (não versionado)
    ├── agents/                  # Sub-agentes especializados
    │   ├── c-analyst.md         # Analisa módulos C do DOOM / Chocolate Doom
    │   ├── rust-architect.md    # Projeta arquitetura Rust (structs, traits, ownership)
    │   ├── rust-reviewer.md     # Revisa código Rust portado (qualidade/idiomaticidade)
    │   └── doc-writer.md        # Redige documentação técnica e didática
    ├── commands/                # Slash-commands do fluxo de port
    │   ├── analyze-module.md    # /analyze-module <nome> — analisa módulo C
    │   ├── port-module.md       # /port-module <nome>    — porta C → Rust
    │   ├── doc-module.md        # /doc-module <nome>     — documenta módulo
    │   └── progress.md          # /progress              — status do port
    └── skills/                  # Skills carregadas conforme o contexto
        ├── didactic-code/       # Regras de código didático e legível
        ├── doom-conventions/    # Terminologia e convenções do engine DOOM
        └── rust-patterns/       # Padrões Rust específicos deste port
```

#### Papel de cada recurso

| Recurso | Função no projeto |
|---------|-------------------|
| [`CLAUDE.md`](CLAUDE.md) | Define convenções de código, mapeamento C → Rust, dependências permitidas, workflow Git (OneFlow) e progresso das fases. Carregado em toda sessão. |
| [`.claude/settings.json`](.claude/settings.json) | Lista de permissões de ferramentas (`cargo build`, `cargo test`, `cargo clippy`, etc.) e **hooks**: `cargo fmt` automático após cada `Edit/Write` em `.rs` e `cargo clippy` + `cargo test` ao final de cada resposta. |
| **Agentes** | Delegação de tarefas complexas para contextos isolados, preservando a janela do agente principal. Cada agente tem ferramentas restritas ao seu papel. |
| **Commands** | Automação de fluxos recorrentes do port — uma única linha (`/port-module wad`) dispara uma sequência estruturada de análise, implementação e revisão. |
| **Skills** | Injeção de conhecimento sob demanda: terminologia do DOOM, padrões Rust idiomáticos e regras de didática só entram no contexto quando a tarefa as exige. |

Essa organização tornou explícitos os critérios de qualidade do port
(clippy limpo, testes passando, comentários em português referenciando
o C original) e permitiu que cada fase fosse conduzida com consistência
— do WAD loader ao renderer BSP à integração SDL2.

## Licença

Código do port: MIT / Apache-2.0 (a definir).

DOOM original: GPL v2 (id Software, 1997).
Freedoom (assets): BSD-style.

**Isenção de responsabilidade:** Este software é distribuído
"COMO ESTÁ", sem garantia de qualquer tipo. Em nenhum evento
os autores serão responsáveis por qualquer reclamação, dano
ou outra responsabilidade, seja em ação de contrato, ato
ilícito ou de outra forma, decorrente de, fora de ou em
conexão com o software ou o uso ou outros negócios no software.

## Créditos

- **id Software** — DOOM original (1993)
- **Chocolate Doom team** — referência de portabilidade
- **Freedoom project** — assets livres

Projeto educacional desenvolvido como estudo prático de port
C → Rust **assistido por IA** — integralmente com o
**[Claude Code](https://claude.com/claude-code)** da Anthropic.
Detalhes dos recursos utilizados na seção
[Desenvolvimento Assistido por Claude Code](#desenvolvimento-assistido-por-claude-code).

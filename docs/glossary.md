# Glossario — DOOM Rust Port

Termos tecnicos do engine DOOM e da linguagem Rust usados neste projeto.

## Termos do Engine DOOM

### Formato de Dados

| Termo | Descricao |
|-------|-----------|
| **WAD** | Where's All the Data — arquivo container com todos os assets do jogo |
| **IWAD** | Internal WAD — WAD principal do jogo (doom.wad, doom2.wad, freedoom.wad) |
| **PWAD** | Patch WAD — WAD de mod que sobrescreve lumps do IWAD |
| **Lump** | Entrada individual dentro de um WAD (textura, mapa, som, sprite) |
| **Patch** | Formato de imagem do DOOM — colunas de pixels com transparencia |
| **Flat** | Textura de 64x64 para pisos e tetos (sem transparencia) |
| **Sprite** | Imagem de um objeto/monstro vista de um angulo especifico |
| **PLAYPAL** | Lump com as 14 paletas de 256 cores do DOOM |
| **COLORMAP** | Lump com tabelas de mapeamento de cor para diferentes niveis de luz |

### Geometria de Mapa

| Termo | Descricao |
|-------|-----------|
| **Vertex** | Ponto 2D (x, y) em coordenadas do mapa |
| **Linedef** | Linha entre 2 vertices; pode ter 1 lado (parede solida) ou 2 lados (portal) |
| **Sidedef** | Lado de uma linedef; referencia texturas (upper, middle, lower) e o sector |
| **Sector** | Area poligonal com altura de piso/teto, nivel de luz e tipo especial |
| **Subsector** | Subdivisao convexa de um sector, gerada pelo BSP builder |
| **Seg** | Segmento de linedef dentro de um subsector |
| **Node** | No da BSP tree; contem uma partition line que divide o espaco em dois |
| **BSP** | Binary Space Partition — arvore binaria que divide o mapa para rendering eficiente |
| **Blockmap** | Grid regular sobreposta ao mapa para deteccao rapida de colisao |
| **Reject table** | Matriz pre-calculada de visibilidade entre pares de sectors |

### Rendering

| Termo | Descricao |
|-------|-----------|
| **Visplane** | Superficie horizontal (piso ou teto) visivel na tela |
| **Drawseg** | Registro de um segmento de parede durante o rendering |
| **Clip range** | Faixa angular de colunas da tela ja preenchidas |
| **Column** | Faixa vertical de pixels; DOOM desenha paredes coluna por coluna |
| **Span** | Faixa horizontal de pixels; usado para pisos e tetos |
| **Occlusion** | Processo de marcar partes da tela como ja desenhadas |
| **Front-to-back** | Ordem de rendering: paredes proximas primeiro, depois distantes |

### Game Logic

| Termo | Descricao |
|-------|-----------|
| **Thinker** | Qualquer entidade que executa logica a cada tic (mobj, teto movel, etc) |
| **Mobj** | Map Object — entidade no mapa (jogador, monstro, projetil, item) |
| **State** | Frame de animacao de um mobj; definido em info.c como state_t |
| **Tic** | Unidade de tempo do jogo: 1/35 de segundo (28.57ms) |
| **Thing** | Definicao estatica de um tipo de mobj (posicao, tipo, flags) |
| **Ticcmd** | Comando de input de um jogador para um tic (movimento, tiro, uso) |
| **Gametic** | Contador global de tics desde o inicio do jogo |

### Matematica

| Termo | Descricao |
|-------|-----------|
| **Fixed-point** | Formato numerico 16.16 bits (i32): 16 bits inteiros + 16 fracionarios |
| **fixed_t** | Tipo C original para fixed-point: `typedef int fixed_t` |
| **FRACBITS** | Numero de bits fracionarios: 16 |
| **FRACUNIT** | Valor 1.0 em fixed-point: 65536 (1 << 16) |
| **BAM** | Binary Angle Measurement — angulo como u32 (0 = 0 graus, 0xFFFFFFFF ~ 360) |
| **Fine angle** | Indice na tabela trigonometrica de 8192 entradas |

### Subsistemas (prefixos do codigo C)

| Prefixo | Subsistema |
|---------|------------|
| `r_` | Rendering (renderizacao) |
| `p_` | Play/game logic (logica de jogo) |
| `d_` | DOOM main (inicializacao, loop principal) |
| `i_` | Implementation (interface com o SO/hardware) |
| `w_` | WAD (sistema de arquivos) |
| `s_` | Sound (logica de audio) |
| `m_` | Misc (utilidades, menus, matematica) |
| `g_` | Game (fluxo de jogo) |
| `v_` | Video (operacoes de framebuffer) |
| `z_` | Zone (gerenciamento de memoria) |
| `f_` | Finale (telas de transicao) |
| `hu_` | Heads-Up (display overlay) |
| `st_` | Status bar |
| `wi_` | Wipe/Intermission (tela entre niveis) |
| `am_` | Automap |

## Termos Rust Usados no Port

| Termo | Descricao no contexto do projeto |
|-------|-----------|
| **Newtype** | Struct wrapper (ex: `struct Fixed(i32)`) para seguranca de tipos |
| **Trait** | Interface Rust; usado para Thinker e outros comportamentos polimorficos |
| **Enum dispatch** | Usar enum ao inves de trait object para polimorfismo sem alocacao |
| **Ownership** | Modelo de posse de memoria do Rust; substitui o z_zone.c do DOOM |
| **Lifetime** | Anotacao de tempo de vida de referencias; garante seguranca de memoria |
| **Result<T,E>** | Tipo para operacoes que podem falhar; substitui codigos de erro do C |
| **#[inline]** | Hint para o compilador inlinear funcoes pequenas (como operadores Fixed) |
| **wrapping_add** | Soma com overflow intencional; usado em angulos BAM |

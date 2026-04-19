---
name: doom-conventions
description: >
  Convencoes e terminologia do engine DOOM. Carrega automaticamente
  quando o contexto envolve structs do DOOM, formatos de arquivo WAD,
  BSP trees, rendering, ou qualquer subsistema especifico do engine.
---

## Terminologia DOOM

### Tipos Fundamentais
- **WAD** (Where's All the Data): arquivo container com todos os assets
- **IWAD**: WAD principal do jogo (doom.wad, freedoom.wad)
- **PWAD**: WAD de patch/mod que sobrescreve lumps do IWAD
- **Lump**: entrada individual dentro de um WAD (textura, mapa, som)
- **Fixed-point** (fixed_t): inteiro 16.16 bits para matematica fracionaria

### Geometria de Mapa
- **Vertex**: ponto 2D (x,y) em coordenadas do mapa
- **Linedef**: linha entre 2 vertices, pode ter 1 ou 2 lados
- **Sidedef**: lado de uma linedef, referencia texturas e sector
- **Sector**: poligono convexo com floor/ceiling height e light level
- **Subsector** (ssector): subdivisao convexa de um sector
- **Seg**: segmento de linedef dentro de um subsector
- **Node**: no da BSP tree que divide o mapa em dois half-spaces
- **Blockmap**: grid para deteccao rapida de colisao
- **Reject table**: lookup otimizado de line-of-sight entre sectors

### Rendering
- **Visplane**: superficie horizontal (floor/ceiling) visivel
- **Drawseg**: segmento de parede sendo desenhado
- **Clip range**: faixa angular de colunas de tela ja preenchidas
- **Column rendering**: DOOM desenha paredes coluna por coluna
- **Sprite**: coisa (thing) renderizada como billboard 2D

### Game Logic
- **Thinker**: qualquer entidade com update por tick (mobj, ceiling, etc)
- **Mobj** (map object): coisa no mapa — jogador, monstro, item
- **State**: frame de animacao de um mobj (definido em info.c/h)
- **Tic**: unidade de tempo do jogo (1/35 segundo)
- **Thing**: definicao estatica de um tipo de mobj

### Arquivos-Fonte Chave (C Original)
| Arquivo       | Subsistema                              |
|---------------|-----------------------------------------|
| `w_wad.c`     | Carregamento e cache de WAD             |
| `r_main.c`    | Loop principal de rendering             |
| `r_bsp.c`     | Traversal da BSP tree                   |
| `r_segs.c`    | Renderizacao de segmentos (paredes)     |
| `r_plane.c`   | Renderizacao de visplanes (pisos/tetos) |
| `r_things.c`  | Renderizacao de sprites                 |
| `p_mobj.c`    | Spawn e update de map objects           |
| `p_map.c`     | Movimento e colisao                     |
| `p_tick.c`    | Game tick e thinker list                |
| `p_setup.c`   | Carregamento de nivel                   |
| `d_main.c`    | Inicializacao e loop principal          |
| `i_video.c`   | Interface com hardware de video         |
| `i_sound.c`   | Interface com hardware de audio         |
| `s_sound.c`   | Logica de som (espacializacao)          |
| `m_fixed.c`   | Matematica fixed-point                  |
| `info.c/h`    | Tabelas de estados e tipos de things    |

# Arquitetura do Engine DOOM (1993)

## Visao Geral

O DOOM e composto por ~54.600 linhas de C distribuidas em 50 arquivos .c
e 88 headers .h. O codigo esta organizado em 9 subsistemas principais.

## Estatisticas

| Subsistema        | Arquivos .c | LOC aprox. | Complexidade |
|-------------------|-------------|------------|--------------|
| Game Logic (p_*)  | 19          | 11.800     | Alta         |
| Rendering (r_*)   | 8           | 5.500      | Alta         |
| Menu/HUD          | 9           | 8.300      | Media        |
| Main/Init (d_,i_) | 8           | 3.500      | Media        |
| Data (info.c)     | 1           | 4.670      | Baixa (dados)|
| Tables (tables.c) | 1           | 2.130      | Baixa (dados)|
| Game Flow (g_*)   | 1           | 1.690      | Media        |
| Sound (s_*)       | 2           | 1.100      | Media        |
| Video (v_*)       | 1           | 493        | Baixa        |
| WAD (w_*)         | 1           | 577        | Baixa        |
| Memory (z_*)      | 1           | 467        | Media        |
| Utility (m_*)     | 8           | 1.000      | Baixa        |

## Diagrama de Dependencias

```
                    +------------------+
                    |    i_main.c      |  Ponto de entrada
                    +--------+---------+
                             |
                    +--------v---------+
                    |    d_main.c      |  Inicializacao e loop principal
                    +--------+---------+
                             |
            +----------------+----------------+
            |                |                |
   +--------v------+  +-----v------+  +------v-------+
   |   g_game.c    |  |  d_net.c   |  |  m_menu.c    |
   | Game flow     |  | Networking |  | Menu system  |
   +-------+-------+  +-----+------+  +------+-------+
           |                 |                |
     +-----v------+         |         +------v-------+
     | p_setup.c  |         |         | hu/st/wi/f_* |
     | Map loader |         |         | HUD, screens |
     +-----+------+         |         +--------------+
           |                 |
   +-------v-----------------v--------+
   |        GAME LOGIC (p_*.c)        |
   | p_mobj  p_map   p_enemy  p_inter |
   | p_tick  p_spec  p_doors  p_floor |
   | p_pspr  p_sight p_maputl ...     |
   +-------+------+-------------------+
           |      |
   +-------v------v-------------------+
   |       RENDERING (r_*.c)          |
   | r_main -> r_bsp -> r_segs       |
   |                 -> r_plane       |
   |        r_things  r_draw  r_data  |
   +-------+------+-------------------+
           |      |
   +-------v------v-------------------+
   |     SUPORTE / INFRAESTRUTURA     |
   | w_wad.c    - WAD file access     |
   | v_video.c  - Framebuffer ops     |
   | z_zone.c   - Memory management   |
   | s_sound.c  - Sound logic         |
   | i_video.c  - SDL/platform video  |
   | i_sound.c  - SDL/platform audio  |
   +----------------------------------+
           |
   +-------v--------------------------+
   |       TIPOS BASE (utils)         |
   | m_fixed.c  - Fixed-point math    |
   | tables.c   - Trig lookup tables  |
   | m_bbox.c   - Bounding boxes      |
   | m_random.c - RNG deterministico  |
   | m_swap.c   - Byte order          |
   | m_argv.c   - Command line args   |
   | m_misc.c   - File I/O, config    |
   +----------------------------------+
           |
   +-------v--------------------------+
   |    DEFINICOES / DADOS            |
   | doomdef.h   - Tipos e constantes |
   | doomstat.h  - Estado global (60+)|
   | doomdata.h  - Formatos de mapa   |
   | doomtype.h  - Tipos basicos      |
   | info.c/h    - Sprites e estados  |
   | tables.c/h  - Tabelas trig      |
   +----------------------------------+
```

## Ordem Recomendada de Port

A ordem segue o principio "das folhas para a raiz" — primeiro os
modulos sem dependencias, depois os que dependem deles.

```
Fase 0: utils/fixed, utils/angle, utils/bbox  [CONCLUIDO]
  |
Fase 1: wad (w_wad.c)
  |
Fase 2: m_random, tables completas, m_misc
  |
Fase 3: map (p_setup.c + doomdata.h structs)
  |
Fase 4: video (i_video.c, v_video.c, paletas)
  |
Fase 5: renderer (r_main -> r_bsp -> r_segs -> r_plane -> r_things -> r_draw -> r_data)
  |
Fase 6: game loop (d_main.c, g_game.c, input)
  |
Fase 7: things e colisao (p_mobj, p_map, p_inter, p_enemy, p_tick)
  |
Fase 8: audio (s_sound.c, i_sound.c)
  |
Fase 9: menus e HUD (m_menu, hu_stuff, st_stuff, wi_stuff)
  |
Fase 10: polish (networking, save/load, automap, demos)
```

## Subsistemas Detalhados

### Rendering (r_*.c) — 5.500 LOC

O renderer e o coracao tecnico do DOOM. Funciona assim:

1. `r_main.c` configura o frame (viewpoint, angulos, clipping)
2. `r_bsp.c` percorre a BSP tree front-to-back
3. Para cada subsector visivel:
   - `r_segs.c` desenha segmentos de parede (coluna por coluna)
   - `r_plane.c` registra visplanes (pisos/tetos) para desenho posterior
4. `r_things.c` coleta e ordena sprites visiveis
5. `r_draw.c` contem os drawers de baixo nivel (colunas e spans)
6. `r_data.c` gerencia texturas, patches e sprites do WAD

Arquivos por tamanho:
- r_things.c: 989 linhas (sprites, sorting)
- r_main.c: 898 linhas (setup, viewpoint)
- r_draw.c: 877 linhas (column/span renderers)
- r_data.c: 849 linhas (texture management)
- r_segs.c: 746 linhas (wall segments)
- r_bsp.c: 580 linhas (BSP traversal)
- r_plane.c: 453 linhas (visplanes)
- r_sky.c: 62 linhas (sky rendering)

### Game Logic (p_*.c) — 11.800 LOC

Maior subsistema. Gerencia tudo que "acontece" no jogo:

- **p_enemy.c** (2008 LOC): IA dos monstros — o maior arquivo de logica
- **p_spec.c** (1362 LOC): Efeitos especiais (portas, elevadores, triggers)
- **p_map.c** (1339 LOC): Movimento e deteccao de colisao
- **p_mobj.c** (988 LOC): Spawn, update e remocao de objetos do mapa
- **p_inter.c** (918 LOC): Interacoes (dano, pickups, kills)
- **p_maputl.c** (883 LOC): Utilidades geometricas (line crossing, etc)
- **p_pspr.c** (879 LOC): Armas do jogador (player sprites)
- **p_setup.c** (708 LOC): Carregamento de nivel a partir do WAD
- **p_doors.c** (764 LOC): Logica de portas
- **p_saveg.c** (586 LOC): Save/load game
- **p_floor.c** (555 LOC): Pisos moveis
- **p_switch.c** (654 LOC): Switches e botoes
- **p_user.c** (386 LOC): Controle do jogador
- **p_sight.c** (349 LOC): Line-of-sight entre objetos
- **p_lights.c** (357 LOC): Efeitos de luz
- **p_ceilng.c** (335 LOC): Tetos moveis
- **p_plats.c** (314 LOC): Plataformas
- **p_tick.c** (158 LOC): Game tick e lista de thinkers
- **p_telept.c** (132 LOC): Teleportadores

### WAD System (w_wad.c) — 577 LOC

Sistema de arquivos virtual do DOOM. Depende apenas de:
- doomtype.h, m_swap.h, i_system.h, z_zone.h

Funcoes principais:
- `W_AddFile()`: Abre e registra um WAD
- `W_CheckNumForName()`: Busca lump por nome (hash)
- `W_GetNumForName()`: Busca lump (fatal se nao encontrar)
- `W_CacheLumpNum()`: Carrega lump em memoria
- `W_CacheLumpName()`: Wrapper por nome

### Estado Global (doomstat.h) — 60+ variaveis

O DOOM usa extensivamente variaveis globais. As principais categorias:
- **Configuracao**: gamemode, gamemission, language
- **Estado do jogo**: gameskill, gameepisode, gamemap, gametic
- **Jogadores**: players[4], playeringame[4], consoleplayer
- **Display**: viewwidth, viewheight, automapactive, menuactive
- **Som**: snd_SfxVolume, snd_MusicVolume
- **Networking**: netgame, deathmatch, doomcom
- **Demo**: demoplayback, demorecording

No port Rust, estas globals serao encapsuladas em structs de contexto
passadas por referencia.

//! # Modulo Renderer — Renderizacao por Software
//!
//! Implementa o renderizador software do DOOM: travessia BSP,
//! renderizacao de paredes (column-based), visplanes (pisos/tetos),
//! sprites e drawers de colunas/spans.
//!
//! ## Arquivos C originais
//! - `r_main.c` — Setup de frame e loop de rendering
//! - `r_bsp.c` — BSP traversal
//! - `r_segs.c` — Segmentos de parede
//! - `r_plane.c` — Visplanes (pisos e tetos)
//! - `r_things.c` — Sprites
//! - `r_draw.c` — Column/span drawers
//! - `r_data.c` — Texturas e patches

//! # Modulo Renderer — Renderizacao por Software
//!
//! Implementa o renderizador software do DOOM: travessia BSP,
//! renderizacao de paredes (column-based), visplanes (pisos/tetos),
//! sprites e drawers de colunas/spans.
//!
//! ## Pipeline de Rendering
//!
//! ```text
//! R_RenderPlayerView()
//!   |
//!   +-> R_SetupFrame()       — prepara POV, sin/cos, contadores
//!   +-> R_ClearClipSegs()    — limpa solid segments
//!   +-> R_ClearDrawSegs()    — limpa drawsegs
//!   +-> R_ClearPlanes()      — limpa visplanes
//!   +-> R_ClearSprites()     — limpa vissprites
//!   +-> R_RenderBSPNode()    — travessia BSP (front-to-back)
//!   |     |
//!   |     +-> R_Subsector()  — para cada folha BSP
//!   |           +-> R_AddLine()          — clippa segs
//!   |           +-> R_StoreWallRange()   — renderiza paredes
//!   |           +-> R_AddSprites()       — coleta sprites
//!   |
//!   +-> R_DrawPlanes()       — renderiza pisos e tetos acumulados
//!   +-> R_DrawMasked()       — renderiza sprites (back-to-front)
//! ```
//!
//! ## Submodulos
//!
//! - [`draw`] — Primitivas de desenho: colunas verticais e spans horizontais
//! - [`data`] — Carregamento de texturas, patches e colormaps do WAD
//! - [`state`] — Estado do renderer: POV, projecao, tabelas de luz
//! - [`bsp`] — Travessia da arvore BSP e clipping com solid segments
//! - [`segs`] — Drawsegs: segmentos de parede clippados para rendering
//! - [`plane`] — Visplanes: pisos e tetos (rendering deferido)
//! - [`things`] — Sprites (vissprites) e sky
//!
//! ## Arquivos C originais
//! - `r_main.c` — Setup de frame e loop de rendering
//! - `r_bsp.c` — BSP traversal
//! - `r_segs.c` — Segmentos de parede
//! - `r_plane.c` — Visplanes (pisos e tetos)
//! - `r_things.c` — Sprites
//! - `r_draw.c` — Column/span drawers
//! - `r_data.c` — Texturas e patches
//! - `r_sky.c` — Rendering do ceu

pub mod bsp;
pub mod data;
pub mod draw;
pub mod plane;
pub mod segs;
pub mod state;
pub mod things;

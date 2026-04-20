//! # Modulo Menu — Menus, HUD, Status Bar e Telas Especiais
//!
//! Gerencia toda a interface do usuario do DOOM:
//! menus navegaveis, heads-up display, status bar, automap,
//! tela de intermissao entre niveis, e sequencias de finale.
//!
//! ## Camadas de UI (ordem de desenho)
//!
//! ```text
//! 7. M_Drawer     — Menu (camada mais acima)
//! 6. HU_Drawer    — Mensagens e chat
//! 5. ST_Drawer    — Status bar (barra inferior)
//! 4. AM_Drawer    — Automap (substitui vista do jogo)
//! 3. Vista do jogo — Renderer 3D
//! 2. WI_Drawer    — Intermissao (substitui tudo)
//! 1. F_Drawer     — Finale (substitui tudo)
//! ```
//!
//! ## Responder chain
//!
//! Eventos de input sao processados em cadeia (primeiro que
//! consumir vence): M_Responder → HU_Responder →
//! ST_Responder → AM_Responder → G_Responder
//!
//! ## Submodulos
//!
//! - [`hud_widgets`] — Widgets de texto: HudTextLine, HudScrollText, HudInputText
//! - [`hud`] — Heads-up display: mensagens, titulo, chat
//! - [`st_widgets`] — Widgets da status bar: StNumber, StPercent, StMultIcon, StBinIcon
//! - [`statusbar`] — Status bar: face, paletas, ammo/health/armor
//! - [`menu`] — Sistema de menus: navegacao, skull cursor, modais
//! - [`intermission`] — Tela de intermissao e finale
//! - [`automap`] — Mapa automatico do nivel
//!
//! ## Arquivos C originais
//! - `m_menu.c` — Sistema de menus
//! - `hu_stuff.c` / `hu_lib.c` — HUD e widgets de texto
//! - `st_stuff.c` / `st_lib.c` — Status bar e widgets numericos
//! - `wi_stuff.c` — Tela de intermissao
//! - `f_finale.c` — Sequencias de finale
//! - `am_map.c` — Automap

pub mod automap;
pub mod hud;
pub mod hud_widgets;
pub mod intermission;
pub mod navigation;
pub mod st_widgets;
pub mod statusbar;

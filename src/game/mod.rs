//! # Modulo Game — Loop Principal, Input, Estado, Objetos e Colisao
//!
//! Gerencia todo o runtime do jogo DOOM: o game loop a 35 Hz,
//! processamento de input, maquina de estados, sistema de thinkers,
//! map objects (mobjs), e mecanica de colisao.
//!
//! ## Arquitetura do game loop
//!
//! ```text
//! D_DoomLoop() — loop infinito
//!   |
//!   +-> I_StartTic()         — plataforma le input (SDL2)
//!   +-> D_ProcessEvents()    — despacha eventos para responders
//!   +-> G_BuildTiccmd()      — converte teclas/mouse em ticcmd
//!   +-> G_Ticker()           — executa um tick logico
//!   |     |
//!   |     +-> process_action()       — transicoes de estado
//!   |     +-> P_RunThinkers()        — atualiza todos os mobjs
//!   |     +-> P_CheckPosition()      — colisao via blockmap
//!   |
//!   +-> S_UpdateSounds()     — posiciona audio 3D
//!   +-> D_Display()          — renderiza frame
//! ```
//!
//! ## Submodulos
//!
//! - [`events`] — Tipos de eventos, TicCmd, constantes de botoes e teclas
//! - [`state`] — Maquina de estados do jogo, GameState, enums
//! - [`tick`] — Sistema de timing e execucao do game loop
//! - [`input`] — Mapeamento de input, G_BuildTiccmd, key bindings
//! - [`thinker`] — Sistema de thinkers (objetos que "pensam")
//! - [`info`] — MobjFlags, MobjInfo, State, tipos e constantes
//! - [`mobj`] — Map objects: entidades do mundo (jogador, monstros, etc.)
//! - [`maputil`] — Blockmap, line opening, reject table
//! - [`movement`] — Colisao e movimento (P_TryMove, P_CheckPosition)
//!
//! ## Arquivos C originais
//! - `d_main.c` — Inicializacao e loop principal
//! - `d_event.h` / `d_ticcmd.h` — Eventos e comandos de tick
//! - `d_net.c` — TryRunTics, scheduling
//! - `g_game.c` — G_Ticker, G_BuildTiccmd, G_Responder
//! - `p_tick.c` — Thinker system, P_Ticker
//! - `p_mobj.c` / `p_mobj.h` — Map objects
//! - `p_map.c` — Colisao e movimento
//! - `p_maputl.c` — Blockmap, utilidades de mapa
//! - `info.h` / `info.c` — Tabelas de estados e mobjinfo

pub mod events;
pub mod info;
pub mod input;
pub mod maputil;
pub mod mobj;
pub mod movement;
pub mod state;
pub mod thinker;
pub mod tick;

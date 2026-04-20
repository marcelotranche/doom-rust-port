//! # Modulo Game — Loop Principal, Input e Estado
//!
//! Gerencia o game loop, processamento de input, e maquina de estados
//! do jogo DOOM. Este modulo e o coracao do engine: coordena a
//! execucao de ticks logicos a 35 Hz, converte input de plataforma
//! em comandos de jogo, e gerencia transicoes entre estados
//! (gameplay, intermission, finale, demo screen).
//!
//! ## Pipeline de um tick
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
//!   |     +-> check_special_buttons() — pause, save
//!   |     +-> P_Ticker()             — fisica e thinkers (GS_LEVEL)
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
//!
//! ## Arquivos C originais
//! - `d_main.c` — Inicializacao e loop principal (D_DoomLoop)
//! - `d_event.h` — Tipos de eventos e botoes
//! - `d_ticcmd.h` — Estrutura ticcmd_t
//! - `d_net.c` — TryRunTics, scheduling de ticks
//! - `g_game.c` — G_Ticker, G_BuildTiccmd, G_Responder
//! - `doomdef.h` — Constantes (TICRATE, MAXPLAYERS, etc.)

pub mod events;
pub mod input;
pub mod state;
pub mod tick;

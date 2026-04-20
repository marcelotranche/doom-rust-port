//! # DOOM Rust — Port Educacional
//!
//! Este crate implementa o engine do DOOM (1993, id Software) em Rust
//! idiomatico, servindo como material didatico para aprendizado
//! simultaneo de arquitetura de game engines classicos e da linguagem Rust.
//!
//! ## Organizacao dos Modulos
//!
//! Os modulos seguem a mesma separacao do codigo C original:
//!
//! - [`utils`] — Tipos fundamentais: fixed-point math, angulos, bounding boxes
//! - [`wad`] — Carregamento e acesso a arquivos WAD (Where's All the Data)
//! - [`map`] — Geometria de mapa: BSP tree, sectors, linedefs
//! - [`renderer`] — Renderizacao por software: paredes, pisos, sprites
//! - [`video`] — Framebuffer, paletas e interface com SDL2
//! - [`game`] — Game loop, thinkers, logica do jogador
//! - [`sound`] — Subsistema de audio e musica
//! - [`menu`] — Menus, HUD e telas de intermissao
//! - [`net`] — Networking para multiplayer
//!
//! ## Codigo C Original
//! Fonte primaria: Linuxdoom 1.10 (`references/DOOM-master/linuxdoom-1.10/`)

pub mod args;
pub mod engine;
pub mod game;
pub mod map;
pub mod menu;
pub mod net;
pub mod renderer;
pub mod sound;
pub mod utils;
pub mod video;
pub mod wad;

//! # Modulo Sound — Audio e Musica
//!
//! Gerencia todo o audio do DOOM: efeitos sonoros (SFX) com
//! espacializacao 3D e musica de fundo (formato MUS/MIDI).
//!
//! ## Arquitetura de audio do DOOM
//!
//! ```text
//! Game Layer (S_*)          Platform Layer (I_*)
//! +-----------------+       +------------------+
//! | S_StartSound()  | ----> | I_StartSound()   |
//! | S_ChangeMusic() | ----> | I_PlaySong()     |
//! | S_UpdateSounds()| ----> | I_UpdateSound()  |
//! +-----------------+       +------------------+
//!        |                         |
//!   espacializacao           SDL2 / hardware
//!   prioridade               mixer real
//!   atenuacao
//! ```
//!
//! O game layer calcula volume, separacao estereo e prioridade.
//! O platform layer faz o mixing real via SDL2.
//!
//! ## Submodulos
//!
//! - [`types`] — Metadados de SFX/musica, constantes, tabelas
//! - [`system`] — Sistema de SFX: canais, espacializacao, mixer
//! - [`music`] — Sistema de musica: reproducao, pausa, formato MUS
//!
//! ## Arquivos C originais
//! - `s_sound.c` — Game layer de audio (espacializacao, prioridade)
//! - `i_sound.c` — Platform layer (interface com hardware)
//! - `sounds.h` / `sounds.c` — Tabelas de metadados de SFX e musica

pub mod music;
pub mod system;
pub mod types;

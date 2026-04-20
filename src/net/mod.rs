//! # Modulo Net — Networking Multiplayer
//!
//! Implementa o protocolo de rede do DOOM para multiplayer:
//! sincronizacao lockstep deterministico de inputs via UDP.
//!
//! ## Arquitetura de rede
//!
//! ```text
//! Setup Layer                  Game Layer           Transport Layer
//! +------------------+         +---------------+    +----------------+
//! | D_CheckNetGame() |         | NetUpdate()   |    | serialize()    |
//! | D_ArbitrateNet() | ------> | TryRunTics()  | -> | PacketSend()   |
//! | GameConfig       |         | GetPackets()  | <- | PacketGet()    |
//! +------------------+         +---------------+    +----------------+
//!       |                           |                     |
//!   handshake                  lockstep              UDP sockets
//!   config broadcast           ring buffer           byte-swapping
//!   player detection           retransmissao         non-blocking
//! ```
//!
//! ## Modelo de sincronizacao
//!
//! O DOOM usa lockstep determinístico: todos os jogadores
//! executam os mesmos tics com os mesmos inputs. A rede
//! sincroniza apenas os inputs (ticcmds), nao o estado.
//! O jogo so avanca quando tem inputs de TODOS os jogadores.
//!
//! ## Submodulos
//!
//! - [`types`] — Tipos de pacotes, constantes, DoomData/DoomCom, NetTicCmd
//! - [`sync`] — Sincronizacao lockstep: nettics, maketic, gametic, ring buffers
//! - [`transport`] — Serializacao de pacotes, trait NetTransport, loopback
//! - [`setup`] — Handshake de setup, GameConfig, arbitragem multiplayer
//!
//! ## Arquivos C originais
//! - `d_net.c` / `d_net.h` — Protocolo de rede, sincronizacao
//! - `i_net.c` / `i_net.h` — Sockets UDP, byte-swapping

pub mod setup;
pub mod sync;
pub mod transport;
pub mod types;

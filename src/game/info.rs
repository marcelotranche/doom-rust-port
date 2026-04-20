//! # Informacoes de Objetos — Estados, Tipos e Flags
//!
//! Define os tipos fundamentais do sistema de objetos do DOOM:
//!
//! - **MobjFlags**: flags de propriedades (solido, disparavel, etc.)
//! - **MobjType**: tipo do objeto (jogador, imp, barril, etc.)
//! - **StateNum**: estado de animacao (andar, atacar, morrer, etc.)
//! - **State**: definicao de um frame de animacao
//! - **MobjInfo**: template de propriedades para cada tipo de objeto
//! - **SpriteNum**: indice de sprite
//!
//! ## Tabelas geradas
//!
//! No DOOM original, as tabelas de estados e mobjinfo sao geradas
//! pela ferramenta `multigen` e contidas em `info.c` (>6000 linhas).
//! Aqui definimos os tipos e uma selecao representativa para
//! fins educacionais. A tabela completa sera carregada do WAD
//! ou gerada em fases futuras.
//!
//! ## Arquivo C original: `info.h`, `info.c`, `p_mobj.h` (mobjflag_t)
//!
//! ## Conceitos que o leitor vai aprender
//! - Bitflags para propriedades de objetos
//! - State machines para animacao de sprites
//! - Data-driven design: comportamento definido por tabelas, nao codigo

use bitflags::bitflags;

use crate::utils::fixed::{Fixed, FRACUNIT};

// ---------------------------------------------------------------------------
// Mobj Flags
// ---------------------------------------------------------------------------

bitflags! {
    /// Flags de propriedades de um mobj.
    ///
    /// Cada flag controla um aspecto do comportamento do objeto:
    /// colisao, rendering, IA, fisica, etc.
    ///
    /// C original: `mobjflag_t` em `p_mobj.h`
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MobjFlags: u32 {
        /// Chamar P_SpecialThing ao tocar (itens coletaveis).
        const SPECIAL      = 0x0000_0001;
        /// Bloqueia movimento de outros objetos.
        const SOLID        = 0x0000_0002;
        /// Pode receber dano.
        const SHOOTABLE    = 0x0000_0004;
        /// Nao usar sector links (invisivel mas tocavel).
        const NOSECTOR     = 0x0000_0008;
        /// Nao usar blockmap links (inerte mas visivel).
        const NOBLOCKMAP   = 0x0000_0010;
        /// Monstro surdo — nao ativado por som.
        const AMBUSH       = 0x0000_0020;
        /// Acabou de ser atingido — vai contra-atacar.
        const JUSTHIT      = 0x0000_0040;
        /// Acabou de atacar — espera antes de atacar de novo.
        const JUSTATTACKED = 0x0000_0080;
        /// Spawna no teto em vez do chao.
        const SPAWNCEILING = 0x0000_0100;
        /// Sem gravidade — flutua na altura atual.
        const NOGRAVITY    = 0x0000_0200;
        /// Permite pular de alturas.
        const DROPOFF      = 0x0000_0400;
        /// Jogador pode coletar itens.
        const PICKUP       = 0x0000_0800;
        /// Noclip — atravessa paredes (cheat).
        const NOCLIP       = 0x0000_1000;
        /// Desliza ao longo de paredes (jogador).
        const SLIDE        = 0x0000_2000;
        /// Pode flutuar ativamente (cacodemon, pain elemental).
        const FLOAT        = 0x0000_4000;
        /// Nao verifica alturas em teleportes.
        const TELEPORT     = 0x0000_8000;
        /// E um projetil — explode ao atingir algo.
        const MISSILE      = 0x0001_0000;
        /// Droppado por monstro (nao spawned no mapa).
        const DROPPED      = 0x0002_0000;
        /// Usa efeito fuzz (spectre, invisibilidade).
        const SHADOW       = 0x0004_0000;
        /// Nao sangra ao ser atingido (usa puff).
        const NOBLOOD      = 0x0008_0000;
        /// Corpo morto — desliza ate parar.
        const CORPSE       = 0x0010_0000;
        /// Flutuando para atingir uma altura de ataque.
        const INFLOAT      = 0x0020_0000;
        /// Conta como kill na tela de intermissao.
        const COUNTKILL    = 0x0040_0000;
        /// Conta como item na tela de intermissao.
        const COUNTITEM    = 0x0080_0000;
        /// Skull em voo (lost soul charge).
        const SKULLFLY     = 0x0100_0000;
        /// Nao spawnar em deathmatch (chaves, etc.).
        const NOTDMATCH    = 0x0200_0000;
        /// Bits de translacao de cor para sprites de jogador.
        /// Valores 0x4000000, 0x8000000, 0xC000000 mapeiam para
        /// tabelas de translacao de cor (verde→cinza/marrom/vermelho).
        const TRANSLATION  = 0x0C00_0000;
    }
}

/// Shift para extrair bits de translacao dos MobjFlags.
///
/// C original: `MF_TRANSSHIFT = 26` em `p_mobj.h`
pub const MF_TRANSSHIFT: u32 = 26;

// ---------------------------------------------------------------------------
// Sprites
// ---------------------------------------------------------------------------

/// Indice de sprite — referencia na tabela de nomes de sprites.
///
/// C original: `spritenum_t` em `info.h`
pub type SpriteNum = usize;

/// Numero total de sprites definidos no DOOM.
///
/// C original: `NUMSPRITES` em `info.h`
pub const NUMSPRITES: usize = 138;

// ---------------------------------------------------------------------------
// Estados (state machine)
// ---------------------------------------------------------------------------

/// Indice de estado na tabela global de estados.
///
/// C original: `statenum_t` em `info.h`
pub type StateNum = usize;

/// Estado nulo — indica "sem estado" ou "estado invalido".
pub const S_NULL: StateNum = 0;

/// Numero total de estados definidos no DOOM.
///
/// C original: `NUMSTATES` em `info.h`
pub const NUMSTATES: usize = 967;

/// Flag de frame: sprite com brilho total (ignora iluminacao do sector).
///
/// C original: `#define FF_FULLBRIGHT 0x8000` em `info.h`
pub const FF_FULLBRIGHT: i32 = 0x8000;
/// Mascara para extrair o numero do frame real.
///
/// C original: `#define FF_FRAMEMASK 0x7FFF` em `info.h`
pub const FF_FRAMEMASK: i32 = 0x7FFF;

/// Definicao de um estado de animacao.
///
/// Cada estado descreve um frame de animacao de um sprite:
/// qual sprite usar, qual frame, por quantos ticks, e qual
/// o proximo estado. Opcionalmente, uma funcao de acao e
/// chamada ao entrar no estado.
///
/// C original: `state_t` em `info.h`
#[derive(Debug, Clone)]
pub struct State {
    /// Indice do sprite a usar
    pub sprite: SpriteNum,
    /// Frame do sprite (pode ter FF_FULLBRIGHT ORed)
    pub frame: i32,
    /// Duracao em ticks (-1 = infinito)
    pub tics: i32,
    /// Indice da funcao de acao (0 = nenhuma).
    /// No C original e um function pointer. Aqui usamos
    /// um indice em uma tabela de action functions.
    pub action: usize,
    /// Proximo estado ao terminar os tics
    pub next_state: StateNum,
    /// Parametros auxiliares para IA
    pub misc1: i32,
    pub misc2: i32,
}

impl State {
    /// Cria um estado vazio.
    pub fn new() -> Self {
        State {
            sprite: 0,
            frame: 0,
            tics: -1,
            action: 0,
            next_state: S_NULL,
            misc1: 0,
            misc2: 0,
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tipos de mobj
// ---------------------------------------------------------------------------

/// Tipo de mobj — identifica o "species" do objeto.
///
/// C original: `mobjtype_t` em `info.h`
pub type MobjType = usize;

/// Jogador
pub const MT_PLAYER: MobjType = 0;
/// Zombie (Former Human)
pub const MT_POSSESSED: MobjType = 1;
/// Shotgun Guy
pub const MT_SHOTGUY: MobjType = 2;
/// Arch-vile
pub const MT_VILE: MobjType = 3;
/// Imp
pub const MT_TROOP: MobjType = 9;
/// Demon (Pinky)
pub const MT_SERGEANT: MobjType = 10;
/// Cacodemon
pub const MT_HEAD: MobjType = 19;
/// Baron of Hell
pub const MT_BRUISER: MobjType = 20;
/// Cyberdemon
pub const MT_CYBORG: MobjType = 23;
/// Spider Mastermind
pub const MT_SPIDER: MobjType = 24;
/// Barril explosivo
pub const MT_BARREL: MobjType = 33;
/// Projetil do jogador (pistol/chaingun)
pub const MT_PUFF: MobjType = 36;
/// Projetil de sangue (hitmarker)
pub const MT_BLOOD: MobjType = 37;
/// Projetil do Imp
pub const MT_TROOPSHOT: MobjType = 38;
/// Rocket do jogador
pub const MT_ROCKET: MobjType = 34;
/// Plasma do jogador
pub const MT_PLASMA: MobjType = 35;
/// BFG ball
pub const MT_BFG: MobjType = 42;

/// Numero total de tipos de mobj.
///
/// C original: `NUMMOBJTYPES` em `info.h`
pub const NUMMOBJTYPES: usize = 137;

// ---------------------------------------------------------------------------
// MobjInfo — template de propriedades por tipo
// ---------------------------------------------------------------------------

/// Informacoes estaticas de um tipo de mobj.
///
/// Define as propriedades padrao para cada tipo de objeto:
/// vida, velocidade, tamanho, sons, estados de animacao, etc.
/// Funciona como um "template" — ao spawnar um mobj, seus
/// valores iniciais sao copiados da mobjinfo correspondente.
///
/// C original: `mobjinfo_t` em `info.h`
#[derive(Debug, Clone)]
pub struct MobjInfo {
    /// Numero no editor de mapas (DoomEd number)
    pub doomednum: i32,
    /// Estado inicial ao spawnar
    pub spawn_state: StateNum,
    /// Vida inicial
    pub spawn_health: i32,
    /// Estado ao ver o jogador
    pub see_state: StateNum,
    /// Som ao ver o jogador
    pub see_sound: i32,
    /// Tempo de reacao antes de agir
    pub reaction_time: i32,
    /// Som de ataque
    pub attack_sound: i32,
    /// Estado de dor
    pub pain_state: StateNum,
    /// Chance de entrar em estado de dor (0-255)
    pub pain_chance: i32,
    /// Som de dor
    pub pain_sound: i32,
    /// Estado de ataque melee
    pub melee_state: StateNum,
    /// Estado de ataque a distancia
    pub missile_state: StateNum,
    /// Estado de morte
    pub death_state: StateNum,
    /// Estado de morte violenta (gib)
    pub xdeath_state: StateNum,
    /// Som de morte
    pub death_sound: i32,
    /// Velocidade de movimento
    pub speed: i32,
    /// Raio de colisao (fixed-point)
    pub radius: Fixed,
    /// Altura de colisao (fixed-point)
    pub height: Fixed,
    /// Massa (afeta knockback)
    pub mass: i32,
    /// Dano causado (para projeteis)
    pub damage: i32,
    /// Som de atividade (idle)
    pub active_sound: i32,
    /// Flags de propriedades
    pub flags: MobjFlags,
    /// Estado de ressurreicao (arch-vile)
    pub raise_state: StateNum,
}

impl MobjInfo {
    /// Cria uma mobjinfo vazia.
    pub fn new() -> Self {
        MobjInfo {
            doomednum: -1,
            spawn_state: S_NULL,
            spawn_health: 1000,
            see_state: S_NULL,
            see_sound: 0,
            reaction_time: 8,
            attack_sound: 0,
            pain_state: S_NULL,
            pain_chance: 0,
            pain_sound: 0,
            melee_state: S_NULL,
            missile_state: S_NULL,
            death_state: S_NULL,
            xdeath_state: S_NULL,
            death_sound: 0,
            speed: 0,
            radius: Fixed::from_int(20),
            height: Fixed::from_int(16),
            mass: 100,
            damage: 0,
            active_sound: 0,
            flags: MobjFlags::empty(),
            raise_state: S_NULL,
        }
    }

    /// Cria mobjinfo para o jogador (MT_PLAYER).
    ///
    /// C original: `mobjinfo[MT_PLAYER]` em `info.c`
    pub fn player() -> Self {
        MobjInfo {
            doomednum: -1,
            spawn_health: 100,
            speed: 0,
            radius: Fixed(16 * FRACUNIT),
            height: Fixed(56 * FRACUNIT),
            mass: 100,
            flags: MobjFlags::SOLID
                | MobjFlags::SHOOTABLE
                | MobjFlags::DROPOFF
                | MobjFlags::PICKUP
                | MobjFlags::NOTDMATCH,
            ..MobjInfo::new()
        }
    }

    /// Cria mobjinfo para o Imp (MT_TROOP).
    ///
    /// C original: `mobjinfo[MT_TROOP]` em `info.c`
    pub fn imp() -> Self {
        MobjInfo {
            doomednum: 3001,
            spawn_health: 60,
            speed: 8,
            radius: Fixed(20 * FRACUNIT),
            height: Fixed(56 * FRACUNIT),
            mass: 100,
            pain_chance: 200,
            flags: MobjFlags::SOLID
                | MobjFlags::SHOOTABLE
                | MobjFlags::COUNTKILL,
            ..MobjInfo::new()
        }
    }

    /// Cria mobjinfo para o barril (MT_BARREL).
    ///
    /// C original: `mobjinfo[MT_BARREL]` em `info.c`
    pub fn barrel() -> Self {
        MobjInfo {
            doomednum: 2035,
            spawn_health: 20,
            speed: 0,
            radius: Fixed(10 * FRACUNIT),
            height: Fixed(42 * FRACUNIT),
            mass: 100,
            flags: MobjFlags::SOLID
                | MobjFlags::SHOOTABLE
                | MobjFlags::NOBLOOD,
            ..MobjInfo::new()
        }
    }
}

impl Default for MobjInfo {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Constantes de fisica
// ---------------------------------------------------------------------------

/// Altura maxima de degrau que um mobj pode subir (24 unidades).
///
/// C original: hardcoded `24*FRACUNIT` em `p_map.c`
pub const MAXSTEPHEIGHT: Fixed = Fixed(24 * FRACUNIT);

/// Velocidade maxima de movimento por tick (30 unidades).
///
/// C original: `#define MAXMOVE (30*FRACUNIT)` em `p_local.h`
pub const MAXMOVE: Fixed = Fixed(30 * FRACUNIT);

/// Distancia maxima de uso (abrir portas, switches).
///
/// C original: `#define USERANGE (64*FRACUNIT)` em `p_local.h`
pub const USERANGE: Fixed = Fixed(64 * FRACUNIT);

/// Distancia maxima de ataque melee.
///
/// C original: `#define MELEERANGE (64*FRACUNIT)` em `p_local.h`
pub const MELEERANGE: Fixed = Fixed(64 * FRACUNIT);

/// Distancia maxima de ataque a distancia.
///
/// C original: `#define MISSILERANGE (32*64*FRACUNIT)` em `p_local.h`
pub const MISSILERANGE: Fixed = Fixed(32 * 64 * FRACUNIT);

/// Gravidade aplicada por tick (1 unidade por tick).
///
/// C original: `#define GRAVITY FRACUNIT` em `p_local.h`
pub const GRAVITY: Fixed = Fixed(FRACUNIT);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mobj_flags_basic() {
        let flags = MobjFlags::SOLID | MobjFlags::SHOOTABLE;
        assert!(flags.contains(MobjFlags::SOLID));
        assert!(flags.contains(MobjFlags::SHOOTABLE));
        assert!(!flags.contains(MobjFlags::MISSILE));
    }

    #[test]
    fn mobj_flags_values() {
        // Verificar que os valores correspondem ao C original
        assert_eq!(MobjFlags::SPECIAL.bits(), 1);
        assert_eq!(MobjFlags::SOLID.bits(), 2);
        assert_eq!(MobjFlags::SHOOTABLE.bits(), 4);
        assert_eq!(MobjFlags::MISSILE.bits(), 0x10000);
        assert_eq!(MobjFlags::TRANSLATION.bits(), 0x0C000000);
    }

    #[test]
    fn player_info() {
        let info = MobjInfo::player();
        assert_eq!(info.spawn_health, 100);
        assert!(info.flags.contains(MobjFlags::SOLID));
        assert!(info.flags.contains(MobjFlags::SHOOTABLE));
        assert_eq!(info.height, Fixed(56 * FRACUNIT));
    }

    #[test]
    fn imp_info() {
        let info = MobjInfo::imp();
        assert_eq!(info.doomednum, 3001);
        assert_eq!(info.spawn_health, 60);
        assert_eq!(info.speed, 8);
        assert!(info.flags.contains(MobjFlags::COUNTKILL));
    }

    #[test]
    fn barrel_info() {
        let info = MobjInfo::barrel();
        assert_eq!(info.doomednum, 2035);
        assert!(info.flags.contains(MobjFlags::NOBLOOD));
        assert!(!info.flags.contains(MobjFlags::COUNTKILL));
    }

    #[test]
    fn state_default() {
        let s = State::new();
        assert_eq!(s.tics, -1);
        assert_eq!(s.next_state, S_NULL);
    }

    #[test]
    fn physics_constants() {
        assert_eq!(MAXSTEPHEIGHT, Fixed(24 * FRACUNIT));
        assert_eq!(MAXMOVE, Fixed(30 * FRACUNIT));
        assert_eq!(GRAVITY, Fixed(FRACUNIT));
    }
}

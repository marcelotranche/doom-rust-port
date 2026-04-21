//! # Armas e PSprites — Sistema de Armas do Jogador
//!
//! Implementa o sistema de armas do DOOM: estados de arma,
//! animacao de sprites do jogador (psprites), e disparo.
//!
//! ## PSprites (Player Sprites)
//!
//! PSprites sao sprites desenhados sobre a vista 3D que representam
//! a arma atual do jogador. Cada jogador tem 2 psprites:
//! - `ps_weapon` (0): sprite da arma principal
//! - `ps_flash` (1): sprite do flash de tiro (muzzle flash)
//!
//! ## Maquina de estados da arma
//!
//! ```text
//! Ready (arma parada) → Fire (atirar) → Flash (muzzle flash)
//!                                      → Ready (volta ao estado normal)
//! Ready → Down (abaixar) → Up (levantar nova arma) → Ready
//! ```
//!
//! ## Arquivo C original: `p_pspr.c`, `p_pspr.h`
//!
//! ## Conceitos que o leitor vai aprender
//! - State machine para animacao de armas
//! - Overlay rendering (sprites sobre a vista 3D)
//! - Weapon bobbing baseado no movimento do jogador

use crate::utils::fixed::{Fixed, FRACUNIT};

/// Numero de psprites por jogador.
///
/// C original: `#define NUMPSPRITES 2` em `p_pspr.h`
pub const NUMPSPRITES: usize = 2;

/// Indice do psprite da arma.
pub const PS_WEAPON: usize = 0;
/// Indice do psprite do flash.
pub const PS_FLASH: usize = 1;

/// Tipos de armas do DOOM.
///
/// C original: `weapontype_t` em `d_player.h`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponType {
    Fist = 0,
    Pistol = 1,
    Shotgun = 2,
    Chaingun = 3,
    RocketLauncher = 4,
    Plasma = 5,
    Bfg = 6,
    Chainsaw = 7,
    SuperShotgun = 8,
}

/// Tipos de municao.
///
/// C original: `ammotype_t` em `d_player.h`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmmoType {
    Clip = 0,     // bullets
    Shell = 1,    // shells
    Cell = 2,     // plasma/bfg cells
    Missile = 3,  // rockets
    NoAmmo = 4,   // fist, chainsaw
}

/// Estado de um psprite (frame de animacao da arma).
///
/// C original: `pspdef_t` em `p_pspr.h`
#[derive(Debug, Clone, Copy)]
pub struct PspriteDef {
    /// Estado atual da animacao.
    pub state: WeaponState,
    /// Ticks restantes neste frame.
    pub tics: i32,
    /// Posicao X na tela (para bobbing).
    pub sx: Fixed,
    /// Posicao Y na tela (para raise/lower).
    pub sy: Fixed,
}

/// Estados da arma (maquina de estados simplificada).
///
/// No DOOM original, cada arma tem ~10 estados definidos em info.c.
/// Aqui usamos uma versao simplificada com os estados essenciais.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponState {
    /// Arma pronta para disparar (looping).
    Ready,
    /// Arma subindo (sendo equipada).
    Up,
    /// Arma descendo (sendo guardada).
    Down,
    /// Frame 1 de disparo.
    Fire1,
    /// Frame 2 de disparo.
    Fire2,
    /// Frame 3 de disparo (volta a Ready).
    Fire3,
    /// Flash de tiro (muzzle flash).
    Flash,
    /// Arma nao visivel.
    None,
}

/// Posicao Y da arma quando completamente levantada.
///
/// C original: `#define WEAPONTOP 32*FRACUNIT` em `p_pspr.c`
pub const WEAPONTOP: i32 = 32 * FRACUNIT;

/// Posicao Y da arma quando completamente abaixada.
///
/// C original: `#define WEAPONBOTTOM 128*FRACUNIT` em `p_pspr.c`
pub const WEAPONBOTTOM: i32 = 128 * FRACUNIT;

/// Velocidade de subida/descida da arma (6 unidades por tick).
///
/// C original: `#define LOWERSPEED FRACUNIT*6` em `p_pspr.c`
pub const RAISESPEED: i32 = FRACUNIT * 6;
pub const LOWERSPEED: i32 = FRACUNIT * 6;

/// Informacoes sobre uma arma.
///
/// C original: `weaponinfo_t` em `d_items.h`
#[derive(Debug, Clone, Copy)]
pub struct WeaponInfo {
    /// Tipo de municao usada.
    pub ammo_type: AmmoType,
    /// Municao consumida por tiro.
    pub ammo_per_shot: i32,
    /// Nome do sprite da arma pronta (ex: "PISG" para pistola).
    pub ready_sprite: &'static [u8; 4],
    /// Nome do sprite de disparo (ex: "PISF" para pistol flash).
    pub fire_sprite: &'static [u8; 4],
    /// Ticks no estado de disparo.
    pub fire_tics: i32,
}

/// Tabela de informacoes de armas.
///
/// C original: `weaponinfo[]` em `d_items.c`
pub static WEAPON_INFO: [WeaponInfo; 9] = [
    // Fist
    WeaponInfo {
        ammo_type: AmmoType::NoAmmo,
        ammo_per_shot: 0,
        ready_sprite: b"PUNG",
        fire_sprite: b"PUNG",
        fire_tics: 8,
    },
    // Pistol
    WeaponInfo {
        ammo_type: AmmoType::Clip,
        ammo_per_shot: 1,
        ready_sprite: b"PISG",
        fire_sprite: b"PISF",
        fire_tics: 6,
    },
    // Shotgun
    WeaponInfo {
        ammo_type: AmmoType::Shell,
        ammo_per_shot: 1,
        ready_sprite: b"SHTG",
        fire_sprite: b"SHTF",
        fire_tics: 10,
    },
    // Chaingun
    WeaponInfo {
        ammo_type: AmmoType::Clip,
        ammo_per_shot: 1,
        ready_sprite: b"CHGG",
        fire_sprite: b"CHGF",
        fire_tics: 4,
    },
    // Rocket launcher
    WeaponInfo {
        ammo_type: AmmoType::Missile,
        ammo_per_shot: 1,
        ready_sprite: b"MISG",
        fire_sprite: b"MISF",
        fire_tics: 12,
    },
    // Plasma rifle
    WeaponInfo {
        ammo_type: AmmoType::Cell,
        ammo_per_shot: 1,
        ready_sprite: b"PLSG",
        fire_sprite: b"PLSF",
        fire_tics: 6,
    },
    // BFG 9000
    WeaponInfo {
        ammo_type: AmmoType::Cell,
        ammo_per_shot: 40,
        ready_sprite: b"BFGG",
        fire_sprite: b"BFGF",
        fire_tics: 20,
    },
    // Chainsaw
    WeaponInfo {
        ammo_type: AmmoType::NoAmmo,
        ammo_per_shot: 0,
        ready_sprite: b"SAWG",
        fire_sprite: b"SAWG",
        fire_tics: 8,
    },
    // Super Shotgun
    WeaponInfo {
        ammo_type: AmmoType::Shell,
        ammo_per_shot: 2,
        ready_sprite: b"SHT2",
        fire_sprite: b"SHT2",
        fire_tics: 14,
    },
];

/// Estado completo das armas do jogador.
///
/// C original: campos de `player_t` em `d_player.h`
#[derive(Debug)]
pub struct PlayerWeapons {
    /// Arma atualmente equipada.
    pub ready_weapon: WeaponType,
    /// Arma pendente (trocando para).
    pub pending_weapon: Option<WeaponType>,
    /// Armas possuidas.
    pub weapon_owned: [bool; 9],
    /// Municao por tipo.
    pub ammo: [i32; 4],
    /// Municao maxima por tipo.
    pub max_ammo: [i32; 4],
    /// PSprites (arma + flash).
    pub psprites: [PspriteDef; NUMPSPRITES],
    /// Se o botao de ataque esta pressionado.
    pub attack_down: bool,
    /// Contador de refire (para cadencia rapida).
    pub refire: i32,
}

impl PlayerWeapons {
    /// Cria estado inicial de armas (inicio de jogo).
    pub fn new() -> Self {
        let mut weapon_owned = [false; 9];
        weapon_owned[WeaponType::Fist as usize] = true;
        weapon_owned[WeaponType::Pistol as usize] = true;

        PlayerWeapons {
            ready_weapon: WeaponType::Pistol,
            pending_weapon: None,
            weapon_owned,
            ammo: [50, 0, 0, 0], // 50 bullets
            max_ammo: [200, 50, 300, 50],
            psprites: [
                PspriteDef {
                    state: WeaponState::Ready,
                    tics: 1,
                    sx: Fixed::ZERO,
                    sy: Fixed(WEAPONTOP),
                },
                PspriteDef {
                    state: WeaponState::None,
                    tics: -1,
                    sx: Fixed::ZERO,
                    sy: Fixed::ZERO,
                },
            ],
            attack_down: false,
            refire: 0,
        }
    }

    /// Processa o estado da arma a cada tick.
    ///
    /// Atualiza psprites, processa disparo, troca de arma.
    ///
    /// C original: `P_MovePsprites()` em `p_pspr.c`
    pub fn tick(&mut self, fire_pressed: bool) {
        // Processar psprite da arma
        self.tick_psprite(fire_pressed);
    }

    /// Atualiza o psprite da arma.
    fn tick_psprite(&mut self, fire_pressed: bool) {
        // Decrementar tics
        if self.psprites[PS_WEAPON].tics > 0 {
            self.psprites[PS_WEAPON].tics -= 1;
            if self.psprites[PS_WEAPON].tics > 0 {
                return;
            }
        }

        // Ler estado atual para dispatch
        let current_state = self.psprites[PS_WEAPON].state;

        // Transicao de estado
        match current_state {
            WeaponState::Ready => {
                if fire_pressed && self.can_fire() {
                    self.fire_weapon();
                }
            }
            WeaponState::Up => {
                self.psprites[PS_WEAPON].sy.0 -= RAISESPEED;
                if self.psprites[PS_WEAPON].sy.0 <= WEAPONTOP {
                    self.psprites[PS_WEAPON].sy.0 = WEAPONTOP;
                    self.psprites[PS_WEAPON].state = WeaponState::Ready;
                    self.psprites[PS_WEAPON].tics = 1;
                }
            }
            WeaponState::Down => {
                self.psprites[PS_WEAPON].sy.0 += LOWERSPEED;
                if self.psprites[PS_WEAPON].sy.0 >= WEAPONBOTTOM {
                    self.psprites[PS_WEAPON].sy.0 = WEAPONBOTTOM;
                    if let Some(new_weapon) = self.pending_weapon {
                        self.ready_weapon = new_weapon;
                        self.pending_weapon = None;
                        self.bring_up_weapon();
                    }
                }
            }
            WeaponState::Fire1 => {
                self.psprites[PS_WEAPON].state = WeaponState::Fire2;
                self.psprites[PS_WEAPON].tics = 2;
            }
            WeaponState::Fire2 => {
                self.psprites[PS_WEAPON].state = WeaponState::Fire3;
                self.psprites[PS_WEAPON].tics = 2;
            }
            WeaponState::Fire3 => {
                if fire_pressed && self.can_fire() {
                    self.refire += 1;
                    self.fire_weapon();
                } else {
                    self.refire = 0;
                    self.psprites[PS_WEAPON].state = WeaponState::Ready;
                    self.psprites[PS_WEAPON].tics = 1;
                }
            }
            WeaponState::Flash | WeaponState::None => {}
        }
    }

    /// Verifica se a arma pode disparar (tem municao).
    fn can_fire(&self) -> bool {
        let info = &WEAPON_INFO[self.ready_weapon as usize];
        if info.ammo_type as usize >= 4 {
            return true; // NoAmmo (fist, chainsaw)
        }
        self.ammo[info.ammo_type as usize] >= info.ammo_per_shot
    }

    /// Dispara a arma atual.
    fn fire_weapon(&mut self) {
        let info = &WEAPON_INFO[self.ready_weapon as usize];

        // Consumir municao
        if (info.ammo_type as usize) < 4 {
            self.ammo[info.ammo_type as usize] -= info.ammo_per_shot;
        }

        // Transicionar para estado de disparo
        let psp = &mut self.psprites[PS_WEAPON];
        psp.state = WeaponState::Fire1;
        psp.tics = info.fire_tics / 3; // dividido entre os 3 frames de fire

        // Ativar flash
        self.psprites[PS_FLASH].state = WeaponState::Flash;
        self.psprites[PS_FLASH].tics = 4;
    }

    /// Inicia a animacao de levantar a arma.
    ///
    /// C original: `P_BringUpWeapon()` em `p_pspr.c`
    pub fn bring_up_weapon(&mut self) {
        let psp = &mut self.psprites[PS_WEAPON];
        psp.state = WeaponState::Up;
        psp.sy = Fixed(WEAPONBOTTOM);
        psp.tics = 1;
    }

    /// Inicia a troca de arma.
    pub fn switch_weapon(&mut self, new_weapon: WeaponType) {
        if new_weapon == self.ready_weapon {
            return;
        }
        if !self.weapon_owned[new_weapon as usize] {
            return;
        }
        self.pending_weapon = Some(new_weapon);
        self.psprites[PS_WEAPON].state = WeaponState::Down;
        self.psprites[PS_WEAPON].tics = 1;
    }

    /// Retorna o nome do sprite para o frame atual da arma.
    ///
    /// Formato: 4 chars base + frame letter (A=ready, B-D=fire).
    pub fn current_sprite_name(&self) -> Option<String> {
        let psp = &self.psprites[PS_WEAPON];
        let info = &WEAPON_INFO[self.ready_weapon as usize];

        let (base, frame) = match psp.state {
            WeaponState::Ready | WeaponState::Up | WeaponState::Down => {
                (info.ready_sprite, b'A')
            }
            WeaponState::Fire1 => (info.ready_sprite, b'B'),
            WeaponState::Fire2 => (info.ready_sprite, b'C'),
            WeaponState::Fire3 => (info.ready_sprite, b'B'),
            WeaponState::Flash => (info.fire_sprite, b'A'),
            WeaponState::None => return None,
        };

        let name = format!(
            "{}{}0",
            std::str::from_utf8(base).unwrap_or("PISG"),
            frame as char
        );
        Some(name)
    }

    /// Retorna a posicao Y ajustada para rendering (0 = topo).
    pub fn weapon_y_offset(&self) -> i32 {
        let psp = &self.psprites[PS_WEAPON];
        // WEAPONTOP = 32*FRACUNIT, converter para pixels de tela
        (psp.sy.0 - WEAPONTOP) >> 16
    }
}

impl Default for PlayerWeapons {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state() {
        let pw = PlayerWeapons::new();
        assert_eq!(pw.ready_weapon, WeaponType::Pistol);
        assert!(pw.weapon_owned[WeaponType::Fist as usize]);
        assert!(pw.weapon_owned[WeaponType::Pistol as usize]);
        assert!(!pw.weapon_owned[WeaponType::Shotgun as usize]);
        assert_eq!(pw.ammo[AmmoType::Clip as usize], 50);
        assert_eq!(pw.psprites[PS_WEAPON].state, WeaponState::Ready);
    }

    #[test]
    fn fire_consumes_ammo() {
        let mut pw = PlayerWeapons::new();
        let initial_ammo = pw.ammo[AmmoType::Clip as usize];

        // Pressionar fire
        pw.tick(true);

        // Arma deve estar no estado de disparo
        assert_eq!(pw.psprites[PS_WEAPON].state, WeaponState::Fire1);
        assert_eq!(pw.ammo[AmmoType::Clip as usize], initial_ammo - 1);
    }

    #[test]
    fn fire_cycle_returns_to_ready() {
        let mut pw = PlayerWeapons::new();

        // Fire
        pw.tick(true);
        assert_eq!(pw.psprites[PS_WEAPON].state, WeaponState::Fire1);

        // Rodar ticks ate voltar a Ready (sem segurar fire)
        for _ in 0..20 {
            pw.tick(false);
            if pw.psprites[PS_WEAPON].state == WeaponState::Ready {
                break;
            }
        }
        assert_eq!(pw.psprites[PS_WEAPON].state, WeaponState::Ready);
    }

    #[test]
    fn no_fire_without_ammo() {
        let mut pw = PlayerWeapons::new();
        pw.ammo[AmmoType::Clip as usize] = 0;

        pw.tick(true);

        // Sem municao — arma permanece Ready
        assert_eq!(pw.psprites[PS_WEAPON].state, WeaponState::Ready);
    }

    #[test]
    fn weapon_switch() {
        let mut pw = PlayerWeapons::new();
        pw.weapon_owned[WeaponType::Shotgun as usize] = true;

        pw.switch_weapon(WeaponType::Shotgun);
        assert_eq!(pw.psprites[PS_WEAPON].state, WeaponState::Down);

        // Rodar ticks ate a arma descer e subir
        for _ in 0..50 {
            pw.tick(false);
        }
        assert_eq!(pw.ready_weapon, WeaponType::Shotgun);
        assert_eq!(pw.psprites[PS_WEAPON].state, WeaponState::Ready);
    }

    #[test]
    fn sprite_name_ready() {
        let pw = PlayerWeapons::new();
        let name = pw.current_sprite_name().unwrap();
        assert_eq!(name, "PISGA0");
    }

    #[test]
    fn sprite_name_fire() {
        let mut pw = PlayerWeapons::new();
        pw.tick(true); // fire
        let name = pw.current_sprite_name().unwrap();
        assert_eq!(name, "PISGB0");
    }

    #[test]
    fn fist_no_ammo_needed() {
        let mut pw = PlayerWeapons::new();
        pw.ready_weapon = WeaponType::Fist;
        pw.psprites[PS_WEAPON].state = WeaponState::Ready;
        pw.ammo = [0; 4]; // sem nenhuma municao

        pw.tick(true);

        // Fist nao precisa de municao
        assert_eq!(pw.psprites[PS_WEAPON].state, WeaponState::Fire1);
    }
}

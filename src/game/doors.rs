//! # Portas Verticais — T_VerticalDoor e EV_VerticalDoor
//!
//! Implementa o sistema de portas do DOOM: portas que abrem
//! verticalmente (teto sobe), aguardam um tempo, e fecham.
//!
//! ## Ciclo de vida de uma porta
//!
//! ```text
//! Jogador pressiona Use → P_UseLines encontra linedef com special
//!   → EV_VerticalDoor cria DoorThinker
//!     → Cada tick: T_VerticalDoor move ceiling_height
//!       → Abre (direction=1), espera (direction=0), fecha (direction=-1)
//!       → Ao fechar completamente: remove thinker
//! ```
//!
//! ## Tipos de portas
//!
//! - `Normal`: abre, espera VDOORWAIT ticks, fecha automaticamente
//! - `Open`: abre e fica aberta (one-shot, limpa linedef.special)
//! - `Close`: fecha imediatamente
//! - `BlazeRaise`/`BlazeOpen`: versao rapida (4x velocidade)
//!
//! ## Arquivo C original: `p_doors.c`
//!
//! ## Conceitos que o leitor vai aprender
//! - Thinker pattern para animacao ao longo de multiplos ticks
//! - Maquina de estados com direcao (subindo/esperando/descendo)
//! - Interacao linedef.special → setor.ceiling_height

use crate::game::thinker::Thinker;
use crate::map::types::Sector;
use crate::utils::fixed::{Fixed, FRACUNIT};

/// Velocidade padrao de portas verticais.
///
/// C original: `#define VDOORSPEED FRACUNIT*2` em `p_spec.h`
pub const VDOORSPEED: i32 = FRACUNIT * 2;

/// Tempo de espera no topo (em ticks, 150 = ~4.3 segundos).
///
/// C original: `#define VDOORWAIT 150` em `p_spec.h`
pub const VDOORWAIT: i32 = 150;

/// Tipo de porta vertical.
///
/// C original: `vldoor_e` em `p_spec.h`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorType {
    /// Abre, espera, fecha automaticamente.
    Normal,
    /// Abre e fica aberta permanentemente.
    Open,
    /// Fecha imediatamente.
    Close,
    /// Fecha, espera 30 segundos, e abre.
    Close30ThenOpen,
    /// Versao rapida de Normal (4x velocidade).
    BlazeRaise,
    /// Versao rapida de Open (4x velocidade).
    BlazeOpen,
    /// Versao rapida de Close (4x velocidade).
    BlazeClose,
}

/// Resultado do movimento de um plano (piso/teto).
///
/// C original: `result_e` em `p_spec.h`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveResult {
    /// Movimento normal, ainda nao chegou ao destino.
    Ok,
    /// Chegou ao destino.
    PastDest,
    /// Esmagou algo (nao implementado no port simplificado).
    Crushed,
}

/// Thinker de porta vertical — move o teto de um sector.
///
/// Equivalente a `vldoor_t` no C original (`p_spec.h`).
///
/// A cada tick, move `ceiling_height` do sector referenciado
/// conforme `direction` e `speed`, respeitando limites.
#[derive(Debug)]
pub struct DoorThinker {
    /// Indice do sector que esta porta controla.
    pub sector_index: usize,
    /// Tipo de porta (normal, open, blaze, etc.)
    pub door_type: DoorType,
    /// Altura alvo do teto quando totalmente aberta.
    pub top_height: Fixed,
    /// Velocidade de movimento por tick (em fixed-point).
    pub speed: Fixed,
    /// Direcao: 1 = subindo, 0 = esperando, -1 = descendo, 2 = espera inicial.
    pub direction: i32,
    /// Contador de espera no topo (ou estado inicial).
    pub top_countdown: i32,
    /// Altura do piso do sector (para saber onde fechar).
    pub floor_height: Fixed,
}

impl Thinker for DoorThinker {
    /// Atualiza a porta a cada tick.
    ///
    /// Move o teto do sector conforme a direcao e velocidade.
    /// Quando atinge o destino, muda de estado ou se remove.
    ///
    /// C original: `T_VerticalDoor()` em `p_doors.c`
    fn think(&mut self, sectors: &mut [Sector]) -> bool {
        if self.sector_index >= sectors.len() {
            return false;
        }

        match self.direction {
            0 => {
                // Esperando no topo
                self.top_countdown -= 1;
                if self.top_countdown <= 0 {
                    match self.door_type {
                        DoorType::BlazeRaise | DoorType::Normal => {
                            // Hora de fechar
                            self.direction = -1;
                        }
                        DoorType::Close30ThenOpen => {
                            self.direction = 1;
                        }
                        _ => {}
                    }
                }
                true
            }
            2 => {
                // Espera inicial (raiseIn5Mins)
                self.top_countdown -= 1;
                if self.top_countdown <= 0 {
                    self.direction = 1;
                    self.door_type = DoorType::Normal;
                }
                true
            }
            -1 => {
                // Descendo (fechando)
                let result = self.move_ceiling(sectors, -1);
                match result {
                    MoveResult::PastDest => {
                        match self.door_type {
                            DoorType::BlazeRaise
                            | DoorType::BlazeClose
                            | DoorType::Normal
                            | DoorType::Close => {
                                // Porta fechou completamente — remover thinker
                                return false;
                            }
                            DoorType::Close30ThenOpen => {
                                self.direction = 0;
                                self.top_countdown = 35 * 30;
                            }
                            _ => {}
                        }
                        true
                    }
                    MoveResult::Crushed => {
                        // Porta encontrou obstaculo ao fechar
                        match self.door_type {
                            DoorType::BlazeClose | DoorType::Close => {
                                // Nao reverter — continuar tentando fechar
                            }
                            _ => {
                                // Reverter direcao (porta "bounces" ao encontrar algo)
                                self.direction = 1;
                            }
                        }
                        true
                    }
                    MoveResult::Ok => true,
                }
            }
            1 => {
                // Subindo (abrindo)
                let result = self.move_ceiling(sectors, 1);
                if result == MoveResult::PastDest {
                    match self.door_type {
                        DoorType::BlazeRaise | DoorType::Normal => {
                            // Esperar no topo antes de fechar
                            self.direction = 0;
                            self.top_countdown = VDOORWAIT;
                        }
                        DoorType::Close30ThenOpen | DoorType::BlazeOpen | DoorType::Open => {
                            // Porta abriu completamente — remover thinker
                            return false;
                        }
                        _ => {}
                    }
                }
                true
            }
            _ => true,
        }
    }
}

impl DoorThinker {
    /// Move o teto do sector na direcao indicada.
    ///
    /// Simplificacao de `T_MovePlane()` do C original (`p_floor.c`),
    /// apenas para o caso de ceiling (floorOrCeiling=1).
    fn move_ceiling(&self, sectors: &mut [Sector], direction: i32) -> MoveResult {
        let sector = &mut sectors[self.sector_index];

        match direction {
            -1 => {
                // Descendo
                let dest = self.floor_height;
                if sector.ceiling_height.0 - self.speed.0 < dest.0 {
                    sector.ceiling_height = dest;
                    MoveResult::PastDest
                } else {
                    sector.ceiling_height.0 -= self.speed.0;
                    MoveResult::Ok
                }
            }
            1 => {
                // Subindo
                let dest = self.top_height;
                if sector.ceiling_height.0 + self.speed.0 > dest.0 {
                    sector.ceiling_height = dest;
                    MoveResult::PastDest
                } else {
                    sector.ceiling_height.0 += self.speed.0;
                    MoveResult::Ok
                }
            }
            _ => MoveResult::Ok,
        }
    }
}

/// Encontra o teto mais baixo dos sectors vizinhos.
///
/// Usado para determinar a altura alvo de portas (abrem ate
/// 4 unidades abaixo do teto mais baixo adjacente).
///
/// C original: `P_FindLowestCeilingSurrounding()` em `p_spec.c`
pub fn find_lowest_ceiling_surrounding(
    sector_index: usize,
    sectors: &[Sector],
    linedefs: &[crate::map::types::LineDef],
) -> Fixed {
    let mut min_height = Fixed(i32::MAX);

    for line in linedefs {
        let front = line.front_sector;
        let back = line.back_sector;

        // Verificar se a linedef toca o sector e tem outro lado
        let other = if front == Some(sector_index) {
            back
        } else if back == Some(sector_index) {
            front
        } else {
            continue;
        };

        if let Some(other_idx) = other {
            if other_idx < sectors.len() {
                let h = sectors[other_idx].ceiling_height;
                if h.0 < min_height.0 {
                    min_height = h;
                }
            }
        }
    }

    if min_height.0 == i32::MAX {
        // Fallback: usar o proprio sector
        if sector_index < sectors.len() {
            sectors[sector_index].ceiling_height
        } else {
            Fixed::ZERO
        }
    } else {
        min_height
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::types::Sector;

    fn make_sector(floor: i32, ceiling: i32) -> Sector {
        Sector {
            floor_height: Fixed::from_int(floor),
            ceiling_height: Fixed::from_int(ceiling),
            floor_pic: [0; 8],
            ceiling_pic: [0; 8],
            light_level: 160,
            special: 0,
            tag: 0,
        }
    }

    #[test]
    fn door_opens_and_waits() {
        let mut sectors = vec![make_sector(0, 0)];
        let mut door = DoorThinker {
            sector_index: 0,
            door_type: DoorType::Normal,
            top_height: Fixed::from_int(128),
            speed: Fixed::from_int(2),
            direction: 1,
            top_countdown: VDOORWAIT,
            floor_height: Fixed::from_int(0),
        };

        // Rodar ticks ate a porta abrir completamente
        for _ in 0..100 {
            if !door.think(&mut sectors) {
                break;
            }
        }

        // Porta deve ter atingido top_height e estar esperando
        assert_eq!(door.direction, 0);
        assert_eq!(sectors[0].ceiling_height, Fixed::from_int(128));
    }

    #[test]
    fn door_opens_waits_and_closes() {
        let mut sectors = vec![make_sector(0, 0)];
        let mut door = DoorThinker {
            sector_index: 0,
            door_type: DoorType::Normal,
            top_height: Fixed::from_int(64),
            speed: Fixed::from_int(4),
            direction: 1,
            top_countdown: VDOORWAIT,
            floor_height: Fixed::from_int(0),
        };

        // Abrir
        for _ in 0..20 {
            door.think(&mut sectors);
        }
        assert_eq!(door.direction, 0); // esperando

        // Esperar
        for _ in 0..VDOORWAIT {
            door.think(&mut sectors);
        }
        assert_eq!(door.direction, -1); // fechando

        // Fechar (door deve se remover ao fechar)
        let mut alive = true;
        for _ in 0..20 {
            alive = door.think(&mut sectors);
            if !alive {
                break;
            }
        }
        assert!(!alive); // thinker removido
        assert_eq!(sectors[0].ceiling_height, Fixed::from_int(0));
    }

    #[test]
    fn door_type_open_stays_open() {
        let mut sectors = vec![make_sector(0, 0)];
        let mut door = DoorThinker {
            sector_index: 0,
            door_type: DoorType::Open,
            top_height: Fixed::from_int(64),
            speed: Fixed::from_int(4),
            direction: 1,
            top_countdown: VDOORWAIT,
            floor_height: Fixed::from_int(0),
        };

        // Abrir — deve se remover ao atingir top_height
        let mut alive = true;
        for _ in 0..20 {
            alive = door.think(&mut sectors);
            if !alive {
                break;
            }
        }
        assert!(!alive); // removido apos abrir completamente
        assert_eq!(sectors[0].ceiling_height, Fixed::from_int(64));
    }

    #[test]
    fn blaze_door_is_faster() {
        let mut sectors_normal = vec![make_sector(0, 0)];
        let mut sectors_blaze = vec![make_sector(0, 0)];

        let mut normal = DoorThinker {
            sector_index: 0,
            door_type: DoorType::Normal,
            top_height: Fixed::from_int(128),
            speed: Fixed(VDOORSPEED),
            direction: 1,
            top_countdown: VDOORWAIT,
            floor_height: Fixed::from_int(0),
        };

        let mut blaze = DoorThinker {
            sector_index: 0,
            door_type: DoorType::BlazeRaise,
            top_height: Fixed::from_int(128),
            speed: Fixed(VDOORSPEED * 4),
            direction: 1,
            top_countdown: VDOORWAIT,
            floor_height: Fixed::from_int(0),
        };

        // Rodar 10 ticks
        for _ in 0..10 {
            normal.think(&mut sectors_normal);
            blaze.think(&mut sectors_blaze);
        }

        // Blaze deve estar mais alta
        assert!(sectors_blaze[0].ceiling_height.0 > sectors_normal[0].ceiling_height.0);
    }
}

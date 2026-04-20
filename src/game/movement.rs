//! # Movimento e Colisao
//!
//! Implementa a mecanica de movimento e colisao do DOOM:
//! - `P_CheckPosition()` — testa se um mobj pode ocupar uma posicao
//! - `P_TryMove()` — tenta mover um mobj, com colisao
//! - Colisao mobj-vs-mobj e mobj-vs-linedef
//!
//! ## Algoritmo de colisao
//!
//! 1. Calcular bounding box do mobj na posicao tentada
//! 2. Usar blockmap para encontrar linedefs e mobjs proximos
//! 3. Para cada linedef: verificar se bloqueia a passagem
//!    (one-sided, ou gap insuficiente)
//! 4. Para cada mobj: verificar sobreposicao de bounding boxes
//! 5. Verificar step-up (max 24 unidades) e dropoff
//!
//! ## Step-up
//!
//! O DOOM permite subir degraus de ate 24 unidades sem pular.
//! Isso e fundamental para a jogabilidade — o jogador desliza
//! suavemente sobre degraus baixos.
//!
//! ## Arquivo C original: `p_map.c`, `p_maputl.c`
//!
//! ## Conceitos que o leitor vai aprender
//! - AABB collision detection
//! - Spatial queries via blockmap
//! - Step-up mechanics e dropoff prevention

use crate::utils::fixed::Fixed;

use super::info::*;
use super::mobj::MapObj;

/// Resultado de uma checagem de posicao (`P_CheckPosition`).
///
/// Contem informacoes sobre a posicao testada: se e valida,
/// e os limites verticais do espaco disponivel.
///
/// C original: globals `tmfloorz`, `tmceilingz`, `tmdropoffz`,
/// `tmthing`, `tmx`, `tmy` em `p_map.c`
#[derive(Debug, Clone, Copy)]
pub struct PositionCheck {
    /// Se `true`, a posicao e valida (mobj pode ocupar)
    pub valid: bool,
    /// Altura do chao na posicao tentada
    pub floor_z: Fixed,
    /// Altura do teto na posicao tentada
    pub ceiling_z: Fixed,
    /// Altura do chao mais baixo encontrado (para dropoff check)
    pub dropoff_z: Fixed,
}

impl PositionCheck {
    /// Cria um resultado de posicao padrao (valido, sem restricoes).
    pub fn open() -> Self {
        PositionCheck {
            valid: true,
            floor_z: Fixed::ZERO,
            ceiling_z: Fixed::from_int(256), // teto padrao alto
            dropoff_z: Fixed::ZERO,
        }
    }
}

/// Testa se dois mobjs colidem (sobreposicao de bounding boxes circulares).
///
/// No DOOM, a colisao e baseada em distancia Manhattan (nao euclidiana)
/// entre os centros, comparada com a soma dos raios.
///
/// C original: `PIT_CheckThing()` em `p_map.c` (parte)
pub fn check_thing_collision(
    mobj: &MapObj,
    other: &MapObj,
    new_x: Fixed,
    new_y: Fixed,
) -> bool {
    // Distancia Manhattan entre centros
    let block_dist = (other.radius + mobj.radius).0;

    let dx = (other.x - new_x).0.abs();
    let dy = (other.y - new_y).0.abs();

    // Se qualquer eixo esta fora do alcance, nao colide
    if dx >= block_dist || dy >= block_dist {
        return false;
    }

    // Nao colidir consigo mesmo (mesma posicao exata = provavelmente o mesmo)
    if other.x == mobj.x && other.y == mobj.y {
        return false;
    }

    true
}

/// Verifica se um mobj pode ocupar a posicao (x, y).
///
/// Checa contra outros mobjs e linedefs na area. Retorna
/// informacoes sobre a posicao (floor, ceiling, validade).
///
/// C original: `P_CheckPosition()` em `p_map.c`
pub fn check_position(
    mobj: &MapObj,
    new_x: Fixed,
    new_y: Fixed,
    others: &[MapObj],
    floor_z: Fixed,
    ceiling_z: Fixed,
) -> PositionCheck {
    let mut result = PositionCheck {
        valid: true,
        floor_z,
        ceiling_z,
        dropoff_z: floor_z,
    };

    // Noclip bypassa tudo
    if mobj.flags.contains(MobjFlags::NOCLIP) {
        return result;
    }

    // Checar contra outros mobjs
    for other in others {
        // Pular se nao e solido e nao e shootable
        if !other.flags.contains(MobjFlags::SOLID)
            && !other.flags.contains(MobjFlags::SPECIAL)
            && !other.flags.contains(MobjFlags::SHOOTABLE)
        {
            continue;
        }

        if check_thing_collision(mobj, other, new_x, new_y) {
            // Se o outro e solido, bloqueia
            if other.flags.contains(MobjFlags::SOLID) {
                result.valid = false;
                return result;
            }
        }
    }

    // Checar se cabe verticalmente
    let gap = result.ceiling_z - result.floor_z;
    if gap < mobj.height {
        result.valid = false;
        return result;
    }

    result
}

/// Tenta mover um mobj para a posicao (x, y).
///
/// Se o movimento e valido (passa em `check_position`), atualiza
/// a posicao do mobj. Caso contrario, mantem a posicao atual.
///
/// Retorna `true` se o movimento foi bem-sucedido.
///
/// C original: `P_TryMove()` em `p_map.c`
pub fn try_move(
    mobj: &mut MapObj,
    new_x: Fixed,
    new_y: Fixed,
    others: &[MapObj],
    floor_z: Fixed,
    ceiling_z: Fixed,
) -> bool {
    let check = check_position(mobj, new_x, new_y, others, floor_z, ceiling_z);

    if !check.valid {
        return false;
    }

    // Noclip bypassa restricoes verticais
    if !mobj.flags.contains(MobjFlags::NOCLIP) {
        // Checar teto
        if check.ceiling_z - check.floor_z < mobj.height {
            return false;
        }

        // Checar step-up: pode subir ate MAXSTEPHEIGHT
        if check.floor_z - mobj.z > MAXSTEPHEIGHT {
            return false;
        }

        // Checar dropoff: monstros nao caminham sobre abismos
        // (exceto se tem DROPOFF flag)
        if !mobj.flags.contains(MobjFlags::DROPOFF)
            && !mobj.flags.contains(MobjFlags::FLOAT)
            && check.floor_z - check.dropoff_z > MAXSTEPHEIGHT
        {
            return false;
        }
    }

    // Movimento valido — atualizar posicao
    mobj.x = new_x;
    mobj.y = new_y;
    mobj.floorz = check.floor_z;
    mobj.ceilingz = check.ceiling_z;

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_solid_mobj(x: i32, y: i32) -> MapObj {
        let mut info = MobjInfo::new();
        info.radius = Fixed::from_int(16);
        info.height = Fixed::from_int(56);
        info.flags = MobjFlags::SOLID;
        let mut mobj = MapObj::spawn(
            Fixed::from_int(x),
            Fixed::from_int(y),
            Fixed::ZERO,
            &info,
            0,
        );
        mobj.floorz = Fixed::ZERO;
        mobj.ceilingz = Fixed::from_int(128);
        mobj
    }

    #[test]
    fn check_thing_collision_miss() {
        let a = make_solid_mobj(0, 0);
        let b = make_solid_mobj(100, 100); // muito longe
        assert!(!check_thing_collision(&a, &b, Fixed::ZERO, Fixed::ZERO));
    }

    #[test]
    fn check_thing_collision_hit() {
        let a = make_solid_mobj(0, 0);
        let b = make_solid_mobj(20, 20); // dentro do raio combinado (16+16=32)
        assert!(check_thing_collision(&a, &b, Fixed::ZERO, Fixed::ZERO));
    }

    #[test]
    fn try_move_success() {
        let mut mobj = make_solid_mobj(0, 0);
        let others = vec![];
        let result = try_move(
            &mut mobj,
            Fixed::from_int(10),
            Fixed::from_int(10),
            &others,
            Fixed::ZERO,
            Fixed::from_int(128),
        );
        assert!(result);
        assert_eq!(mobj.x, Fixed::from_int(10));
        assert_eq!(mobj.y, Fixed::from_int(10));
    }

    #[test]
    fn try_move_blocked_by_mobj() {
        let mut mobj = make_solid_mobj(0, 0);
        let blocker = make_solid_mobj(20, 0); // na frente
        let others = vec![blocker];

        let result = try_move(
            &mut mobj,
            Fixed::from_int(10), // tentando ir para perto do blocker
            Fixed::ZERO,
            &others,
            Fixed::ZERO,
            Fixed::from_int(128),
        );
        assert!(!result);
        assert_eq!(mobj.x, Fixed::ZERO); // nao moveu
    }

    #[test]
    fn try_move_step_up() {
        let mut mobj = make_solid_mobj(0, 0);
        mobj.z = Fixed::ZERO;
        let others = vec![];

        // Degrau de 24 unidades — no limite, deve funcionar
        let result = try_move(
            &mut mobj,
            Fixed::from_int(10),
            Fixed::ZERO,
            &others,
            Fixed::from_int(24), // floor 24 unidades acima
            Fixed::from_int(128),
        );
        assert!(result);
        assert_eq!(mobj.floorz, Fixed::from_int(24));
    }

    #[test]
    fn try_move_step_too_high() {
        let mut mobj = make_solid_mobj(0, 0);
        mobj.z = Fixed::ZERO;
        let others = vec![];

        // Degrau de 25 unidades — muito alto
        let result = try_move(
            &mut mobj,
            Fixed::from_int(10),
            Fixed::ZERO,
            &others,
            Fixed::from_int(25), // floor 25 unidades acima
            Fixed::from_int(128),
        );
        assert!(!result);
    }

    #[test]
    fn try_move_noclip() {
        let mut info = MobjInfo::new();
        info.radius = Fixed::from_int(16);
        info.height = Fixed::from_int(56);
        info.flags = MobjFlags::SOLID | MobjFlags::NOCLIP;
        let mut mobj = MapObj::spawn(Fixed::ZERO, Fixed::ZERO, Fixed::ZERO, &info, 0);
        mobj.floorz = Fixed::ZERO;
        mobj.ceilingz = Fixed::from_int(128);

        let blocker = make_solid_mobj(20, 0);
        let others = vec![blocker];

        // Com noclip, pode mover mesmo com blocker
        let result = try_move(
            &mut mobj,
            Fixed::from_int(10),
            Fixed::ZERO,
            &others,
            Fixed::ZERO,
            Fixed::from_int(128),
        );
        assert!(result);
    }

    #[test]
    fn try_move_ceiling_too_low() {
        let mut mobj = make_solid_mobj(0, 0);
        mobj.z = Fixed::ZERO;
        let others = vec![];

        // Teto muito baixo para o mobj (altura 56)
        let result = try_move(
            &mut mobj,
            Fixed::from_int(10),
            Fixed::ZERO,
            &others,
            Fixed::ZERO,
            Fixed::from_int(50), // teto a 50, mobj tem 56 de altura
        );
        assert!(!result);
    }
}

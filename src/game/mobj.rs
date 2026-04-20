//! # Map Objects (Mobjs) — Entidades do Mundo
//!
//! O `MapObj` (mobj) e a estrutura central do DOOM: tudo que existe
//! no mundo do jogo e um mobj — jogador, monstros, projeteis,
//! itens, decoracao, barreis. Cada mobj tem:
//!
//! - Posicao (x, y, z) e momentum (momx, momy, momz)
//! - Uma state machine que controla animacao e comportamento
//! - Flags de propriedades (solido, disparavel, etc.)
//! - Links no blockmap e sector para queries espaciais
//!
//! ## Ciclo de vida de um mobj
//!
//! 1. `spawn()` — cria mobj, inicializa de mobjinfo, liga no mundo
//! 2. `think()` — a cada tick: movimento XY, movimento Z, state machine
//! 3. `remove()` — desliga do mundo, marca para remocao
//!
//! ## State machine
//!
//! Cada mobj tem um estado atual (state) e um contador de tics.
//! A cada tick, o contador decrementa. Quando chega a zero, o mobj
//! transiciona para o proximo estado. Alguns estados chamam funcoes
//! de acao (ex: atirar, atacar, morrer).
//!
//! ## Arquivo C original: `p_mobj.h`, `p_mobj.c`
//!
//! ## Conceitos que o leitor vai aprender
//! - Entity-Component pattern (mobj como entidade universal)
//! - State machine para animacao e IA
//! - Fisica 2.5D (XY com gravidade Z separada)

use crate::utils::angle::Angle;
use crate::utils::fixed::Fixed;

use super::info::*;

/// Map Object — entidade no mundo do DOOM.
///
/// Tudo que se move, e visivel, ou interage no jogo e um MapObj:
/// jogadores, monstros, projeteis, itens, decoracao.
///
/// C original: `mobj_t` em `p_mobj.h`
#[derive(Debug, Clone)]
pub struct MapObj {
    // -- Posicao --

    /// Posicao X no mundo (fixed-point)
    pub x: Fixed,
    /// Posicao Y no mundo (fixed-point)
    pub y: Fixed,
    /// Posicao Z no mundo (fixed-point)
    pub z: Fixed,

    /// Orientacao (angulo de visao)
    pub angle: Angle,

    // -- Rendering --

    /// Indice do sprite atual
    pub sprite: SpriteNum,
    /// Frame atual do sprite (pode ter FF_FULLBRIGHT ORed)
    pub frame: i32,

    // -- Sector/blockmap links --

    /// Indice do subsector onde o mobj esta
    pub subsector: usize,
    /// Altura do chao no ponto do mobj
    pub floorz: Fixed,
    /// Altura do teto no ponto do mobj
    pub ceilingz: Fixed,

    // -- Colisao --

    /// Raio de colisao (fixed-point)
    pub radius: Fixed,
    /// Altura de colisao (fixed-point)
    pub height: Fixed,

    // -- Fisica --

    /// Momentum X (velocidade horizontal)
    pub momx: Fixed,
    /// Momentum Y (velocidade horizontal)
    pub momy: Fixed,
    /// Momentum Z (velocidade vertical — gravidade, pulos)
    pub momz: Fixed,

    // -- State machine --

    /// Tipo do mobj (indice na mobjinfo)
    pub mobj_type: MobjType,
    /// Flags de propriedades
    pub flags: MobjFlags,
    /// Indice do estado atual na tabela de estados
    pub state_num: StateNum,
    /// Contador de tics restantes no estado atual
    pub tics: i32,
    /// Vida restante
    pub health: i32,

    // -- IA --

    /// Direcao de movimento (0-7, 8 = nenhuma)
    pub movedir: i32,
    /// Contador de passos antes de trocar direcao
    pub movecount: i32,
    /// Tempo de reacao antes de atacar
    pub reactiontime: i32,
    /// Se >0, perseguir alvo sem importar o que
    pub threshold: i32,
    /// Ultimo jogador procurado (para IA)
    pub lastlook: i32,

    // -- Validacao --

    /// Contador de validacao (para evitar checagens duplicadas)
    pub validcount: i32,

    // -- Spawn info --

    /// Tipo do mobj original (para respawn em Nightmare)
    pub spawn_type: MobjType,
    /// Posicao X de spawn original
    pub spawn_x: Fixed,
    /// Posicao Y de spawn original
    pub spawn_y: Fixed,
    /// Angulo de spawn original
    pub spawn_angle: Angle,
}

impl MapObj {
    /// Cria um novo mobj com propriedades baseadas no tipo.
    ///
    /// Inicializa o mobj usando a mobjinfo do tipo fornecido.
    /// Posiciona no ponto (x, y, z) e seta o estado inicial.
    ///
    /// C original: `P_SpawnMobj()` em `p_mobj.c`
    pub fn spawn(x: Fixed, y: Fixed, z: Fixed, info: &MobjInfo, mobj_type: MobjType) -> Self {
        MapObj {
            x,
            y,
            z,
            angle: Angle(0),
            sprite: 0,
            frame: 0,
            subsector: 0,
            floorz: Fixed::ZERO,
            ceilingz: Fixed::ZERO,
            radius: info.radius,
            height: info.height,
            momx: Fixed::ZERO,
            momy: Fixed::ZERO,
            momz: Fixed::ZERO,
            mobj_type,
            flags: info.flags,
            state_num: info.spawn_state,
            tics: -1, // Sera setado pelo estado
            health: info.spawn_health,
            movedir: 0,
            movecount: 0,
            reactiontime: info.reaction_time,
            threshold: 0,
            lastlook: 0,
            validcount: 0,
            spawn_type: mobj_type,
            spawn_x: x,
            spawn_y: y,
            spawn_angle: Angle(0),
        }
    }

    /// Atualiza a state machine do mobj.
    ///
    /// Decrementa o contador de tics. Quando chega a zero,
    /// transiciona para o proximo estado.
    ///
    /// C original: parte de `P_MobjThinker()` em `p_mobj.c`
    ///
    /// Retorna `false` se o mobj deve ser removido (estado nulo).
    pub fn update_state(&mut self, states: &[State]) -> bool {
        if self.tics == -1 {
            // Duracao infinita — nao transiciona
            return true;
        }

        self.tics -= 1;
        if self.tics > 0 {
            return true;
        }

        // Transicionar para proximo estado
        let current = &states[self.state_num];
        let next = current.next_state;

        if next == S_NULL {
            // Estado nulo — mobj deve ser removido
            return false;
        }

        self.set_state(next, states);
        true
    }

    /// Seta o estado do mobj.
    ///
    /// C original: `P_SetMobjState()` em `p_mobj.c`
    pub fn set_state(&mut self, state_num: StateNum, states: &[State]) {
        if state_num == S_NULL || state_num >= states.len() {
            self.state_num = S_NULL;
            self.tics = -1;
            return;
        }

        let st = &states[state_num];
        self.state_num = state_num;
        self.tics = st.tics;
        self.sprite = st.sprite;
        self.frame = st.frame;

        // TODO: chamar action function se st.action != 0
    }

    /// Aplica momentum horizontal (XY) ao mobj.
    ///
    /// Move o mobj de acordo com momx/momy. No DOOM completo,
    /// isso envolve P_TryMove com colisao. Aqui, aplicamos
    /// diretamente para fins de teste.
    ///
    /// C original: `P_XYMovement()` em `p_mobj.c`
    pub fn apply_xy_movement(&mut self) {
        if self.momx == Fixed::ZERO && self.momy == Fixed::ZERO {
            return;
        }

        self.x += self.momx;
        self.y += self.momy;

        // Aplicar friccao (0xE800 / 0x10000 ≈ 0.906)
        // Missiles nao tem friccao
        if !self.flags.contains(MobjFlags::MISSILE) {
            if self.flags.contains(MobjFlags::CORPSE) {
                // Corpos perdem momentum mais rapido
                self.momx = self.momx / Fixed::from_int(2);
                self.momy = self.momy / Fixed::from_int(2);
            }

            // Friccao padrao: ~90.6% do momentum a cada tick
            // C original: `mo->momx = FixedMul(mo->momx, FRICTION)`
            // FRICTION = 0xE800
            let friction = Fixed(0xE800);
            self.momx = self.momx * friction;
            self.momy = self.momy * friction;
        }
    }

    /// Aplica gravidade e momentum vertical (Z) ao mobj.
    ///
    /// C original: `P_ZMovement()` em `p_mobj.c`
    pub fn apply_z_movement(&mut self) {
        // Aplicar momentum vertical
        self.z += self.momz;

        // Checar contra chao
        if self.z <= self.floorz {
            self.z = self.floorz;

            if self.momz.0 < 0 {
                self.momz = Fixed::ZERO;
            }
        }

        // Checar contra teto
        if self.z + self.height > self.ceilingz {
            self.z = self.ceilingz - self.height;

            if self.momz.0 > 0 {
                self.momz = Fixed::ZERO;
            }
        }

        // Aplicar gravidade
        if !self.flags.contains(MobjFlags::NOGRAVITY) && self.z > self.floorz {
            self.momz -= GRAVITY;
        }
    }

    /// Verifica se este mobj esta morto.
    pub fn is_dead(&self) -> bool {
        self.health <= 0
    }

    /// Verifica se este mobj e um projetil.
    pub fn is_missile(&self) -> bool {
        self.flags.contains(MobjFlags::MISSILE)
    }

    /// Verifica se este mobj e solido (bloqueia movimento).
    pub fn is_solid(&self) -> bool {
        self.flags.contains(MobjFlags::SOLID)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::fixed::FRACUNIT;

    #[test]
    fn spawn_player() {
        let info = MobjInfo::player();
        let mobj = MapObj::spawn(
            Fixed::from_int(100),
            Fixed::from_int(200),
            Fixed::ZERO,
            &info,
            MT_PLAYER,
        );
        assert_eq!(mobj.x, Fixed::from_int(100));
        assert_eq!(mobj.y, Fixed::from_int(200));
        assert_eq!(mobj.health, 100);
        assert!(mobj.is_solid());
        assert!(!mobj.is_missile());
    }

    #[test]
    fn spawn_imp() {
        let info = MobjInfo::imp();
        let mobj = MapObj::spawn(Fixed::ZERO, Fixed::ZERO, Fixed::ZERO, &info, MT_TROOP);
        assert_eq!(mobj.health, 60);
        assert_eq!(mobj.radius, Fixed(20 * FRACUNIT));
        assert!(mobj.flags.contains(MobjFlags::COUNTKILL));
    }

    #[test]
    fn z_movement_gravity() {
        let info = MobjInfo::imp();
        let mut mobj = MapObj::spawn(
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::from_int(100), // no ar
            &info,
            MT_TROOP,
        );
        mobj.floorz = Fixed::ZERO;
        mobj.ceilingz = Fixed::from_int(256);

        // Aplicar gravidade por varios ticks
        for _ in 0..10 {
            mobj.apply_z_movement();
        }

        // Deve ter caido
        assert!(mobj.z < Fixed::from_int(100));
        // Mas nao abaixo do chao
        assert!(mobj.z >= mobj.floorz);
    }

    #[test]
    fn z_movement_floor_stop() {
        let info = MobjInfo::player();
        let mut mobj = MapObj::spawn(
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::from_int(10),
            &info,
            MT_PLAYER,
        );
        mobj.floorz = Fixed::ZERO;
        mobj.ceilingz = Fixed::from_int(128);
        mobj.momz = Fixed::from_int(-50); // caindo rapido

        mobj.apply_z_movement();

        assert_eq!(mobj.z, Fixed::ZERO); // para no chao
        assert_eq!(mobj.momz, Fixed::ZERO); // momentum zerado
    }

    #[test]
    fn xy_movement_friction() {
        let info = MobjInfo::player();
        let mut mobj = MapObj::spawn(Fixed::ZERO, Fixed::ZERO, Fixed::ZERO, &info, MT_PLAYER);
        mobj.momx = Fixed::from_int(10);

        mobj.apply_xy_movement();

        // Posicao deve ter avancado
        assert!(mobj.x > Fixed::ZERO);
        // Momentum deve ter diminuido (friccao)
        assert!(mobj.momx < Fixed::from_int(10));
        assert!(mobj.momx > Fixed::ZERO);
    }

    #[test]
    fn state_machine() {
        // Criar 3 estados: 0=null, 1→2, 2→0 (remove)
        let states = vec![
            State::new(), // S_NULL
            State {
                tics: 3,
                next_state: 2,
                ..State::new()
            },
            State {
                tics: 2,
                next_state: S_NULL,
                ..State::new()
            },
        ];

        let info = MobjInfo::new();
        let mut mobj = MapObj::spawn(Fixed::ZERO, Fixed::ZERO, Fixed::ZERO, &info, 0);
        mobj.set_state(1, &states);
        assert_eq!(mobj.tics, 3);

        // Tick 1, 2: still in state 1
        assert!(mobj.update_state(&states));
        assert_eq!(mobj.tics, 2);
        assert!(mobj.update_state(&states));
        assert_eq!(mobj.tics, 1);

        // Tick 3: transition to state 2
        assert!(mobj.update_state(&states));
        assert_eq!(mobj.state_num, 2);
        assert_eq!(mobj.tics, 2);

        // Tick 4: state 2, tics 2→1
        assert!(mobj.update_state(&states));

        // Tick 5: tics 1→0, transition to S_NULL → remove
        assert!(!mobj.update_state(&states));
    }

    #[test]
    fn is_dead() {
        let info = MobjInfo::imp();
        let mut mobj = MapObj::spawn(Fixed::ZERO, Fixed::ZERO, Fixed::ZERO, &info, MT_TROOP);
        assert!(!mobj.is_dead());
        mobj.health = 0;
        assert!(mobj.is_dead());
    }
}

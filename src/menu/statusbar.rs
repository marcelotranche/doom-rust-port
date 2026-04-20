//! # Status Bar — A Barra de Status do DOOM
//!
//! A status bar ocupa os 32 pixels inferiores da tela e exibe:
//! - Municao da arma atual (esquerda)
//! - Saude em porcentagem (centro-esquerda)
//! - Face do Doomguy com expressoes dinamicas (centro)
//! - Armor em porcentagem (centro-direita)
//! - Chaves coletadas (direita)
//! - Inventario de armas (centro inferior)
//!
//! ## Face do Doomguy
//!
//! A face tem 42 sprites diferentes controlados por um sistema
//! de prioridade. A face reage ao estado do jogador:
//!
//! ```text
//! Prioridade (maior = mais importante):
//!   9 — Morto (olhos em X)
//!   8 — Quase morto (< 10% HP)
//!   7 — Ouch face (dano grande de uma vez)
//!   7 — Olhando para direcao do dano
//!   6 — Rampage (ataque continuo)
//!   5 — Disparando (evil grin ao matar)
//!   4 — God mode (olhos dourados)
//!   0 — Olhando em frente (idle)
//! ```
//!
//! ## Paletas de dano/bonus
//!
//! A status bar controla a paleta de cores da tela:
//! - Dano: paletas vermelhas (indices 1-8, intensidade por dano)
//! - Bonus: paletas douradas (indices 9-12)
//! - Radiacao: paleta verde (indice 13, com radiation suit)
//!
//! ## Arquivo C original: `st_stuff.c`, `st_stuff.h`
//!
//! ## Conceitos que o leitor vai aprender
//! - Sistema de prioridade para selecao de estado visual
//! - Dirty-checking com widgets compostos
//! - Mapeamento dano→paleta de cores

use super::st_widgets::*;

/// Largura da status bar em pixels.
///
/// C original: `#define ST_WIDTH SCREENWIDTH` (320)
pub const ST_WIDTH: i32 = 320;

/// Altura da status bar em pixels.
///
/// C original: `#define ST_HEIGHT 32` em `st_stuff.c`
pub const ST_HEIGHT: i32 = 32;

/// Posicao Y da status bar na tela.
///
/// C original: `#define ST_Y (SCREENHEIGHT - ST_HEIGHT)` = 168
pub const ST_Y: i32 = 200 - ST_HEIGHT;

// ---------------------------------------------------------------------------
// Face do Doomguy
// ---------------------------------------------------------------------------

/// Numero de niveis de dor na face (5 faixas de saude).
///
/// C original: `#define ST_NUMPAINFACES 5` em `st_stuff.c`
pub const ST_NUMPAINFACES: usize = 5;

/// Faces olhando em frente por nivel de dor.
///
/// C original: `#define ST_NUMSTRAIGHTFACES 3`
pub const ST_NUMSTRAIGHTFACES: usize = 3;

/// Faces olhando para os lados por nivel de dor.
///
/// C original: `#define ST_NUMTURNFACES 2`
pub const ST_NUMTURNFACES: usize = 2;

/// Faces especiais (ouch, evil grin, rampage).
///
/// C original: `#define ST_NUMSPECIALFACES 3`
pub const ST_NUMSPECIALFACES: usize = 3;

/// Stride entre faixas de dor na tabela de faces.
///
/// C original: `#define ST_FACESTRIDE (ST_NUMSTRAIGHTFACES+ST_NUMTURNFACES+ST_NUMSPECIALFACES)`
pub const ST_FACESTRIDE: usize =
    ST_NUMSTRAIGHTFACES + ST_NUMTURNFACES + ST_NUMSPECIALFACES; // 8

/// Numero total de faces do Doomguy.
///
/// 5 niveis * 8 faces + god mode + dead = 42
///
/// C original: `#define ST_NUMFACES (ST_FACESTRIDE*ST_NUMPAINFACES+2)`
pub const ST_NUMFACES: usize = ST_FACESTRIDE * ST_NUMPAINFACES + 2; // 42

/// Indice da face de god mode.
pub const ST_GODFACE: usize = ST_NUMPAINFACES * ST_FACESTRIDE;

/// Indice da face morta.
pub const ST_DEADFACE: usize = ST_GODFACE + 1;

// ---------------------------------------------------------------------------
// Paletas
// ---------------------------------------------------------------------------

/// Indice da primeira paleta vermelha (dano).
///
/// C original: `#define STARTREDPALS 1`
pub const STARTREDPALS: i32 = 1;

/// Numero de paletas vermelhas.
///
/// C original: `#define NUMREDPALS 8`
pub const NUMREDPALS: i32 = 8;

/// Indice da primeira paleta dourada (bonus).
///
/// C original: `#define STARTBONUSPALS 9`
pub const STARTBONUSPALS: i32 = 9;

/// Numero de paletas douradas.
///
/// C original: `#define NUMBONUSPALS 4`
pub const NUMBONUSPALS: i32 = 4;

/// Paleta verde (radiation suit).
///
/// C original: `#define RADIATIONPAL 13`
pub const RADIATIONPAL: i32 = 13;

// ---------------------------------------------------------------------------
// Posicoes dos widgets na status bar
// ---------------------------------------------------------------------------

/// Posicao X da municao.
pub const ST_AMMOX: i32 = 44;
/// Posicao Y da municao.
pub const ST_AMMOY: i32 = ST_Y + 3;
/// Posicao X da saude.
pub const ST_HEALTHX: i32 = 90;
/// Posicao Y da saude.
pub const ST_HEALTHY: i32 = ST_Y + 3;
/// Posicao X da face.
pub const ST_FACEX: i32 = 143;
/// Posicao Y da face.
pub const ST_FACEY: i32 = ST_Y;
/// Posicao X do armor.
pub const ST_ARMORX: i32 = 221;
/// Posicao Y do armor.
pub const ST_ARMORY: i32 = ST_Y + 3;

// ---------------------------------------------------------------------------
// Numero de tipos de chaves
// ---------------------------------------------------------------------------

/// Numero de tipos de chaves/cards.
///
/// C original: `NUMCARDS` em `doomdef.h` (6: blue/yellow/red card + skull)
pub const NUMCARDS: usize = 6;

/// Numero de tipos de armas.
pub const NUMWEAPONS: usize = 9;

/// Numero de tipos de municao.
pub const NUMAMMO: usize = 4;

// ---------------------------------------------------------------------------
// StatusBar
// ---------------------------------------------------------------------------

/// Status bar do DOOM — exibe informacoes do jogador.
///
/// C original: variaveis locais em `st_stuff.c`
/// (`w_ready`, `w_health`, `w_armor`, `w_faces`, etc.)
#[derive(Debug)]
pub struct StatusBar {
    /// Widget de municao da arma atual
    pub w_ammo: StNumber,
    /// Widget de saude (com %)
    pub w_health: StPercent,
    /// Widget de armor (com %)
    pub w_armor: StPercent,
    /// Widget da face do Doomguy
    pub w_face: StMultIcon,
    /// Widgets de armas no inventario (6 armas visiveis)
    pub w_arms: [StMultIcon; 6],
    /// Widgets de chaves
    pub w_keyboxes: [StMultIcon; 3],
    /// Widgets de municao por tipo (4 tipos)
    pub w_ammo_counts: [StNumber; 4],
    /// Widgets de municao maxima por tipo
    pub w_max_ammo: [StNumber; 4],
    /// Indice da face atual
    pub face_index: usize,
    /// Contador de prioridade da face
    pub face_priority: i32,
    /// Contador de ticks para mudanca de face
    pub face_count: i32,
    /// Ultimo valor de saude (para detectar dano)
    pub old_health: i32,
    /// Paleta atual da tela
    pub palette: i32,
    /// Se a status bar esta visivel
    pub status_bar_on: bool,
    /// Se precisa redesenhar completamente
    pub needs_refresh: bool,
}

impl StatusBar {
    /// Cria uma nova status bar com todos os widgets.
    ///
    /// C original: `ST_createWidgets()` em `st_stuff.c`
    pub fn new() -> Self {
        StatusBar {
            w_ammo: StNumber::new(ST_AMMOX, ST_AMMOY, 3),
            w_health: StPercent::new(ST_HEALTHX, ST_HEALTHY),
            w_armor: StPercent::new(ST_ARMORX, ST_ARMORY),
            w_face: StMultIcon::new(ST_FACEX, ST_FACEY),
            w_arms: [
                StMultIcon::new(111, ST_Y + 4),
                StMultIcon::new(123, ST_Y + 4),
                StMultIcon::new(135, ST_Y + 4),
                StMultIcon::new(111, ST_Y + 14),
                StMultIcon::new(123, ST_Y + 14),
                StMultIcon::new(135, ST_Y + 14),
            ],
            w_keyboxes: [
                StMultIcon::new(239, ST_Y + 3),
                StMultIcon::new(239, ST_Y + 13),
                StMultIcon::new(239, ST_Y + 23),
            ],
            w_ammo_counts: [
                StNumber::new(288, ST_Y + 5, 3),
                StNumber::new(288, ST_Y + 11, 3),
                StNumber::new(288, ST_Y + 23, 3),
                StNumber::new(288, ST_Y + 17, 3),
            ],
            w_max_ammo: [
                StNumber::new(314, ST_Y + 5, 3),
                StNumber::new(314, ST_Y + 11, 3),
                StNumber::new(314, ST_Y + 23, 3),
                StNumber::new(314, ST_Y + 17, 3),
            ],
            face_index: 0,
            face_priority: 0,
            face_count: 0,
            old_health: -1,
            palette: 0,
            status_bar_on: true,
            needs_refresh: true,
        }
    }

    /// Inicializa a status bar para um novo nivel.
    ///
    /// C original: `ST_Start()` → `ST_initData()` em `st_stuff.c`
    pub fn start(&mut self) {
        self.face_index = 0;
        self.face_priority = 0;
        self.face_count = 0;
        self.old_health = -1;
        self.palette = 0;
        self.needs_refresh = true;
    }

    /// Atualiza a status bar a cada tick.
    ///
    /// Atualiza a face do Doomguy e todos os widgets
    /// baseado no estado atual do jogador.
    ///
    /// C original: `ST_Ticker()` em `st_stuff.c`
    pub fn ticker(&mut self, player: &PlayerStatusInfo) {
        if !self.status_bar_on {
            return;
        }

        // Atualizar face do Doomguy
        self.update_face(player);

        // Atualizar paleta baseado no dano/bonus
        self.update_palette(player);

        // Atualizar widgets
        let refresh = self.needs_refresh;
        self.w_ammo.update(player.ammo, refresh);
        self.w_health.update(player.health, refresh);
        self.w_armor.update(player.armor, refresh);
        self.w_face.update(self.face_index as i32, refresh);

        // Atualizar armas
        for (i, w) in self.w_arms.iter_mut().enumerate() {
            let idx = if player.weapon_owned[i + 1] { 1 } else { 0 };
            w.update(idx, refresh);
        }

        // Atualizar chaves
        for (i, w) in self.w_keyboxes.iter_mut().enumerate() {
            let idx = if player.cards[i] { i as i32 } else { -1 };
            w.update(idx, refresh);
        }

        // Atualizar contagens de municao
        for (i, w) in self.w_ammo_counts.iter_mut().enumerate() {
            w.update(player.ammo_counts[i], refresh);
        }
        for (i, w) in self.w_max_ammo.iter_mut().enumerate() {
            w.update(player.max_ammo[i], refresh);
        }

        self.needs_refresh = false;
    }

    /// Atualiza a face do Doomguy baseado no estado do jogador.
    ///
    /// Sistema de prioridade: estados mais importantes
    /// (morte, dano) sobrepoe estados menos importantes (idle).
    ///
    /// C original: `ST_updateFaceWidget()` em `st_stuff.c`
    fn update_face(&mut self, player: &PlayerStatusInfo) {
        // Decrementar contador
        if self.face_count > 0 {
            self.face_count -= 1;
            if self.face_count > 0 {
                return; // Manter face atual
            }
        }

        // Calcular faixa de dor baseada na saude
        let pain_offset = self.pain_offset(player.health);

        // Prioridade 9: Morto
        if player.health <= 0 {
            self.face_index = ST_DEADFACE;
            self.face_priority = 9;
            self.face_count = 1;
            return;
        }

        // Prioridade 4: God mode
        if player.cheats_godmode {
            self.face_index = ST_GODFACE;
            self.face_priority = 4;
            self.face_count = 1;
            return;
        }

        // Prioridade 7: Ouch face (dano grande)
        if player.health - self.old_health < -20 && self.face_priority < 8 {
            self.face_index = pain_offset * ST_FACESTRIDE;
            // Usar ouch face (indice 6 dentro do stride)
            self.face_index += ST_NUMSTRAIGHTFACES + ST_NUMTURNFACES; // ouch
            self.face_priority = 7;
            self.face_count = 35; // 1 segundo
            self.old_health = player.health;
            return;
        }

        // Prioridade 0: Olhando em frente (idle)
        self.face_index = pain_offset * ST_FACESTRIDE;
        self.face_priority = 0;
        self.face_count = 15; // ~0.4 segundos entre mudancas

        self.old_health = player.health;
    }

    /// Calcula a faixa de dor baseada na saude (0-4).
    ///
    /// C original: `ST_calcPainOffset()` em `st_stuff.c`
    fn pain_offset(&self, health: i32) -> usize {
        let health = health.clamp(0, 100);
        // C original: ST_NUMPAINFACES * (100 - health) / 100
        // health 100 → 0 (saudavel), health 0 → 4 (quase morto)
        let offset = (ST_NUMPAINFACES * (100 - health as usize)) / 101;
        offset.min(ST_NUMPAINFACES - 1)
    }

    /// Atualiza a paleta de cores da tela.
    ///
    /// Seleciona entre paleta normal, vermelha (dano),
    /// dourada (bonus) ou verde (radiation suit).
    ///
    /// C original: `ST_doPaletteStuff()` em `st_stuff.c`
    fn update_palette(&mut self, player: &PlayerStatusInfo) {
        let new_palette;

        if player.damage_count > 0 {
            // Paletas vermelhas proporcionais ao dano
            let pal = (player.damage_count + 7) >> 3;
            new_palette = STARTREDPALS + pal.min(NUMREDPALS);
        } else if player.bonus_count > 0 {
            // Paletas douradas proporcionais ao bonus
            let pal = (player.bonus_count + 7) >> 3;
            new_palette = STARTBONUSPALS + pal.min(NUMBONUSPALS);
        } else if player.powers_ironfeet > 0 {
            // Paleta verde (radiation suit)
            new_palette = RADIATIONPAL;
        } else {
            new_palette = 0; // paleta normal
        }

        self.palette = new_palette;
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Informacoes do jogador para a status bar
// ---------------------------------------------------------------------------

/// Informacoes do jogador necessarias para atualizar a status bar.
///
/// Struct intermediaria que desacopla a status bar do estado
/// completo do jogador. No DOOM original, os widgets apontam
/// diretamente para campos do `player_t`.
#[derive(Debug, Clone)]
pub struct PlayerStatusInfo {
    /// Saude atual (0-200)
    pub health: i32,
    /// Armor atual (0-200)
    pub armor: i32,
    /// Municao da arma atual
    pub ammo: i32,
    /// Armas possuidas (indice 0 = fist, sempre true)
    pub weapon_owned: [bool; NUMWEAPONS],
    /// Chaves possuidas (blue/yellow/red)
    pub cards: [bool; 3],
    /// Contagem de municao por tipo
    pub ammo_counts: [i32; NUMAMMO],
    /// Municao maxima por tipo
    pub max_ammo: [i32; NUMAMMO],
    /// Contagem de dano (para paleta vermelha)
    pub damage_count: i32,
    /// Contagem de bonus (para paleta dourada)
    pub bonus_count: i32,
    /// Duracao restante de radiation suit
    pub powers_ironfeet: i32,
    /// Se godmode esta ativo
    pub cheats_godmode: bool,
}

impl PlayerStatusInfo {
    /// Cria um estado de jogador padrao (inicio de nivel).
    pub fn new() -> Self {
        let mut weapon_owned = [false; NUMWEAPONS];
        weapon_owned[0] = true; // fist
        weapon_owned[1] = true; // pistol

        PlayerStatusInfo {
            health: 100,
            armor: 0,
            ammo: 50,
            weapon_owned,
            cards: [false; 3],
            ammo_counts: [50, 0, 0, 0], // bullets, shells, cells, rockets
            max_ammo: [200, 50, 300, 50],
            damage_count: 0,
            bonus_count: 0,
            powers_ironfeet: 0,
            cheats_godmode: false,
        }
    }
}

impl Default for PlayerStatusInfo {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statusbar_init() {
        let sb = StatusBar::new();
        assert!(sb.status_bar_on);
        assert!(sb.needs_refresh);
        assert_eq!(sb.face_index, 0);
    }

    #[test]
    fn statusbar_ticker_updates_widgets() {
        let mut sb = StatusBar::new();
        let player = PlayerStatusInfo::new();
        sb.ticker(&player);
        assert!(!sb.needs_refresh); // refresh consumido
        assert_eq!(sb.w_health.value(), 100);
        assert_eq!(sb.w_armor.value(), 0);
    }

    #[test]
    fn face_dead() {
        let mut sb = StatusBar::new();
        let mut player = PlayerStatusInfo::new();
        player.health = 0;
        sb.ticker(&player);
        assert_eq!(sb.face_index, ST_DEADFACE);
    }

    #[test]
    fn face_godmode() {
        let mut sb = StatusBar::new();
        let mut player = PlayerStatusInfo::new();
        player.cheats_godmode = true;
        sb.ticker(&player);
        assert_eq!(sb.face_index, ST_GODFACE);
    }

    #[test]
    fn face_ouch() {
        let mut sb = StatusBar::new();
        let mut player = PlayerStatusInfo::new();

        // Primeiro tick — saude normal
        sb.ticker(&player);
        let first_face = sb.face_index;
        assert_ne!(first_face, ST_DEADFACE);

        // Dano grande (mais de 20 de uma vez)
        sb.face_count = 0; // forcar atualizacao
        player.health = 50; // -50 HP
        sb.ticker(&player);
        // Face deve ter mudado para ouch (offset 5 ou 6 no stride)
        assert_ne!(sb.face_index, first_face);
    }

    #[test]
    fn pain_offset_ranges() {
        let sb = StatusBar::new();
        assert_eq!(sb.pain_offset(100), 0); // saudavel
        assert_eq!(sb.pain_offset(80), 0);
        assert_eq!(sb.pain_offset(50), 2); // meio
        assert_eq!(sb.pain_offset(10), 4); // quase morto
        assert_eq!(sb.pain_offset(0), 4);
    }

    #[test]
    fn palette_damage() {
        let mut sb = StatusBar::new();
        let mut player = PlayerStatusInfo::new();
        player.damage_count = 20;
        sb.ticker(&player);
        assert!(sb.palette >= STARTREDPALS);
        assert!(sb.palette <= STARTREDPALS + NUMREDPALS);
    }

    #[test]
    fn palette_bonus() {
        let mut sb = StatusBar::new();
        let mut player = PlayerStatusInfo::new();
        player.bonus_count = 10;
        sb.ticker(&player);
        assert!(sb.palette >= STARTBONUSPALS);
    }

    #[test]
    fn palette_radiation() {
        let mut sb = StatusBar::new();
        let mut player = PlayerStatusInfo::new();
        player.powers_ironfeet = 100;
        sb.ticker(&player);
        assert_eq!(sb.palette, RADIATIONPAL);
    }

    #[test]
    fn palette_normal() {
        let mut sb = StatusBar::new();
        let player = PlayerStatusInfo::new();
        sb.ticker(&player);
        assert_eq!(sb.palette, 0);
    }

    #[test]
    fn face_constants() {
        assert_eq!(ST_FACESTRIDE, 8);
        assert_eq!(ST_NUMFACES, 42);
        assert_eq!(ST_GODFACE, 40);
        assert_eq!(ST_DEADFACE, 41);
    }

    #[test]
    fn player_info_defaults() {
        let p = PlayerStatusInfo::new();
        assert_eq!(p.health, 100);
        assert!(p.weapon_owned[0]); // fist
        assert!(p.weapon_owned[1]); // pistol
        assert!(!p.weapon_owned[2]); // shotgun nao
        assert_eq!(p.max_ammo[0], 200); // max bullets
    }
}

//! # Tela de Intermissao — Estatisticas entre Niveis
//!
//! A tela de intermissao aparece entre niveis e exibe:
//! - Estatisticas do nivel concluido (kills, items, secrets)
//! - Tempo de conclusao vs tempo par
//! - Mapa do episodio com localizacao do proximo nivel
//!
//! ## Maquina de estados
//!
//! ```text
//! StatCount ──────> ShowNextLoc ──────> proximo nivel
//!   |                   |
//!   contadores          "You Are Here"
//!   animados            piscando no mapa
//!   kills/items/        do episodio
//!   secrets/time
//! ```
//!
//! ## Contadores animados
//!
//! Os contadores de kills/items/secrets incrementam gradualmente
//! do 0 ate o valor real, criando um efeito dramatico. O jogador
//! pode pressionar qualquer tecla para pular a animacao.
//!
//! ## Animacoes de fundo
//!
//! O mapa de episodio (DOOM 1) tem animacoes de fundo:
//! - ANIM_ALWAYS: repete continuamente (luzes piscando)
//! - ANIM_RANDOM: dispara em intervalos aleatorios
//! - ANIM_LEVEL: dispara quando o nivel correspondente e completado
//!
//! ## Arquivo C original: `wi_stuff.c`, `wi_stuff.h`
//!
//! ## Conceitos que o leitor vai aprender
//! - Contadores animados com aceleracao por input
//! - Maquina de estados para fluxo de tela
//! - Animacoes de fundo com tipos diferentes

/// Estado da tela de intermissao.
///
/// C original: `stateenum_t` em `wi_stuff.h`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntermissionState {
    /// Contando estatisticas
    StatCount,
    /// Mostrando localizacao do proximo nivel no mapa
    ShowNextLoc,
}

/// Tipo de animacao de fundo.
///
/// C original: `animenum_t` em `wi_stuff.c`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimType {
    /// Repete sempre (luzes, maquinas)
    Always,
    /// Dispara aleatoriamente
    Random,
    /// Dispara quando nivel especifico e completado
    Level,
}

/// Animacao de fundo do mapa de episodio.
///
/// C original: `anim_t` em `wi_stuff.c`
#[derive(Debug, Clone)]
pub struct BackgroundAnim {
    /// Tipo da animacao
    pub anim_type: AnimType,
    /// Periodo entre frames (em ticks)
    pub period: i32,
    /// Numero de frames
    pub num_frames: usize,
    /// Posicao na tela
    pub x: i32,
    /// Posicao na tela
    pub y: i32,
    /// Frame atual
    pub current_frame: usize,
    /// Tick do proximo frame
    pub next_tic: i32,
    /// Contador de ticks
    pub counter: i32,
}

impl BackgroundAnim {
    /// Cria uma animacao de tipo "always" (loop continuo).
    pub fn always(period: i32, num_frames: usize, x: i32, y: i32) -> Self {
        BackgroundAnim {
            anim_type: AnimType::Always,
            period,
            num_frames,
            x,
            y,
            current_frame: 0,
            next_tic: period,
            counter: 0,
        }
    }

    /// Atualiza o frame da animacao.
    ///
    /// Retorna `true` se o frame mudou.
    pub fn update(&mut self, tic: i32) -> bool {
        if tic < self.next_tic {
            return false;
        }

        match self.anim_type {
            AnimType::Always => {
                self.current_frame = (self.current_frame + 1) % self.num_frames;
                self.next_tic = tic + self.period;
                true
            }
            AnimType::Random => {
                self.current_frame = (self.current_frame + 1) % self.num_frames;
                self.next_tic = tic + self.period;
                true
            }
            AnimType::Level => false, // controlado externamente
        }
    }
}

// ---------------------------------------------------------------------------
// Dados do nivel concluido
// ---------------------------------------------------------------------------

/// Estatisticas do nivel para a tela de intermissao.
///
/// C original: `wbstartstruct_t` / `wbplayerstruct_t` em `wi_stuff.h`
#[derive(Debug, Clone)]
pub struct LevelStats {
    /// Episodio do nivel concluido (0-based)
    pub episode: i32,
    /// Mapa do nivel concluido (0-based)
    pub last_map: i32,
    /// Proximo mapa a ser carregado
    pub next_map: i32,
    /// Total de monstros no nivel
    pub max_kills: i32,
    /// Monstros eliminados pelo jogador
    pub kills: i32,
    /// Total de itens no nivel
    pub max_items: i32,
    /// Itens coletados
    pub items: i32,
    /// Total de segredos no nivel
    pub max_secrets: i32,
    /// Segredos descobertos
    pub secrets: i32,
    /// Tempo de conclusao (em ticks)
    pub time: i32,
    /// Tempo par do nivel (em ticks)
    pub par_time: i32,
}

impl LevelStats {
    /// Cria estatisticas vazias.
    pub fn new() -> Self {
        LevelStats {
            episode: 0,
            last_map: 0,
            next_map: 1,
            max_kills: 0,
            kills: 0,
            max_items: 0,
            items: 0,
            max_secrets: 0,
            secrets: 0,
            time: 0,
            par_time: 0,
        }
    }

    /// Calcula a porcentagem de kills.
    pub fn kill_percent(&self) -> i32 {
        if self.max_kills == 0 {
            return 100;
        }
        (self.kills * 100) / self.max_kills
    }

    /// Calcula a porcentagem de itens.
    pub fn item_percent(&self) -> i32 {
        if self.max_items == 0 {
            return 100;
        }
        (self.items * 100) / self.max_items
    }

    /// Calcula a porcentagem de segredos.
    pub fn secret_percent(&self) -> i32 {
        if self.max_secrets == 0 {
            return 100;
        }
        (self.secrets * 100) / self.max_secrets
    }
}

impl Default for LevelStats {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// IntermissionScreen
// ---------------------------------------------------------------------------

/// Velocidade de incremento dos contadores (por tick).
///
/// C original: `#define COUNTRATE 3` / incrementos hardcoded
pub const COUNT_RATE: i32 = 2;

/// Duracao da fase ShowNextLoc antes de avancar (em ticks).
pub const SHOWNEXTLOC_DELAY: i32 = 4 * 35; // 4 segundos

/// Tela de intermissao — exibe estatisticas entre niveis.
///
/// C original: variaveis locais em `wi_stuff.c`
/// (`state`, `cnt_kills`, `cnt_items`, `cnt_secret`, etc.)
#[derive(Debug)]
pub struct IntermissionScreen {
    /// Estado atual
    pub state: IntermissionState,
    /// Estatisticas do nivel
    pub stats: LevelStats,
    /// Contador animado de kills (0 ate kills%)
    pub cnt_kills: i32,
    /// Contador animado de items (0 ate items%)
    pub cnt_items: i32,
    /// Contador animado de secrets (0 ate secrets%)
    pub cnt_secrets: i32,
    /// Contador animado de tempo (segundos)
    pub cnt_time: i32,
    /// Contador animado de par time (segundos)
    pub cnt_par: i32,
    /// Se o jogador pressionou para acelerar
    pub accelerate: bool,
    /// Tick counter local
    pub tic_count: i32,
    /// Animacoes de fundo
    pub anims: Vec<BackgroundAnim>,
    /// Delay restante para ShowNextLoc
    pub next_loc_delay: i32,
}

impl IntermissionScreen {
    /// Inicia a tela de intermissao com as estatisticas do nivel.
    ///
    /// C original: `WI_Start()` em `wi_stuff.c`
    pub fn new(stats: LevelStats) -> Self {
        IntermissionScreen {
            state: IntermissionState::StatCount,
            stats,
            cnt_kills: 0,
            cnt_items: 0,
            cnt_secrets: 0,
            cnt_time: 0,
            cnt_par: 0,
            accelerate: false,
            tic_count: 0,
            anims: Vec::new(),
            next_loc_delay: SHOWNEXTLOC_DELAY,
        }
    }

    /// Atualiza a tela de intermissao a cada tick.
    ///
    /// Incrementa contadores ou espera para avancar.
    ///
    /// C original: `WI_Ticker()` em `wi_stuff.c`
    pub fn ticker(&mut self) {
        self.tic_count += 1;

        // Atualizar animacoes de fundo
        for anim in &mut self.anims {
            anim.update(self.tic_count);
        }

        match self.state {
            IntermissionState::StatCount => self.update_stat_count(),
            IntermissionState::ShowNextLoc => self.update_show_next_loc(),
        }
    }

    /// Atualiza a fase de contagem de estatisticas.
    ///
    /// Incrementa contadores gradualmente ate atingir o valor real.
    /// Se `accelerate`, pula direto para os valores finais.
    fn update_stat_count(&mut self) {
        if self.accelerate {
            // Pular para valores finais
            self.cnt_kills = self.stats.kill_percent();
            self.cnt_items = self.stats.item_percent();
            self.cnt_secrets = self.stats.secret_percent();
            self.cnt_time = self.stats.time / 35; // ticks para segundos
            self.cnt_par = self.stats.par_time / 35;
            self.state = IntermissionState::ShowNextLoc;
            self.accelerate = false;
            return;
        }

        let mut all_done = true;

        // Incrementar kills
        let target_kills = self.stats.kill_percent();
        if self.cnt_kills < target_kills {
            self.cnt_kills += COUNT_RATE;
            if self.cnt_kills > target_kills {
                self.cnt_kills = target_kills;
            }
            all_done = false;
        }

        // Incrementar items
        let target_items = self.stats.item_percent();
        if self.cnt_items < target_items {
            self.cnt_items += COUNT_RATE;
            if self.cnt_items > target_items {
                self.cnt_items = target_items;
            }
            all_done = false;
        }

        // Incrementar secrets
        let target_secrets = self.stats.secret_percent();
        if self.cnt_secrets < target_secrets {
            self.cnt_secrets += COUNT_RATE;
            if self.cnt_secrets > target_secrets {
                self.cnt_secrets = target_secrets;
            }
            all_done = false;
        }

        // Incrementar tempo
        let target_time = self.stats.time / 35;
        if self.cnt_time < target_time {
            self.cnt_time += 3; // 3 segundos por tick
            if self.cnt_time > target_time {
                self.cnt_time = target_time;
            }
            all_done = false;
        }

        // Incrementar par time
        let target_par = self.stats.par_time / 35;
        if self.cnt_par < target_par {
            self.cnt_par += 3;
            if self.cnt_par > target_par {
                self.cnt_par = target_par;
            }
            all_done = false;
        }

        if all_done {
            self.state = IntermissionState::ShowNextLoc;
        }
    }

    /// Atualiza a fase de localizacao do proximo nivel.
    fn update_show_next_loc(&mut self) {
        if self.accelerate {
            self.next_loc_delay = 0;
            self.accelerate = false;
        }

        self.next_loc_delay -= 1;
    }

    /// Processa input do jogador (qualquer tecla acelera).
    ///
    /// C original: `WI_Responder()` em `wi_stuff.c`
    pub fn responder(&mut self, _key: u8, key_down: bool) -> bool {
        if key_down {
            self.accelerate = true;
            return true;
        }
        false
    }

    /// Verifica se a intermissao terminou.
    pub fn is_finished(&self) -> bool {
        self.state == IntermissionState::ShowNextLoc && self.next_loc_delay <= 0
    }
}

// ---------------------------------------------------------------------------
// Finale
// ---------------------------------------------------------------------------

/// Estado da tela de finale.
///
/// C original: `finalestage` em `f_finale.c`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinaleStage {
    /// Text crawl (historia do episodio)
    TextCrawl,
    /// Imagem artistica
    ArtScreen,
    /// Desfile de monstros (cast call)
    CastSequence,
}

/// Velocidade do text crawl (ticks por caractere).
///
/// C original: `#define TEXTSPEED 3` em `f_finale.c`
pub const TEXTSPEED: i32 = 3;

/// Espera apos o text crawl antes de avancar (ticks).
///
/// C original: `#define TEXTWAIT 250` em `f_finale.c`
pub const TEXTWAIT: i32 = 250;

/// Entrada no desfile de monstros (cast sequence).
///
/// C original: `castinfo_t` em `f_finale.c`
#[derive(Debug, Clone)]
pub struct CastEntry {
    /// Nome do monstro
    pub name: &'static str,
    /// Tipo do monstro (indice em mobjinfo)
    pub mobj_type: usize,
}

/// Sequencia de cast do DOOM (desfile de monstros).
///
/// C original: `castorder[]` em `f_finale.c`
pub fn cast_order() -> Vec<CastEntry> {
    vec![
        CastEntry { name: "Zombieman", mobj_type: 1 },
        CastEntry { name: "Shotgun Guy", mobj_type: 2 },
        CastEntry { name: "Heavy Weapon Dude", mobj_type: 3 },
        CastEntry { name: "Imp", mobj_type: 9 },
        CastEntry { name: "Demon", mobj_type: 10 },
        CastEntry { name: "Lost Soul", mobj_type: 15 },
        CastEntry { name: "Cacodemon", mobj_type: 19 },
        CastEntry { name: "Hell Knight", mobj_type: 16 },
        CastEntry { name: "Baron of Hell", mobj_type: 6 },
        CastEntry { name: "Arachnotron", mobj_type: 22 },
        CastEntry { name: "Pain Elemental", mobj_type: 23 },
        CastEntry { name: "Revenant", mobj_type: 20 },
        CastEntry { name: "Mancubus", mobj_type: 21 },
        CastEntry { name: "Arch-Vile", mobj_type: 24 },
        CastEntry { name: "The Spider Mastermind", mobj_type: 7 },
        CastEntry { name: "The Cyberdemon", mobj_type: 8 },
        CastEntry { name: "Our Hero", mobj_type: 0 },
    ]
}

/// Tela de finale — historia, arte e desfile de monstros.
///
/// C original: globals em `f_finale.c`
/// (`finalestage`, `finalecount`, `finaletext`, etc.)
#[derive(Debug)]
pub struct FinaleScreen {
    /// Estagio atual
    pub stage: FinaleStage,
    /// Contador de ticks
    pub count: i32,
    /// Texto da historia do episodio
    pub text: String,
    /// Nome do flat de fundo (tile 64x64)
    pub flat_name: String,
    /// Indice do monstro atual no cast (se em CastSequence)
    pub cast_num: usize,
    /// Ticks restantes no frame atual do cast
    pub cast_tics: i32,
    /// Se o monstro esta na animacao de morte
    pub cast_death: bool,
    /// Se o monstro esta atacando
    pub cast_attacking: bool,
    /// Sequencia de monstros
    pub cast_order: Vec<CastEntry>,
}

impl FinaleScreen {
    /// Cria uma tela de finale.
    ///
    /// C original: `F_StartFinale()` em `f_finale.c`
    pub fn new(text: &str, flat_name: &str) -> Self {
        FinaleScreen {
            stage: FinaleStage::TextCrawl,
            count: 0,
            text: text.to_string(),
            flat_name: flat_name.to_string(),
            cast_num: 0,
            cast_tics: 0,
            cast_death: false,
            cast_attacking: false,
            cast_order: cast_order(),
        }
    }

    /// Atualiza o finale a cada tick.
    ///
    /// C original: `F_Ticker()` em `f_finale.c`
    pub fn ticker(&mut self) {
        self.count += 1;

        match self.stage {
            FinaleStage::TextCrawl => {
                // Verificar se todo o texto foi exibido + espera
                let text_end = self.text.len() as i32 * TEXTSPEED + TEXTWAIT;
                if self.count > text_end {
                    self.stage = FinaleStage::ArtScreen;
                }
            }
            FinaleStage::ArtScreen => {
                // Espera input para avancar
            }
            FinaleStage::CastSequence => {
                self.update_cast();
            }
        }
    }

    /// Atualiza o desfile de monstros.
    fn update_cast(&mut self) {
        if self.cast_tics > 0 {
            self.cast_tics -= 1;
            return;
        }

        if self.cast_death {
            // Proximo monstro
            self.cast_num += 1;
            if self.cast_num >= self.cast_order.len() {
                self.cast_num = 0; // loop
            }
            self.cast_death = false;
            self.cast_attacking = false;
            self.cast_tics = 15 * 35 / 10; // ~1.5 segundos
        } else {
            // Ciclar animacao do monstro
            self.cast_tics = 12; // ~0.34 segundos
        }
    }

    /// Processa input do jogador.
    ///
    /// C original: `F_Responder()` em `f_finale.c`
    pub fn responder(&mut self, _key: u8, key_down: bool) -> bool {
        if !key_down {
            return false;
        }

        match self.stage {
            FinaleStage::TextCrawl => {
                // Pular text crawl
                self.stage = FinaleStage::ArtScreen;
                true
            }
            FinaleStage::ArtScreen => false, // proximo nivel pelo game loop
            FinaleStage::CastSequence => {
                // Matar monstro atual
                if !self.cast_death {
                    self.cast_death = true;
                    self.cast_tics = 12;
                }
                true
            }
        }
    }

    /// Inicia o desfile de monstros (cast sequence).
    ///
    /// C original: `F_StartCast()` em `f_finale.c`
    pub fn start_cast(&mut self) {
        self.stage = FinaleStage::CastSequence;
        self.cast_num = 0;
        self.cast_death = false;
        self.cast_attacking = false;
        self.cast_tics = 15 * 35 / 10;
    }

    /// Retorna quantos caracteres do texto devem ser visiveis.
    ///
    /// C original: logica em `F_TextWrite()` de `f_finale.c`
    pub fn visible_chars(&self) -> usize {
        let chars = self.count / TEXTSPEED;
        (chars as usize).min(self.text.len())
    }

    /// Retorna o nome do monstro atual no cast.
    pub fn current_cast_name(&self) -> &str {
        if self.cast_num < self.cast_order.len() {
            self.cast_order[self.cast_num].name
        } else {
            ""
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Intermission tests ===

    #[test]
    fn intermission_init() {
        let stats = LevelStats::new();
        let inter = IntermissionScreen::new(stats);
        assert_eq!(inter.state, IntermissionState::StatCount);
        assert_eq!(inter.cnt_kills, 0);
        assert!(!inter.accelerate);
    }

    #[test]
    fn intermission_count_up() {
        let mut stats = LevelStats::new();
        stats.max_kills = 10;
        stats.kills = 10; // 100%
        stats.max_items = 5;
        stats.items = 5; // 100%
        stats.max_secrets = 1;
        stats.secrets = 1; // 100%

        let mut inter = IntermissionScreen::new(stats);

        // Ticker ate contadores chegarem a 100%
        for _ in 0..200 {
            inter.ticker();
        }

        assert_eq!(inter.cnt_kills, 100);
        assert_eq!(inter.cnt_items, 100);
        assert_eq!(inter.cnt_secrets, 100);
        assert_eq!(inter.state, IntermissionState::ShowNextLoc);
    }

    #[test]
    fn intermission_accelerate() {
        let mut stats = LevelStats::new();
        stats.max_kills = 100;
        stats.kills = 50;

        let mut inter = IntermissionScreen::new(stats);
        inter.responder(b' ', true); // acelerar
        inter.ticker();

        assert_eq!(inter.cnt_kills, 50); // pula direto
        assert_eq!(inter.state, IntermissionState::ShowNextLoc);
    }

    #[test]
    fn intermission_finished() {
        let stats = LevelStats::new();
        let mut inter = IntermissionScreen::new(stats);

        // Acelerar para ShowNextLoc
        inter.accelerate = true;
        inter.ticker();
        assert_eq!(inter.state, IntermissionState::ShowNextLoc);

        // Esperar delay ou acelerar novamente
        inter.accelerate = true;
        inter.ticker();
        assert!(inter.is_finished());
    }

    #[test]
    fn level_stats_percents() {
        let mut stats = LevelStats::new();
        stats.max_kills = 10;
        stats.kills = 5;
        assert_eq!(stats.kill_percent(), 50);

        stats.max_items = 0;
        assert_eq!(stats.item_percent(), 100); // 0/0 = 100%

        stats.max_secrets = 3;
        stats.secrets = 1;
        assert_eq!(stats.secret_percent(), 33);
    }

    #[test]
    fn bg_anim_always() {
        let mut anim = BackgroundAnim::always(10, 3, 0, 0);
        assert_eq!(anim.current_frame, 0);

        // Antes do periodo — nao muda
        assert!(!anim.update(5));
        assert_eq!(anim.current_frame, 0);

        // No periodo — muda
        assert!(anim.update(10));
        assert_eq!(anim.current_frame, 1);

        // Proxima mudanca em tic 20
        assert!(!anim.update(15));
        assert!(anim.update(20));
        assert_eq!(anim.current_frame, 2);

        // Wraparound
        assert!(anim.update(30));
        assert_eq!(anim.current_frame, 0);
    }

    // === Finale tests ===

    #[test]
    fn finale_text_crawl() {
        let finale = FinaleScreen::new("Hello World", "FLOOR4_8");
        assert_eq!(finale.stage, FinaleStage::TextCrawl);
        assert_eq!(finale.visible_chars(), 0);
    }

    #[test]
    fn finale_text_reveals() {
        let mut finale = FinaleScreen::new("ABC", "FLOOR4_8");

        // Cada caractere aparece a cada TEXTSPEED ticks
        for _ in 0..TEXTSPEED {
            finale.ticker();
        }
        assert_eq!(finale.visible_chars(), 1);

        for _ in 0..TEXTSPEED {
            finale.ticker();
        }
        assert_eq!(finale.visible_chars(), 2);
    }

    #[test]
    fn finale_skip_text() {
        let mut finale = FinaleScreen::new("Long text...", "FLOOR4_8");
        assert_eq!(finale.stage, FinaleStage::TextCrawl);

        finale.responder(b' ', true); // pular
        assert_eq!(finale.stage, FinaleStage::ArtScreen);
    }

    #[test]
    fn finale_cast_sequence() {
        let mut finale = FinaleScreen::new("", "FLOOR4_8");
        finale.start_cast();
        assert_eq!(finale.stage, FinaleStage::CastSequence);
        assert_eq!(finale.current_cast_name(), "Zombieman");
        assert!(!finale.cast_death);

        // Pressionar tecla mata o monstro
        finale.responder(b' ', true);
        assert!(finale.cast_death);
    }

    #[test]
    fn cast_order_complete() {
        let order = cast_order();
        assert_eq!(order.len(), 17);
        assert_eq!(order[0].name, "Zombieman");
        assert_eq!(order[16].name, "Our Hero");
    }

    #[test]
    fn finale_constants() {
        assert_eq!(TEXTSPEED, 3);
        assert_eq!(TEXTWAIT, 250);
    }
}

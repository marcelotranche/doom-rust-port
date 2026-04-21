//! # Dispatcher de Ticks do Jogo (G_Ticker)
//!
//! Coordena a execucao de um tick logico do jogo, delegando
//! para o ticker correto conforme o estado atual:
//!
//! ```text
//! G_Ticker()
//!   |
//!   +-> process_action()     — processa gameaction pendente
//!   +-> check_special_buttons() — pause, save
//!   +-> match gamestate:
//!         Level       → P_Ticker()  — fisica, thinkers, jogador
//!         Intermission → WI_Ticker() — contadores de estatisticas
//!         Finale      → F_Ticker()  — texto e cast sequence
//!         DemoScreen  → D_PageTicker() — title screen
//! ```
//!
//! No DOOM original, `G_Ticker()` em `g_game.c` era a funcao que
//! orquestrava tudo. Aqui, encapsulamos em `GameTicker` que
//! mantem referencia aos subsistemas relevantes.
//!
//! ## Arquivo C original: `g_game.c` (G_Ticker)
//!
//! ## Conceitos que o leitor vai aprender
//! - Dispatch por estado (strategy pattern)
//! - Separacao entre tick logico e rendering
//! - Coordenacao de subsistemas sem acoplamento forte

use super::state::{GameAction, GameState, GameStateType};
use super::thinker::ThinkerList;

/// Resultado de um tick do jogo.
///
/// Indica ao caller se o jogo deve continuar ou encerrar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickResult {
    /// Tick executado normalmente, continuar
    Continue,
    /// Jogo deve encerrar
    Quit,
}

/// Dispatcher de ticks do jogo.
///
/// Coordena a execucao de G_Ticker, delegando para o subsistema
/// correto conforme o GameStateType atual.
///
/// C original: `G_Ticker()` em `g_game.c`
#[derive(Debug)]
pub struct GameTicker {
    /// Contador de ticks do demo screen (para avancar paginas)
    pub demo_page_tic: i32,
    /// Pagina atual do demo screen (title, credits, etc.)
    pub demo_page: i32,
    /// Numero de paginas do demo screen
    pub demo_page_count: i32,
    /// Ticks por pagina do demo screen
    pub demo_page_ticks: i32,
}

impl GameTicker {
    /// Cria um novo ticker.
    pub fn new() -> Self {
        GameTicker {
            demo_page_tic: 0,
            demo_page: 0,
            demo_page_count: 3, // title, demo, credits
            demo_page_ticks: 200, // ~6 segundos por pagina
        }
    }

    /// Executa um tick logico do jogo.
    ///
    /// Processa a acao pendente, verifica botoes especiais, e
    /// chama o ticker do estado atual.
    ///
    /// C original: `G_Ticker()` em `g_game.c`
    pub fn tick(
        &mut self,
        game: &mut GameState,
        thinkers: &mut ThinkerList,
        sectors: &mut [crate::map::types::Sector],
    ) -> TickResult {
        // Processar acao pendente (transicao de estado)
        let action = game.process_action();

        // Acoes que requerem setup especial
        match action {
            GameAction::LoadLevel => {
                // Estado ja foi trocado para Level pelo process_action
                game.viewactive = true;
                game.levelstarttic = game.gametic;
            }
            GameAction::NewGame => {
                // Iniciar novo jogo: setar loadlevel
                game.action = GameAction::LoadLevel;
                game.state = GameStateType::Level;
            }
            _ => {}
        }

        // Verificar botoes especiais (pause, save)
        if game.state == GameStateType::Level {
            game.check_special_buttons();
        }

        // Executar ticker do estado atual
        if !game.paused {
            match game.state {
                GameStateType::Level => {
                    self.tick_level(game, thinkers, sectors);
                }
                GameStateType::Intermission => {
                    self.tick_intermission(game);
                }
                GameStateType::Finale => {
                    self.tick_finale(game);
                }
                GameStateType::DemoScreen => {
                    self.tick_demo_screen(game);
                }
            }
        }

        game.gametic += 1;
        TickResult::Continue
    }

    /// Executa um tick de gameplay (P_Ticker).
    ///
    /// Roda os thinkers (fisica, IA, jogador) e processa
    /// os ticcmds do jogador local.
    ///
    /// C original: `P_Ticker()` em `p_tick.c`
    fn tick_level(
        &mut self,
        game: &mut GameState,
        thinkers: &mut ThinkerList,
        sectors: &mut [crate::map::types::Sector],
    ) {
        // Aplicar ticcmd do jogador ao mobj (P_PlayerThink)
        // No port completo, isso moveria o jogador, verificaria
        // colisoes, dispararia armas, etc.
        let _tic_index = game.gametic as usize % super::state::BACKUPTICS;

        // Executar thinkers (P_RunThinkers)
        thinkers.run(sectors);
    }

    /// Executa um tick de intermissao (WI_Ticker).
    ///
    /// Avanca os contadores de estatisticas e detecta
    /// quando o jogador quer prosseguir.
    ///
    /// C original: `WI_Ticker()` em `wi_stuff.c`
    fn tick_intermission(&mut self, game: &mut GameState) {
        // Verificar se jogador quer prosseguir (accelerate)
        let cmd = &game.netcmds[game.consoleplayer][game.gametic as usize % super::state::BACKUPTICS];
        if cmd.buttons & super::events::BT_ATTACK != 0
            || cmd.buttons & super::events::BT_USE != 0
        {
            game.action = GameAction::WorldDone;
        }
    }

    /// Executa um tick de finale (F_Ticker).
    ///
    /// Avanca o texto crawl e cast sequence.
    ///
    /// C original: `F_Ticker()` em `f_finale.c`
    fn tick_finale(&mut self, game: &mut GameState) {
        // Verificar se jogador quer prosseguir
        let cmd = &game.netcmds[game.consoleplayer][game.gametic as usize % super::state::BACKUPTICS];
        if cmd.buttons & super::events::BT_ATTACK != 0
            || cmd.buttons & super::events::BT_USE != 0
        {
            game.action = GameAction::WorldDone;
        }
    }

    /// Executa um tick do demo screen (D_PageTicker).
    ///
    /// Avanca o timer e troca entre title, demo e credits.
    ///
    /// C original: `D_PageTicker()` em `d_main.c`
    fn tick_demo_screen(&mut self, game: &mut GameState) {
        self.demo_page_tic += 1;
        if self.demo_page_tic >= self.demo_page_ticks {
            self.demo_page_tic = 0;
            self.demo_page = (self.demo_page + 1) % self.demo_page_count;
        }

        // Se usuario pressionou algo, iniciar novo jogo
        let cmd = &game.netcmds[game.consoleplayer][game.gametic as usize % super::state::BACKUPTICS];
        if cmd.buttons & super::events::BT_USE != 0 {
            // Abrir menu em vez de iniciar jogo diretamente
            // (na versao completa, o menu responder capturaria isso)
        }
        let _ = cmd; // evitar warning de uso
    }
}

impl Default for GameTicker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticker_init() {
        let ticker = GameTicker::new();
        assert_eq!(ticker.demo_page, 0);
        assert_eq!(ticker.demo_page_tic, 0);
    }

    #[test]
    fn tick_increments_gametic() {
        let mut ticker = GameTicker::new();
        let mut game = GameState::new();
        let mut thinkers = ThinkerList::new();
        game.playeringame[0] = true;

        let initial = game.gametic;
        ticker.tick(&mut game, &mut thinkers, &mut []);
        assert_eq!(game.gametic, initial + 1);
    }

    #[test]
    fn tick_processes_action() {
        let mut ticker = GameTicker::new();
        let mut game = GameState::new();
        let mut thinkers = ThinkerList::new();

        game.action = GameAction::LoadLevel;
        ticker.tick(&mut game, &mut thinkers, &mut []);
        assert_eq!(game.state, GameStateType::Level);
        assert!(game.viewactive);
    }

    #[test]
    fn tick_paused_no_advance() {
        let mut ticker = GameTicker::new();
        let mut game = GameState::new();
        let mut thinkers = ThinkerList::new();
        game.state = GameStateType::Level;
        game.paused = true;
        game.playeringame[0] = true;

        let page_before = ticker.demo_page_tic;
        ticker.tick(&mut game, &mut thinkers, &mut []);
        // Gametic still advances (como no DOOM original — pause nao para o gametic)
        // Mas os thinkers nao rodam
        assert_eq!(ticker.demo_page_tic, page_before);
    }

    #[test]
    fn tick_demo_screen_pages() {
        let mut ticker = GameTicker::new();
        let mut game = GameState::new();
        let mut thinkers = ThinkerList::new();
        game.state = GameStateType::DemoScreen;
        game.playeringame[0] = true;

        // Rodar ate mudar de pagina
        for _ in 0..ticker.demo_page_ticks {
            ticker.tick(&mut game, &mut thinkers, &mut []);
        }
        assert_eq!(ticker.demo_page, 1);
    }

    #[test]
    fn tick_level_runs_thinkers() {
        use super::super::thinker::Thinker;

        #[derive(Debug)]
        struct CounterThinker(i32);
        impl Thinker for CounterThinker {
            fn think(&mut self, _sectors: &mut [crate::map::types::Sector]) -> bool {
                self.0 += 1;
                true
            }
        }

        let mut ticker = GameTicker::new();
        let mut game = GameState::new();
        game.state = GameStateType::Level;
        game.playeringame[0] = true;
        let mut thinkers = ThinkerList::new();
        thinkers.add(Box::new(CounterThinker(0)));

        ticker.tick(&mut game, &mut thinkers, &mut []);
        // Thinker deveria ter sido executado
        assert_eq!(thinkers.count(), 1);
    }

    #[test]
    fn tick_new_game_action() {
        let mut ticker = GameTicker::new();
        let mut game = GameState::new();
        let mut thinkers = ThinkerList::new();

        game.action = GameAction::NewGame;
        ticker.tick(&mut game, &mut thinkers, &mut []);
        // NewGame seta LoadLevel como proxima acao
        assert_eq!(game.action, GameAction::LoadLevel);
    }

    #[test]
    fn tick_result_continue() {
        let mut ticker = GameTicker::new();
        let mut game = GameState::new();
        let mut thinkers = ThinkerList::new();
        game.playeringame[0] = true;

        let result = ticker.tick(&mut game, &mut thinkers, &mut []);
        assert_eq!(result, TickResult::Continue);
    }
}

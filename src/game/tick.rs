//! # Game Loop e Sistema de Ticks
//!
//! Implementa o loop principal do DOOM e o sistema de scheduling
//! de ticks que garante que o jogo roda a exatamente 35 Hz
//! independente do frame rate de rendering.
//!
//! ## Loop principal (D_DoomLoop)
//!
//! ```text
//! loop {
//!     I_StartFrame()         — I/O sincronico
//!     if singletics:
//!         I_StartTic()       — ler input
//!         D_ProcessEvents()  — despachar eventos
//!         G_BuildTiccmd()    — converter input em ticcmd
//!         G_Ticker()         — executar tick logico
//!         gametic++
//!     else:
//!         TryRunTics()       — adaptativo: 1+ ticks conforme necessario
//!     S_UpdateSounds()       — posicionar audio 3D
//!     D_Display()            — renderizar frame
//! }
//! ```
//!
//! ## TryRunTics — agendamento adaptativo
//!
//! Em single player, TryRunTics calcula quantos ticks passaram
//! desde o ultimo frame (baseado no tempo real) e executa
//! esse numero de ticks do jogo. Isso garante:
//!
//! - Em maquinas rapidas: multiplos frames de rendering por tick
//! - Em maquinas lentas: multiplos ticks por frame
//!
//! Em multiplayer, tambem aguarda que todos os peers tenham
//! enviado seus ticcmds antes de avancar.
//!
//! ## Arquivo C original: `d_main.c` (D_DoomLoop), `d_net.c` (TryRunTics)
//!
//! ## Conceitos que o leitor vai aprender
//! - Fixed timestep game loop (35 Hz)
//! - Desacoplamento entre logica (tick) e rendering (frame)
//! - Agendamento adaptativo de ticks

use std::time::Instant;

use super::state::{GameState, GameStateType, GameAction, TICRATE};

/// Duracao de um tick em microsegundos (1/35 segundo ≈ 28571 us).
const TICK_DURATION_US: u64 = 1_000_000 / TICRATE as u64;

/// Duracao de um tick para uso em testes.
#[cfg(test)]
const TICK_DURATION: std::time::Duration = std::time::Duration::from_micros(TICK_DURATION_US);

/// Sistema de timing e execucao do game loop.
///
/// Gerencia o relogio do jogo e controla quantos ticks
/// logicos devem ser executados a cada iteracao do loop.
///
/// C original: variaveis em `d_net.c` — `gametime`, `oldentertics`,
/// e logica de `TryRunTics()`
#[derive(Debug)]
pub struct TickSystem {
    /// Instante em que o jogo comecou (referencia para I_GetTime)
    start_time: Instant,
    /// Ultimo tick de tempo real processado
    old_enter_tics: i32,
    /// Se true, executa exatamente 1 tick por frame (debug)
    pub singletics: bool,
}

impl TickSystem {
    /// Cria um novo sistema de ticks.
    pub fn new() -> Self {
        TickSystem {
            start_time: Instant::now(),
            old_enter_tics: 0,
            singletics: false,
        }
    }

    /// Retorna o tempo atual em ticks (1/35s cada).
    ///
    /// Equivalente a `I_GetTime()` no C original, que retorna
    /// o numero de ticks desde o inicio do jogo.
    ///
    /// C original: `I_GetTime()` em `i_system.c`
    pub fn get_time(&self) -> i32 {
        let elapsed = self.start_time.elapsed();
        let us = elapsed.as_micros() as u64;
        (us / TICK_DURATION_US) as i32
    }

    /// Calcula quantos ticks logicos devem ser executados.
    ///
    /// Em single player, retorna o numero de ticks de tempo real
    /// que passaram desde a ultima chamada. Em multiplayer,
    /// seria limitado pelos ticks disponíveis de todos os peers.
    ///
    /// C original: logica de contagem em `TryRunTics()` em `d_net.c`
    pub fn calc_tics_to_run(&mut self, game: &GameState) -> i32 {
        let enter_tic = self.get_time();
        let real_tics = enter_tic - self.old_enter_tics;
        self.old_enter_tics = enter_tic;

        // Em single player, executar tantos ticks quanto o tempo real indica
        // Em multiplayer, seria limitado por lowtic (menor tic disponivel de todos os peers)
        let available_tics = enter_tic - game.gametic;

        let counts = if real_tics < available_tics - 1 {
            real_tics + 1
        } else if real_tics < available_tics {
            real_tics
        } else {
            available_tics
        };

        // Sempre executar pelo menos 1 tick
        counts.max(1)
    }

    /// Reseta o timer (para quando um novo nivel e carregado, etc.)
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.old_enter_tics = 0;
    }
}

impl Default for TickSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Resultado de uma iteracao do game loop.
///
/// Indica ao caller o que aconteceu para que ele possa
/// coordenar rendering e audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopAction {
    /// Ticks foram executados normalmente, renderizar frame
    Render,
    /// O jogo deve ser encerrado
    Quit,
}

/// Executa uma iteracao do game loop.
///
/// Esta funcao implementa o corpo do `D_DoomLoop` do C original.
/// Retorna `LoopAction` indicando se deve renderizar ou encerrar.
///
/// O caller (main) e responsavel por:
/// 1. Chamar I_StartFrame() antes
/// 2. Chamar esta funcao
/// 3. Chamar S_UpdateSounds() e D_Display() depois
///
/// C original: corpo do `while(1)` em `D_DoomLoop()` em `d_main.c`
pub fn run_tic(game: &mut GameState, tick_system: &mut TickSystem) -> LoopAction {
    let counts = if tick_system.singletics {
        1
    } else {
        tick_system.calc_tics_to_run(game)
    };

    for _ in 0..counts {
        // Processar acao pendente (mudanca de estado)
        let action = game.process_action();

        // Se a acao era quit-like, sair
        if action == GameAction::Screenshot {
            // Screenshot nao para o jogo, mas e processado
        }

        // Verificar botoes especiais (pause, save) nos ticcmds
        game.check_special_buttons();

        // Executar ticker do estado atual
        // TODO: chamar P_Ticker(), WI_Ticker(), F_Ticker(), D_PageTicker()
        // conforme o gamestate
        match game.state {
            GameStateType::Level => {
                // TODO: P_Ticker() — fisica, thinkers, jogador
            }
            GameStateType::Intermission => {
                // TODO: WI_Ticker() — tela de estatisticas
            }
            GameStateType::Finale => {
                // TODO: F_Ticker() — tela final
            }
            GameStateType::DemoScreen => {
                // TODO: D_PageTicker() — avancar demo screen
            }
        }

        game.gametic += 1;
    }

    LoopAction::Render
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn tick_system_init() {
        let ts = TickSystem::new();
        assert!(!ts.singletics);
        // get_time deve retornar 0 ou proximo de 0 logo apos criacao
        let t = ts.get_time();
        assert!(t >= 0 && t <= 1);
    }

    #[test]
    fn tick_system_get_time_advances() {
        let ts = TickSystem::new();
        let t1 = ts.get_time();
        // Aguardar um pouco mais que um tick
        std::thread::sleep(Duration::from_millis(30)); // ~1 tick
        let t2 = ts.get_time();
        assert!(t2 >= t1 + 1, "tempo deve avancar: t1={}, t2={}", t1, t2);
    }

    #[test]
    fn tick_system_reset() {
        let mut ts = TickSystem::new();
        std::thread::sleep(Duration::from_millis(60));
        let before = ts.get_time();
        assert!(before >= 1);
        ts.reset();
        let after = ts.get_time();
        assert!(after <= 1);
    }

    #[test]
    fn run_tic_basic() {
        let mut game = GameState::new();
        game.playeringame[0] = true;
        let mut ts = TickSystem::new();
        ts.singletics = true; // Executar exatamente 1 tick

        let initial_tic = game.gametic;
        let action = run_tic(&mut game, &mut ts);
        assert_eq!(action, LoopAction::Render);
        assert_eq!(game.gametic, initial_tic + 1);
    }

    #[test]
    fn run_tic_processes_action() {
        let mut game = GameState::new();
        let mut ts = TickSystem::new();
        ts.singletics = true;

        game.action = GameAction::LoadLevel;
        run_tic(&mut game, &mut ts);
        assert_eq!(game.state, GameStateType::Level);
        assert_eq!(game.action, GameAction::Nothing);
    }

    #[test]
    fn tick_duration_constant() {
        // 1/35 segundo ≈ 28571 microsegundos
        assert_eq!(TICK_DURATION_US, 28571);
        assert_eq!(TICK_DURATION, Duration::from_micros(28571));
    }
}

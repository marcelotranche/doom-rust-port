//! # Estado do Jogo e Maquina de Estados
//!
//! Gerencia o estado global do jogo DOOM: em qual fase estamos
//! (jogando, intermission, finale, demo), qual acao pendente
//! (carregar fase, salvar, novo jogo), e parametros da sessao
//! (dificuldade, episodio, mapa, jogadores).
//!
//! ## Maquina de estados
//!
//! ```text
//!              ga_newgame
//!    DEMOSCREEN ---------> LEVEL
//!        ^                  |  ^
//!        |    ga_completed  |  | ga_loadlevel
//!        |                  v  |
//!        |             INTERMISSION
//!        |                  |
//!        |    ga_victory    v
//!        +<------------- FINALE
//! ```
//!
//! A cada tick, `G_Ticker` primeiro processa a `gameaction` pendente
//! (mudancas de estado), depois executa o ticker do estado atual
//! (P_Ticker para gameplay, WI_Ticker para intermission, etc.)
//!
//! ## Arquivo C original: `g_game.c`, `doomdef.h`, `doomstat.h`
//!
//! ## Conceitos que o leitor vai aprender
//! - Maquina de estados para fluxo de jogo
//! - Separacao entre acao pendente e estado atual
//! - Gerenciamento de sessao (dificuldade, episodio, jogadores)

use super::events::TicCmd;

/// Numero maximo de jogadores (multiplayer ate 4).
///
/// C original: `#define MAXPLAYERS 4` em `doomdef.h`
pub const MAXPLAYERS: usize = 4;

/// Taxa de ticks do jogo: 35 ticks por segundo.
///
/// C original: `#define TICRATE 35` em `doomdef.h`
pub const TICRATE: u32 = 35;

/// Numero de tics armazenados no buffer circular de comandos.
///
/// C original: `#define BACKUPTICS 12` em `d_net.h`
pub const BACKUPTICS: usize = 12;

// ---------------------------------------------------------------------------
// Enums de estado
// ---------------------------------------------------------------------------

/// Estado atual do jogo — determina qual subsistema processa ticks.
///
/// C original: `gamestate_t` em `doomdef.h`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStateType {
    /// Jogando um nivel (gameplay ativo)
    Level,
    /// Tela de intermissao entre niveis (estatisticas)
    Intermission,
    /// Tela final do jogo (texto + arte)
    Finale,
    /// Tela de demo/atracao (title screen, demos automaticas)
    DemoScreen,
}

/// Acao pendente — processada no inicio do proximo tick.
///
/// Permite que qualquer parte do codigo solicite uma transicao
/// de estado que sera executada no momento seguro (inicio do tick).
///
/// C original: `gameaction_t` em `d_event.h`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameAction {
    /// Nenhuma acao pendente
    Nothing,
    /// Carregar um nivel
    LoadLevel,
    /// Iniciar novo jogo
    NewGame,
    /// Carregar jogo salvo
    LoadGame,
    /// Salvar jogo
    SaveGame,
    /// Reproduzir demo
    PlayDemo,
    /// Nivel completado — ir para intermission
    Completed,
    /// Vitoria — ir para finale
    Victory,
    /// Intermission/finale terminado — proximo nivel ou volta ao menu
    WorldDone,
    /// Capturar screenshot
    Screenshot,
}

/// Nivel de dificuldade do jogo.
///
/// C original: `skill_t` em `doomdef.h`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Skill {
    /// I'm Too Young to Die (dano reduzido, ammo dobrada)
    Baby,
    /// Hey, Not Too Rough
    Easy,
    /// Hurt Me Plenty (padrao)
    Medium,
    /// Ultra-Violence
    Hard,
    /// Nightmare! (monstros respawnam, ataque dobrado)
    Nightmare,
}

// ---------------------------------------------------------------------------
// Estado global do jogo
// ---------------------------------------------------------------------------

/// Estado global do jogo DOOM.
///
/// Centraliza todas as variaveis globais de estado que no C original
/// eram globals espalhados em `doomstat.h` e `g_game.c`.
///
/// C original: globals em `g_game.c` e `doomstat.h`
#[derive(Debug)]
pub struct GameState {
    // -- Maquina de estados --

    /// Estado atual do jogo
    pub state: GameStateType,
    /// Acao pendente para o proximo tick
    pub action: GameAction,

    // -- Parametros da sessao --

    /// Nivel de dificuldade
    pub skill: Skill,
    /// Episodio atual (1-4 para DOOM, 1 para DOOM II)
    pub episode: i32,
    /// Mapa atual (1-9 para DOOM, 1-32 para DOOM II)
    pub map: i32,

    // -- Controle de jogadores --

    /// Jogador que recebe input e e exibido por padrao
    pub consoleplayer: usize,
    /// Jogador cuja visao esta sendo exibida (pode mudar com spy mode)
    pub displayplayer: usize,
    /// Quais jogadores estao ativos
    pub playeringame: [bool; MAXPLAYERS],

    // -- Contadores de tick --

    /// Tick logico atual do jogo
    pub gametic: i32,
    /// Tick em que o nivel atual comecou
    pub levelstarttic: i32,

    // -- Flags de sessao --

    /// Jogo pausado
    pub paused: bool,
    /// Enviar evento de pausa no proximo tic
    pub sendpause: bool,
    /// Enviar evento de save no proximo tic
    pub sendsave: bool,
    /// Jogo iniciado pelo usuario (pode salvar/encerrar)
    pub usergame: bool,
    /// Modo deathmatch
    pub deathmatch: bool,
    /// Jogo em rede (packets broadcast)
    pub netgame: bool,
    /// View esta ativa (vs. fullscreen status bar, automap)
    pub viewactive: bool,

    // -- Demo --

    /// Gravando demo
    pub demorecording: bool,
    /// Reproduzindo demo
    pub demoplayback: bool,

    // -- Estatisticas do nivel --

    /// Total de monstros no nivel
    pub totalkills: i32,
    /// Total de itens no nivel
    pub totalitems: i32,
    /// Total de segredos no nivel
    pub totalsecret: i32,

    // -- Buffer de comandos de rede --

    /// Proximo tic para o qual construir comandos
    pub maketic: i32,
    /// Comandos locais do jogador (buffer circular)
    pub localcmds: [TicCmd; BACKUPTICS],
    /// Comandos de rede de todos os jogadores (buffer circular)
    pub netcmds: [[TicCmd; BACKUPTICS]; MAXPLAYERS],
    /// Checksums de consistencia para verificacao em netgame
    pub consistancy: [[i16; BACKUPTICS]; MAXPLAYERS],
}

impl GameState {
    /// Cria um novo estado de jogo com valores padrao.
    pub fn new() -> Self {
        GameState {
            state: GameStateType::DemoScreen,
            action: GameAction::Nothing,
            skill: Skill::Medium,
            episode: 1,
            map: 1,
            consoleplayer: 0,
            displayplayer: 0,
            playeringame: [false; MAXPLAYERS],
            gametic: 0,
            levelstarttic: 0,
            paused: false,
            sendpause: false,
            sendsave: false,
            usergame: false,
            deathmatch: false,
            netgame: false,
            viewactive: true,
            demorecording: false,
            demoplayback: false,
            totalkills: 0,
            totalitems: 0,
            totalsecret: 0,
            maketic: 0,
            localcmds: [TicCmd::new(); BACKUPTICS],
            netcmds: [[TicCmd::new(); BACKUPTICS]; MAXPLAYERS],
            consistancy: [[0; BACKUPTICS]; MAXPLAYERS],
        }
    }

    /// Processa a acao pendente, executando a transicao de estado.
    ///
    /// No DOOM original, cada acao chama uma funcao `G_Do*()` que
    /// faz a transicao. Aqui, retornamos a acao que foi processada
    /// para que o caller possa executar a logica associada.
    ///
    /// C original: switch em `G_Ticker()` em `g_game.c`
    pub fn process_action(&mut self) -> GameAction {
        let action = self.action;
        match action {
            GameAction::Nothing => {}
            GameAction::LoadLevel => {
                self.state = GameStateType::Level;
                self.action = GameAction::Nothing;
            }
            GameAction::NewGame => {
                // A logica de G_DoNewGame define skill/episode/map
                // e depois chama G_InitNew que seta ga_loadlevel
                self.action = GameAction::Nothing;
            }
            GameAction::Completed => {
                self.state = GameStateType::Intermission;
                self.action = GameAction::Nothing;
            }
            GameAction::Victory => {
                self.state = GameStateType::Finale;
                self.action = GameAction::Nothing;
            }
            GameAction::WorldDone => {
                // Volta para gameplay ou menu dependendo do contexto
                self.action = GameAction::Nothing;
            }
            GameAction::Screenshot => {
                self.action = GameAction::Nothing;
            }
            _ => {
                // LoadGame, SaveGame, PlayDemo — processados externamente
                self.action = GameAction::Nothing;
            }
        }
        action
    }

    /// Verifica e processa botoes especiais (pause, save) nos ticcmds.
    ///
    /// C original: loop de `BT_SPECIAL` em `G_Ticker()` em `g_game.c`
    pub fn check_special_buttons(&mut self) {
        for i in 0..MAXPLAYERS {
            if self.playeringame[i] {
                let cmd = &self.netcmds[i][self.gametic as usize % BACKUPTICS];
                if cmd.buttons & super::events::BT_SPECIAL != 0 {
                    match cmd.buttons & super::events::BT_SPECIALMASK {
                        super::events::BTS_PAUSE => {
                            self.paused = !self.paused;
                        }
                        super::events::BTS_SAVEGAME => {
                            self.action = GameAction::SaveGame;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gamestate_init() {
        let gs = GameState::new();
        assert_eq!(gs.state, GameStateType::DemoScreen);
        assert_eq!(gs.action, GameAction::Nothing);
        assert_eq!(gs.skill, Skill::Medium);
        assert_eq!(gs.gametic, 0);
        assert!(!gs.playeringame[0]);
    }

    #[test]
    fn process_action_load_level() {
        let mut gs = GameState::new();
        gs.action = GameAction::LoadLevel;
        let action = gs.process_action();
        assert_eq!(action, GameAction::LoadLevel);
        assert_eq!(gs.state, GameStateType::Level);
        assert_eq!(gs.action, GameAction::Nothing);
    }

    #[test]
    fn process_action_completed() {
        let mut gs = GameState::new();
        gs.state = GameStateType::Level;
        gs.action = GameAction::Completed;
        let action = gs.process_action();
        assert_eq!(action, GameAction::Completed);
        assert_eq!(gs.state, GameStateType::Intermission);
    }

    #[test]
    fn process_action_victory() {
        let mut gs = GameState::new();
        gs.state = GameStateType::Level;
        gs.action = GameAction::Victory;
        gs.process_action();
        assert_eq!(gs.state, GameStateType::Finale);
    }

    #[test]
    fn process_action_nothing() {
        let mut gs = GameState::new();
        let action = gs.process_action();
        assert_eq!(action, GameAction::Nothing);
        assert_eq!(gs.state, GameStateType::DemoScreen);
    }

    #[test]
    fn check_special_pause() {
        let mut gs = GameState::new();
        gs.playeringame[0] = true;
        gs.netcmds[0][0].buttons =
            super::super::events::BT_SPECIAL | super::super::events::BTS_PAUSE;
        assert!(!gs.paused);
        gs.check_special_buttons();
        assert!(gs.paused);
        // Toggle de volta
        gs.check_special_buttons();
        assert!(!gs.paused);
    }
}

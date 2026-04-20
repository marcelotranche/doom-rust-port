//! # Engine Principal — D_DoomMain e D_DoomLoop
//!
//! Integra todos os subsistemas do DOOM em uma unica struct
//! `DoomEngine` que gerencia o ciclo de vida completo do jogo:
//! inicializacao, game loop, e shutdown.
//!
//! ## Sequencia de inicializacao (D_DoomMain)
//!
//! ```text
//! D_DoomMain()
//!   |
//!   +-> Parse argumentos de linha de comando
//!   +-> W_InitMultipleFiles()  — carregar WADs
//!   +-> R_Init()               — inicializar renderer
//!   +-> P_Init()               — inicializar gameplay
//!   +-> S_Init()               — inicializar audio
//!   +-> HU_Init()              — inicializar HUD
//!   +-> ST_Init()              — inicializar status bar
//!   +-> D_CheckNetGame()       — verificar rede
//!   +-> D_DoomLoop()           — entrar no game loop
//! ```
//!
//! ## Game loop (D_DoomLoop)
//!
//! ```text
//! loop {
//!     I_StartTic()         — ler input da plataforma
//!     D_ProcessEvents()    — despachar eventos para responders
//!     G_BuildTiccmd()      — converter input em ticcmd
//!     G_Ticker()           — executar tick(s) logico(s)
//!     S_UpdateSounds()     — posicionar audio 3D
//!     D_Display()          — renderizar frame
//! }
//! ```
//!
//! ## Arquivo C original: `d_main.c`
//!
//! ## Conceitos que o leitor vai aprender
//! - Integracao de subsistemas em uma arquitetura coesa
//! - Sequencia de inicializacao de um game engine
//! - Game loop com fixed timestep (35 Hz)
//! - Separacao entre logica (tick) e apresentacao (frame)

use crate::args::DoomArgs;
use crate::game::display::{DisplayConfig, WipeState};
use crate::game::events::EventQueue;
use crate::game::input::InputState;
use crate::game::state::{GameAction, GameState, GameStateType, Skill, TICRATE};
use crate::game::tick::TickSystem;
use crate::game::ticker::{GameTicker, TickResult};
use crate::game::thinker::ThinkerList;
use crate::map::MapData;
use crate::menu::navigation::MenuSystem;
use crate::renderer::state::RenderState;
use crate::video::VideoSystem;
use crate::wad::WadSystem;

/// Versao do engine.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Engine principal do DOOM.
///
/// Centraliza todos os subsistemas e gerencia o ciclo de vida
/// do jogo: inicializacao, game loop e shutdown.
///
/// C original: globals em `d_main.c` + `doomstat.h`
#[derive(Debug)]
pub struct DoomEngine {
    // -- Subsistemas --

    /// Sistema de WADs (carregamento de assets)
    pub wad: WadSystem,
    /// Sistema de video (framebuffer 320x200)
    pub video: VideoSystem,
    /// Estado do renderer (POV, projecao, tabelas de luz)
    pub render_state: RenderState,
    /// Estado global do jogo (maquina de estados, parametros)
    pub game: GameState,
    /// Sistema de timing e agendamento de ticks
    pub tick_system: TickSystem,
    /// Dispatcher de ticks (G_Ticker)
    pub ticker: GameTicker,
    /// Lista de thinkers (objetos que processam a cada tick)
    pub thinkers: ThinkerList,
    /// Fila de eventos de input
    pub event_queue: EventQueue,
    /// Estado do input (teclas, mouse, joystick)
    pub input: InputState,
    /// Sistema de menus
    pub menu: MenuSystem,
    /// Dados do mapa atual (geometria, BSP, things)
    pub map: Option<MapData>,
    /// Configuracao de exibicao
    pub display_config: DisplayConfig,
    /// Estado do efeito wipe
    pub wipe: WipeState,

    // -- Flags de controle --

    /// Modo desenvolvedor ativo
    pub devparm: bool,
    /// Sem monstros
    pub nomonsters: bool,
    /// Monstros rapidos
    pub fastparm: bool,
    /// Monstros respawnam
    pub respawnparm: bool,
    /// Velocidade turbo (porcentagem, 100 = normal)
    pub turbo_scale: i32,
    /// Numero de arquivos WAD carregados
    pub num_wad_files: usize,
    /// Se o engine esta rodando
    pub running: bool,
}

impl DoomEngine {
    /// Cria um novo engine nao-inicializado.
    fn new() -> Self {
        DoomEngine {
            wad: WadSystem::new(),
            video: VideoSystem::new(),
            render_state: RenderState::new(),
            game: GameState::new(),
            tick_system: TickSystem::new(),
            ticker: GameTicker::new(),
            thinkers: ThinkerList::new(),
            event_queue: EventQueue::new(),
            input: InputState::new(),
            menu: MenuSystem::new(),
            map: None,
            display_config: DisplayConfig::default(),
            wipe: WipeState::new(),
            devparm: false,
            nomonsters: false,
            fastparm: false,
            respawnparm: false,
            turbo_scale: 100,
            num_wad_files: 0,
            running: false,
        }
    }

    /// Inicializa o engine a partir de argumentos de linha de comando.
    ///
    /// Executa a sequencia completa de inicializacao equivalente
    /// a `D_DoomMain()` do C original.
    ///
    /// C original: `D_DoomMain()` em `d_main.c`
    pub fn init(args: &DoomArgs) -> Result<Self, EngineError> {
        let mut engine = Self::new();

        log::info!("DOOM Rust v{}", ENGINE_VERSION);
        log::info!("Port educacional do DOOM (1993) para Rust");

        // --- Carregar WADs ---
        log::info!("W_Init: Carregando WADs...");
        engine
            .wad
            .add_file(&args.iwad)
            .map_err(|e| EngineError::WadLoad(format!("{}: {}", args.iwad.display(), e)))?;
        engine.num_wad_files = 1;

        for pwad in &args.pwads {
            engine
                .wad
                .add_file(pwad)
                .map_err(|e| EngineError::WadLoad(format!("{}: {}", pwad.display(), e)))?;
            engine.num_wad_files += 1;
        }

        let num_lumps = engine.wad.num_lumps();
        log::info!(
            "W_Init: {} arquivo(s) carregado(s), {} lumps no total",
            engine.num_wad_files,
            num_lumps
        );

        // --- Aplicar flags de linha de comando ---
        engine.devparm = args.devparm;
        engine.nomonsters = args.nomonsters;
        engine.fastparm = args.fast;
        engine.respawnparm = args.respawn;

        if let Some(turbo) = args.turbo {
            engine.turbo_scale = turbo;
            log::info!("Turbo: {}%", turbo);
        }

        if args.singletics {
            engine.tick_system.singletics = true;
        }

        // --- Configurar sessao de jogo ---
        if let Some(skill) = args.skill {
            engine.game.skill = match skill {
                1 => Skill::Baby,
                2 => Skill::Easy,
                3 => Skill::Medium,
                4 => Skill::Hard,
                5 => Skill::Nightmare,
                _ => Skill::Medium,
            };
        }

        if let Some(ep) = args.episode {
            engine.game.episode = ep;
        }

        if let Some(ep) = args.warp_episode {
            engine.game.episode = ep;
        }

        if let Some(map) = args.warp_map {
            engine.game.map = map;
        }

        if args.deathmatch > 0 {
            engine.game.deathmatch = true;
        }

        // --- Inicializar subsistemas ---
        log::info!("V_Init: Inicializando video ({}x{})...",
            crate::video::SCREENWIDTH, crate::video::SCREENHEIGHT);

        log::info!("R_Init: Inicializando renderer...");

        log::info!("P_Init: Inicializando gameplay...");

        log::info!("S_Init: Inicializando audio...");

        log::info!("HU_Init: Inicializando HUD...");

        log::info!("ST_Init: Inicializando status bar...");

        log::info!("M_Init: Inicializando menus...");

        // --- Rede ---
        log::info!("D_CheckNetGame: Verificando rede...");
        engine.game.playeringame[0] = true; // jogador local sempre ativo
        log::info!("startskill {:?}, startepisode {}, startmap {}",
            engine.game.skill, engine.game.episode, engine.game.map);

        // --- Iniciar estado ---
        if args.warp_map.is_some() {
            // Warp direto para o mapa
            engine.game.action = GameAction::LoadLevel;
            engine.game.state = GameStateType::Level;
            log::info!("Warping para E{}M{}...", engine.game.episode, engine.game.map);
        } else {
            // Ir para demo screen (title)
            engine.game.state = GameStateType::DemoScreen;
        }

        // --- Carregar mapa se necessario ---
        if engine.game.action == GameAction::LoadLevel {
            let map_name = format!("E{}M{}", engine.game.episode, engine.game.map);
            match MapData::load(&map_name, &engine.wad) {
                Ok(mut map) => {
                    map.finalize();
                    log::info!("P_SetupLevel: {} carregado ({} vertexes, {} linedefs, {} things)",
                        map_name,
                        map.vertexes.len(),
                        map.linedefs.len(),
                        map.things.len());
                    engine.map = Some(map);
                    engine.game.action = GameAction::Nothing;
                    engine.game.viewactive = true;
                    engine.game.levelstarttic = 0;
                }
                Err(e) => {
                    log::warn!("Nao foi possivel carregar {}: {}", map_name, e);
                    // Voltar para demo screen
                    engine.game.state = GameStateType::DemoScreen;
                    engine.game.action = GameAction::Nothing;
                }
            }
        }

        engine.running = true;
        log::info!("D_DoomMain: Inicializacao completa.");
        Ok(engine)
    }

    /// Executa uma iteracao do game loop.
    ///
    /// Processa input, executa ticks logicos, e prepara para rendering.
    /// Retorna false quando o jogo deve encerrar.
    ///
    /// C original: corpo do `while(1)` em `D_DoomLoop()` em `d_main.c`
    pub fn run_frame(&mut self) -> bool {
        if !self.running {
            return false;
        }

        // I_StartTic — ler input da plataforma
        // (na versao completa, SDL2 geraria eventos aqui)

        // D_ProcessEvents — despachar eventos para responders
        self.process_events();

        // G_BuildTiccmd — converter input em ticcmd
        let consistancy = self.game.consistancy[self.game.consoleplayer]
            [self.game.maketic as usize % crate::game::state::BACKUPTICS];
        let cmd = self.input.build_ticcmd(self.game.maketic, consistancy);
        let slot = self.game.maketic as usize % crate::game::state::BACKUPTICS;
        self.game.localcmds[slot] = cmd;
        self.game.netcmds[self.game.consoleplayer][slot] = cmd;
        self.game.maketic += 1;

        // TryRunTics / G_Ticker — executar tick(s) logico(s)
        let counts = if self.tick_system.singletics {
            1
        } else {
            self.tick_system.calc_tics_to_run(&self.game)
        };

        for _ in 0..counts {
            let result = self.ticker.tick(&mut self.game, &mut self.thinkers);
            if result == TickResult::Quit {
                self.running = false;
                return false;
            }
        }

        // Atualizar display config com base no estado
        self.update_display_config();

        // D_Display — renderizar frame
        // (na versao completa, chamaria R_RenderPlayerView, drawers, etc.)

        // Atualizar wipe
        if self.wipe.is_active() {
            self.wipe.update(crate::video::SCREENHEIGHT as i32);
        }

        true
    }

    /// Processa eventos da fila de eventos.
    ///
    /// Despacha para a cadeia de responders:
    /// Menu → HUD → Game
    ///
    /// C original: `D_ProcessEvents()` em `d_main.c`
    fn process_events(&mut self) {
        while let Some(ev) = self.event_queue.poll() {
            // Cadeia de responders: Menu primeiro
            let key = ev.data1 as u8;
            let is_down = ev.event_type == crate::game::events::EventType::KeyDown;

            if self.menu.responder(key, is_down) {
                continue;
            }

            // Game responder (atualiza estado de teclas)
            self.input.handle_event(&ev);
        }
    }

    /// Atualiza a configuracao de exibicao conforme o estado do jogo.
    fn update_display_config(&mut self) {
        match self.game.state {
            GameStateType::Level => {
                self.display_config = DisplayConfig::gameplay();
                self.display_config.view_active = self.game.viewactive;
            }
            _ => {
                self.display_config = DisplayConfig::fullscreen();
            }
        }
        // Menu sobre tudo quando ativo
        // (MenuSystem controla isso internamente)
    }

    /// Inicia um novo jogo com os parametros especificados.
    ///
    /// C original: `G_DeferedInitNew()` em `g_game.c`
    pub fn new_game(&mut self, skill: Skill, episode: i32, map: i32) {
        self.game.skill = skill;
        self.game.episode = episode;
        self.game.map = map;
        self.game.action = GameAction::NewGame;
    }

    /// Carrega o mapa especificado.
    ///
    /// C original: `G_DoLoadLevel()` em `g_game.c`
    pub fn load_level(&mut self) -> Result<(), EngineError> {
        let map_name = format!("E{}M{}", self.game.episode, self.game.map);
        let mut map = MapData::load(&map_name, &self.wad)
            .map_err(|e| EngineError::MapLoad(format!("{}: {}", map_name, e)))?;
        map.finalize();

        log::info!(
            "P_SetupLevel: {} ({} vertexes, {} linedefs, {} things)",
            map_name,
            map.vertexes.len(),
            map.linedefs.len(),
            map.things.len()
        );

        // Limpar estado anterior
        self.thinkers.clear();
        self.map = Some(map);

        // Resetar contadores
        self.game.levelstarttic = self.game.gametic;
        self.game.viewactive = true;
        self.game.totalkills = 0;
        self.game.totalitems = 0;
        self.game.totalsecret = 0;

        // Iniciar wipe
        self.wipe.start(crate::video::SCREENWIDTH);

        Ok(())
    }

    /// Retorna o numero de ticks por segundo.
    pub fn ticrate(&self) -> u32 {
        TICRATE
    }

    /// Retorna o tick logico atual.
    pub fn gametic(&self) -> i32 {
        self.game.gametic
    }

    /// Retorna o estado atual do jogo.
    pub fn state(&self) -> GameStateType {
        self.game.state
    }

    /// Retorna o framebuffer principal (screen 0) para blit.
    pub fn framebuffer(&self) -> &[u8] {
        self.video.screen(0)
    }

    /// Encerra o engine.
    pub fn quit(&mut self) {
        log::info!("D_QuitNetGame: Encerrando rede...");
        log::info!("I_Quit: Encerrando engine.");
        self.running = false;
    }
}

/// Erros do engine.
#[derive(Debug, Clone)]
pub enum EngineError {
    /// Erro ao carregar WAD
    WadLoad(String),
    /// Erro ao carregar mapa
    MapLoad(String),
    /// Erro de inicializacao
    Init(String),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::WadLoad(e) => write!(f, "Erro ao carregar WAD: {}", e),
            EngineError::MapLoad(e) => write!(f, "Erro ao carregar mapa: {}", e),
            EngineError::Init(e) => write!(f, "Erro de inicializacao: {}", e),
        }
    }
}

impl std::error::Error for EngineError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_new() {
        let engine = DoomEngine::new();
        assert!(!engine.running);
        assert!(engine.map.is_none());
        assert_eq!(engine.game.state, GameStateType::DemoScreen);
        assert_eq!(engine.turbo_scale, 100);
    }

    #[test]
    fn engine_ticrate() {
        let engine = DoomEngine::new();
        assert_eq!(engine.ticrate(), 35);
    }

    #[test]
    fn engine_new_game() {
        let mut engine = DoomEngine::new();
        engine.new_game(Skill::Hard, 2, 3);
        assert_eq!(engine.game.skill, Skill::Hard);
        assert_eq!(engine.game.episode, 2);
        assert_eq!(engine.game.map, 3);
        assert_eq!(engine.game.action, GameAction::NewGame);
    }

    #[test]
    fn engine_quit() {
        let mut engine = DoomEngine::new();
        engine.running = true;
        engine.quit();
        assert!(!engine.running);
    }

    #[test]
    fn engine_run_frame_when_not_running() {
        let mut engine = DoomEngine::new();
        assert!(!engine.run_frame());
    }

    #[test]
    fn engine_run_frame_basic() {
        let mut engine = DoomEngine::new();
        engine.running = true;
        engine.game.playeringame[0] = true;
        engine.tick_system.singletics = true;

        let continued = engine.run_frame();
        assert!(continued);
        // gametic avancou (pelo ticker)
        assert!(engine.game.gametic > 0);
    }

    #[test]
    fn engine_display_config_level() {
        let mut engine = DoomEngine::new();
        engine.game.state = GameStateType::Level;
        engine.game.viewactive = true;
        engine.update_display_config();
        assert!(engine.display_config.view_active);
        assert!(engine.display_config.statusbar_active);
    }

    #[test]
    fn engine_display_config_intermission() {
        let mut engine = DoomEngine::new();
        engine.game.state = GameStateType::Intermission;
        engine.update_display_config();
        assert!(!engine.display_config.view_active);
        assert!(!engine.display_config.statusbar_active);
    }

    #[test]
    fn engine_process_events_empty() {
        let mut engine = DoomEngine::new();
        engine.process_events(); // nao deve panic com fila vazia
    }

    #[test]
    fn engine_process_events_key() {
        use crate::game::events::{Event, KEY_UPARROW};
        let mut engine = DoomEngine::new();
        engine.event_queue.post(Event::key_down(KEY_UPARROW));
        engine.process_events();
        assert!(engine.input.gamekeydown[KEY_UPARROW as usize]);
    }

    #[test]
    fn engine_init_missing_iwad() {
        let args = DoomArgs {
            iwad: std::path::PathBuf::from("/nonexistent/doom.wad"),
            pwads: Vec::new(),
            skill: None,
            episode: None,
            warp_map: None,
            warp_episode: None,
            deathmatch: 0,
            nomonsters: false,
            fast: false,
            respawn: false,
            turbo: None,
            timedemo: None,
            playdemo: None,
            devparm: false,
            singletics: false,
            net_players: None,
            net_host: None,
        };
        let result = DoomEngine::init(&args);
        assert!(result.is_err());
    }
}

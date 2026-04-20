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
use crate::menu::navigation::{MenuAction, MenuSystem};
use crate::renderer::bsp::{BspTraversal, WallSegment};
use crate::renderer::data::TextureData;
use crate::renderer::draw::ColumnDrawer;
use crate::renderer::state::RenderState;
use crate::utils::angle::{Angle, ANGLETOFINESHIFT, FINEMASK};
use crate::utils::fixed::{Fixed, FRACBITS, FRACUNIT};
use crate::utils::tables::{fine_cosine, fine_sine};
use crate::video::{VideoSystem, SCREENHEIGHT, SCREENWIDTH};
use crate::wad::WadSystem;

/// Versao do engine.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Dados pre-calculados para rendering de planos (piso/teto).
///
/// Contem tabelas de projecao perspectiva e cache de texturas flat.
/// C original: globals `yslope[]`, `distscale[]`, `basexscale`, `baseyscale`
struct PlaneRenderData {
    /// Lookup de projecao: distancia por linha Y.
    /// yslope[y] = FixedDiv(viewwidth/2, abs(y - centery))
    yslope: [i32; SCREENHEIGHT],
    /// Escala de distancia por coluna X (correcao coseno).
    /// distscale[x] = 1 / cos(xtoviewangle[x])
    distscale: [i32; SCREENWIDTH],
    /// Stepping X na textura de flat por unidade de distancia
    #[allow(dead_code)]
    basexscale: i32,
    /// Stepping Y na textura de flat por unidade de distancia
    #[allow(dead_code)]
    baseyscale: i32,
}

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
    /// Dados de textura carregados do WAD (patches, flats, colormaps)
    pub texture_data: Option<TextureData>,
    /// Drawer de colunas (lookup tables para framebuffer)
    pub column_drawer: ColumnDrawer,
    /// Cache de flats carregados do WAD (construido uma vez ao carregar mapa)
    flat_cache: std::collections::HashMap<[u8; 8], Vec<u8>>,
    /// Tabela de projecao yslope (constante, calculada uma vez na init)
    plane_yslope: [i32; SCREENHEIGHT],
    /// Escala de distancia por coluna (constante, calculada uma vez na init)
    plane_distscale: [i32; SCREENWIDTH],
    /// Configuracao de exibicao
    pub display_config: DisplayConfig,
    /// Estado do efeito wipe
    pub wipe: WipeState,

    // -- Posicao do jogador --

    /// Posicao X do jogador no mapa (fixed-point, 16.16)
    ///
    /// C original: `player->mo->x` (fixed_t)
    pub player_x: Fixed,
    /// Posicao Y do jogador no mapa (fixed-point, 16.16)
    pub player_y: Fixed,
    /// Angulo do jogador em BAM (Binary Angle Measurement, u32)
    ///
    /// C original: `player->mo->angle` (angle_t, unsigned 32-bit)
    /// 0x00000000 = leste, 0x40000000 = norte, 0x80000000 = oeste
    pub player_angle: u32,

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
            texture_data: None,
            column_drawer: ColumnDrawer::new(),
            flat_cache: std::collections::HashMap::new(),
            plane_yslope: [0i32; SCREENHEIGHT],
            plane_distscale: [0i32; SCREENWIDTH],
            display_config: DisplayConfig::default(),
            wipe: WipeState::new(),
            player_x: Fixed(0),
            player_y: Fixed(0),
            player_angle: Angle::ANG90.0, // 90 graus = norte
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
        // R_InitData: carregar texturas, flats, sprites, colormaps
        match TextureData::load(&engine.wad) {
            Ok(td) => {
                log::info!(
                    "R_InitData: {} texturas, {} flats, {} sprites",
                    td.textures.len(),
                    td.num_flats,
                    td.num_sprite_lumps,
                );
                engine.texture_data = Some(td);
            }
            Err(e) => {
                log::warn!("R_InitData: falha ao carregar texturas: {}", e);
            }
        }

        // Pre-calcular tabelas de projecao de planos (constantes)
        engine.init_plane_tables();

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
                    engine.init_player_position();
                    engine.build_flat_cache();
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

        // Processar acoes do menu (New Game, Episode, Skill, etc.)
        self.process_menu_actions();

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

        // Aplicar movimento do jogador (para automap interativo)
        if self.game.state == GameStateType::Level {
            self.apply_player_movement();
        }

        // Atualizar display config com base no estado
        self.update_display_config();

        // D_Display — renderizar frame
        self.d_display();

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

            // No DemoScreen, qualquer tecla abre o menu principal
            if is_down && self.game.state == GameStateType::DemoScreen && !self.menu.active {
                self.menu.open();
                continue;
            }

            // Game responder (atualiza estado de teclas)
            self.input.handle_event(&ev);
        }
    }

    /// Processa acoes pendentes do sistema de menus.
    ///
    /// Quando o jogador seleciona um item no menu, o MenuSystem grava
    /// a acao em `last_action`. O engine consome e executa:
    /// - SubMenu: navega para o submenu apropriado
    /// - ChooseEpisode: seleciona o episodio e abre menu de skill
    /// - ChooseSkill: inicia o jogo com o episodio e skill selecionados
    /// - QuitGame: encerra o engine
    ///
    /// C original: callbacks em `menuitem_t` em `m_menu.c`
    fn process_menu_actions(&mut self) {
        let action = match self.menu.take_action() {
            Some(a) => a,
            None => return,
        };

        match action {
            MenuAction::SubMenu => {
                // Mapear item do main menu para submenu
                let submenu = match self.menu.current_menu {
                    0 => match self.menu.item_on {
                        0 => Some(1), // New Game → Episode menu
                        1 => Some(3), // Options → Options menu
                        2 => Some(4), // Load Game → Load menu
                        3 => Some(5), // Save Game → Save menu
                        _ => None,
                    },
                    _ => None,
                };

                if let Some(menu_idx) = submenu {
                    self.menu.current_menu = menu_idx;
                    self.menu.item_on = self.menu.menus[menu_idx].last_on;
                }
            }

            MenuAction::ChooseEpisode => {
                // Selecionar episodio e abrir menu de skill
                self.game.episode = self.menu.item_on as i32 + 1;
                self.menu.current_menu = 2; // Skill menu
                self.menu.item_on = self.menu.menus[2].last_on;
            }

            MenuAction::ChooseSkill => {
                // Iniciar novo jogo com episodio e skill selecionados
                let skill = match self.menu.item_on {
                    0 => Skill::Baby,
                    1 => Skill::Easy,
                    2 => Skill::Medium,
                    3 => Skill::Hard,
                    4 => Skill::Nightmare,
                    _ => Skill::Medium,
                };

                self.menu.close();
                self.game.skill = skill;
                self.game.map = 1;
                self.game.action = GameAction::LoadLevel;
                self.game.state = GameStateType::Level;

                // Carregar o mapa
                let map_name = format!("E{}M{}", self.game.episode, self.game.map);
                match MapData::load(&map_name, &self.wad) {
                    Ok(mut map) => {
                        map.finalize();
                        log::info!("P_SetupLevel: {} carregado", map_name);
                        self.map = Some(map);
                        self.game.action = GameAction::Nothing;
                        self.game.viewactive = true;
                        self.game.levelstarttic = self.game.gametic;
                        self.init_player_position();
                        self.build_flat_cache();
                    }
                    Err(e) => {
                        log::warn!("Nao foi possivel carregar {}: {}", map_name, e);
                        self.game.state = GameStateType::DemoScreen;
                        self.game.action = GameAction::Nothing;
                    }
                }
            }

            MenuAction::QuitGame => {
                self.running = false;
            }

            MenuAction::NewGame => {
                // Abrir episodio menu diretamente
                self.menu.current_menu = 1;
                self.menu.item_on = 0;
            }

            _ => {
                // Outras acoes (Options, Volume, etc.) — TODO
            }
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
        self.init_player_position();
        self.build_flat_cache();

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

    /// Inicializa a posicao do jogador a partir do Player 1 start
    /// thing do mapa carregado.
    ///
    /// C original: thing_type 1 = Player 1 start em `p_mobj.c`
    fn init_player_position(&mut self) {
        if let Some(ref map) = self.map {
            // Thing type 1 = Player 1 start
            if let Some(start) = map.things.iter().find(|t| t.thing_type == 1) {
                self.player_x = start.x;
                self.player_y = start.y;
                // Converter angulo de graus (0-360) para BAM
                // C original: ANG45 * (mthing->angle / 45)
                let deg = ((start.angle as i32 % 360 + 360) % 360) as u64;
                self.player_angle = (deg * 0x1_0000_0000u64 / 360) as u32;
            }
        }
    }

    /// Aplica o ticcmd atual para mover o jogador.
    ///
    /// Converte forwardmove/angleturn do ticcmd em deslocamento
    /// e verifica colisao com linedefs antes de aplicar o movimento.
    ///
    /// Usa o sistema completo de colisao do DOOM: P_BoxOnLineSide para
    /// teste geometrico AABB-vs-linha, P_LineOpening para gap vertical
    /// em linedefs two-sided, e P_SlideMove para deslizar ao longo
    /// de paredes quando bloqueado.
    ///
    /// C original: `P_MovePlayer()` em `p_user.c` +
    ///             `P_XYMovement()` / `P_TryMove()` em `p_map.c`
    fn apply_player_movement(&mut self) {
        let slot = (self.game.gametic as usize).wrapping_sub(1) % crate::game::state::BACKUPTICS;
        let cmd = self.game.localcmds[slot];

        // Rotacao (P_MovePlayer: mo->angle += cmd->angleturn<<16)
        self.player_angle = self.player_angle.wrapping_add((cmd.angleturn as u32) << 16);

        // Calcular deslocamento usando tabelas do DOOM (fixed-point)
        // C original: P_Thrust(player, angle, forwardmove * 2048)
        let mut momx = Fixed(0);
        let mut momy = Fixed(0);

        if cmd.forwardmove != 0 {
            let fine_angle = (self.player_angle >> ANGLETOFINESHIFT) as usize & FINEMASK;
            let thrust = Fixed(cmd.forwardmove as i32 * 2048);
            momx += thrust * fine_cosine(fine_angle);
            momy += thrust * fine_sine(fine_angle);
        }

        if cmd.sidemove != 0 {
            let strafe_angle = self.player_angle.wrapping_sub(Angle::ANG90.0);
            let fine_angle = (strafe_angle >> ANGLETOFINESHIFT) as usize & FINEMASK;
            let thrust = Fixed(cmd.sidemove as i32 * 2048);
            momx += thrust * fine_cosine(fine_angle);
            momy += thrust * fine_sine(fine_angle);
        }

        if momx.0 == 0 && momy.0 == 0 {
            return;
        }

        let new_x = self.player_x + momx;
        let new_y = self.player_y + momy;

        // P_TryMove: checar colisao com todas as linedefs relevantes
        if !self.try_move(new_x, new_y) {
            // Bloqueado — P_SlideMove: projetar velocidade na parede
            self.slide_move(momx, momy);
        }
    }

    /// Tenta mover o jogador para (new_x, new_y).
    ///
    /// Verifica colisao AABB com todas as linedefs, incluindo:
    /// - Linedefs one-sided (paredes solidas)
    /// - Linedefs com flag BLOCKING
    /// - Linedefs two-sided com gap vertical insuficiente
    /// - Step height maximo de 24 unidades (MAXSTEPHEIGHT)
    ///
    /// C original: `P_TryMove()` + `P_CheckPosition()` + `PIT_CheckLine()` em `p_map.c`
    fn try_move(&mut self, new_x: Fixed, new_y: Fixed) -> bool {
        use crate::map::types::LineDefFlags;

        let map = match &self.map {
            Some(m) => m,
            None => return true,
        };

        // Constantes do DOOM original
        const PLAYER_RADIUS: i32 = 16 * FRACUNIT; // 16 unidades em fixed-point
        const PLAYER_HEIGHT: i32 = 56 * FRACUNIT;  // 56 unidades
        const MAXSTEPHEIGHT: i32 = 24 * FRACUNIT;  // altura maxima de degrau

        // Bounding box do jogador na nova posicao (em fixed-point)
        let tmbox = [
            new_y.0 + PLAYER_RADIUS, // BOXTOP
            new_y.0 - PLAYER_RADIUS, // BOXBOTTOM
            new_x.0 - PLAYER_RADIUS, // BOXLEFT
            new_x.0 + PLAYER_RADIUS, // BOXRIGHT
        ];

        // Altura do piso atual do jogador
        let current_floor = Fixed::from_int(self.find_player_floor_height());

        // tmfloorz e tmceilingz rastreiam o piso mais alto e teto mais
        // baixo de todos os sectors que a AABB toca na nova posicao.
        // Inicializados com os valores do sector de destino.
        let dest_floor = self.find_floor_height_at(new_x, new_y);
        let dest_ceil = self.find_ceiling_height_at(new_x, new_y);
        let mut tmfloorz = dest_floor;
        let mut tmceilingz = dest_ceil;
        let mut tmdropoffz = dest_floor;

        // PIT_CheckLine: iterar todas as linedefs
        for ld in &map.linedefs {
            let v1 = &map.vertexes[ld.v1];
            let v2 = &map.vertexes[ld.v2];

            // Bounding box da linedef (em fixed-point)
            let line_bbox = [
                v1.y.0.max(v2.y.0), // BOXTOP
                v1.y.0.min(v2.y.0), // BOXBOTTOM
                v1.x.0.min(v2.x.0), // BOXLEFT
                v1.x.0.max(v2.x.0), // BOXRIGHT
            ];

            // Quick reject: AABBs nao se sobrepoem
            if tmbox[3] <= line_bbox[2]  // right <= line_left
                || tmbox[2] >= line_bbox[3]  // left >= line_right
                || tmbox[0] <= line_bbox[1]  // top <= line_bottom
                || tmbox[1] >= line_bbox[0]  // bottom >= line_top
            {
                continue;
            }

            // P_BoxOnLineSide: teste AABB vs linha infinita
            let side = Self::box_on_line_side(&tmbox, v1.x.0, v1.y.0, ld);
            if side != -1 {
                // AABB inteira de um lado da linha — nao cruza
                continue;
            }

            // A AABB cruza esta linedef

            // One-sided = parede solida, bloqueia sempre
            if ld.sidenum[1].is_none() {
                return false;
            }

            // Flags de bloqueio
            if ld.flags.contains(LineDefFlags::BLOCKING) {
                return false;
            }

            // P_LineOpening: calcular gap vertical da linedef two-sided
            let (opentop, openbottom, _openrange, lowfloor) =
                Self::line_opening(ld, map);

            // Ajustar tmfloorz / tmceilingz (PIT_CheckLine linhas 227-237)
            if opentop < tmceilingz {
                tmceilingz = opentop;
            }
            if openbottom > tmfloorz {
                tmfloorz = openbottom;
            }
            if lowfloor < tmdropoffz {
                tmdropoffz = lowfloor;
            }
        }

        // P_TryMove: verificacoes finais de altura

        // Cabe verticalmente?
        if tmceilingz - tmfloorz < PLAYER_HEIGHT {
            return false;
        }

        // Teto baixo demais para a posicao atual do jogador?
        if tmceilingz - current_floor.0 < PLAYER_HEIGHT {
            return false;
        }

        // Degrau alto demais? (maximo 24 unidades)
        if tmfloorz - current_floor.0 > MAXSTEPHEIGHT {
            return false;
        }

        // Dropoff muito grande? (nao ficar sobre um abismo > 24 unidades)
        if tmfloorz - tmdropoffz > MAXSTEPHEIGHT {
            return false;
        }

        // Movimento valido
        self.player_x = new_x;
        self.player_y = new_y;
        true
    }

    /// Desliza o jogador ao longo de paredes quando o movimento direto
    /// e bloqueado.
    ///
    /// Implementacao simplificada de P_SlideMove: tenta o movimento
    /// em cada eixo separadamente, e se ambos falharem, projeta
    /// a velocidade na direcao da parede mais proxima.
    ///
    /// C original: `P_SlideMove()` + `P_HitSlideLine()` em `p_map.c`
    fn slide_move(&mut self, momx: Fixed, momy: Fixed) {
        // Tentativa 1: mover apenas no eixo X
        let try_x = self.player_x + momx;
        if self.try_move(try_x, self.player_y) {
            return;
        }

        // Tentativa 2: mover apenas no eixo Y
        let try_y = self.player_y + momy;
        if self.try_move(self.player_x, try_y) {
            return;
        }

        // Tentativa 3: mover com metade da velocidade em cada eixo
        let half_x = self.player_x + Fixed(momx.0 / 2);
        let half_y = self.player_y + Fixed(momy.0 / 2);
        if self.try_move(half_x, half_y) {
            return;
        }

        // Tentativa 4: quartos
        if self.try_move(half_x, self.player_y) {
            return;
        }
        // Ultimo recurso: tentar apenas Y
        let _ = self.try_move(self.player_x, half_y);
    }

    /// Determina de que lado de uma linedef infinita a AABB esta.
    ///
    /// Retorna 0 (front), 1 (back) ou -1 (cruza a linha).
    /// Usa o slopetype da linedef para otimizacao (fast path para
    /// linhas horizontais e verticais).
    ///
    /// C original: `P_BoxOnLineSide()` em `p_maputl.c`
    fn box_on_line_side(
        tmbox: &[i32; 4], // [top, bottom, left, right]
        v1x: i32,
        v1y: i32,
        ld: &crate::map::types::LineDef,
    ) -> i32 {
        use crate::map::types::SlopeType;

        let (p1, p2) = match ld.slope_type {
            SlopeType::Horizontal => {
                let mut a = (tmbox[0] > v1y) as i32; // top > v1.y
                let mut b = (tmbox[1] > v1y) as i32; // bottom > v1.y
                if ld.dx.0 < 0 {
                    a ^= 1;
                    b ^= 1;
                }
                (a, b)
            }
            SlopeType::Vertical => {
                let mut a = (tmbox[3] < v1x) as i32; // right < v1.x
                let mut b = (tmbox[2] < v1x) as i32; // left < v1.x
                if ld.dy.0 < 0 {
                    a ^= 1;
                    b ^= 1;
                }
                (a, b)
            }
            SlopeType::Positive => {
                let a = Self::point_on_line_side(tmbox[2], tmbox[0], v1x, v1y, ld); // left, top
                let b = Self::point_on_line_side(tmbox[3], tmbox[1], v1x, v1y, ld); // right, bottom
                (a, b)
            }
            SlopeType::Negative => {
                let a = Self::point_on_line_side(tmbox[3], tmbox[0], v1x, v1y, ld); // right, top
                let b = Self::point_on_line_side(tmbox[2], tmbox[1], v1x, v1y, ld); // left, bottom
                (a, b)
            }
        };

        if p1 == p2 {
            p1
        } else {
            -1
        }
    }

    /// Determina de que lado de uma linedef um ponto esta.
    ///
    /// Retorna 0 (front/esquerda) ou 1 (back/direita).
    /// Usa cross product: (dy >> 16) * (px - v1x) vs (py - v1y) * (dx >> 16)
    ///
    /// C original: `P_PointOnLineSide()` em `p_maputl.c`
    fn point_on_line_side(
        x: i32,
        y: i32,
        v1x: i32,
        v1y: i32,
        ld: &crate::map::types::LineDef,
    ) -> i32 {
        // Fast path para linhas ortogonais
        if ld.dx.0 == 0 {
            return if x <= v1x {
                (ld.dy.0 > 0) as i32
            } else {
                (ld.dy.0 < 0) as i32
            };
        }
        if ld.dy.0 == 0 {
            return if y <= v1y {
                (ld.dx.0 < 0) as i32
            } else {
                (ld.dx.0 > 0) as i32
            };
        }

        // Cross product para determinar o lado
        let dx = x - v1x;
        let dy = y - v1y;
        let left = (ld.dy.0 >> FRACBITS) as i64 * dx as i64;
        let right = dy as i64 * (ld.dx.0 >> FRACBITS) as i64;

        if right < left {
            0 // front side
        } else {
            1 // back side
        }
    }

    /// Calcula a abertura vertical de uma linedef two-sided.
    ///
    /// Retorna (opentop, openbottom, openrange, lowfloor) em fixed-point.
    /// - opentop: teto mais baixo entre os dois sectors
    /// - openbottom: piso mais alto entre os dois sectors
    /// - openrange: espaco livre vertical (opentop - openbottom)
    /// - lowfloor: piso mais baixo (para deteccao de dropoff)
    ///
    /// C original: `P_LineOpening()` em `p_maputl.c`
    fn line_opening(
        ld: &crate::map::types::LineDef,
        map: &crate::map::MapData,
    ) -> (i32, i32, i32, i32) {
        // One-sided: sem abertura
        let back_idx = match ld.back_sector {
            Some(idx) => idx,
            None => return (0, 0, 0, 0),
        };
        let front_idx = match ld.front_sector {
            Some(idx) => idx,
            None => return (0, 0, 0, 0),
        };

        if front_idx >= map.sectors.len() || back_idx >= map.sectors.len() {
            return (0, 0, 0, 0);
        }

        let front = &map.sectors[front_idx];
        let back = &map.sectors[back_idx];

        let opentop = front.ceiling_height.0.min(back.ceiling_height.0);

        let (openbottom, lowfloor) = if front.floor_height.0 > back.floor_height.0 {
            (front.floor_height.0, back.floor_height.0)
        } else {
            (back.floor_height.0, front.floor_height.0)
        };

        let openrange = opentop - openbottom;
        (opentop, openbottom, openrange, lowfloor)
    }

    /// Encontra a altura do teto no sector que contem o ponto (x, y).
    ///
    /// Usa BSP traversal para encontrar o subsector de destino.
    fn find_ceiling_height_at(&self, x: Fixed, y: Fixed) -> i32 {
        let map = match &self.map {
            Some(m) => m,
            None => return i32::MAX,
        };

        if map.nodes.is_empty() || map.subsectors.is_empty() {
            return i32::MAX;
        }

        let mut node_id = (map.nodes.len() - 1) as u16;
        loop {
            if node_id & crate::map::types::NF_SUBSECTOR != 0 {
                let ss_id = (node_id & !crate::map::types::NF_SUBSECTOR) as usize;
                if ss_id < map.subsectors.len() {
                    let sector_idx = map.subsectors[ss_id].sector;
                    if sector_idx < map.sectors.len() {
                        return map.sectors[sector_idx].ceiling_height.0;
                    }
                }
                return i32::MAX;
            }

            if (node_id as usize) >= map.nodes.len() {
                return i32::MAX;
            }

            let node = &map.nodes[node_id as usize];
            let side = RenderState::point_on_side(x, y, node);
            node_id = node.children[side];
        }
    }

    /// Encontra a altura do piso no sector que contem o ponto (x, y).
    ///
    /// Usa BSP traversal para encontrar o subsector de destino.
    fn find_floor_height_at(&self, x: Fixed, y: Fixed) -> i32 {
        let map = match &self.map {
            Some(m) => m,
            None => return 0,
        };

        if map.nodes.is_empty() || map.subsectors.is_empty() {
            return 0;
        }

        let mut node_id = (map.nodes.len() - 1) as u16;
        loop {
            if node_id & crate::map::types::NF_SUBSECTOR != 0 {
                let ss_id = (node_id & !crate::map::types::NF_SUBSECTOR) as usize;
                if ss_id < map.subsectors.len() {
                    let sector_idx = map.subsectors[ss_id].sector;
                    if sector_idx < map.sectors.len() {
                        return map.sectors[sector_idx].floor_height.0;
                    }
                }
                return 0;
            }

            if (node_id as usize) >= map.nodes.len() {
                return 0;
            }

            let node = &map.nodes[node_id as usize];
            let side = RenderState::point_on_side(x, y, node);
            node_id = node.children[side];
        }
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

    /// Renderiza o frame atual no framebuffer.
    ///
    /// Seleciona o drawer apropriado conforme o estado do jogo:
    /// - Level: automap top-down do mapa carregado
    /// - DemoScreen: TITLEPIC do WAD ou padrao de teste
    /// - Intermission/Finale: padrao com cor de fundo
    ///
    /// C original: `D_Display()` em `d_main.c`
    fn d_display(&mut self) {
        match self.game.state {
            GameStateType::Level => {
                self.render_player_view();
            }
            GameStateType::DemoScreen => {
                self.draw_title_screen();
            }
            GameStateType::Intermission => {
                // Tela de intermissao — cor de fundo azul escuro
                let screen = self.video.screen_mut(0);
                for pixel in screen.iter_mut() {
                    *pixel = 0xC7; // azul escuro na paleta DOOM
                }
            }
            GameStateType::Finale => {
                // Tela de finale — cor de fundo preto
                let screen = self.video.screen_mut(0);
                for pixel in screen.iter_mut() {
                    *pixel = 0;
                }
            }
        }

        // Menu overlay — desenha por cima de tudo quando ativo
        if self.menu.active {
            self.draw_menu();
        }
    }

    /// Desenha o menu ativo sobre o framebuffer.
    ///
    /// Carrega patches do WAD para cada item de menu e desenha
    /// na posicao definida pelo menu. Um indicador (seta) marca
    /// o item selecionado.
    ///
    /// C original: `M_Drawer()` em `m_menu.c`
    fn draw_menu(&mut self) {
        let menu_idx = self.menu.current_menu;
        let menu_x = self.menu.menus[menu_idx].x;
        let menu_y = self.menu.menus[menu_idx].y;
        let item_on = self.menu.item_on;
        let items: Vec<_> = self.menu.menus[menu_idx]
            .items
            .iter()
            .map(|item| item.name.to_string())
            .collect();

        let line_height = crate::menu::navigation::LINEHEIGHT;

        for (i, lump_name) in items.iter().enumerate() {
            let y = menu_y + (i as i32) * line_height;

            if !lump_name.is_empty() {
                if let Ok(data) = self.wad.read_lump_by_name(lump_name) {
                    if data.len() > 8 {
                        self.video.draw_patch(menu_x, y, 0, &data);
                    }
                }
            }

            // Desenhar indicador de selecao (seta ">>" em pixels)
            if i == item_on {
                self.draw_selector(menu_x - 20, y);
            }
        }
    }

    /// Desenha um indicador de selecao (cursor) ao lado do item de menu.
    ///
    /// No DOOM original, seria o skull cursor animado (M_SKULL1/M_SKULL2).
    /// Aqui tentamos carregar o skull do WAD; se nao encontrar,
    /// desenhamos um marcador simples.
    fn draw_selector(&mut self, x: i32, y: i32) {
        // Tentar carregar skull cursor do WAD
        let skull_name = if self.menu.skull_frame == 0 {
            "M_SKULL1"
        } else {
            "M_SKULL2"
        };

        if let Ok(data) = self.wad.read_lump_by_name(skull_name) {
            if data.len() > 8 {
                self.video.draw_patch(x, y, 0, &data);
                return;
            }
        }

        // Fallback: retangulo branco como indicador
        let w = crate::video::SCREENWIDTH;
        let h = crate::video::SCREENHEIGHT;
        let screen = self.video.screen_mut(0);
        for dy in 0..10 {
            for dx in 0..10 {
                let px = x + dx;
                let py = y + dy + 3;
                if px >= 0 && px < w as i32 && py >= 0 && py < h as i32 {
                    screen[py as usize * w + px as usize] = 0x04; // vermelho DOOM
                }
            }
        }
    }

    /// Desenha a tela de titulo (TITLEPIC) do WAD.
    ///
    /// Tenta carregar o lump TITLEPIC como um flat raw de 320x200.
    /// Se nao encontrar, desenha um padrao de teste colorido.
    ///
    /// C original: `D_PageDrawer()` em `d_main.c`
    fn draw_title_screen(&mut self) {
        // Nomes dos lumps de tela por pagina do demo screen
        let page_names = ["TITLEPIC", "CREDIT", "HELP1"];
        let page = (self.ticker.demo_page as usize) % page_names.len();
        let lump_name = page_names[page];

        if let Ok(data) = self.wad.read_lump_by_name(lump_name) {
            if data.len() == crate::video::SCREEN_SIZE {
                // Lump raw 320x200 (alguns WADs armazenam assim)
                let screen = self.video.screen_mut(0);
                screen.copy_from_slice(&data);
            } else if data.len() > 8 {
                // Lump em formato patch (column-based) — formato padrao
                // Limpar tela antes de desenhar o patch
                let screen = self.video.screen_mut(0);
                for pixel in screen.iter_mut() {
                    *pixel = 0;
                }
                self.video.draw_patch(0, 0, 0, &data);
            }
            return;
        }

        // Fallback: padrao de teste para verificar que o pipeline funciona
        self.draw_test_pattern();
    }

    /// Desenha um padrao de teste colorido no framebuffer.
    ///
    /// Barras verticais usando diferentes indices da paleta DOOM.
    /// Util para verificar que o pipeline video esta funcionando.
    fn draw_test_pattern(&mut self) {
        let screen = self.video.screen_mut(0);
        let w = crate::video::SCREENWIDTH;
        let h = crate::video::SCREENHEIGHT;

        for y in 0..h {
            for x in 0..w {
                // Barras verticais coloridas (16 barras de 20px)
                let bar = x / 20;
                // Usar diferentes rampas de cor da paleta DOOM
                let brightness = (y * 16 / h) as u8;
                let color = (bar as u8 * 16).wrapping_add(brightness);
                screen[y * w + x] = color;
            }
        }
    }

    /// Desenha uma vista automap (top-down) do mapa carregado.
    /// Mantido para futuro toggle via tecla TAB.
    ///
    /// Renderiza as linedefs do mapa como linhas coloridas no
    /// framebuffer, similar ao automap do DOOM (tecla TAB).
    /// Paredes one-sided em vermelho, two-sided em cinza/marrom.
    ///
    /// C original: `AM_Drawer()` em `am_map.c`
    #[allow(dead_code)]
    fn draw_automap(&mut self) {
        let w = crate::video::SCREENWIDTH;
        let h = crate::video::SCREENHEIGHT;

        // Limpar tela com cor de fundo escura
        let screen = self.video.screen_mut(0);
        for pixel in screen.iter_mut() {
            *pixel = 0; // preto
        }

        // Precisamos do mapa para desenhar
        let map = match &self.map {
            Some(m) => m,
            None => return,
        };

        if map.vertexes.is_empty() || map.linedefs.is_empty() {
            return;
        }

        // Calcular bounding box do mapa (em coordenadas inteiras)
        let mut min_x = i32::MAX;
        let mut max_x = i32::MIN;
        let mut min_y = i32::MAX;
        let mut max_y = i32::MIN;

        for v in &map.vertexes {
            let ix = v.x.to_int();
            let iy = v.y.to_int();
            min_x = min_x.min(ix);
            max_x = max_x.max(ix);
            min_y = min_y.min(iy);
            max_y = max_y.max(iy);
        }

        let map_width = max_x - min_x;
        let map_height = max_y - min_y;

        if map_width <= 0 || map_height <= 0 {
            return;
        }

        // Escala fixa: ~1 pixel por unidade de mapa, centrado no jogador
        // Zoom que mostra area de ~300 unidades de mapa na tela
        let scale: i64 = ((w as i64) << 16) / (map_width as i64).max(1);
        let scale_y_val: i64 = ((h as i64) << 16) / (map_height as i64).max(1);
        let scale = scale.min(scale_y_val);

        // Centralizar no jogador
        let center_x = w as i32 / 2;
        let center_y = h as i32 / 2;
        let px = self.player_x.to_int();
        let py = self.player_y.to_int();

        // Converter coordenada do mapa para pixel na tela
        let to_screen = |mx: i32, my: i32| -> (i32, i32) {
            let sx = center_x + (((mx - px) as i64 * scale) >> 16) as i32;
            // Y invertido: no DOOM y cresce para cima, na tela para baixo
            let sy = center_y - (((my - py) as i64 * scale) >> 16) as i32;
            (sx, sy)
        };

        // Desenhar cada linedef como uma linha
        // Copiar dados necessarios para evitar borrow conflict
        let lines: Vec<_> = map.linedefs.iter().map(|ld| {
            let v1 = map.vertexes[ld.v1];
            let v2 = map.vertexes[ld.v2];
            let two_sided = ld.flags.contains(crate::map::types::LineDefFlags::TWO_SIDED);
            (v1.x.to_int(), v1.y.to_int(), v2.x.to_int(), v2.y.to_int(), two_sided)
        }).collect();

        // Posicao do jogador na tela (sempre no centro)
        let player_screen = to_screen(px, py);
        let player_angle_bam = self.player_angle;

        let screen = self.video.screen_mut(0);

        for &(v1x, v1y, v2x, v2y, two_sided) in &lines {
            let (x1, y1) = to_screen(v1x, v1y);
            let (x2, y2) = to_screen(v2x, v2y);

            // Cor: vermelho para one-sided, cinza/marrom para two-sided
            let color: u8 = if two_sided { 0x60 } else { 0xAC };

            // Bresenham line drawing
            draw_line(screen, w, h, x1, y1, x2, y2, color);
        }

        // Desenhar seta do jogador (triangulo apontando na direcao do angulo)
        // Converter BAM para radianos: BAM / 2^32 * 2*PI
        let angle_rad = (player_angle_bam as f64) * (2.0 * std::f64::consts::PI) / 4_294_967_296.0;
        let arrow_len: f64 = 8.0;
        let (psx, psy) = player_screen;

        // Ponta da seta
        let tip_x = psx + (arrow_len * angle_rad.cos()) as i32;
        let tip_y = psy - (arrow_len * angle_rad.sin()) as i32;

        // Dois cantos traseiros da seta
        let back_angle1 = angle_rad + 2.5;
        let back_angle2 = angle_rad - 2.5;
        let back_len: f64 = 5.0;
        let b1x = psx + (back_len * back_angle1.cos()) as i32;
        let b1y = psy - (back_len * back_angle1.sin()) as i32;
        let b2x = psx + (back_len * back_angle2.cos()) as i32;
        let b2y = psy - (back_len * back_angle2.sin()) as i32;

        // Cor branca (0x04 = branco brilhante na paleta DOOM)
        draw_line(screen, w, h, tip_x, tip_y, b1x, b1y, 0x04);
        draw_line(screen, w, h, tip_x, tip_y, b2x, b2y, 0x04);
        draw_line(screen, w, h, b1x, b1y, b2x, b2y, 0x04);
    }

    /// Renderiza a cena 3D em primeira pessoa.
    ///
    /// Implementa o pipeline de rendering do DOOM:
    /// 1. Configurar camera (R_SetupFrame)
    /// 2. Travessia BSP (R_RenderBSPNode) para coletar paredes visiveis
    /// 3. Renderizar paredes com perspectiva coluna-a-coluna
    /// 4. Preencher pisos e tetos com cores planas
    ///
    /// C original: `R_RenderPlayerView()` em `r_main.c`
    fn render_player_view(&mut self) {
        // 1. Configurar camera a partir da posicao do jogador
        self.setup_camera();

        // 2. Preencher framebuffer com cor preta (sera sobrescrito por paredes/pisos)
        {
            let screen = self.video.screen_mut(0);
            for pixel in screen.iter_mut().take(SCREENWIDTH * SCREENHEIGHT) {
                *pixel = 0;
            }
        }

        // 3. Calcular basexscale/baseyscale (depende de viewangle, muda por frame)
        let angle_idx = ((self.render_state.viewangle.0.wrapping_sub(Angle::ANG90.0))
            >> ANGLETOFINESHIFT) as usize
            & FINEMASK;
        let basexscale =
            (fine_cosine(angle_idx) / Fixed(self.render_state.centerxfrac.0)).0;
        let baseyscale =
            -(fine_sine(angle_idx) / Fixed(self.render_state.centerxfrac.0)).0;

        // 4. Travessia BSP para coletar segmentos de parede visiveis
        let mut bsp = BspTraversal::new();
        if let Some(ref map) = self.map {
            bsp.render_bsp(map, &self.render_state);
        }

        // 5. Renderizar paredes coletadas com perspectiva e flats texturizados
        // yslope e distscale sao tabelas constantes pre-calculadas na init
        let plane_data = PlaneRenderData {
            yslope: self.plane_yslope,
            distscale: self.plane_distscale,
            basexscale,
            baseyscale,
        };
        let wall_segments = std::mem::take(&mut bsp.wall_ranges);
        self.render_walls(&wall_segments, &plane_data);
    }

    /// Constroi cache de dados de flat (piso/teto) para o mapa atual.
    ///
    /// Carrega todos os flats referenciados pelos sectors do mapa
    /// uma unica vez, armazenando em self.flat_cache.
    /// Chamado ao carregar um novo mapa, nao a cada frame.
    ///
    /// C original: `R_InitFlats()` em `r_data.c`
    fn build_flat_cache(&mut self) {
        self.flat_cache.clear();
        let map = match &self.map {
            Some(m) => m,
            None => return,
        };
        let td = match &self.texture_data {
            Some(td) => td,
            None => return,
        };

        // Coletar nomes unicos de flats usados nos sectors
        for sector in &map.sectors {
            for flat_name in [&sector.floor_pic, &sector.ceiling_pic] {
                if flat_name[0] == 0 || self.flat_cache.contains_key(flat_name) {
                    continue;
                }
                // Encontrar o lump do flat
                let end = flat_name.iter().position(|&b| b == 0).unwrap_or(8);
                let name_str = std::str::from_utf8(&flat_name[..end]).unwrap_or("");
                if let Some(flat_num) = td.flat_num_for_name(name_str, &self.wad) {
                    let lump_idx = td.first_flat + flat_num;
                    if let Ok(data) = self.wad.read_lump(lump_idx) {
                        if data.len() >= 4096 {
                            self.flat_cache.insert(*flat_name, data[..4096].to_vec());
                        }
                    }
                }
            }
        }
    }

    /// Inicializa as tabelas de projecao de planos (yslope e distscale).
    ///
    /// Estas tabelas sao constantes (dependem apenas das dimensoes da tela
    /// e da tabela xtoviewangle, que nao muda). Calculadas uma unica vez
    /// na inicializacao, evitando recalculo a cada frame.
    ///
    /// C original: parte de `R_InitTextureMapping()` e `R_SetupFrame()` em `r_main.c`
    fn init_plane_tables(&mut self) {
        // yslope[y] = FixedDiv(viewwidth/2, abs(y - centery))
        for (i, slot) in self.plane_yslope.iter_mut().enumerate() {
            let dy = ((i as i32 - SCREENHEIGHT as i32 / 2) << FRACBITS) + FRACUNIT / 2;
            let dy = dy.abs();
            if dy > 0 {
                *slot = (Fixed::from_int(SCREENWIDTH as i32 / 2) / Fixed(dy)).0;
            }
        }

        // distscale[x] = 1 / cos(xtoviewangle[x])
        for (i, slot) in self.plane_distscale.iter_mut().enumerate() {
            let cos_adj = fine_cosine(
                (self.render_state.xtoviewangle[i].0 >> ANGLETOFINESHIFT) as usize & FINEMASK,
            );
            if cos_adj.0.abs() > 0 {
                *slot = (Fixed(FRACUNIT) / cos_adj).0.abs();
            }
        }
    }

    /// Configura a camera do renderer a partir da posicao do jogador.
    ///
    /// Posicao ja esta em fixed-point e angulo ja esta em BAM,
    /// entao basta atribuir diretamente ao render_state.
    ///
    /// C original: `R_SetupFrame()` em `r_main.c`
    fn setup_camera(&mut self) {
        self.render_state.viewx = self.player_x;
        self.render_state.viewy = self.player_y;

        // Altura dos olhos: piso do sector + 41 unidades (VIEWHEIGHT padrao)
        let floor_h = self.find_player_floor_height();
        self.render_state.viewz = Fixed::from_int(floor_h + 41);

        // Angulo ja esta em BAM — atribuir diretamente
        self.render_state.viewangle = Angle(self.player_angle);

        // Pre-calcular sin/cos e incrementar contadores de frame
        self.render_state.setup_frame();
    }

    /// Encontra a altura do piso no sector onde o jogador esta.
    ///
    /// Percorre a BSP tree ate encontrar o subsector que contem
    /// a posicao do jogador, e retorna a altura do piso desse sector.
    ///
    /// C original: `R_PointInSubsector()` em `r_main.c`
    fn find_player_floor_height(&self) -> i32 {
        let map = match &self.map {
            Some(m) => m,
            None => return 0,
        };

        if map.nodes.is_empty() || map.subsectors.is_empty() {
            return 0;
        }

        let px = self.player_x;
        let py = self.player_y;
        let mut node_id = (map.nodes.len() - 1) as u16;

        loop {
            if node_id & crate::map::types::NF_SUBSECTOR != 0 {
                let ss_id = (node_id & !crate::map::types::NF_SUBSECTOR) as usize;
                if ss_id < map.subsectors.len() {
                    let sector_idx = map.subsectors[ss_id].sector;
                    if sector_idx < map.sectors.len() {
                        return map.sectors[sector_idx].floor_height.to_int();
                    }
                }
                return 0;
            }

            if (node_id as usize) >= map.nodes.len() {
                return 0;
            }

            let node = &map.nodes[node_id as usize];
            let side = RenderState::point_on_side(px, py, node);
            node_id = node.children[side];
        }
    }

    /// Calcula a escala perspectiva para um angulo de visao global.
    ///
    /// Dado o angulo de uma coluna da tela, calcula a escala na qual
    /// a parede deve ser desenhada. Paredes mais proximas tem escala maior.
    ///
    /// C original: `R_ScaleFromGlobalAngle()` em `r_main.c`
    fn scale_from_global_angle(
        &self,
        visangle: Angle,
        rw_normalangle: Angle,
        rw_distance: Fixed,
    ) -> i32 {
        let anglea = Angle(
            Angle::ANG90
                .0
                .wrapping_add(visangle.0.wrapping_sub(self.render_state.viewangle.0)),
        );
        let angleb = Angle(
            Angle::ANG90
                .0
                .wrapping_add(visangle.0.wrapping_sub(rw_normalangle.0)),
        );

        let sinea = fine_sine((anglea.0 >> ANGLETOFINESHIFT) as usize & FINEMASK);
        let sineb = fine_sine((angleb.0 >> ANGLETOFINESHIFT) as usize & FINEMASK);

        // num = FixedMul(projection, sineb)
        let num = (self.render_state.projection * sineb).0;
        // den = FixedMul(rw_distance, sinea)
        let den = (rw_distance * sinea).0;

        if den > (num >> 16) {
            let scale = (Fixed(num) / Fixed(den)).0;
            scale.clamp(256, 64 * FRACUNIT)
        } else {
            64 * FRACUNIT
        }
    }

    /// Renderiza os segmentos de parede coletados pela travessia BSP.
    ///
    /// Para cada segmento visivel:
    /// 1. Calcula a distancia perpendicular a parede
    /// 2. Resolve texturas do sidedef (mid/top/bottom)
    /// 3. Calcula rw_offset para mapeamento horizontal de textura
    /// 4. Para cada coluna: calcula texturecolumn, dc_iscale, colormap
    /// 5. Desenha colunas de parede com textura real via draw_column()
    /// 6. Preenche teto acima e piso abaixo com cor plana
    /// 7. Atualiza arrays de clipping para oclusao correta
    ///
    /// C original: `R_StoreWallRange()` + `R_RenderSegLoop()` em `r_segs.c`
    #[allow(clippy::too_many_lines)]
    fn render_walls(&mut self, wall_segments: &[WallSegment], planes: &PlaneRenderData) {
        use crate::utils::tables::fine_tangent;

        let map = match &self.map {
            Some(m) => m,
            None => return,
        };

        // Arrays de clipping: controlam quais regioes da tela ja foram
        // preenchidas por paredes mais proximas.
        let mut floorclip = vec![SCREENHEIGHT as i16; SCREENWIDTH];
        let mut ceilingclip = vec![-1i16; SCREENWIDTH];

        // Constante de LIGHTSCALESHIFT (C original: 12)
        const LIGHTSCALESHIFT: usize = 12;

        for ws in wall_segments {
            if ws.seg_index >= map.segs.len() {
                continue;
            }
            let seg = &map.segs[ws.seg_index];
            if seg.front_sector >= map.sectors.len() {
                continue;
            }
            let v1 = &map.vertexes[seg.v1];
            let front_sector = &map.sectors[seg.front_sector];
            let is_solid = seg.back_sector.is_none();

            // --- Calcular rw_distance ---
            let rw_normalangle = Angle(seg.angle.wrapping_add(Angle::ANG90.0));

            let offset_angle_raw = rw_normalangle.0.wrapping_sub(ws.angle1.0);
            let offset_angle = if offset_angle_raw > Angle::ANG180.0 {
                Angle(0u32.wrapping_sub(offset_angle_raw))
            } else {
                Angle(offset_angle_raw)
            };
            let offset_angle = if offset_angle.0 > Angle::ANG90.0 {
                Angle::ANG90
            } else {
                offset_angle
            };

            let dist_angle = Angle(Angle::ANG90.0.wrapping_sub(offset_angle.0));
            let hyp = self.render_state.point_to_dist(v1.x, v1.y);
            let sineval =
                fine_sine((dist_angle.0 >> ANGLETOFINESHIFT) as usize & FINEMASK);
            let rw_distance = hyp * sineval;

            if rw_distance.0 == 0 {
                continue;
            }

            // --- Escala nas bordas do segmento ---
            let x1 = ws.x1.max(0);
            let x2 = ws.x2.min(SCREENWIDTH as i32 - 1);
            if x1 > x2 {
                continue;
            }

            let rw_scale = self.scale_from_global_angle(
                Angle(
                    self.render_state.viewangle.0
                        .wrapping_add(self.render_state.xtoviewangle[x1 as usize].0),
                ),
                rw_normalangle,
                rw_distance,
            );

            let scale2 = if x2 > x1 {
                self.scale_from_global_angle(
                    Angle(
                        self.render_state.viewangle.0
                            .wrapping_add(self.render_state.xtoviewangle[x2 as usize].0),
                    ),
                    rw_normalangle,
                    rw_distance,
                )
            } else {
                rw_scale
            };

            let rw_scalestep = if x2 > x1 {
                (scale2 - rw_scale) / (x2 - x1)
            } else {
                0
            };

            // --- Alturas do mundo (nao-shiftadas para comparacoes) ---
            let worldtop_full = front_sector.ceiling_height.0 - self.render_state.viewz.0;
            let worldbottom_full = front_sector.floor_height.0 - self.render_state.viewz.0;

            let (worldhigh_full, worldlow_full) = if let Some(back_idx) = seg.back_sector {
                if back_idx < map.sectors.len() {
                    let back = &map.sectors[back_idx];
                    (
                        back.ceiling_height.0 - self.render_state.viewz.0,
                        back.floor_height.0 - self.render_state.viewz.0,
                    )
                } else {
                    (worldtop_full, worldbottom_full)
                }
            } else {
                (worldtop_full, worldbottom_full)
            };

            let has_upper = !is_solid && worldhigh_full < worldtop_full;
            let has_lower = !is_solid && worldlow_full > worldbottom_full;

            // Shift para HEIGHTBITS (>> 4) apos comparacoes
            let worldtop = worldtop_full >> 4;
            let worldbottom = worldbottom_full >> 4;
            let worldhigh = worldhigh_full >> 4;
            let worldlow = worldlow_full >> 4;

            // --- Resolver texturas do sidedef ---
            let sidedef = if seg.sidedef < map.sidedefs.len() {
                &map.sidedefs[seg.sidedef]
            } else {
                continue;
            };
            let linedef = if seg.linedef < map.linedefs.len() {
                &map.linedefs[seg.linedef]
            } else {
                continue;
            };

            // Resolver nomes de textura para indices
            let tex_data = self.texture_data.as_ref();
            let mid_tex = tex_data.and_then(|td| {
                Self::resolve_texture_name(&sidedef.mid_texture, td)
            });
            let top_tex = tex_data.and_then(|td| {
                Self::resolve_texture_name(&sidedef.top_texture, td)
            });
            let bot_tex = tex_data.and_then(|td| {
                Self::resolve_texture_name(&sidedef.bottom_texture, td)
            });

            let segtextured = mid_tex.is_some() || top_tex.is_some() || bot_tex.is_some();

            // --- Calcular rw_offset (horizontal texture offset) ---
            // C original: r_segs.c linhas 619-636
            let (rw_offset, rw_centerangle) = if segtextured {
                let mut oa = rw_normalangle.0.wrapping_sub(ws.angle1.0);
                if oa > Angle::ANG180.0 {
                    oa = 0u32.wrapping_sub(oa);
                }
                if oa > Angle::ANG90.0 {
                    oa = Angle::ANG90.0;
                }
                let sineval2 = fine_sine((oa >> ANGLETOFINESHIFT) as usize & FINEMASK);
                let mut offset = hyp * sineval2;

                if rw_normalangle.0.wrapping_sub(ws.angle1.0) < Angle::ANG180.0 {
                    offset = Fixed(0) - offset;
                }
                offset = offset + sidedef.texture_offset + seg.offset;

                let center = Angle(
                    Angle::ANG90.0.wrapping_add(
                        self.render_state.viewangle.0.wrapping_sub(rw_normalangle.0),
                    ),
                );
                (offset, center)
            } else {
                (Fixed(0), Angle::ANG0)
            };

            // --- Calcular dc_texturemid (vertical alignment com pegging) ---
            let rw_midtexturemid = if is_solid {
                if let Some(tex_idx) = mid_tex {
                    if linedef.flags.contains(crate::map::types::LineDefFlags::DONT_PEG_BOTTOM) {
                        // Bottom pegging: bottom of texture at floor
                        let tex_h = tex_data.map_or(0, |td| td.texture_height[tex_idx].0);
                        Fixed(front_sector.floor_height.0 + tex_h - self.render_state.viewz.0)
                            + sidedef.row_offset
                    } else {
                        // Top pegging: top of texture at ceiling
                        Fixed(worldtop_full) + sidedef.row_offset
                    }
                } else {
                    Fixed(worldtop_full) + sidedef.row_offset
                }
            } else {
                Fixed(0)
            };

            let rw_toptexturemid = if has_upper {
                if let Some(tex_idx) = top_tex {
                    if linedef.flags.contains(crate::map::types::LineDefFlags::DONT_PEG_TOP) {
                        Fixed(worldtop_full) + sidedef.row_offset
                    } else {
                        // Bottom of texture at back ceiling
                        let tex_h = tex_data.map_or(0, |td| td.texture_height[tex_idx].0);
                        let back_ceil = if let Some(bi) = seg.back_sector {
                            map.sectors[bi].ceiling_height.0
                        } else {
                            front_sector.ceiling_height.0
                        };
                        Fixed(back_ceil + tex_h - self.render_state.viewz.0)
                            + sidedef.row_offset
                    }
                } else {
                    Fixed(worldtop_full) + sidedef.row_offset
                }
            } else {
                Fixed(0)
            };

            let rw_bottomtexturemid = if has_lower {
                if linedef.flags.contains(crate::map::types::LineDefFlags::DONT_PEG_BOTTOM) {
                    Fixed(worldtop_full) + sidedef.row_offset
                } else {
                    Fixed(worldlow_full) + sidedef.row_offset
                }
            } else {
                Fixed(0)
            };

            // --- Nivel de luz do sector ---
            let lightnum = ((front_sector.light_level as usize) >> 4).min(15);
            // Variacao por orientacao da parede
            let v1y = map.vertexes[seg.v1].y.0;
            let v2y = map.vertexes[seg.v2].y.0;
            let v1x = map.vertexes[seg.v1].x.0;
            let v2x = map.vertexes[seg.v2].x.0;
            let lightnum = if v1y == v2y {
                lightnum.saturating_sub(1) // paredes horizontais mais escuras
            } else if v1x == v2x {
                (lightnum + 1).min(15) // paredes verticais mais claras
            } else {
                lightnum
            };

            let shade = lightnum as u8;
            // Fallback colors se flat nao for encontrado
            let ceil_color_fb = shade / 4;
            let floor_color_fb = 0x80 + (shade / 2).min(7);

            // Lookup flat data para piso e teto
            let floor_flat = self.flat_cache.get(&front_sector.floor_pic);
            let ceil_flat = self.flat_cache.get(&front_sector.ceiling_pic);

            // Alturas absolutas do piso/teto (para planeheight)
            let floor_h = front_sector.floor_height.0;
            let ceil_h = front_sector.ceiling_height.0;
            let viewz = self.render_state.viewz.0;

            // --- Calcular fracs e steps (20.12 format) ---
            let centery4 = self.render_state.centeryfrac.0 >> 4;
            let mut topfrac =
                centery4 - ((worldtop as i64 * rw_scale as i64) >> FRACBITS) as i32;
            let mut bottomfrac =
                centery4 - ((worldbottom as i64 * rw_scale as i64) >> FRACBITS) as i32;
            let topstep =
                -((rw_scalestep as i64 * worldtop as i64) >> FRACBITS) as i32;
            let bottomstep =
                -((rw_scalestep as i64 * worldbottom as i64) >> FRACBITS) as i32;

            let mut pixhigh = if has_upper {
                centery4 - ((worldhigh as i64 * rw_scale as i64) >> FRACBITS) as i32
            } else {
                0
            };
            let pixhighstep = if has_upper {
                -((rw_scalestep as i64 * worldhigh as i64) >> FRACBITS) as i32
            } else {
                0
            };
            let mut pixlow = if has_lower {
                centery4 - ((worldlow as i64 * rw_scale as i64) >> FRACBITS) as i32
            } else {
                0
            };
            let pixlowstep = if has_lower {
                -((rw_scalestep as i64 * worldlow as i64) >> FRACBITS) as i32
            } else {
                0
            };

            // Referencia aos colormaps para iluminacao de flats
            let colormaps_ref = self.texture_data.as_ref().map(|td| td.colormaps.as_slice());

            // --- Renderizar colunas ---
            let screen = self.video.screen_mut(0);
            let sh = SCREENHEIGHT as i32;
            let mut cur_scale = rw_scale;

            for x in x1..=x2 {
                let xu = x as usize;

                // Topo e base da parede
                let mut yl = (topfrac + 0xFFF) >> 12;
                if yl < ceilingclip[xu] as i32 + 1 {
                    yl = ceilingclip[xu] as i32 + 1;
                }
                let mut yh = bottomfrac >> 12;
                if yh >= floorclip[xu] as i32 {
                    yh = floorclip[xu] as i32 - 1;
                }

                // --- Calcular texturecolumn e colormap (se texturizado) ---
                let (texturecolumn, colormap_offset) = if segtextured {
                    // C original: r_segs.c linhas 265-274
                    let angle = (rw_centerangle.0
                        .wrapping_add(self.render_state.xtoviewangle[xu].0))
                        >> ANGLETOFINESHIFT;
                    let tangent = fine_tangent(angle as usize & FINEMASK);
                    let tc = (rw_offset - rw_distance * tangent).0 >> FRACBITS;

                    // Colormap baseada na escala (distancia)
                    let light_idx = ((cur_scale as usize) >> LIGHTSCALESHIFT)
                        .min(crate::renderer::state::MAXLIGHTSCALE - 1);
                    let cm = self.render_state.scalelight[lightnum][light_idx];
                    (tc, cm)
                } else {
                    (0, 0)
                };

                // dc_iscale = 0xFFFFFFFF / rw_scale
                let dc_iscale = if cur_scale > 0 {
                    Fixed((0xFFFF_FFFFu32 / cur_scale as u32) as i32)
                } else {
                    Fixed(0x7FFF_FFFF)
                };

                if is_solid {
                    // --- Parede solida (one-sided) ---

                    // Teto com textura flat
                    let ct = (ceilingclip[xu] as i32 + 1).max(0);
                    let cb = (yl - 1).min(sh - 1);
                    Self::draw_flat_column(
                        screen, xu, ct, cb, ceil_flat, ceil_color_fb,
                        (ceil_h - viewz).abs(), planes, &self.render_state,
                        lightnum, colormaps_ref,
                    );

                    // Parede com textura
                    if let Some(tex_idx) = mid_tex {
                        if yl <= yh && yl >= 0 && yh < sh {
                            if let Some(td) = &self.texture_data {
                                let source = td.get_column(tex_idx, texturecolumn);
                                if !source.is_empty() {
                                    let colormap = &td.colormaps[colormap_offset..];
                                    self.column_drawer.draw_column(
                                        screen, xu, yl, yh, dc_iscale,
                                        rw_midtexturemid, source, colormap,
                                    );
                                }
                            }
                        }
                    } else {
                        let wt = yl.max(0);
                        let wb = yh.min(sh - 1);
                        for y in wt..=wb {
                            screen[y as usize * SCREENWIDTH + xu] = 0x60 + shade;
                        }
                    }

                    // Piso com textura flat
                    let ft = (yh + 1).max(0);
                    let fb = (floorclip[xu] as i32 - 1).min(sh - 1);
                    Self::draw_flat_column(
                        screen, xu, ft, fb, floor_flat, floor_color_fb,
                        (floor_h - viewz).abs(), planes, &self.render_state,
                        lightnum, colormaps_ref,
                    );

                    ceilingclip[xu] = sh as i16;
                    floorclip[xu] = -1;
                } else {
                    // --- Parede two-sided ---

                    // Teto com textura flat
                    let mark_ceiling = worldhigh_full != worldtop_full || has_upper;
                    if mark_ceiling {
                        let ct = (ceilingclip[xu] as i32 + 1).max(0);
                        let cb = (yl - 1).min(sh - 1);
                        Self::draw_flat_column(
                            screen, xu, ct, cb, ceil_flat, ceil_color_fb,
                            (ceil_h - viewz).abs(), planes, &self.render_state,
                            lightnum, colormaps_ref,
                        );
                    }

                    // Upper wall
                    if has_upper {
                        let mut mid = pixhigh >> 12;
                        if mid >= floorclip[xu] as i32 {
                            mid = floorclip[xu] as i32 - 1;
                        }
                        if mid >= yl {
                            if let Some(tex_idx) = top_tex {
                                if yl >= 0 && mid < sh {
                                    if let Some(td) = &self.texture_data {
                                        let source = td.get_column(tex_idx, texturecolumn);
                                        if !source.is_empty() {
                                            let colormap = &td.colormaps[colormap_offset..];
                                            self.column_drawer.draw_column(
                                                screen, xu, yl, mid, dc_iscale,
                                                rw_toptexturemid, source, colormap,
                                            );
                                        }
                                    }
                                }
                            } else {
                                let wt = yl.max(0);
                                let wb = mid.min(sh - 1);
                                for y in wt..=wb {
                                    screen[y as usize * SCREENWIDTH + xu] = 0x60 + shade;
                                }
                            }
                            ceilingclip[xu] = mid as i16;
                        } else {
                            ceilingclip[xu] = (yl - 1) as i16;
                        }
                    } else {
                        ceilingclip[xu] = (yl - 1) as i16;
                    }

                    // Lower wall
                    if has_lower {
                        let mut mid = (pixlow + 0xFFF) >> 12;
                        if mid <= ceilingclip[xu] as i32 {
                            mid = ceilingclip[xu] as i32 + 1;
                        }
                        if mid <= yh {
                            if let Some(tex_idx) = bot_tex {
                                if mid >= 0 && yh < sh {
                                    if let Some(td) = &self.texture_data {
                                        let source = td.get_column(tex_idx, texturecolumn);
                                        if !source.is_empty() {
                                            let colormap = &td.colormaps[colormap_offset..];
                                            self.column_drawer.draw_column(
                                                screen, xu, mid, yh, dc_iscale,
                                                rw_bottomtexturemid, source, colormap,
                                            );
                                        }
                                    }
                                }
                            } else {
                                let wt = mid.max(0);
                                let wb = yh.min(sh - 1);
                                for y in wt..=wb {
                                    screen[y as usize * SCREENWIDTH + xu] = 0x60 + shade;
                                }
                            }
                            floorclip[xu] = mid as i16;
                        } else {
                            floorclip[xu] = (yh + 1) as i16;
                        }
                    } else {
                        floorclip[xu] = (yh + 1) as i16;
                    }

                    // Piso com textura flat
                    let mark_floor = worldlow_full != worldbottom_full || has_lower;
                    if mark_floor {
                        let ft = (yh + 1).max(0);
                        let old_fc = floorclip[xu] as i32;
                        let fb = old_fc.min(sh) - 1;
                        Self::draw_flat_column(
                            screen, xu, ft, fb, floor_flat, floor_color_fb,
                            (floor_h - viewz).abs(), planes, &self.render_state,
                            lightnum, colormaps_ref,
                        );
                    }
                }

                topfrac += topstep;
                bottomfrac += bottomstep;
                cur_scale += rw_scalestep;
                if has_upper {
                    pixhigh += pixhighstep;
                }
                if has_lower {
                    pixlow += pixlowstep;
                }
            }
        }
    }

    /// Renderiza uma coluna vertical de flat (piso ou teto) com textura.
    ///
    /// Para cada pixel vertical na faixa [yt..yb], calcula a coordenada
    /// de textura 64x64 usando projecao perspectiva (R_MapPlane simplificado).
    /// Se o flat nao estiver no cache, usa cor de fallback.
    ///
    /// C original: `R_MapPlane()` em `r_plane.c`, adaptado de spans para colunas
    #[allow(clippy::too_many_arguments)]
    fn draw_flat_column(
        screen: &mut [u8],
        x: usize,
        yt: i32,
        yb: i32,
        flat_data: Option<&Vec<u8>>,
        fallback_color: u8,
        planeheight: i32,
        planes: &PlaneRenderData,
        render_state: &RenderState,
        lightnum: usize,
        colormaps: Option<&[u8]>,
    ) {
        let sh = SCREENHEIGHT as i32;
        if yt > yb || x >= SCREENWIDTH {
            return;
        }

        let flat = match flat_data {
            Some(data) if data.len() >= 4096 => data,
            _ => {
                // Sem flat: preencher com cor de fallback
                for y in yt.max(0)..=yb.min(sh - 1) {
                    screen[y as usize * SCREENWIDTH + x] = fallback_color;
                }
                return;
            }
        };

        // Valores constantes para esta coluna (nao dependem de Y)
        let planeheight_fixed = planeheight.abs() as i64;
        let viewx = render_state.viewx.0;
        let viewy = render_state.viewy.0;
        let distscale_x = planes.distscale[x] as i64;

        // Angulo e trig sao constantes para coluna x
        let angle = render_state.viewangle.0.wrapping_add(render_state.xtoviewangle[x].0)
            >> ANGLETOFINESHIFT;
        let cos_val = fine_cosine(angle as usize & FINEMASK).0 as i64;
        let sin_val = fine_sine(angle as usize & FINEMASK).0 as i64;
        let neg_viewy = -viewy;

        let y_start = yt.max(0);
        let y_end = yb.min(sh - 1);

        for y in y_start..=y_end {
            let yu = y as usize;

            // distance = planeheight * yslope[y] (FixedMul)
            let distance = (planeheight_fixed * planes.yslope[yu] as i64) >> FRACBITS;

            // length = distance * distscale[x] (FixedMul)
            let length = (distance * distscale_x) >> FRACBITS;

            // Posicao no mundo da textura
            let xfrac = viewx.wrapping_add(((cos_val * length) >> FRACBITS) as i32);
            let yfrac = neg_viewy.wrapping_sub(((sin_val * length) >> FRACBITS) as i32);

            // Coordenadas na textura 64x64
            let tx = ((xfrac >> FRACBITS) & 63) as usize;
            let ty = ((yfrac >> FRACBITS) & 63) as usize;
            let src_pixel = flat[ty * 64 + tx];

            // Colormap baseada em distancia (zlight)
            let pixel = if let Some(cm) = colormaps {
                let z_idx = ((distance as usize) >> crate::renderer::state::LIGHTZSHIFT)
                    .min(crate::renderer::state::MAXLIGHTZ - 1);
                let cm_offset = render_state.zlight[lightnum][z_idx];
                let idx = cm_offset + src_pixel as usize;
                if idx < cm.len() { cm[idx] } else { src_pixel }
            } else {
                src_pixel
            };

            screen[yu * SCREENWIDTH + x] = pixel;
        }
    }

    /// Resolve o nome de uma textura de sidedef para indice no TextureData.
    ///
    /// Retorna None se o nome for "-" (sem textura) ou nao encontrado.
    fn resolve_texture_name(name: &[u8; 8], td: &TextureData) -> Option<usize> {
        // "-" ou nome vazio = sem textura
        if name[0] == b'-' || name[0] == 0 {
            return None;
        }
        let end = name.iter().position(|&b| b == 0).unwrap_or(8);
        let name_str = std::str::from_utf8(&name[..end]).unwrap_or("");
        td.texture_num_for_name(name_str)
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


/// Desenha uma linha no framebuffer usando algoritmo de Bresenham.
/// Usado pelo automap.
///
/// Clippa coordenadas contra os limites da tela antes de desenhar.
#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
fn draw_line(
    screen: &mut [u8],
    screen_w: usize,
    screen_h: usize,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: u8,
) {
    let mut x0 = x0;
    let mut y0 = y0;

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx: i32 = if x0 < x1 { 1 } else { -1 };
    let sy: i32 = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        // Desenhar pixel se dentro da tela
        if x0 >= 0 && x0 < screen_w as i32 && y0 >= 0 && y0 < screen_h as i32 {
            screen[y0 as usize * screen_w + x0 as usize] = color;
        }

        if x0 == x1 && y0 == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
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
        // Colocar em Level para que a tecla nao abra o menu
        engine.game.state = GameStateType::Level;
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

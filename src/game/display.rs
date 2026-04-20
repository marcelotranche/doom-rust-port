//! # Coordenador de Drawing (D_Display)
//!
//! Controla o que e desenhado na tela a cada frame, delegando
//! para os subsistemas corretos na ordem correta (Z-order):
//!
//! ```text
//! D_Display()
//!   |
//!   +-> match gamestate:
//!   |     Level       → R_RenderPlayerView()
//!   |     Intermission → WI_Drawer()
//!   |     Finale      → F_Drawer()
//!   |     DemoScreen  → D_PageDrawer()
//!   |
//!   +-> if Level:
//!   |     +-> AM_Drawer()  — automap (se ativo)
//!   |     +-> ST_Drawer()  — status bar
//!   |     +-> HU_Drawer()  — HUD messages
//!   |
//!   +-> M_Drawer()         — menu (se ativo, sobre tudo)
//!   +-> I_FinishUpdate()   — flush framebuffer para tela
//! ```
//!
//! ## Wipe effect
//!
//! Na transicao entre estados (ex: entrando num nivel),
//! o DOOM faz um efeito "melt" (wipe) onde a tela antiga
//! derrete para revelar a nova. Isso e implementado via
//! colunas que descem em velocidades aleatorias.
//!
//! ## Arquivo C original: `d_main.c` (D_Display)
//!
//! ## Conceitos que o leitor vai aprender
//! - Composicao de camadas de UI
//! - Z-ordering de elementos visuais
//! - Maquina de estados para rendering condicional

use super::state::GameStateType;

/// Configuracao de exibicao do frame.
///
/// Agrupa flags que controlam quais elementos sao desenhados.
#[derive(Debug, Clone)]
pub struct DisplayConfig {
    /// Se o automap esta ativo (substitui a vista 3D)
    pub automap_active: bool,
    /// Se o menu esta ativo (desenhado sobre tudo)
    pub menu_active: bool,
    /// Se a vista 3D do jogo esta ativa
    pub view_active: bool,
    /// Se a status bar esta visivel
    pub statusbar_active: bool,
    /// Se o HUD esta ativo
    pub hud_active: bool,
}

impl DisplayConfig {
    /// Configuracao padrao para gameplay.
    pub fn gameplay() -> Self {
        DisplayConfig {
            automap_active: false,
            menu_active: false,
            view_active: true,
            statusbar_active: true,
            hud_active: true,
        }
    }

    /// Configuracao para tela cheia (intermission, finale, demo).
    pub fn fullscreen() -> Self {
        DisplayConfig {
            automap_active: false,
            menu_active: false,
            view_active: false,
            statusbar_active: false,
            hud_active: false,
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self::gameplay()
    }
}

/// Camada de UI a ser desenhada.
///
/// Representa cada elemento visual na ordem correta de Z-order.
/// O caller itera sobre as camadas retornadas por `layers_for_state`
/// e desenha cada uma.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawLayer {
    /// Vista 3D do jogo (R_RenderPlayerView)
    GameView,
    /// Automap (AM_Drawer) — substitui GameView quando ativo
    Automap,
    /// Status bar (ST_Drawer)
    StatusBar,
    /// HUD / mensagens (HU_Drawer)
    Hud,
    /// Tela de intermissao (WI_Drawer) — fullscreen
    Intermission,
    /// Tela de finale (F_Drawer) — fullscreen
    Finale,
    /// Tela de demo/title (D_PageDrawer) — fullscreen
    DemoScreen,
    /// Menu (M_Drawer) — sobre tudo
    Menu,
    /// Wipe transition entre estados
    Wipe,
}

/// Determina as camadas de desenho para o estado atual.
///
/// Retorna um Vec de DrawLayers na ordem correta de Z-order
/// (primeiro e desenhado primeiro = mais ao fundo).
///
/// C original: switch em `D_Display()` em `d_main.c`
pub fn layers_for_state(
    state: GameStateType,
    config: &DisplayConfig,
) -> Vec<DrawLayer> {
    let mut layers = Vec::with_capacity(5);

    match state {
        GameStateType::Level => {
            if config.automap_active {
                layers.push(DrawLayer::Automap);
            } else if config.view_active {
                layers.push(DrawLayer::GameView);
            }
            if config.statusbar_active {
                layers.push(DrawLayer::StatusBar);
            }
            if config.hud_active {
                layers.push(DrawLayer::Hud);
            }
        }
        GameStateType::Intermission => {
            layers.push(DrawLayer::Intermission);
        }
        GameStateType::Finale => {
            layers.push(DrawLayer::Finale);
        }
        GameStateType::DemoScreen => {
            layers.push(DrawLayer::DemoScreen);
        }
    }

    // Menu e sempre desenhado por cima se ativo
    if config.menu_active {
        layers.push(DrawLayer::Menu);
    }

    layers
}

/// Estado do efeito wipe (melt transition).
///
/// C original: `wipe_*` em `f_wipe.c`
#[derive(Debug, Clone)]
pub struct WipeState {
    /// Se um wipe esta em andamento
    pub active: bool,
    /// Offset Y de cada coluna (320 colunas)
    pub y_offsets: Vec<i32>,
    /// Tick atual do wipe
    pub tick: i32,
}

impl WipeState {
    /// Cria um estado de wipe inativo.
    pub fn new() -> Self {
        WipeState {
            active: false,
            y_offsets: Vec::new(),
            tick: 0,
        }
    }

    /// Inicia um novo wipe.
    ///
    /// Gera offsets aleatorios para as 320 colunas, criando
    /// o efeito de "derretimento" da tela.
    ///
    /// C original: `wipe_initMelt()` em `f_wipe.c`
    pub fn start(&mut self, screen_width: usize) {
        self.active = true;
        self.tick = 0;
        self.y_offsets = Vec::with_capacity(screen_width);

        // Gerar offsets iniciais (aleatorios entre -15 e 0)
        // Usa um PRNG simples baseado no DOOM original
        let mut rng = 1u32;
        for _ in 0..screen_width {
            rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
            let offset = -(((rng >> 16) % 16) as i32);
            self.y_offsets.push(offset);
        }
    }

    /// Avanca o wipe por um tick.
    ///
    /// Cada coluna desce a uma velocidade proporcional ao seu offset.
    /// Retorna true se o wipe esta completo (todas as colunas chegaram ao fundo).
    ///
    /// C original: `wipe_doMelt()` em `f_wipe.c`
    pub fn update(&mut self, screen_height: i32) -> bool {
        if !self.active {
            return true;
        }

        self.tick += 1;
        let mut done = true;

        for y in &mut self.y_offsets {
            if *y < 0 {
                *y += 1;
                done = false;
            } else if *y < screen_height {
                let dy = if *y < 16 { (*y + 1).max(1) } else { 8 };
                *y += dy;
                if *y >= screen_height {
                    *y = screen_height;
                }
                done = false;
            }
        }

        if done {
            self.active = false;
        }
        done
    }

    /// Verifica se o wipe esta em andamento.
    pub fn is_active(&self) -> bool {
        self.active
    }
}

impl Default for WipeState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layers_level_gameplay() {
        let config = DisplayConfig::gameplay();
        let layers = layers_for_state(GameStateType::Level, &config);
        assert_eq!(layers, vec![DrawLayer::GameView, DrawLayer::StatusBar, DrawLayer::Hud]);
    }

    #[test]
    fn layers_level_automap() {
        let mut config = DisplayConfig::gameplay();
        config.automap_active = true;
        let layers = layers_for_state(GameStateType::Level, &config);
        assert_eq!(layers[0], DrawLayer::Automap);
        assert!(!layers.contains(&DrawLayer::GameView));
    }

    #[test]
    fn layers_level_with_menu() {
        let mut config = DisplayConfig::gameplay();
        config.menu_active = true;
        let layers = layers_for_state(GameStateType::Level, &config);
        assert_eq!(*layers.last().unwrap(), DrawLayer::Menu);
    }

    #[test]
    fn layers_intermission() {
        let config = DisplayConfig::fullscreen();
        let layers = layers_for_state(GameStateType::Intermission, &config);
        assert_eq!(layers, vec![DrawLayer::Intermission]);
    }

    #[test]
    fn layers_finale() {
        let config = DisplayConfig::fullscreen();
        let layers = layers_for_state(GameStateType::Finale, &config);
        assert_eq!(layers, vec![DrawLayer::Finale]);
    }

    #[test]
    fn layers_demo_screen() {
        let config = DisplayConfig::fullscreen();
        let layers = layers_for_state(GameStateType::DemoScreen, &config);
        assert_eq!(layers, vec![DrawLayer::DemoScreen]);
    }

    #[test]
    fn wipe_lifecycle() {
        let mut wipe = WipeState::new();
        assert!(!wipe.is_active());

        wipe.start(320);
        assert!(wipe.is_active());
        assert_eq!(wipe.y_offsets.len(), 320);

        // Todos os offsets iniciais devem ser entre -15 e 0
        for &y in &wipe.y_offsets {
            assert!(y >= -15 && y <= 0, "offset fora do range: {}", y);
        }
    }

    #[test]
    fn wipe_update_completes() {
        let mut wipe = WipeState::new();
        wipe.start(10); // tela pequena para teste rapido

        let screen_height = 200;
        let mut done = false;
        for _ in 0..300 {
            done = wipe.update(screen_height);
            if done {
                break;
            }
        }
        assert!(done, "wipe deveria completar em 300 ticks");
        assert!(!wipe.is_active());
    }

    #[test]
    fn wipe_all_columns_reach_bottom() {
        let mut wipe = WipeState::new();
        wipe.start(5);

        let screen_height = 50;
        for _ in 0..200 {
            if wipe.update(screen_height) {
                break;
            }
        }

        for &y in &wipe.y_offsets {
            assert_eq!(y, screen_height);
        }
    }

    #[test]
    fn display_config_defaults() {
        let config = DisplayConfig::gameplay();
        assert!(config.view_active);
        assert!(config.statusbar_active);
        assert!(config.hud_active);
        assert!(!config.automap_active);
        assert!(!config.menu_active);
    }

    #[test]
    fn display_config_fullscreen() {
        let config = DisplayConfig::fullscreen();
        assert!(!config.view_active);
        assert!(!config.statusbar_active);
        assert!(!config.hud_active);
    }
}

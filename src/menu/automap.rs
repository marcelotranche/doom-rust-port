//! # Automap — Mapa Automatico do Nivel
//!
//! O automap e um overlay que mostra a planta baixa do nivel,
//! desenhado usando as linedefs do mapa com cores codificadas:
//!
//! ```text
//! Cor        | Significado
//! -----------+----------------------------------
//! Vermelho   | Parede solida (one-sided linedef)
//! Amarelo    | Diferenca de teto (ceiling change)
//! Marrom     | Diferenca de piso (floor change)
//! Cinza      | Passagem sem diferenca
//! Verde      | Things (monstros, itens) — cheating
//! Branco     | Jogador
//! ```
//!
//! ## Coordenadas e zoom
//!
//! O automap trabalha com dois sistemas de coordenadas:
//! - Map coords (fixed-point, unidades do mapa)
//! - Screen coords (pixels inteiros, framebuffer)
//!
//! As macros MTOF (Map-To-Framebuffer) e FTOM (Framebuffer-To-Map)
//! convertem entre os dois sistemas usando a escala de zoom atual.
//!
//! ## Line clipping
//!
//! Linhas fora da viewport sao clipadas usando o algoritmo
//! Cohen-Sutherland antes de serem rasterizadas com Bresenham.
//!
//! ## Arquivo C original: `am_map.c`, `am_map.h`
//!
//! ## Conceitos que o leitor vai aprender
//! - Transformacao de coordenadas (map ↔ screen)
//! - Cohen-Sutherland line clipping
//! - Controle de zoom e pan com input
//! - Cores codificadas para tipos de geometria

use crate::utils::fixed::Fixed;

// ---------------------------------------------------------------------------
// Cores do automap (indices na paleta)
// ---------------------------------------------------------------------------

/// Cor de paredes solidas (one-sided linedefs).
///
/// C original: `#define REDS (256-5*16)` = 176
pub const AM_COLOR_WALL: u8 = 176;

/// Cor de mudanca de piso (floor height difference).
///
/// C original: `#define BROWNS 64`
pub const AM_COLOR_FDIFF: u8 = 64;

/// Cor de mudanca de teto (ceiling height difference).
///
/// C original: `#define YELLOWS 231`
pub const AM_COLOR_CDIFF: u8 = 231;

/// Cor de passagens sem diferenca (same floor/ceiling).
///
/// C original: `#define GRAYS (256-32+5)` = 96 (aprox)
pub const AM_COLOR_FLAT: u8 = 96;

/// Cor de things (monstros, itens) — so com cheat.
///
/// C original: `#define GREENS (7*16)` = 112
pub const AM_COLOR_THING: u8 = 112;

/// Cor do jogador.
///
/// C original: `#define WHITE (256-47)` = 209
pub const AM_COLOR_PLAYER: u8 = 209;

/// Cor de secret walls (mostradas como paredes normais sem cheat).
///
/// C original: usa REDS para secret walls
pub const AM_COLOR_SECRET: u8 = 252;

/// Cor da grid.
///
/// C original: `#define GRIDCOLORS (GRAYS + GRAYSRANGE/2)` ≈ 104
pub const AM_COLOR_GRID: u8 = 104;

/// Cor da mira (crosshair).
pub const AM_COLOR_XHAIR: u8 = 209;

// ---------------------------------------------------------------------------
// Constantes de controle
// ---------------------------------------------------------------------------

/// Velocidade de pan (unidades de mapa por tick).
///
/// C original: `#define F_PANINC 4` (scaled by MAPBLOCKUNITS)
pub const AM_PAN_INC: i32 = 4 * 128;

/// Fator de zoom in (multiplicador, 16.16 fixed).
///
/// C original: `#define M_ZOOMIN ((int)(1.02*FRACUNIT))`
pub const AM_ZOOM_IN: i32 = (1.02 * 65536.0) as i32;

/// Fator de zoom out.
///
/// C original: `#define M_ZOOMOUT ((int)(FRACUNIT/1.02))`
pub const AM_ZOOM_OUT: i32 = (65536.0 / 1.02) as i32;

/// Tamanho da grid em unidades de mapa.
///
/// C original: `#define MAPBLOCKUNITS 128`
pub const AM_GRID_SIZE: i32 = 128;

/// Numero maximo de marks (waypoints).
///
/// C original: `#define AM_NUMMARKPOINTS 10`
pub const AM_NUMMARKPOINTS: usize = 10;

// ---------------------------------------------------------------------------
// Ponto e linha no mapa
// ---------------------------------------------------------------------------

/// Ponto em coordenadas do mapa (fixed-point).
///
/// C original: `mpoint_t` em `am_map.c`
#[derive(Debug, Clone, Copy)]
pub struct MapPoint {
    /// Coordenada X (fixed-point)
    pub x: Fixed,
    /// Coordenada Y (fixed-point)
    pub y: Fixed,
}

impl MapPoint {
    /// Cria um ponto no mapa.
    pub fn new(x: Fixed, y: Fixed) -> Self {
        MapPoint { x, y }
    }
}

/// Ponto em coordenadas da tela (inteiro).
///
/// C original: `fpoint_t` em `am_map.c`
#[derive(Debug, Clone, Copy)]
pub struct ScreenPoint {
    /// Coordenada X em pixels
    pub x: i32,
    /// Coordenada Y em pixels
    pub y: i32,
}

/// Linha no mapa (dois pontos).
///
/// C original: `mline_t` em `am_map.c`
#[derive(Debug, Clone, Copy)]
pub struct MapLine {
    /// Ponto inicial
    pub a: MapPoint,
    /// Ponto final
    pub b: MapPoint,
}

/// Linha na tela (dois pontos inteiros).
///
/// C original: `fline_t` em `am_map.c`
#[derive(Debug, Clone, Copy)]
pub struct ScreenLine {
    /// Ponto inicial
    pub ax: i32,
    pub ay: i32,
    /// Ponto final
    pub bx: i32,
    pub by: i32,
}

// ---------------------------------------------------------------------------
// Cohen-Sutherland clipping
// ---------------------------------------------------------------------------

/// Outcodes para Cohen-Sutherland clipping.
const OUTCODE_TOP: u8 = 1;
const OUTCODE_BOTTOM: u8 = 2;
const OUTCODE_RIGHT: u8 = 4;
const OUTCODE_LEFT: u8 = 8;

/// Calcula o outcode de um ponto em relacao a um retangulo.
fn compute_outcode(x: i32, y: i32, xmin: i32, ymin: i32, xmax: i32, ymax: i32) -> u8 {
    let mut code = 0u8;
    if y > ymax {
        code |= OUTCODE_TOP;
    }
    if y < ymin {
        code |= OUTCODE_BOTTOM;
    }
    if x > xmax {
        code |= OUTCODE_RIGHT;
    }
    if x < xmin {
        code |= OUTCODE_LEFT;
    }
    code
}

/// Clipa uma linha contra um retangulo usando Cohen-Sutherland.
///
/// Retorna `Some((ax, ay, bx, by))` com os pontos clipados,
/// ou `None` se a linha esta totalmente fora.
///
/// C original: `AM_clipMline()` em `am_map.c`
#[allow(clippy::too_many_arguments)]
pub fn clip_line(
    mut ax: i32,
    mut ay: i32,
    mut bx: i32,
    mut by: i32,
    xmin: i32,
    ymin: i32,
    xmax: i32,
    ymax: i32,
) -> Option<(i32, i32, i32, i32)> {
    let mut outcode_a = compute_outcode(ax, ay, xmin, ymin, xmax, ymax);
    let mut outcode_b = compute_outcode(bx, by, xmin, ymin, xmax, ymax);

    loop {
        // Trivially accept
        if (outcode_a | outcode_b) == 0 {
            return Some((ax, ay, bx, by));
        }

        // Trivially reject
        if (outcode_a & outcode_b) != 0 {
            return None;
        }

        // Pick the point outside the rectangle
        let outcode_out = if outcode_a != 0 { outcode_a } else { outcode_b };

        let (cx, cy);

        if outcode_out & OUTCODE_TOP != 0 {
            cx = ax + (bx - ax) * (ymax - ay) / (by - ay);
            cy = ymax;
        } else if outcode_out & OUTCODE_BOTTOM != 0 {
            cx = ax + (bx - ax) * (ymin - ay) / (by - ay);
            cy = ymin;
        } else if outcode_out & OUTCODE_RIGHT != 0 {
            cy = ay + (by - ay) * (xmax - ax) / (bx - ax);
            cx = xmax;
        } else {
            cy = ay + (by - ay) * (xmin - ax) / (bx - ax);
            cx = xmin;
        }

        if outcode_out == outcode_a {
            ax = cx;
            ay = cy;
            outcode_a = compute_outcode(ax, ay, xmin, ymin, xmax, ymax);
        } else {
            bx = cx;
            by = cy;
            outcode_b = compute_outcode(bx, by, xmin, ymin, xmax, ymax);
        }
    }
}

// ---------------------------------------------------------------------------
// Automap
// ---------------------------------------------------------------------------

/// Automap — exibe a planta baixa do nivel.
///
/// C original: globals em `am_map.c`
/// (`automapactive`, `scale_mtof`, `m_x`, `m_y`, etc.)
#[derive(Debug)]
pub struct Automap {
    /// Se o automap esta ativo (visivel)
    pub active: bool,

    /// Viewport do mapa (em coordenadas de mapa)
    pub map_x: Fixed,
    pub map_y: Fixed,
    pub map_w: Fixed,
    pub map_h: Fixed,

    /// Viewport do framebuffer (em pixels)
    pub fb_x: i32,
    pub fb_y: i32,
    pub fb_w: i32,
    pub fb_h: i32,

    /// Escala map-to-framebuffer (16.16 fixed-point)
    pub scale_mtof: i32,
    /// Escala framebuffer-to-map (inversa)
    pub scale_ftom: i32,

    /// Velocidade de pan atual
    pub pan_x: i32,
    pub pan_y: i32,

    /// Se o mapa segue o jogador automaticamente
    pub follow_player: bool,
    /// Se a grid esta visivel
    pub show_grid: bool,
    /// Nivel de cheat (0=normal, 1=all lines, 2=all lines+things)
    pub cheat_level: u8,

    /// Marks (waypoints) colocados pelo jogador
    pub marks: Vec<Option<MapPoint>>,
    /// Indice do proximo mark
    pub mark_index: usize,

    /// Limites do mapa (calculados das linedefs)
    pub min_x: Fixed,
    pub min_y: Fixed,
    pub max_x: Fixed,
    pub max_y: Fixed,
}

impl Automap {
    /// Cria um novo automap.
    pub fn new(fb_w: i32, fb_h: i32) -> Self {
        Automap {
            active: false,
            map_x: Fixed::ZERO,
            map_y: Fixed::ZERO,
            map_w: Fixed::from_int(fb_w),
            map_h: Fixed::from_int(fb_h),
            fb_x: 0,
            fb_y: 0,
            fb_w,
            fb_h,
            scale_mtof: 0x10000, // 1:1 inicialmente
            scale_ftom: 0x10000,
            pan_x: 0,
            pan_y: 0,
            follow_player: true,
            show_grid: false,
            cheat_level: 0,
            marks: vec![None; AM_NUMMARKPOINTS],
            mark_index: 0,
            min_x: Fixed::ZERO,
            min_y: Fixed::ZERO,
            max_x: Fixed::ZERO,
            max_y: Fixed::ZERO,
        }
    }

    /// Ativa o automap.
    ///
    /// C original: `AM_Start()` em `am_map.c`
    pub fn start(&mut self) {
        self.active = true;
        // Escala para caber o mapa inteiro na tela
        self.fit_to_screen();
    }

    /// Desativa o automap.
    ///
    /// C original: `AM_Stop()` em `am_map.c`
    pub fn stop(&mut self) {
        self.active = false;
        self.pan_x = 0;
        self.pan_y = 0;
    }

    /// Define os limites do mapa (chamado ao carregar nivel).
    pub fn set_bounds(&mut self, min_x: Fixed, min_y: Fixed, max_x: Fixed, max_y: Fixed) {
        self.min_x = min_x;
        self.min_y = min_y;
        self.max_x = max_x;
        self.max_y = max_y;
    }

    /// Ajusta a escala para caber o mapa inteiro na tela.
    fn fit_to_screen(&mut self) {
        let map_range_x = (self.max_x - self.min_x).0;
        let map_range_y = (self.max_y - self.min_y).0;

        if map_range_x <= 0 || map_range_y <= 0 {
            return;
        }

        // Calcular escala que cabe o mapa na viewport
        let scale_x = ((self.fb_w as i64) << 16) / map_range_x as i64;
        let scale_y = ((self.fb_h as i64) << 16) / map_range_y as i64;

        // Usar a menor escala (para caber ambas as dimensoes)
        self.scale_mtof = scale_x.min(scale_y) as i32;
        if self.scale_mtof != 0 {
            self.scale_ftom = (0x10000i64 * 0x10000 / self.scale_mtof as i64) as i32;
        }

        // Centralizar
        self.map_x = self.min_x;
        self.map_y = self.min_y;
    }

    /// Converte coordenada X do mapa para tela.
    ///
    /// C original: `CXMTOF(x)` macro em `am_map.c`
    pub fn map_to_screen_x(&self, x: Fixed) -> i32 {
        let dx = (x - self.map_x).0 as i64;
        self.fb_x + ((dx * self.scale_mtof as i64) >> 32) as i32
    }

    /// Converte coordenada Y do mapa para tela.
    ///
    /// C original: `CYMTOF(y)` macro em `am_map.c`
    /// Nota: Y e invertido (mapa Y cresce para cima, tela para baixo)
    pub fn map_to_screen_y(&self, y: Fixed) -> i32 {
        let dy = (y - self.map_y).0 as i64;
        self.fb_y + self.fb_h - ((dy * self.scale_mtof as i64) >> 32) as i32
    }

    /// Atualiza o automap a cada tick.
    ///
    /// Aplica pan e segue o jogador se ativado.
    ///
    /// C original: `AM_Ticker()` em `am_map.c`
    pub fn ticker(&mut self, player_x: Fixed, player_y: Fixed) {
        if !self.active {
            return;
        }

        // Seguir jogador
        if self.follow_player {
            let half_w = Fixed((((self.map_w.0 >> 1) as i64 * self.scale_ftom as i64) >> 16) as i32);
            let half_h = Fixed((((self.map_h.0 >> 1) as i64 * self.scale_ftom as i64) >> 16) as i32);
            self.map_x = player_x - half_w;
            self.map_y = player_y - half_h;
        } else {
            // Aplicar pan
            self.map_x += Fixed(self.pan_x);
            self.map_y += Fixed(self.pan_y);
        }
    }

    /// Processa input do automap.
    ///
    /// Retorna `true` se o evento foi consumido.
    ///
    /// C original: `AM_Responder()` em `am_map.c`
    pub fn responder(&mut self, key: u8, key_down: bool) -> bool {
        if !self.active {
            // TAB abre o automap
            if key_down && key == 9 {
                self.start();
                return true;
            }
            return false;
        }

        if !key_down {
            // Soltar tecla de pan
            match key {
                0xac => self.pan_x = 0,     // seta direita
                0xab => self.pan_x = 0,     // seta esquerda
                0xad => self.pan_y = 0,     // seta cima
                0xae => self.pan_y = 0,     // seta baixo
                _ => {}
            }
            return false;
        }

        match key {
            // TAB fecha o automap
            9 => {
                self.stop();
                true
            }

            // Seta direita — pan direita
            0xac => {
                if !self.follow_player {
                    self.pan_x = AM_PAN_INC;
                }
                true
            }

            // Seta esquerda — pan esquerda
            0xab => {
                if !self.follow_player {
                    self.pan_x = -AM_PAN_INC;
                }
                true
            }

            // Seta cima — pan cima
            0xad => {
                if !self.follow_player {
                    self.pan_y = AM_PAN_INC;
                }
                true
            }

            // Seta baixo — pan baixo
            0xae => {
                if !self.follow_player {
                    self.pan_y = -AM_PAN_INC;
                }
                true
            }

            // '+' / '=' — zoom in
            b'=' | b'+' => {
                self.zoom_in();
                true
            }

            // '-' — zoom out
            b'-' => {
                self.zoom_out();
                true
            }

            // '0' — fit to screen
            b'0' => {
                self.fit_to_screen();
                true
            }

            // 'f' — toggle follow player
            b'f' => {
                self.follow_player = !self.follow_player;
                true
            }

            // 'g' — toggle grid
            b'g' => {
                self.show_grid = !self.show_grid;
                true
            }

            // 'm' — add mark
            b'm' => {
                self.add_mark(self.map_x + Fixed(self.map_w.0 / 2), self.map_y + Fixed(self.map_h.0 / 2));
                true
            }

            // 'c' — clear marks
            b'c' => {
                self.clear_marks();
                true
            }

            _ => false,
        }
    }

    /// Aplica zoom in.
    pub fn zoom_in(&mut self) {
        self.scale_mtof = ((self.scale_mtof as i64 * AM_ZOOM_IN as i64) >> 16) as i32;
        if self.scale_mtof != 0 {
            self.scale_ftom = (0x10000i64 * 0x10000 / self.scale_mtof as i64) as i32;
        }
    }

    /// Aplica zoom out.
    pub fn zoom_out(&mut self) {
        self.scale_mtof = ((self.scale_mtof as i64 * AM_ZOOM_OUT as i64) >> 16) as i32;
        if self.scale_mtof != 0 {
            self.scale_ftom = (0x10000i64 * 0x10000 / self.scale_mtof as i64) as i32;
        }
    }

    /// Adiciona um mark na posicao atual.
    pub fn add_mark(&mut self, x: Fixed, y: Fixed) {
        self.marks[self.mark_index] = Some(MapPoint::new(x, y));
        self.mark_index = (self.mark_index + 1) % AM_NUMMARKPOINTS;
    }

    /// Limpa todos os marks.
    pub fn clear_marks(&mut self) {
        for mark in &mut self.marks {
            *mark = None;
        }
        self.mark_index = 0;
    }

    /// Retorna o numero de marks ativos.
    pub fn mark_count(&self) -> usize {
        self.marks.iter().filter(|m| m.is_some()).count()
    }
}

impl Default for Automap {
    fn default() -> Self {
        Self::new(320, 200)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automap_init() {
        let am = Automap::new(320, 200);
        assert!(!am.active);
        assert!(am.follow_player);
        assert!(!am.show_grid);
        assert_eq!(am.cheat_level, 0);
    }

    #[test]
    fn automap_start_stop() {
        let mut am = Automap::new(320, 200);
        am.set_bounds(
            Fixed::from_int(0),
            Fixed::from_int(0),
            Fixed::from_int(4096),
            Fixed::from_int(4096),
        );
        am.start();
        assert!(am.active);

        am.stop();
        assert!(!am.active);
    }

    #[test]
    fn automap_toggle_tab() {
        let mut am = Automap::new(320, 200);

        // TAB abre
        assert!(am.responder(9, true));
        assert!(am.active);

        // TAB fecha
        assert!(am.responder(9, true));
        assert!(!am.active);
    }

    #[test]
    fn automap_toggle_grid() {
        let mut am = Automap::new(320, 200);
        am.start();
        assert!(!am.show_grid);

        am.responder(b'g', true);
        assert!(am.show_grid);

        am.responder(b'g', true);
        assert!(!am.show_grid);
    }

    #[test]
    fn automap_toggle_follow() {
        let mut am = Automap::new(320, 200);
        am.start();
        assert!(am.follow_player);

        am.responder(b'f', true);
        assert!(!am.follow_player);
    }

    #[test]
    fn automap_marks() {
        let mut am = Automap::new(320, 200);
        am.start();

        am.add_mark(Fixed::from_int(100), Fixed::from_int(200));
        assert_eq!(am.mark_count(), 1);

        am.add_mark(Fixed::from_int(300), Fixed::from_int(400));
        assert_eq!(am.mark_count(), 2);

        am.clear_marks();
        assert_eq!(am.mark_count(), 0);
    }

    #[test]
    fn automap_zoom() {
        let mut am = Automap::new(320, 200);
        am.start();
        let original_scale = am.scale_mtof;

        am.zoom_in();
        assert!(am.scale_mtof > original_scale);

        am.zoom_out();
        am.zoom_out();
        assert!(am.scale_mtof < original_scale);
    }

    #[test]
    fn automap_follow_player() {
        let mut am = Automap::new(320, 200);
        am.start();

        am.ticker(Fixed::from_int(500), Fixed::from_int(300));
        // O mapa deve ter se movido para centrar no jogador
        // (exato depende da escala, mas map_x deve ter mudado)
    }

    #[test]
    fn cohen_sutherland_inside() {
        // Linha totalmente dentro
        let result = clip_line(10, 10, 90, 90, 0, 0, 100, 100);
        assert_eq!(result, Some((10, 10, 90, 90)));
    }

    #[test]
    fn cohen_sutherland_outside() {
        // Linha totalmente fora (a direita)
        let result = clip_line(200, 10, 300, 90, 0, 0, 100, 100);
        assert_eq!(result, None);
    }

    #[test]
    fn cohen_sutherland_partial() {
        // Linha parcialmente dentro (diagonal atravessando o retangulo)
        let result = clip_line(-50, 50, 150, 50, 0, 0, 100, 100);
        assert!(result.is_some());
        let (ax, ay, bx, by) = result.unwrap();
        assert_eq!(ax, 0);
        assert_eq!(ay, 50);
        assert_eq!(bx, 100);
        assert_eq!(by, 50);
    }

    #[test]
    fn automap_map_to_screen() {
        let mut am = Automap::new(320, 200);
        am.map_x = Fixed::ZERO;
        am.map_y = Fixed::ZERO;
        am.scale_mtof = 0x10000; // 1:1

        let sx = am.map_to_screen_x(Fixed::from_int(100));
        assert_eq!(sx, 100);

        // Y e invertido
        let sy = am.map_to_screen_y(Fixed::from_int(0));
        assert_eq!(sy, 200); // bottom
    }

    #[test]
    fn automap_colors() {
        assert_eq!(AM_COLOR_WALL, 176);
        assert_eq!(AM_COLOR_THING, 112);
        assert_eq!(AM_COLOR_PLAYER, 209);
    }
}

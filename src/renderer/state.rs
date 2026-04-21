//! # Estado do Renderer e Funcoes Utilitarias
//!
//! Gerencia o estado global do renderer: posicao da camera, angulo de visao,
//! tabelas de projecao, tabelas de iluminacao, e funcoes utilitarias de
//! geometria usadas por todo o pipeline de rendering.
//!
//! ## Ponto de vista (POV)
//!
//! O DOOM renderiza a cena a partir de uma camera definida por:
//! - Posicao: viewx, viewy, viewz (fixed-point)
//! - Direcao: viewangle (BAM)
//! - Seno/cosseno pre-calculados: viewcos, viewsin
//!
//! ## Projecao perspectiva
//!
//! A projecao converte coordenadas 3D do mundo para 2D na tela:
//! - `centerxfrac`, `centeryfrac`: centro da tela em fixed-point
//! - `projection`: distancia focal (proporcional a largura da tela)
//! - `viewangletox[]`: mapeia angulo de visao -> coluna X na tela
//! - `xtoviewangle[]`: mapeia coluna X -> angulo de visao
//!
//! ## Iluminacao
//!
//! O DOOM simula iluminacao por distancia usando tabelas pre-calculadas:
//! - `scalelight[level][scale]`: para paredes (baseado na escala do seg)
//! - `zlight[level][depth]`: para pisos/tetos (baseado na profundidade)
//!
//! ## Arquivo C original: `r_main.c` / `r_state.h`
//!
//! ## Conceitos que o leitor vai aprender
//! - Projecao perspectiva com inteiros e lookup tables
//! - Sistema de iluminacao por distancia do DOOM
//! - Funcoes geometricas com Binary Angle Measurement

use crate::map::types::Node;
use crate::utils::angle::{Angle, ANGLETOFINESHIFT, FINEANGLES, FINEMASK};
use crate::utils::fixed::{Fixed, FRACBITS, FRACUNIT};
use crate::utils::tables::{fine_cosine, fine_sine, slope_div, TAN_TO_ANGLE};
use crate::video::SCREENWIDTH;

/// Field of view em fine angles: 2048 = 90 graus.
///
/// C original: `#define FIELDOFVIEW 2048` em `r_main.c`
pub const FIELDOFVIEW: usize = 2048;

/// Numero de niveis de luz do sector.
///
/// C original: `#define LIGHTLEVELS 16` em `r_local.h`
pub const LIGHTLEVELS: usize = 16;

/// Numero de niveis de escala para tabela de luz de paredes.
///
/// C original: `#define MAXLIGHTSCALE 48` em `r_local.h`
pub const MAXLIGHTSCALE: usize = 48;

/// Numero de niveis de profundidade para tabela de luz de pisos.
///
/// C original: `#define MAXLIGHTZ 128` em `r_local.h`
pub const MAXLIGHTZ: usize = 128;

/// Shift de nivel de luz.
///
/// C original: `#define LIGHTSEGSHIFT 4` em `r_local.h`
pub const LIGHTSEGSHIFT: usize = 4;

/// Shift de profundidade de luz.
///
/// C original: `#define LIGHTZSHIFT 20` em `r_local.h`
pub const LIGHTZSHIFT: usize = 20;

/// Estado do renderer — ponto de vista, projecao e iluminacao.
///
/// Encapsula todo o estado global que no C original eram dezenas
/// de variaveis globais em `r_main.c` e `r_state.h`.
#[derive(Debug)]
pub struct RenderState {
    // -- Ponto de vista (POV) --

    /// Posicao X da camera no mundo.
    /// C original: `fixed_t viewx` em `r_main.c`
    pub viewx: Fixed,
    /// Posicao Y da camera no mundo.
    pub viewy: Fixed,
    /// Posicao Z da camera (altura dos olhos).
    pub viewz: Fixed,
    /// Angulo de visao (direcao que a camera aponta).
    /// C original: `angle_t viewangle` em `r_main.c`
    pub viewangle: Angle,
    /// Cosseno do angulo de visao (pre-calculado).
    pub viewcos: Fixed,
    /// Seno do angulo de visao (pre-calculado).
    pub viewsin: Fixed,

    // -- Projecao --

    /// Centro X da tela em pixels.
    pub centerx: i32,
    /// Centro Y da tela em pixels.
    pub centery: i32,
    /// Centro X em fixed-point (centerx << FRACBITS).
    pub centerxfrac: Fixed,
    /// Centro Y em fixed-point (centery << FRACBITS).
    pub centeryfrac: Fixed,
    /// Distancia focal de projecao.
    /// C original: `fixed_t projection` em `r_main.c`
    pub projection: Fixed,

    /// Mapeia angulo de visao para coluna X na tela.
    /// C original: `int viewangletox[FINEANGLES/2]` em `r_main.c`
    pub viewangletox: Vec<i32>,

    /// Mapeia coluna X para angulo de visao.
    /// C original: `angle_t xtoviewangle[SCREENWIDTH+1]` em `r_main.c`
    pub xtoviewangle: Vec<Angle>,

    /// Angulo de clipping (metade do FOV).
    /// C original: `angle_t clipangle` em `r_main.c`
    pub clipangle: Angle,

    // -- Iluminacao --

    /// Tabela de luz para paredes: [nivel_luz][escala] -> indice de colormap.
    /// C original: `lighttable_t* scalelight[LIGHTLEVELS][MAXLIGHTSCALE]`
    pub scalelight: Vec<Vec<usize>>,

    /// Tabela de luz para pisos: [nivel_luz][profundidade] -> indice de colormap.
    /// C original: `lighttable_t* zlight[LIGHTLEVELS][MAXLIGHTZ]`
    pub zlight: Vec<Vec<usize>>,

    /// Colormap fixa (ex: invulnerability, light goggles).
    /// None = usar iluminacao normal.
    /// C original: `lighttable_t* fixedcolormap`
    pub fixed_colormap: Option<usize>,

    /// Luz extra do flash de arma.
    /// C original: `int extralight`
    pub extralight: i32,

    // -- Contadores --

    /// Numero do frame atual (para profiling).
    pub frame_count: u32,
    /// Contador de validacao (para evitar visitar nodes duplicados).
    /// C original: `int validcount`
    pub valid_count: u32,
}

impl RenderState {
    /// Cria um novo estado do renderer com valores padrao.
    pub fn new() -> Self {
        let mut state = RenderState {
            viewx: Fixed::ZERO,
            viewy: Fixed::ZERO,
            viewz: Fixed::ZERO,
            viewangle: Angle::ANG0,
            viewcos: Fixed::UNIT,
            viewsin: Fixed::ZERO,

            centerx: (SCREENWIDTH / 2) as i32,
            centery: 84, // VIEWHEIGHT / 2 (168 / 2, nao SCREENHEIGHT / 2)
            centerxfrac: Fixed((SCREENWIDTH as i32 / 2) << FRACBITS),
            centeryfrac: Fixed(84 << FRACBITS),
            projection: Fixed((SCREENWIDTH as i32 / 2) << FRACBITS),

            viewangletox: vec![0i32; FINEANGLES / 2],
            xtoviewangle: vec![Angle::ANG0; SCREENWIDTH + 1],
            clipangle: Angle::ANG0,

            scalelight: vec![vec![0usize; MAXLIGHTSCALE]; LIGHTLEVELS],
            zlight: vec![vec![0usize; MAXLIGHTZ]; LIGHTLEVELS],
            fixed_colormap: None,
            extralight: 0,

            frame_count: 0,
            valid_count: 1,
        };

        state.init_tables();
        state.init_light_tables();

        state
    }

    /// Inicializa as tabelas de projecao (viewangletox, xtoviewangle).
    ///
    /// C original: `R_InitTextureMapping()` em `r_main.c`
    fn init_tables(&mut self) {
        // focallength = FixedDiv(centerxfrac, finetangent[FINEANGLES/4 + FIELDOFVIEW/2])
        // Indice: 2048 + 1024 = 3072 para FOV de 90 graus
        let fov_tangent = fine_tangent(FINEANGLES / 4 + FIELDOFVIEW / 2);
        let focallength = self.centerxfrac / fov_tangent;

        // Mapear angulos para colunas X
        // t = FixedMul(finetangent[i], focallength)
        // viewangletox[i] = (centerxfrac - t + FRACUNIT - 1) >> FRACBITS
        for i in 0..FINEANGLES / 2 {
            let tangent = fine_tangent(i);
            let t = if tangent.0 > FRACUNIT * 2 {
                -1
            } else if tangent.0 < -FRACUNIT * 2 {
                (SCREENWIDTH + 1) as i32
            } else {
                // FixedMul(tangent, focallength)
                let t = (tangent * focallength).0;
                let x = (self.centerxfrac.0 - t + FRACUNIT - 1) >> FRACBITS;
                x.clamp(-1, (SCREENWIDTH + 1) as i32)
            };
            self.viewangletox[i] = t;
        }

        // Construir xtoviewangle (inverso):
        // Para cada coluna X, encontrar o menor angulo que mapeia para X
        for x in 0..=SCREENWIDTH {
            let mut i = 0;
            while i < FINEANGLES / 2 && self.viewangletox[i] > x as i32 {
                i += 1;
            }
            let angle_raw = ((i as u32) << ANGLETOFINESHIFT).wrapping_sub(Angle::ANG90.0);
            self.xtoviewangle[x] = Angle(angle_raw);
        }

        // Fencepost correction: remover sentinelas de viewangletox
        // C original: loop final de R_InitTextureMapping
        for i in 0..FINEANGLES / 2 {
            if self.viewangletox[i] == -1 {
                self.viewangletox[i] = 0;
            } else if self.viewangletox[i] == (SCREENWIDTH + 1) as i32 {
                self.viewangletox[i] = SCREENWIDTH as i32;
            }
        }

        // Clip angle
        self.clipangle = self.xtoviewangle[0];
    }

    /// Inicializa as tabelas de iluminacao.
    ///
    /// Pre-calcula indices de colormap para cada combinacao de
    /// nivel de luz do sector e distancia (escala/profundidade).
    ///
    /// C original: `R_InitLightTables()` em `r_main.c`
    fn init_light_tables(&mut self) {
        // DISTMAP controla o falloff de iluminacao com distancia
        // C original: `#define DISTMAP 2` em `r_main.c`
        const DISTMAP: usize = 2;

        for i in 0..LIGHTLEVELS {
            let startmap = ((LIGHTLEVELS - 1 - i) * 2) * NUMCOLORMAPS / LIGHTLEVELS;

            // scalelight: iluminacao de paredes baseada na escala (rw_scale)
            // C original: level = startmap - j*SCREENWIDTH/(viewwidth<<detailshift)/DISTMAP
            // Com viewwidth=SCREENWIDTH=320, detailshift=0: level = startmap - j/2
            for j in 0..MAXLIGHTSCALE {
                let level = startmap.saturating_sub(j / DISTMAP);
                let clamped = level.min(NUMCOLORMAPS - 1);
                self.scalelight[i][j] = clamped * 256;
            }

            // zlight: iluminacao de flats baseada na profundidade (distancia Z)
            // C original: scale = FixedDiv((SCREENWIDTH/2*FRACUNIT), (j+1)<<LIGHTZSHIFT)
            //             scale >>= LIGHTSCALESHIFT
            //             level = startmap - scale/DISTMAP
            // Simplifica para: level = startmap - (SCREENWIDTH/2)/((j+1)*DISTMAP)
            for j in 0..MAXLIGHTZ {
                let scale = (SCREENWIDTH / 2) / (j + 1);
                let level = startmap.saturating_sub(scale / DISTMAP);
                let clamped = level.min(NUMCOLORMAPS - 1);
                self.zlight[i][j] = clamped * 256;
            }
        }
    }

    /// Prepara o estado para renderizar um novo frame.
    ///
    /// Atualiza viewcos/viewsin a partir de viewangle e incrementa
    /// contadores de frame.
    ///
    /// C original: `R_SetupFrame()` em `r_main.c`
    pub fn setup_frame(&mut self) {
        self.frame_count += 1;
        self.valid_count += 1;

        // Pre-calcular seno/cosseno do angulo de visao
        let fine_angle = (self.viewangle.0 >> ANGLETOFINESHIFT) as usize;
        self.viewsin = fine_sine(fine_angle & FINEMASK);
        self.viewcos = fine_cosine(fine_angle & FINEMASK);
    }

    /// Determina em qual lado de uma partition line um ponto esta.
    ///
    /// Retorna 0 para o lado da frente, 1 para o lado de tras.
    /// Usado na travessia BSP para decidir qual sub-arvore visitar primeiro.
    ///
    /// C original: `R_PointOnSide()` em `r_main.c`
    ///
    /// Otimizacao: para linhas horizontais ou verticais, basta comparar
    /// uma coordenada. Para linhas diagonais, calcula produto vetorial.
    pub fn point_on_side(x: Fixed, y: Fixed, node: &Node) -> usize {
        if node.dx.0 == 0 {
            return if x.0 <= node.x.0 {
                if node.dy.0 > 0 { 1 } else { 0 }
            } else if node.dy.0 > 0 {
                0
            } else {
                1
            };
        }

        if node.dy.0 == 0 {
            return if y.0 <= node.y.0 {
                if node.dx.0 < 0 { 1 } else { 0 }
            } else if node.dx.0 < 0 {
                0
            } else {
                1
            };
        }

        let dx = x.0 - node.x.0;
        let dy = y.0 - node.y.0;

        // Otimizacao via sign bits
        if (node.dy.0 ^ node.dx.0 ^ dx ^ dy) & (0x80000000u32 as i32) != 0 {
            return if (node.dy.0 ^ dx) & (0x80000000u32 as i32) != 0 {
                1
            } else {
                0
            };
        }

        let left = Fixed(node.dy.0 >> FRACBITS) * Fixed(dx);
        let right = Fixed(dy) * Fixed(node.dx.0 >> FRACBITS);

        if right.0 < left.0 { 0 } else { 1 }
    }

    /// Calcula o angulo entre a camera e um ponto no mundo.
    ///
    /// Usa a tabela tantoangle[] para converter tangente em angulo BAM.
    /// Trata os 8 octantes separadamente para manter precisao.
    ///
    /// C original: `R_PointToAngle()` em `r_main.c`
    pub fn point_to_angle(&self, x: Fixed, y: Fixed) -> Angle {
        let mut px = x.0 - self.viewx.0;
        let mut py = y.0 - self.viewy.0;

        if px == 0 && py == 0 {
            return Angle::ANG0;
        }

        if px >= 0 {
            if py >= 0 {
                if px > py {
                    // octante 0
                    Angle(TAN_TO_ANGLE[slope_div(py as u32, px as u32) as usize])
                } else {
                    // octante 1
                    Angle(Angle::ANG90.0.wrapping_sub(1).wrapping_sub(
                        TAN_TO_ANGLE[slope_div(px as u32, py as u32) as usize],
                    ))
                }
            } else {
                py = -py;
                if px > py {
                    // octante 8
                    Angle(0u32.wrapping_sub(
                        TAN_TO_ANGLE[slope_div(py as u32, px as u32) as usize],
                    ))
                } else {
                    // octante 7
                    Angle(Angle::ANG270.0.wrapping_add(
                        TAN_TO_ANGLE[slope_div(px as u32, py as u32) as usize],
                    ))
                }
            }
        } else {
            px = -px;
            if py >= 0 {
                if px > py {
                    // octante 3
                    Angle(Angle::ANG180.0.wrapping_sub(1).wrapping_sub(
                        TAN_TO_ANGLE[slope_div(py as u32, px as u32) as usize],
                    ))
                } else {
                    // octante 2
                    Angle(Angle::ANG90.0.wrapping_add(
                        TAN_TO_ANGLE[slope_div(px as u32, py as u32) as usize],
                    ))
                }
            } else {
                py = -py;
                if px > py {
                    // octante 4
                    Angle(Angle::ANG180.0.wrapping_add(
                        TAN_TO_ANGLE[slope_div(py as u32, px as u32) as usize],
                    ))
                } else {
                    // octante 5
                    Angle(Angle::ANG270.0.wrapping_sub(1).wrapping_sub(
                        TAN_TO_ANGLE[slope_div(px as u32, py as u32) as usize],
                    ))
                }
            }
        }
    }

    /// Calcula a distancia entre a camera e um ponto.
    ///
    /// Usa a identidade: dist = dx / cos(angle) onde angle e o angulo
    /// entre o eixo X e a linha camera->ponto.
    ///
    /// C original: `R_PointToDist()` em `r_main.c`
    pub fn point_to_dist(&self, x: Fixed, y: Fixed) -> Fixed {
        let mut dx = (x.0 - self.viewx.0).abs();
        let mut dy = (y.0 - self.viewy.0).abs();

        if dy > dx {
            std::mem::swap(&mut dx, &mut dy);
        }

        if dx == 0 {
            return Fixed::ZERO;
        }

        let angle = TAN_TO_ANGLE
            [((Fixed(dy) / Fixed(dx)).0 >> DBITS) as usize]
            .wrapping_add(Angle::ANG90.0);
        let fine = (angle >> ANGLETOFINESHIFT) as usize;

        let sine_val = fine_sine(fine & FINEMASK);
        if sine_val.0 != 0 {
            Fixed(dx) / sine_val
        } else {
            Fixed::MAX
        }
    }
}

impl Default for RenderState {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper: retorna tangente de fine angle como Fixed.
fn fine_tangent(angle: usize) -> Fixed {
    use crate::utils::tables::fine_tangent as ft;
    ft(angle)
}

/// Bits para lookup em tantoangle.
///
/// C original: `#define DBITS (FRACBITS-SLOPEBITS)` = 16 - 11 = 5
/// SLOPEBITS=11 define a precisao da tabela tantoangle (2048 entradas).
/// O shift converte o resultado de FixedDiv para um indice na tabela.
const DBITS: i32 = FRACBITS - 11; // FRACBITS - SLOPEBITS = 5

/// Numero de colormaps disponivel.
const NUMCOLORMAPS: usize = 32;

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifica que o estado inicializa com valores razoaveis.
    #[test]
    fn render_state_init() {
        let state = RenderState::new();
        assert_eq!(state.centerx, (SCREENWIDTH / 2) as i32);
        assert_eq!(state.centery, 84); // viewheight/2 = 168/2
        assert!(state.clipangle.0 > 0);
        assert_eq!(state.frame_count, 0);
        assert_eq!(state.valid_count, 1);
    }

    /// Verifica que setup_frame incrementa contadores e calcula sin/cos.
    #[test]
    fn setup_frame_basic() {
        let mut state = RenderState::new();
        state.viewangle = Angle::ANG90;
        state.setup_frame();

        assert_eq!(state.frame_count, 1);
        assert_eq!(state.valid_count, 2);
        // viewsin a 90 graus deve ser ~FRACUNIT
        assert!(state.viewsin.0.abs() > FRACUNIT / 2);
    }

    /// Verifica point_on_side com partition line vertical.
    #[test]
    fn point_on_side_vertical() {
        let node = Node {
            x: Fixed::from_int(100),
            y: Fixed::ZERO,
            dx: Fixed::ZERO,
            dy: Fixed::from_int(1),
            bbox: [[Fixed::ZERO; 4]; 2],
            children: [0, 0],
        };

        // Ponto a esquerda da linha (x < node.x, dy > 0) -> lado 1 (tras)
        assert_eq!(
            RenderState::point_on_side(Fixed::from_int(50), Fixed::ZERO, &node),
            1,
        );
        // Ponto a direita da linha (x > node.x, dy > 0) -> lado 0 (frente)
        assert_eq!(
            RenderState::point_on_side(Fixed::from_int(150), Fixed::ZERO, &node),
            0,
        );
    }

    /// Verifica point_on_side com partition line horizontal.
    #[test]
    fn point_on_side_horizontal() {
        let node = Node {
            x: Fixed::ZERO,
            y: Fixed::from_int(100),
            dx: Fixed::from_int(1),
            dy: Fixed::ZERO,
            bbox: [[Fixed::ZERO; 4]; 2],
            children: [0, 0],
        };

        // C original: if (y <= node->y) return node->dx < 0;
        //             return node->dx > 0;
        // dx=1 > 0: ponto abaixo (y<=node.y) -> return dx<0 = false = 0 (frente)
        assert_eq!(
            RenderState::point_on_side(Fixed::ZERO, Fixed::from_int(50), &node),
            0,
        );
        // Ponto acima (y > node.y, dx > 0) -> return dx>0 = true = 1 (tras)
        assert_eq!(
            RenderState::point_on_side(Fixed::ZERO, Fixed::from_int(150), &node),
            1,
        );
    }

    /// Verifica point_to_angle para direcoes cardinais.
    #[test]
    fn point_to_angle_cardinal() {
        let state = RenderState::new();
        // Camera na origem

        // Ponto a direita: angulo ~0
        let angle = state.point_to_angle(Fixed::from_int(100), Fixed::ZERO);
        assert!(angle.0 < Angle::ANG45.0 || angle.0 > Angle::ANG270.0);

        // Ponto acima: angulo ~90
        let angle = state.point_to_angle(Fixed::ZERO, Fixed::from_int(100));
        let diff = if angle.0 > Angle::ANG90.0 {
            angle.0 - Angle::ANG90.0
        } else {
            Angle::ANG90.0 - angle.0
        };
        assert!(diff < Angle::ANG45.0);
    }

    /// Verifica que tabelas de iluminacao sao preenchidas.
    #[test]
    fn light_tables_populated() {
        let state = RenderState::new();
        // Nivel de luz maximo, escala maxima -> colormap mais clara
        assert!(state.scalelight[LIGHTLEVELS - 1][MAXLIGHTSCALE - 1] < NUMCOLORMAPS * 256);
        // Nivel de luz minimo, escala minima -> colormap mais escura
        assert!(state.scalelight[0][0] > 0);
    }

    /// Verifica que `point_to_dist` calcula a distancia correta (hipotenusa).
    ///
    /// Com DBITS=5 (FRACBITS - SLOPEBITS), um ponto a 45 graus com
    /// dx=dy=100 deve retornar ~141 (sqrt(2)*100), nao 100.
    /// Este teste captura o bug onde DBITS era 19 (ANGLETOFINESHIFT),
    /// fazendo todos os indices da TAN_TO_ANGLE colapsarem para ~0
    /// e retornando simplesmente dx ao inves da hipotenusa real.
    #[test]
    fn point_to_dist_diagonal() {
        let mut state = RenderState::new();
        // Camera na origem olhando para a direita
        state.viewx = Fixed::ZERO;
        state.viewy = Fixed::ZERO;

        // Ponto a 45 graus: (100, 100) -> distancia = sqrt(2) * 100 ≈ 141.42
        let dist = state.point_to_dist(Fixed::from_int(100), Fixed::from_int(100));
        let dist_int = dist.to_int();

        // Com DBITS=5 correto, deve ser ~141 (tolerancia de ±5 para
        // imprecisao de ponto-fixo e tabela discretizada).
        assert!(
            (136..=146).contains(&dist_int),
            "point_to_dist(100,100) deveria ser ~141, mas retornou {}",
            dist_int
        );
    }

    /// Verifica que `point_to_dist` retorna dx quando dy=0 (angulo 0).
    #[test]
    fn point_to_dist_axis_aligned() {
        let mut state = RenderState::new();
        state.viewx = Fixed::ZERO;
        state.viewy = Fixed::ZERO;

        // Ponto alinhado no eixo X: distancia = dx exato
        let dist = state.point_to_dist(Fixed::from_int(200), Fixed::ZERO);
        let dist_int = dist.to_int();
        assert!(
            (198..=202).contains(&dist_int),
            "point_to_dist(200,0) deveria ser ~200, mas retornou {}",
            dist_int
        );
    }
}

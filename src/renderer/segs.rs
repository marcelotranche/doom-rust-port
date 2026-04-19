//! # Rendering de Segmentos de Parede
//!
//! Apos a travessia BSP determinar quais segs sao visiveis, este modulo
//! renderiza cada segmento coluna por coluna, calculando:
//!
//! - Escala perspectiva para cada coluna
//! - Coordenadas de textura (horizontal e vertical)
//! - Limites de clipping para pisos e tetos (openings)
//! - Quais texturas desenhar (upper, middle, lower)
//!
//! Um drawseg armazena as informacoes necessarias para renderizar
//! um segmento de parede ja clippado. Os drawsegs sao processados
//! em ordem durante a travessia BSP e depois usados para clippar sprites.
//!
//! ## Arquivo C original: `r_segs.c`
//!
//! ## Conceitos que o leitor vai aprender
//! - Perspectiva coluna-a-coluna para paredes verticais
//! - Silhouette clipping entre sectors adjacentes
//! - Openings para delimitar pisos/tetos visiveis

use crate::utils::fixed::Fixed;

/// Numero maximo de drawsegs por frame.
///
/// C original: `#define MAXDRAWSEGS 256` em `r_local.h`
pub const MAXDRAWSEGS: usize = 256;

/// Drawseg — informacoes de um segmento de parede clippado e pronto
/// para rendering e clipping de sprites.
///
/// C original: `drawseg_t` em `r_defs.h`
#[derive(Debug, Clone)]
pub struct DrawSeg {
    /// Indice do seg no array de segs do mapa
    pub seg_index: usize,

    /// Range de colunas X na tela (x1..=x2)
    pub x1: i32,
    pub x2: i32,

    /// Escala perspectiva na coluna x1 e x2
    pub scale1: Fixed,
    pub scale2: Fixed,
    /// Incremento de escala por coluna (para interpolacao)
    pub scale_step: Fixed,

    /// Tipo de silhouette para clipping de sprites.
    /// Bit 0 = silhouette bottom, Bit 1 = silhouette top
    pub silhouette: u32,

    /// Altura do piso do front sector (para sprite clipping)
    pub bsilheight: Fixed,
    /// Altura do teto do front sector (para sprite clipping)
    pub tsilheight: Fixed,
}

/// Flag de silhouette: clippar sprites por baixo.
pub const SIL_BOTTOM: u32 = 1;
/// Flag de silhouette: clippar sprites por cima.
pub const SIL_TOP: u32 = 2;
/// Flag de silhouette: clippar sprites por ambos os lados.
pub const SIL_BOTH: u32 = 3;

impl DrawSeg {
    /// Cria um drawseg vazio com valores padrao.
    pub fn new() -> Self {
        DrawSeg {
            seg_index: 0,
            x1: 0,
            x2: 0,
            scale1: Fixed::ZERO,
            scale2: Fixed::ZERO,
            scale_step: Fixed::ZERO,
            silhouette: 0,
            bsilheight: Fixed::ZERO,
            tsilheight: Fixed::ZERO,
        }
    }
}

impl Default for DrawSeg {
    fn default() -> Self {
        Self::new()
    }
}

/// Estado do rendering de segmentos de parede.
///
/// Acumula drawsegs durante a travessia BSP para uso posterior
/// no clipping de sprites.
#[derive(Debug)]
pub struct SegRenderer {
    /// Array de drawsegs acumulados neste frame.
    /// C original: `drawseg_t drawsegs[MAXDRAWSEGS]` em `r_bsp.c`
    pub drawsegs: Vec<DrawSeg>,
}

impl SegRenderer {
    /// Cria um novo renderer de segmentos.
    pub fn new() -> Self {
        SegRenderer {
            drawsegs: Vec::with_capacity(MAXDRAWSEGS),
        }
    }

    /// Limpa os drawsegs para um novo frame.
    ///
    /// C original: `R_ClearDrawSegs()` em `r_bsp.c`
    pub fn clear(&mut self) {
        self.drawsegs.clear();
    }
}

impl Default for SegRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drawseg_defaults() {
        let ds = DrawSeg::new();
        assert_eq!(ds.x1, 0);
        assert_eq!(ds.silhouette, 0);
    }

    #[test]
    fn seg_renderer_clear() {
        let mut sr = SegRenderer::new();
        sr.drawsegs.push(DrawSeg::new());
        assert_eq!(sr.drawsegs.len(), 1);
        sr.clear();
        assert_eq!(sr.drawsegs.len(), 0);
    }
}

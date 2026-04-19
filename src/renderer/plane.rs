//! # Visplanes — Rendering de Pisos e Tetos
//!
//! O DOOM renderiza pisos e tetos como "visplanes": superficies
//! horizontais contiguas com a mesma textura, altura e iluminacao.
//!
//! O rendering e deferido: durante a travessia BSP, visplanes sao
//! acumulados. Apos a travessia, `R_DrawPlanes()` renderiza todos
//! os visplanes de uma vez.
//!
//! Cada visplane mantem um array de spans (ranges horizontais)
//! para cada linha da tela, definindo onde a textura de piso/teto
//! e visivel naquela linha.
//!
//! ## Limitacoes do DOOM original
//!
//! O DOOM tem um limite fixo de visplanes (MAXVISPLANES = 128).
//! Se este limite for excedido, ocorre o famoso "visplane overflow"
//! que causa crash. Mappers precisam respeitar este limite.
//!
//! ## Arquivo C original: `r_plane.c`
//!
//! ## Conceitos que o leitor vai aprender
//! - Rendering deferido de superficies horizontais
//! - Textura affine mapping para pisos (64x64 flats)
//! - Visplane merging para otimizar rendering

use crate::utils::fixed::Fixed;
use crate::video::{SCREENHEIGHT, SCREENWIDTH};

/// Numero maximo de visplanes por frame.
///
/// C original: `#define MAXVISPLANES 128` em `r_plane.c`
pub const MAXVISPLANES: usize = 128;

/// Numero maximo de openings (spans de clipping).
///
/// C original: `#define MAXOPENINGS SCREENWIDTH*64` em `r_plane.c`
pub const MAXOPENINGS: usize = SCREENWIDTH * 64;

/// Visplane — uma superficie horizontal visivel (piso ou teto).
///
/// C original: `visplane_t` em `r_defs.h`
#[derive(Debug, Clone)]
pub struct Visplane {
    /// Altura do plano em fixed-point
    pub height: Fixed,
    /// Indice da textura flat (piso/teto)
    pub pic_num: i32,
    /// Nivel de luz do sector
    pub light_level: i32,
    /// Range de colunas X onde este visplane aparece
    pub min_x: i32,
    pub max_x: i32,
    /// Para cada coluna X, o topo visivel deste plano.
    /// Valor 0xFF = coluna nao usada.
    pub top: Vec<u8>,
    /// Para cada coluna X, o bottom visivel deste plano.
    pub bottom: Vec<u8>,
}

impl Visplane {
    /// Cria um visplane vazio.
    pub fn new() -> Self {
        Visplane {
            height: Fixed::ZERO,
            pic_num: 0,
            light_level: 0,
            min_x: i32::MAX,
            max_x: i32::MIN,
            top: vec![0xFF; SCREENWIDTH],
            bottom: vec![0; SCREENWIDTH],
        }
    }
}

impl Default for Visplane {
    fn default() -> Self {
        Self::new()
    }
}

/// Sistema de gerenciamento de visplanes.
///
/// Acumula visplanes durante a travessia BSP e os renderiza
/// ao final do frame.
#[derive(Debug)]
pub struct PlaneRenderer {
    /// Array de visplanes acumulados neste frame.
    pub visplanes: Vec<Visplane>,

    /// Visplane do piso atual (durante processamento de subsector).
    pub floor_plane: Option<usize>,
    /// Visplane do teto atual.
    pub ceiling_plane: Option<usize>,

    /// Arrays de clipping: para cada coluna X, o topo e bottom
    /// da regiao onde pisos/tetos podem ser desenhados.
    pub floor_clip: Vec<i16>,
    pub ceiling_clip: Vec<i16>,

    /// Openings array para armazenar spans.
    pub openings: Vec<i16>,
    pub last_opening: usize,
}

impl PlaneRenderer {
    /// Cria um novo sistema de visplanes.
    pub fn new() -> Self {
        PlaneRenderer {
            visplanes: Vec::with_capacity(MAXVISPLANES),
            floor_plane: None,
            ceiling_plane: None,
            floor_clip: vec![SCREENHEIGHT as i16; SCREENWIDTH],
            ceiling_clip: vec![-1; SCREENWIDTH],
            openings: vec![0; MAXOPENINGS],
            last_opening: 0,
        }
    }

    /// Limpa todos os visplanes para um novo frame.
    ///
    /// C original: `R_ClearPlanes()` em `r_plane.c`
    pub fn clear(&mut self) {
        self.visplanes.clear();
        self.floor_plane = None;
        self.ceiling_plane = None;
        self.last_opening = 0;

        // Resetar arrays de clipping
        for i in 0..SCREENWIDTH {
            self.floor_clip[i] = SCREENHEIGHT as i16;
            self.ceiling_clip[i] = -1;
        }
    }

    /// Encontra ou cria um visplane com os parametros dados.
    ///
    /// Se ja existe um visplane com a mesma altura, textura e luz,
    /// reutiliza-o. Caso contrario, cria um novo.
    ///
    /// C original: `R_FindPlane()` em `r_plane.c`
    pub fn find_plane(&mut self, height: Fixed, pic_num: i32, light_level: i32) -> usize {
        // Procurar visplane existente com mesmos parametros
        for (i, vp) in self.visplanes.iter().enumerate() {
            if vp.height == height && vp.pic_num == pic_num && vp.light_level == light_level {
                return i;
            }
        }

        // Criar novo visplane
        let mut vp = Visplane::new();
        vp.height = height;
        vp.pic_num = pic_num;
        vp.light_level = light_level;
        self.visplanes.push(vp);
        self.visplanes.len() - 1
    }
}

impl Default for PlaneRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visplane_init() {
        let vp = Visplane::new();
        assert_eq!(vp.top.len(), SCREENWIDTH);
        assert!(vp.top.iter().all(|&v| v == 0xFF));
    }

    #[test]
    fn plane_renderer_clear() {
        let mut pr = PlaneRenderer::new();
        pr.find_plane(Fixed::ZERO, 0, 128);
        assert_eq!(pr.visplanes.len(), 1);
        pr.clear();
        assert_eq!(pr.visplanes.len(), 0);
    }

    #[test]
    fn find_plane_reuse() {
        let mut pr = PlaneRenderer::new();
        let a = pr.find_plane(Fixed::ZERO, 5, 128);
        let b = pr.find_plane(Fixed::ZERO, 5, 128);
        assert_eq!(a, b); // Mesmo visplane reutilizado
    }

    #[test]
    fn find_plane_different() {
        let mut pr = PlaneRenderer::new();
        let a = pr.find_plane(Fixed::ZERO, 5, 128);
        let b = pr.find_plane(Fixed::from_int(128), 5, 128);
        assert_ne!(a, b); // Alturas diferentes = visplanes diferentes
    }
}

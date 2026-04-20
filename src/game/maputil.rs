//! # Utilidades de Mapa — Blockmap e Queries Espaciais
//!
//! O DOOM usa um "blockmap" como estrutura de aceleracao espacial
//! para queries de colisao. O mundo do mapa e dividido em blocos
//! de 128×128 unidades. Cada bloco contem listas de linedefs e
//! mobjs que o interceptam, permitindo queries rapidas do tipo
//! "quais objetos estao perto de (x, y)?".
//!
//! ## Blockmap
//!
//! ```text
//! +-------+-------+-------+
//! | (0,2) | (1,2) | (2,2) |   Cada celula contem:
//! +-------+-------+-------+   - Lista de linedefs
//! | (0,1) | (1,1) | (2,1) |   - Lista de mobjs (via bnext/bprev)
//! +-------+-------+-------+
//! | (0,0) | (1,0) | (2,0) |   Tamanho de bloco: 128 unidades
//! +-------+-------+-------+
//! ```
//!
//! ## Line Opening
//!
//! Para linhas two-sided (fronteiras entre sectors), `line_opening()`
//! calcula o espaco vertical transitavel: o topo aberto (menor teto),
//! o fundo aberto (maior chao), e o range transitavel.
//!
//! ## Arquivo C original: `p_maputl.c`, `p_setup.c`
//!
//! ## Conceitos que o leitor vai aprender
//! - Spatial hashing para aceleracao de colisao
//! - Grid-based queries (blockmap iteration)
//! - Gap calculation para passagem entre sectors

use crate::utils::fixed::Fixed;

// ---------------------------------------------------------------------------
// Constantes de blockmap
// ---------------------------------------------------------------------------

/// Tamanho de um bloco em unidades do mapa (128 unidades).
///
/// C original: `MAPBLOCKUNITS = 128` em `p_local.h`
pub const MAPBLOCKUNITS: i32 = 128;

/// Shift para converter coordenada em indice de bloco.
///
/// MAPBLOCKSHIFT = FRACBITS + 7 = 16 + 7 = 23
///
/// C original: `#define MAPBLOCKSHIFT (FRACBITS+7)` em `p_local.h`
pub const MAPBLOCKSHIFT: i32 = 23;

/// Mascara para operacoes de bloco.
///
/// C original: `#define MAPBMASK (MAPBLOCKSIZE-1)` em `p_local.h`
pub const MAPBMASK: i32 = (MAPBLOCKUNITS << 16) - 1;

// ---------------------------------------------------------------------------
// Blockmap
// ---------------------------------------------------------------------------

/// Blockmap — grid de aceleracao espacial para colisao.
///
/// Divide o mapa em blocos de 128×128 unidades. Cada bloco
/// contem indices de linedefs que o atravessam.
///
/// C original: `blockmap`, `blockmaplump`, `bmaporgx/y`, etc. em `p_setup.c`
#[derive(Debug, Clone)]
pub struct Blockmap {
    /// Origem X do blockmap (fixed-point)
    pub origin_x: Fixed,
    /// Origem Y do blockmap (fixed-point)
    pub origin_y: Fixed,
    /// Largura em blocos
    pub width: i32,
    /// Altura em blocos
    pub height: i32,
    /// Para cada bloco, lista de indices de linedefs.
    /// `lines[block_y * width + block_x]` = indices de linedefs nesse bloco.
    pub lines: Vec<Vec<usize>>,
}

impl Blockmap {
    /// Cria um blockmap a partir dos dados brutos do WAD.
    ///
    /// O lump BLOCKMAP no WAD contem:
    /// - origin_x, origin_y (short, em unidades de mapa)
    /// - width, height (short, em blocos)
    /// - offsets[width*height] (short, offset para cada bloco)
    /// - Para cada bloco: lista de linedefs terminada por -1
    ///
    /// C original: `P_LoadBlockMap()` em `p_setup.c`
    pub fn from_raw(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }

        let origin_x = i16::from_le_bytes([data[0], data[1]]) as i32;
        let origin_y = i16::from_le_bytes([data[2], data[3]]) as i32;
        let width = i16::from_le_bytes([data[4], data[5]]) as i32;
        let height = i16::from_le_bytes([data[6], data[7]]) as i32;

        if width <= 0 || height <= 0 {
            return None;
        }

        let num_blocks = (width * height) as usize;
        let header_size = 8 + num_blocks * 2; // 4 shorts de header + offsets

        if data.len() < header_size {
            return None;
        }

        // Ler offsets de cada bloco
        let mut lines = Vec::with_capacity(num_blocks);
        for i in 0..num_blocks {
            let offset_pos = 8 + i * 2;
            let offset =
                i16::from_le_bytes([data[offset_pos], data[offset_pos + 1]]) as usize * 2;

            let mut block_lines = Vec::new();
            // Cada bloco comeca com 0 (marcador) seguido de indices de linedefs,
            // terminado por -1 (0xFFFF)
            let mut pos = offset;
            // Pular marcador 0
            if pos + 2 <= data.len() {
                pos += 2;
            }
            while pos + 2 <= data.len() {
                let val = i16::from_le_bytes([data[pos], data[pos + 1]]);
                if val == -1 {
                    break;
                }
                block_lines.push(val as usize);
                pos += 2;
            }

            lines.push(block_lines);
        }

        Some(Blockmap {
            origin_x: Fixed::from_int(origin_x),
            origin_y: Fixed::from_int(origin_y),
            width,
            height,
            lines,
        })
    }

    /// Converte coordenada X do mapa em indice de bloco X.
    pub fn to_block_x(&self, x: Fixed) -> i32 {
        ((x - self.origin_x).0 >> MAPBLOCKSHIFT).max(0).min(self.width - 1)
    }

    /// Converte coordenada Y do mapa em indice de bloco Y.
    pub fn to_block_y(&self, y: Fixed) -> i32 {
        ((y - self.origin_y).0 >> MAPBLOCKSHIFT).max(0).min(self.height - 1)
    }

    /// Retorna as linedefs no bloco (bx, by).
    pub fn block_lines(&self, bx: i32, by: i32) -> &[usize] {
        if bx < 0 || bx >= self.width || by < 0 || by >= self.height {
            return &[];
        }
        let idx = (by * self.width + bx) as usize;
        &self.lines[idx]
    }
}

// ---------------------------------------------------------------------------
// Line Opening — espaco transitavel entre sectors
// ---------------------------------------------------------------------------

/// Resultado de `line_opening()`: o espaco vertical transitavel
/// atraves de uma linha two-sided.
///
/// C original: globals `opentop`, `openbottom`, `openrange`, `lowfloor`
/// em `p_maputl.c`
#[derive(Debug, Clone, Copy)]
pub struct LineOpening {
    /// Topo da abertura (menor teto dos dois sectors)
    pub open_top: Fixed,
    /// Fundo da abertura (maior chao dos dois sectors)
    pub open_bottom: Fixed,
    /// Tamanho vertical da abertura (open_top - open_bottom)
    pub open_range: Fixed,
    /// Chao mais baixo dos dois sectors (para calculo de dropoff)
    pub low_floor: Fixed,
}

impl LineOpening {
    /// Calcula a abertura vertical entre dois sectors adjacentes.
    ///
    /// Dados os pisos e tetos de dois sectors, determina o espaco
    /// transitavel: o menor teto e o maior chao definem a abertura.
    ///
    /// C original: `P_LineOpening()` em `p_maputl.c`
    pub fn calculate(
        front_floor: Fixed,
        front_ceiling: Fixed,
        back_floor: Fixed,
        back_ceiling: Fixed,
    ) -> Self {
        // Topo: menor teto
        let open_top = if front_ceiling < back_ceiling {
            front_ceiling
        } else {
            back_ceiling
        };

        // Fundo: maior chao
        let (open_bottom, low_floor) = if front_floor > back_floor {
            (front_floor, back_floor)
        } else {
            (back_floor, front_floor)
        };

        let open_range = open_top - open_bottom;

        LineOpening {
            open_top,
            open_bottom,
            open_range,
            low_floor,
        }
    }
}

// ---------------------------------------------------------------------------
// Reject table
// ---------------------------------------------------------------------------

/// Tabela de rejeicao — otimizacao para line-of-sight.
///
/// Matriz de bits NxN (N = numero de sectors). Se o bit (i, j)
/// esta setado, os sectors i e j NAO se veem, e nenhuma checagem
/// de line-of-sight e necessaria entre eles.
///
/// C original: `rejectmatrix` em `p_setup.c`
#[derive(Debug, Clone)]
pub struct RejectTable {
    /// Dados brutos da tabela de bits
    data: Vec<u8>,
    /// Numero de sectors
    num_sectors: usize,
}

impl RejectTable {
    /// Cria uma reject table a partir dos dados do WAD.
    pub fn from_raw(data: Vec<u8>, num_sectors: usize) -> Self {
        RejectTable { data, num_sectors }
    }

    /// Verifica se dois sectors nao se veem (rejeitados).
    ///
    /// Retorna `true` se os sectors estao rejeitados (nao precisam
    /// de checagem de line-of-sight).
    pub fn is_rejected(&self, sector1: usize, sector2: usize) -> bool {
        let bit = sector1 * self.num_sectors + sector2;
        let byte_idx = bit / 8;
        let bit_idx = bit % 8;
        if byte_idx >= self.data.len() {
            return false;
        }
        self.data[byte_idx] & (1 << bit_idx) != 0
    }

    /// Cria uma reject table vazia (nenhum sector rejeitado).
    pub fn empty(num_sectors: usize) -> Self {
        let size = (num_sectors * num_sectors).div_ceil(8);
        RejectTable {
            data: vec![0; size],
            num_sectors,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn line_opening_symmetric() {
        let opening = LineOpening::calculate(
            Fixed::from_int(0),   // front floor
            Fixed::from_int(128), // front ceiling
            Fixed::from_int(32),  // back floor (higher)
            Fixed::from_int(96),  // back ceiling (lower)
        );

        assert_eq!(opening.open_top, Fixed::from_int(96));     // menor teto
        assert_eq!(opening.open_bottom, Fixed::from_int(32));  // maior chao
        assert_eq!(opening.open_range, Fixed::from_int(64));   // 96 - 32
        assert_eq!(opening.low_floor, Fixed::from_int(0));     // menor chao
    }

    #[test]
    fn line_opening_step() {
        // Simula um degrau: chao sobe 24, teto igual
        let opening = LineOpening::calculate(
            Fixed::from_int(0),    // front floor
            Fixed::from_int(128),  // front ceiling
            Fixed::from_int(24),   // back floor (+24)
            Fixed::from_int(128),  // back ceiling (igual)
        );

        assert_eq!(opening.open_bottom, Fixed::from_int(24));
        assert_eq!(opening.open_range, Fixed::from_int(104)); // 128 - 24
    }

    #[test]
    fn blockmap_coords() {
        let bm = Blockmap {
            origin_x: Fixed::ZERO,
            origin_y: Fixed::ZERO,
            width: 10,
            height: 10,
            lines: vec![vec![]; 100],
        };

        // Bloco 0,0 para coordenada 0,0
        assert_eq!(bm.to_block_x(Fixed::ZERO), 0);
        assert_eq!(bm.to_block_y(Fixed::ZERO), 0);

        // Bloco 1 para coordenada 128
        assert_eq!(bm.to_block_x(Fixed::from_int(128)), 1);

        // Bloco 0 para coordenada 127 (ainda no bloco 0)
        assert_eq!(bm.to_block_x(Fixed::from_int(127)), 0);

        // Clampar ao maximo
        assert_eq!(bm.to_block_x(Fixed::from_int(5000)), 9);
    }

    #[test]
    fn reject_table_basic() {
        // 4 sectors, reject entre (0,1)
        let mut rt = RejectTable::empty(4);
        // Setar bit para (0, 1) = bit 1
        rt.data[0] |= 0b0000_0010;

        assert!(rt.is_rejected(0, 1));
        assert!(!rt.is_rejected(0, 0));
        assert!(!rt.is_rejected(1, 0));
    }

    #[test]
    fn reject_table_empty() {
        let rt = RejectTable::empty(10);
        for i in 0..10 {
            for j in 0..10 {
                assert!(!rt.is_rejected(i, j));
            }
        }
    }

    #[test]
    fn blockmap_block_lines() {
        let mut lines = vec![vec![]; 4];
        lines[0] = vec![0, 1, 2]; // bloco (0,0) tem linedefs 0,1,2
        lines[3] = vec![5, 6];    // bloco (1,1) tem linedefs 5,6

        let bm = Blockmap {
            origin_x: Fixed::ZERO,
            origin_y: Fixed::ZERO,
            width: 2,
            height: 2,
            lines,
        };

        assert_eq!(bm.block_lines(0, 0), &[0, 1, 2]);
        assert_eq!(bm.block_lines(1, 0), &[] as &[usize]);
        assert_eq!(bm.block_lines(1, 1), &[5, 6]);
        // Fora dos limites
        assert_eq!(bm.block_lines(-1, 0), &[] as &[usize]);
    }
}

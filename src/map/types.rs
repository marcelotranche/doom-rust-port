//! # Tipos de Geometria de Mapa
//!
//! Define as structs que representam a geometria de um nivel do DOOM:
//! vertices, linedefs, sidedefs, sectors, segs, subsectors e nodes.
//!
//! Existem dois conjuntos de structs:
//! - **Map*** (MapVertex, MapLineDef, etc): formato binario no WAD (on-disk)
//! - **Vertex, LineDef, etc**: formato em runtime com campos expandidos
//!
//! ## Arquivos C originais
//! - `doomdata.h` — structs on-disk (mapvertex_t, maplinedef_t, etc)
//! - `r_defs.h` — structs runtime (vertex_t, line_t, sector_t, etc)
//! - `p_setup.c` — funcoes de carregamento (P_LoadVertexes, etc)
//!
//! ## Conceitos que o leitor vai aprender
//! - Separacao entre formato de disco e formato de runtime
//! - Como o DOOM usa indices ao inves de ponteiros no WAD
//! - BSP tree como estrutura de dados para rendering

use bitflags::bitflags;

use crate::utils::fixed::Fixed;

// ---------------------------------------------------------------------------
// Formato on-disk (lido diretamente do WAD)
// Estes structs mapeiam byte-a-byte o conteudo dos lumps do WAD.
// Todos os campos sao i16 little-endian no arquivo.
// ---------------------------------------------------------------------------

/// Vertice no formato do WAD (4 bytes).
/// C original: `mapvertex_t` em `doomdata.h`
#[derive(Debug, Clone, Copy)]
pub struct MapVertex {
    pub x: i16,
    pub y: i16,
}

/// LineDef no formato do WAD (14 bytes).
/// C original: `maplinedef_t` em `doomdata.h`
#[derive(Debug, Clone, Copy)]
pub struct MapLineDef {
    pub v1: i16,
    pub v2: i16,
    pub flags: i16,
    pub special: i16,
    pub tag: i16,
    /// sidenum[1] sera -1 se a linedef tiver apenas um lado
    pub sidenum: [i16; 2],
}

/// SideDef no formato do WAD (30 bytes).
/// C original: `mapsidedef_t` em `doomdata.h`
#[derive(Debug, Clone)]
pub struct MapSideDef {
    pub texture_offset: i16,
    pub row_offset: i16,
    pub top_texture: [u8; 8],
    pub bottom_texture: [u8; 8],
    pub mid_texture: [u8; 8],
    pub sector: i16,
}

/// Sector no formato do WAD (26 bytes).
/// C original: `mapsector_t` em `doomdata.h`
#[derive(Debug, Clone)]
pub struct MapSector {
    pub floor_height: i16,
    pub ceiling_height: i16,
    pub floor_pic: [u8; 8],
    pub ceiling_pic: [u8; 8],
    pub light_level: i16,
    pub special: i16,
    pub tag: i16,
}

/// Seg no formato do WAD (12 bytes).
/// C original: `mapseg_t` em `doomdata.h`
#[derive(Debug, Clone, Copy)]
pub struct MapSeg {
    pub v1: i16,
    pub v2: i16,
    pub angle: i16,
    pub linedef: i16,
    pub side: i16,
    pub offset: i16,
}

/// SubSector no formato do WAD (4 bytes).
/// C original: `mapsubsector_t` em `doomdata.h`
#[derive(Debug, Clone, Copy)]
pub struct MapSubSector {
    pub num_segs: i16,
    pub first_seg: i16,
}

/// Node BSP no formato do WAD (28 bytes).
/// C original: `mapnode_t` em `doomdata.h`
#[derive(Debug, Clone, Copy)]
pub struct MapNode {
    /// Partition line: ponto de origem (x, y)
    pub x: i16,
    pub y: i16,
    /// Partition line: direcao (dx, dy)
    pub dx: i16,
    pub dy: i16,
    /// Bounding boxes dos filhos: bbox[0] = direito, bbox[1] = esquerdo
    /// Cada bbox tem 4 valores: top, bottom, left, right
    pub bbox: [[i16; 4]; 2],
    /// Filhos: indice de node, ou indice de subsector se bit NF_SUBSECTOR estiver setado
    pub children: [u16; 2],
}

/// Thing no formato do WAD (10 bytes).
/// C original: `mapthing_t` em `doomdata.h`
#[derive(Debug, Clone, Copy)]
pub struct MapThing {
    pub x: i16,
    pub y: i16,
    pub angle: i16,
    pub thing_type: i16,
    pub options: i16,
}

// ---------------------------------------------------------------------------
// Formato runtime (usado pelo engine apos carregamento)
// Campos expandidos de i16 para Fixed ou indices usize.
// ---------------------------------------------------------------------------

/// Vertice em runtime — coordenadas expandidas para fixed-point.
///
/// No WAD, vertices sao i16. No runtime, sao fixed_t (i32 << 16).
/// C original: `vertex_t` em `r_defs.h`
#[derive(Debug, Clone, Copy, Default)]
pub struct Vertex {
    pub x: Fixed,
    pub y: Fixed,
}

/// Sector em runtime — alturas em fixed-point, texturas como indices.
///
/// C original: `sector_t` em `r_defs.h`
/// Nota: campos de runtime como thinglist, soundtarget, specialdata
/// serao adicionados em fases posteriores.
#[derive(Debug, Clone)]
pub struct Sector {
    pub floor_height: Fixed,
    pub ceiling_height: Fixed,
    /// Nome da textura do piso (ate 8 chars)
    pub floor_pic: [u8; 8],
    /// Nome da textura do teto
    pub ceiling_pic: [u8; 8],
    pub light_level: i16,
    pub special: i16,
    pub tag: i16,
}

/// SideDef em runtime — offsets em fixed-point.
///
/// C original: `side_t` em `r_defs.h`
#[derive(Debug, Clone)]
pub struct SideDef {
    pub texture_offset: Fixed,
    pub row_offset: Fixed,
    pub top_texture: [u8; 8],
    pub bottom_texture: [u8; 8],
    pub mid_texture: [u8; 8],
    /// Indice do sector que este sidedef enfrenta
    pub sector_index: usize,
}

/// Tipo de inclinacao de uma linedef, para otimizacao de colisao.
///
/// C original: `slopetype_t` em `r_defs.h`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlopeType {
    Horizontal,
    Vertical,
    Positive,
    Negative,
}

/// LineDef em runtime — com campos pre-calculados.
///
/// C original: `line_t` em `r_defs.h`
#[derive(Debug, Clone)]
pub struct LineDef {
    /// Indices dos vertices
    pub v1: usize,
    pub v2: usize,
    /// Delta pre-calculado (v2 - v1)
    pub dx: Fixed,
    pub dy: Fixed,
    pub flags: LineDefFlags,
    pub special: i16,
    pub tag: i16,
    /// Indices dos sidedefs. sidenum[1] = None se one-sided
    pub sidenum: [Option<usize>; 2],
    /// Tipo de inclinacao (para otimizacao de colisao)
    pub slope_type: SlopeType,
    /// Indices dos sectors frontal e traseiro
    pub front_sector: Option<usize>,
    pub back_sector: Option<usize>,
}

bitflags! {
    /// Flags de uma LineDef.
    ///
    /// C original: `#define ML_BLOCKING 1` etc. em `doomdata.h`
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct LineDefFlags: i16 {
        /// Solida, e um obstaculo (bloqueia jogador e monstros)
        const BLOCKING       = 1;
        /// Bloqueia apenas monstros
        const BLOCK_MONSTERS = 2;
        /// Tem dois lados (portal entre sectors)
        const TWO_SIDED      = 4;
        /// Textura superior nao acompanha movimento do sector
        const DONT_PEG_TOP   = 8;
        /// Textura inferior nao acompanha movimento do sector
        const DONT_PEG_BOTTOM = 16;
        /// Segredo no automap — aparece como parede solida
        const SECRET         = 32;
        /// Bloqueia propagacao de som
        const SOUND_BLOCK    = 64;
        /// Nao desenhar no automap
        const DONT_DRAW      = 128;
        /// Ja foi vista — desenhar no automap
        const MAPPED         = 256;
    }
}

/// Seg (segmento de linha) em runtime.
///
/// C original: `seg_t` em `r_defs.h`
#[derive(Debug, Clone, Copy)]
pub struct Seg {
    pub v1: usize,
    pub v2: usize,
    pub offset: Fixed,
    pub angle: u32,
    pub sidedef: usize,
    pub linedef: usize,
    pub front_sector: usize,
    /// None para linedefs one-sided
    pub back_sector: Option<usize>,
}

/// SubSector em runtime — regiao convexa do mapa.
///
/// C original: `subsector_t` em `r_defs.h`
#[derive(Debug, Clone, Copy)]
pub struct SubSector {
    /// Indice do sector que contem este subsector
    pub sector: usize,
    pub num_lines: usize,
    pub first_line: usize,
}

/// Flag que indica que um child de node e um subsector.
///
/// C original: `#define NF_SUBSECTOR 0x8000` em `doomdata.h`
pub const NF_SUBSECTOR: u16 = 0x8000;

/// Node BSP em runtime — partition line e filhos.
///
/// C original: `node_t` em `r_defs.h`
#[derive(Debug, Clone)]
pub struct Node {
    /// Partition line: ponto de origem
    pub x: Fixed,
    pub y: Fixed,
    /// Partition line: direcao
    pub dx: Fixed,
    pub dy: Fixed,
    /// Bounding boxes dos filhos [direito][esquerdo], cada um com [top, bottom, left, right]
    pub bbox: [[Fixed; 4]; 2],
    /// Filhos: indice de node ou subsector (se NF_SUBSECTOR bit setado)
    pub children: [u16; 2],
}

/// Thing (objeto) em runtime — posicao e tipo.
///
/// C original: `mapthing_t` em `doomdata.h`
#[derive(Debug, Clone, Copy)]
pub struct Thing {
    pub x: Fixed,
    pub y: Fixed,
    pub angle: i16,
    pub thing_type: i16,
    pub options: i16,
}

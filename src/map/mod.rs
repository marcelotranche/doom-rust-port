//! # Modulo Map — Geometria e BSP
//!
//! Carrega e gerencia a geometria dos niveis: vertices, linedefs,
//! sidedefs, sectors, BSP tree (nodes/subsectors/segs) e things.
//!
//! ## Fluxo de carregamento
//!
//! No DOOM original, `P_SetupLevel()` em `p_setup.c` chama uma serie
//! de funcoes `P_LoadXxx()` que leem lumps do WAD e convertem do formato
//! on-disk (i16) para o formato runtime (fixed-point, indices usize).
//!
//! Os lumps de um mapa aparecem na seguinte ordem no WAD:
//! ```text
//! E1M1       <- marker lump (tamanho 0)
//! THINGS     <- objetos no mapa
//! LINEDEFS   <- paredes/portais
//! SIDEDEFS   <- texturas das paredes
//! VERTEXES   <- coordenadas dos vertices
//! SEGS       <- segmentos BSP
//! SSECTORS   <- subsectors (regioes convexas)
//! NODES      <- arvore BSP
//! SECTORS    <- setores (pisos/tetos)
//! REJECT     <- tabela de rejeicao (visibilidade)
//! BLOCKMAP   <- grid de colisao
//! ```
//!
//! ## Arquivo C original: `p_setup.c`
//!
//! ## Conceitos que o leitor vai aprender
//! - Como o DOOM carrega dados binarios de um WAD
//! - Conversao de formato on-disk para runtime
//! - Indices relativos vs absolutos em estruturas de dados

pub mod types;

use std::io::Cursor;

use byteorder::{LittleEndian, ReadBytesExt};
use thiserror::Error;

use crate::utils::fixed::Fixed;
use crate::wad::{WadError, WadSystem};

use types::*;

/// Offsets dos lumps de mapa relativos ao marker lump.
///
/// No WAD, o mapa comeca com um marker lump (ex: "E1M1") seguido
/// dos lumps de dados em ordem fixa. Estes offsets sao usados em
/// `P_SetupLevel()` para calcular o indice de cada lump.
///
/// C original: `#define ML_THINGS 1` etc. em `p_setup.c`
const ML_THINGS: usize = 1;
const ML_LINEDEFS: usize = 2;
const ML_SIDEDEFS: usize = 3;
const ML_VERTEXES: usize = 4;
const ML_SEGS: usize = 5;
const ML_SSECTORS: usize = 6;
const ML_NODES: usize = 7;
const ML_SECTORS: usize = 8;
// const ML_REJECT: usize = 9;   // sera usado em fases posteriores
// const ML_BLOCKMAP: usize = 10; // sera usado em fases posteriores

/// Erros que podem ocorrer ao carregar um mapa.
#[derive(Error, Debug)]
pub enum MapError {
    #[error("Mapa '{0}' nao encontrado no WAD")]
    MapNotFound(String),

    #[error("Lump de mapa com tamanho invalido: esperado multiplo de {expected_entry_size}, encontrado {actual_size} bytes")]
    InvalidLumpSize {
        expected_entry_size: usize,
        actual_size: usize,
    },

    #[error("Indice de sidedef invalido: {0}")]
    InvalidSideDefIndex(usize),

    #[error("Indice de sector invalido: {0}")]
    InvalidSectorIndex(usize),

    #[error("Erro no WAD: {0}")]
    Wad(#[from] WadError),

    #[error("Erro de I/O ao ler dados do mapa: {0}")]
    Io(#[from] std::io::Error),
}

/// Dados de um mapa carregado — todas as estruturas geometricas.
///
/// Equivalente ao conjunto de globals em `p_setup.c`:
/// - `vertexes`, `numvertexes`
/// - `sectors`, `numsectors`
/// - `sidedefs`, `numsides`
/// - `linedefs`, `numlines`
/// - `segs`, `numsegs`
/// - `subsectors`, `numsubsectors`
/// - `nodes`, `numnodes`
/// - `things` (processados via `P_SpawnMapThing`)
///
/// Em Rust, encapsulamos em uma struct com ownership de todos os arrays.
#[derive(Debug)]
pub struct MapData {
    pub vertexes: Vec<Vertex>,
    pub sectors: Vec<Sector>,
    pub sidedefs: Vec<SideDef>,
    pub linedefs: Vec<LineDef>,
    pub segs: Vec<Seg>,
    pub subsectors: Vec<SubSector>,
    pub nodes: Vec<Node>,
    pub things: Vec<Thing>,
}

impl MapData {
    /// Carrega um mapa completo a partir do WAD.
    ///
    /// Recebe o nome do mapa (ex: "E1M1" ou "MAP01") e o sistema WAD.
    /// Le cada lump na ordem correta e converte para formato runtime.
    ///
    /// C original: `P_SetupLevel()` em `p_setup.c`
    ///
    /// A ordem de carregamento importa! Sectors devem ser carregados
    /// antes de sidedefs (que referenciam sectors por indice), e
    /// sidedefs antes de linedefs.
    pub fn load(map_name: &str, wad: &WadSystem) -> Result<Self, MapError> {
        let lump_index = wad
            .find_lump(map_name)
            .ok_or_else(|| MapError::MapNotFound(map_name.to_string()))?;

        // Carregar na ordem de dependencia:
        // 1. Vertexes e Sectors (sem dependencias)
        // 2. SideDefs (depende de sectors)
        // 3. LineDefs (depende de vertexes e sidedefs)
        // 4. Segs (depende de vertexes, linedefs, sidedefs, sectors)
        // 5. SubSectors
        // 6. Nodes
        // 7. Things

        let vertexes = Self::load_vertexes(wad, lump_index + ML_VERTEXES)?;
        let sectors = Self::load_sectors(wad, lump_index + ML_SECTORS)?;
        let sidedefs = Self::load_sidedefs(wad, lump_index + ML_SIDEDEFS)?;
        let linedefs = Self::load_linedefs(wad, lump_index + ML_LINEDEFS, &sidedefs)?;
        let segs = Self::load_segs(wad, lump_index + ML_SEGS, &linedefs, &sidedefs)?;
        let subsectors = Self::load_subsectors(wad, lump_index + ML_SSECTORS)?;
        let nodes = Self::load_nodes(wad, lump_index + ML_NODES)?;
        let things = Self::load_things(wad, lump_index + ML_THINGS)?;

        log::info!(
            "Mapa '{}' carregado: {} vertexes, {} sectors, {} sidedefs, \
             {} linedefs, {} segs, {} subsectors, {} nodes, {} things",
            map_name,
            vertexes.len(),
            sectors.len(),
            sidedefs.len(),
            linedefs.len(),
            segs.len(),
            subsectors.len(),
            nodes.len(),
            things.len(),
        );

        Ok(MapData {
            vertexes,
            sectors,
            sidedefs,
            linedefs,
            segs,
            subsectors,
            nodes,
            things,
        })
    }

    /// Carrega vertices do lump VERTEXES.
    ///
    /// Cada vertice on-disk tem 4 bytes (2x i16: x, y).
    /// Em runtime, coordenadas sao convertidas para fixed-point
    /// via shift left de 16 bits (i16 -> fixed_t).
    ///
    /// C original: `P_LoadVertexes()` em `p_setup.c`
    fn load_vertexes(wad: &WadSystem, lump_index: usize) -> Result<Vec<Vertex>, MapError> {
        let data = wad.read_lump(lump_index)?;
        let entry_size = 4; // sizeof(mapvertex_t)
        if data.len() % entry_size != 0 {
            return Err(MapError::InvalidLumpSize {
                expected_entry_size: entry_size,
                actual_size: data.len(),
            });
        }

        let count = data.len() / entry_size;
        let mut reader = Cursor::new(&data);
        let mut vertexes = Vec::with_capacity(count);

        for _ in 0..count {
            let x = reader.read_i16::<LittleEndian>()?;
            let y = reader.read_i16::<LittleEndian>()?;
            vertexes.push(Vertex {
                x: Fixed::from_int(x as i32),
                y: Fixed::from_int(y as i32),
            });
        }

        Ok(vertexes)
    }

    /// Carrega sectors do lump SECTORS.
    ///
    /// Cada sector on-disk tem 26 bytes:
    /// - floor_height (i16), ceiling_height (i16)
    /// - floor_pic (8 bytes), ceiling_pic (8 bytes)
    /// - light_level (i16), special (i16), tag (i16)
    ///
    /// C original: `P_LoadSectors()` em `p_setup.c`
    fn load_sectors(wad: &WadSystem, lump_index: usize) -> Result<Vec<Sector>, MapError> {
        let data = wad.read_lump(lump_index)?;
        let entry_size = 26; // sizeof(mapsector_t)
        if data.len() % entry_size != 0 {
            return Err(MapError::InvalidLumpSize {
                expected_entry_size: entry_size,
                actual_size: data.len(),
            });
        }

        let count = data.len() / entry_size;
        let mut reader = Cursor::new(&data);
        let mut sectors = Vec::with_capacity(count);

        for _ in 0..count {
            let floor_height = reader.read_i16::<LittleEndian>()?;
            let ceiling_height = reader.read_i16::<LittleEndian>()?;

            let mut floor_pic = [0u8; 8];
            std::io::Read::read_exact(&mut reader, &mut floor_pic)?;
            let mut ceiling_pic = [0u8; 8];
            std::io::Read::read_exact(&mut reader, &mut ceiling_pic)?;

            let light_level = reader.read_i16::<LittleEndian>()?;
            let special = reader.read_i16::<LittleEndian>()?;
            let tag = reader.read_i16::<LittleEndian>()?;

            sectors.push(Sector {
                floor_height: Fixed::from_int(floor_height as i32),
                ceiling_height: Fixed::from_int(ceiling_height as i32),
                floor_pic,
                ceiling_pic,
                light_level,
                special,
                tag,
            });
        }

        Ok(sectors)
    }

    /// Carrega sidedefs do lump SIDEDEFS.
    ///
    /// Cada sidedef on-disk tem 30 bytes:
    /// - texture_offset (i16), row_offset (i16)
    /// - top_texture (8 bytes), bottom_texture (8 bytes), mid_texture (8 bytes)
    /// - sector (i16)
    ///
    /// C original: `P_LoadSideDefs()` em `p_setup.c`
    fn load_sidedefs(wad: &WadSystem, lump_index: usize) -> Result<Vec<SideDef>, MapError> {
        let data = wad.read_lump(lump_index)?;
        let entry_size = 30; // sizeof(mapsidedef_t)
        if data.len() % entry_size != 0 {
            return Err(MapError::InvalidLumpSize {
                expected_entry_size: entry_size,
                actual_size: data.len(),
            });
        }

        let count = data.len() / entry_size;
        let mut reader = Cursor::new(&data);
        let mut sidedefs = Vec::with_capacity(count);

        for _ in 0..count {
            let texture_offset = reader.read_i16::<LittleEndian>()?;
            let row_offset = reader.read_i16::<LittleEndian>()?;

            let mut top_texture = [0u8; 8];
            std::io::Read::read_exact(&mut reader, &mut top_texture)?;
            let mut bottom_texture = [0u8; 8];
            std::io::Read::read_exact(&mut reader, &mut bottom_texture)?;
            let mut mid_texture = [0u8; 8];
            std::io::Read::read_exact(&mut reader, &mut mid_texture)?;

            let sector = reader.read_i16::<LittleEndian>()?;

            sidedefs.push(SideDef {
                texture_offset: Fixed::from_int(texture_offset as i32),
                row_offset: Fixed::from_int(row_offset as i32),
                top_texture,
                bottom_texture,
                mid_texture,
                sector_index: sector as usize,
            });
        }

        Ok(sidedefs)
    }

    /// Carrega linedefs do lump LINEDEFS.
    ///
    /// Cada linedef on-disk tem 14 bytes:
    /// - v1 (i16), v2 (i16), flags (i16), special (i16), tag (i16)
    /// - sidenum[0] (i16), sidenum[1] (i16)
    ///
    /// Alem de converter campos, calcula valores derivados:
    /// - dx, dy (delta entre vertices — precisaria dos vertices, mas o DOOM
    ///   calcula isso em P_LoadLineDefs com acesso a vertexes ja carregados.
    ///   Aqui fazemos o mesmo padrao, mas dx/dy serao preenchidos quando
    ///   os vertices estiverem disponiveis via `finalize_linedefs()`.)
    /// - slope_type (classificacao da inclinacao)
    /// - front_sector, back_sector (via sidedef -> sector)
    ///
    /// C original: `P_LoadLineDefs()` em `p_setup.c`
    fn load_linedefs(
        wad: &WadSystem,
        lump_index: usize,
        sidedefs: &[SideDef],
    ) -> Result<Vec<LineDef>, MapError> {
        let data = wad.read_lump(lump_index)?;
        let entry_size = 14; // sizeof(maplinedef_t)
        if data.len() % entry_size != 0 {
            return Err(MapError::InvalidLumpSize {
                expected_entry_size: entry_size,
                actual_size: data.len(),
            });
        }

        let count = data.len() / entry_size;
        let mut reader = Cursor::new(&data);
        let mut linedefs = Vec::with_capacity(count);

        for _ in 0..count {
            let v1 = reader.read_i16::<LittleEndian>()? as usize;
            let v2 = reader.read_i16::<LittleEndian>()? as usize;
            let flags_raw = reader.read_i16::<LittleEndian>()?;
            let special = reader.read_i16::<LittleEndian>()?;
            let tag = reader.read_i16::<LittleEndian>()?;
            let side0 = reader.read_i16::<LittleEndian>()?;
            let side1 = reader.read_i16::<LittleEndian>()?;

            // Converter sidenum: -1 (0xFFFF) significa sem sidedef
            // C original: `if (mld->sidenum[j] != -1)`
            let sidenum0 = if side0 == -1 {
                None
            } else {
                Some(side0 as usize)
            };
            let sidenum1 = if side1 == -1 {
                None
            } else {
                Some(side1 as usize)
            };

            let flags =
                LineDefFlags::from_bits_truncate(flags_raw);

            // Determinar sectors frontal e traseiro via sidedefs
            // C original: `ld->frontsector = sides[ld->sidenum[0]].sector`
            // C original: `if (sidenum[1] != -1) backsector = sides[sidenum[1]].sector`
            // backsector e definido por sidenum[1], nao pela flag TWO_SIDED
            let front_sector = sidenum0.map(|i| sidedefs[i].sector_index);
            let back_sector = sidenum1.map(|i| sidedefs[i].sector_index);

            // dx/dy e slope_type serao calculados em finalize()
            // quando os vertices estiverem disponiveis
            linedefs.push(LineDef {
                v1,
                v2,
                dx: Fixed::ZERO,
                dy: Fixed::ZERO,
                flags,
                special,
                tag,
                sidenum: [sidenum0, sidenum1],
                slope_type: SlopeType::Horizontal, // placeholder
                front_sector,
                back_sector,
            });
        }

        Ok(linedefs)
    }

    /// Carrega segs do lump SEGS.
    ///
    /// Cada seg on-disk tem 12 bytes:
    /// - v1 (i16), v2 (i16), angle (i16), linedef (i16), side (i16), offset (i16)
    ///
    /// O campo angle e convertido de i16 para u32 (BAM) via shift left de 16 bits.
    /// O front_sector e back_sector sao determinados pela linedef e pelo lado (side).
    ///
    /// C original: `P_LoadSegs()` em `p_setup.c`
    fn load_segs(
        wad: &WadSystem,
        lump_index: usize,
        linedefs: &[LineDef],
        sidedefs: &[SideDef],
    ) -> Result<Vec<Seg>, MapError> {
        let data = wad.read_lump(lump_index)?;
        let entry_size = 12; // sizeof(mapseg_t)
        if data.len() % entry_size != 0 {
            return Err(MapError::InvalidLumpSize {
                expected_entry_size: entry_size,
                actual_size: data.len(),
            });
        }

        let count = data.len() / entry_size;
        let mut reader = Cursor::new(&data);
        let mut segs = Vec::with_capacity(count);

        for _ in 0..count {
            let v1 = reader.read_i16::<LittleEndian>()? as usize;
            let v2 = reader.read_i16::<LittleEndian>()? as usize;
            let angle = reader.read_i16::<LittleEndian>()?;
            let linedef_index = reader.read_i16::<LittleEndian>()? as usize;
            let side = reader.read_i16::<LittleEndian>()?;
            let offset = reader.read_i16::<LittleEndian>()?;

            // Converter angle de i16 para BAM (u32)
            // C original: `li->angle = (unsigned)ml->angle << 16`
            let angle_bam = (angle as u16 as u32) << 16;

            let ld = &linedefs[linedef_index];

            // Determinar sidedef pelo lado (0 = frente, 1 = tras)
            // C original: `li->sidedef = &sides[ldef->sidenum[side]]`
            let sidedef_index = ld.sidenum[side as usize]
                .expect("seg referencia lado inexistente da linedef");

            // front_sector = sector do sidedef deste lado
            // C original: `li->frontsector = sides[ldef->sidenum[side]].sector`
            let front_sector = sidedefs[sidedef_index].sector_index;

            // back_sector: se a linedef tem flag TWO_SIDED, pegar o sector do outro lado
            // C original:
            // ```c
            // if (ldef->flags & ML_TWOSIDED)
            //     li->backsector = sides[ldef->sidenum[side^1]].sector;
            // else
            //     li->backsector = 0;  // NULL
            // ```
            let back_sector = if ld.flags.contains(LineDefFlags::TWO_SIDED) {
                ld.sidenum[side as usize ^ 1].map(|i| sidedefs[i].sector_index)
            } else {
                None
            };

            segs.push(Seg {
                v1,
                v2,
                offset: Fixed::from_int(offset as i32),
                angle: angle_bam,
                sidedef: sidedef_index,
                linedef: linedef_index,
                front_sector,
                back_sector,
            });
        }

        Ok(segs)
    }

    /// Carrega subsectors do lump SSECTORS.
    ///
    /// Cada subsector on-disk tem 4 bytes:
    /// - num_segs (i16), first_seg (i16)
    ///
    /// Nota: o campo `sector` do SubSector runtime e determinado pelo
    /// primeiro seg do subsector. E preenchido em `finalize()`.
    ///
    /// C original: `P_LoadSubsectors()` em `p_setup.c`
    fn load_subsectors(wad: &WadSystem, lump_index: usize) -> Result<Vec<SubSector>, MapError> {
        let data = wad.read_lump(lump_index)?;
        let entry_size = 4; // sizeof(mapsubsector_t)
        if data.len() % entry_size != 0 {
            return Err(MapError::InvalidLumpSize {
                expected_entry_size: entry_size,
                actual_size: data.len(),
            });
        }

        let count = data.len() / entry_size;
        let mut reader = Cursor::new(&data);
        let mut subsectors = Vec::with_capacity(count);

        for _ in 0..count {
            let num_segs = reader.read_i16::<LittleEndian>()? as usize;
            let first_seg = reader.read_i16::<LittleEndian>()? as usize;

            // sector sera preenchido em finalize() via segs[first_seg].front_sector
            subsectors.push(SubSector {
                sector: 0, // placeholder — sera preenchido em finalize()
                num_lines: num_segs,
                first_line: first_seg,
            });
        }

        Ok(subsectors)
    }

    /// Carrega nodes do lump NODES.
    ///
    /// Cada node on-disk tem 28 bytes:
    /// - x, y, dx, dy (4x i16) — partition line
    /// - bbox[2][4] (8x i16) — bounding boxes dos filhos
    /// - children[2] (2x u16) — filhos (indice de node ou subsector)
    ///
    /// C original: `P_LoadNodes()` em `p_setup.c`
    fn load_nodes(wad: &WadSystem, lump_index: usize) -> Result<Vec<Node>, MapError> {
        let data = wad.read_lump(lump_index)?;
        let entry_size = 28; // sizeof(mapnode_t)
        if data.len() % entry_size != 0 {
            return Err(MapError::InvalidLumpSize {
                expected_entry_size: entry_size,
                actual_size: data.len(),
            });
        }

        let count = data.len() / entry_size;
        let mut reader = Cursor::new(&data);
        let mut nodes = Vec::with_capacity(count);

        for _ in 0..count {
            let x = reader.read_i16::<LittleEndian>()?;
            let y = reader.read_i16::<LittleEndian>()?;
            let dx = reader.read_i16::<LittleEndian>()?;
            let dy = reader.read_i16::<LittleEndian>()?;

            // Bounding boxes: bbox[0] = direito, bbox[1] = esquerdo
            // Cada bbox tem 4 valores: top, bottom, left, right
            let mut bbox = [[Fixed::ZERO; 4]; 2];
            for child_bbox in &mut bbox {
                for val in child_bbox.iter_mut() {
                    let raw = reader.read_i16::<LittleEndian>()?;
                    *val = Fixed::from_int(raw as i32);
                }
            }

            let child0 = reader.read_u16::<LittleEndian>()?;
            let child1 = reader.read_u16::<LittleEndian>()?;

            nodes.push(Node {
                x: Fixed::from_int(x as i32),
                y: Fixed::from_int(y as i32),
                dx: Fixed::from_int(dx as i32),
                dy: Fixed::from_int(dy as i32),
                bbox,
                children: [child0, child1],
            });
        }

        Ok(nodes)
    }

    /// Carrega things do lump THINGS.
    ///
    /// Cada thing on-disk tem 10 bytes:
    /// - x (i16), y (i16), angle (i16), type (i16), options (i16)
    ///
    /// C original: `P_LoadThings()` em `p_setup.c`
    /// Nota: no DOOM original, P_LoadThings chama P_SpawnMapThing()
    /// para cada thing. Aqui, apenas carregamos os dados brutos.
    fn load_things(wad: &WadSystem, lump_index: usize) -> Result<Vec<Thing>, MapError> {
        let data = wad.read_lump(lump_index)?;
        let entry_size = 10; // sizeof(mapthing_t)
        if data.len() % entry_size != 0 {
            return Err(MapError::InvalidLumpSize {
                expected_entry_size: entry_size,
                actual_size: data.len(),
            });
        }

        let count = data.len() / entry_size;
        let mut reader = Cursor::new(&data);
        let mut things = Vec::with_capacity(count);

        for _ in 0..count {
            let x = reader.read_i16::<LittleEndian>()?;
            let y = reader.read_i16::<LittleEndian>()?;
            let angle = reader.read_i16::<LittleEndian>()?;
            let thing_type = reader.read_i16::<LittleEndian>()?;
            let options = reader.read_i16::<LittleEndian>()?;

            things.push(Thing {
                x: Fixed::from_int(x as i32),
                y: Fixed::from_int(y as i32),
                angle,
                thing_type,
                options,
            });
        }

        Ok(things)
    }

    /// Finaliza o carregamento calculando campos derivados.
    ///
    /// Preenche:
    /// - LineDef.dx, LineDef.dy (delta entre vertices)
    /// - LineDef.slope_type (classificacao da inclinacao)
    /// - SubSector.sector (sector do primeiro seg)
    ///
    /// C original: estas computacoes sao feitas inline em `P_LoadLineDefs()`
    /// e `P_LoadSubsectors()`, mas aqui as separamos para maior clareza.
    pub fn finalize(&mut self) {
        // Calcular dx, dy e slope_type das linedefs
        for ld in &mut self.linedefs {
            let v1 = &self.vertexes[ld.v1];
            let v2 = &self.vertexes[ld.v2];
            ld.dx = v2.x - v1.x;
            ld.dy = v2.y - v1.y;

            // Classificar inclinacao para otimizacao de colisao
            // C original: logica em `P_LoadLineDefs()`
            if ld.dx.0 == 0 {
                ld.slope_type = SlopeType::Vertical;
            } else if ld.dy.0 == 0 {
                ld.slope_type = SlopeType::Horizontal;
            } else if (ld.dy.0 ^ ld.dx.0) >= 0 {
                // Mesmo sinal: inclinacao positiva
                ld.slope_type = SlopeType::Positive;
            } else {
                ld.slope_type = SlopeType::Negative;
            }
        }

        // Preencher sector dos subsectors via primeiro seg
        // C original: `ss->sector = segs[ss->firstline].sidedef->sector`
        for ss in &mut self.subsectors {
            if ss.first_line < self.segs.len() {
                ss.sector = self.segs[ss.first_line].front_sector;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: cria um WAD sintetico em memoria com lumps de mapa.
    ///
    /// Constroi um arquivo WAD minimo com um marker lump seguido dos
    /// lumps de dados do mapa, permitindo testar o carregamento sem
    /// depender de um WAD real.
    fn create_test_wad(
        map_name: &str,
        vertexes: &[u8],
        sectors: &[u8],
        sidedefs: &[u8],
        linedefs: &[u8],
        segs: &[u8],
        ssectors: &[u8],
        nodes: &[u8],
        things: &[u8],
    ) -> Vec<u8> {
        // Lumps na ordem: marker, THINGS, LINEDEFS, SIDEDEFS, VERTEXES,
        //                 SEGS, SSECTORS, NODES, SECTORS
        let lumps_data: Vec<(&str, &[u8])> = vec![
            (map_name, &[]),   // marker
            ("THINGS", things),
            ("LINEDEFS", linedefs),
            ("SIDEDEFS", sidedefs),
            ("VERTEXES", vertexes),
            ("SEGS", segs),
            ("SSECTORS", ssectors),
            ("NODES", nodes),
            ("SECTORS", sectors),
        ];

        // Calcular offsets
        let header_size = 12u32;
        let mut data_offset = header_size;
        let mut offsets = Vec::new();
        for (_, lump_data) in &lumps_data {
            offsets.push(data_offset);
            data_offset += lump_data.len() as u32;
        }
        let dir_offset = data_offset;

        let mut wad = Vec::new();

        // Header: "IWAD" + num_lumps + dir_offset
        wad.extend_from_slice(b"IWAD");
        wad.extend_from_slice(&(lumps_data.len() as u32).to_le_bytes());
        wad.extend_from_slice(&dir_offset.to_le_bytes());

        // Dados dos lumps
        for (_, lump_data) in &lumps_data {
            wad.extend_from_slice(lump_data);
        }

        // Diretorio
        for (i, (name, lump_data)) in lumps_data.iter().enumerate() {
            wad.extend_from_slice(&offsets[i].to_le_bytes()); // offset
            wad.extend_from_slice(&(lump_data.len() as u32).to_le_bytes()); // size
            let mut name_bytes = [0u8; 8];
            for (j, b) in name.bytes().take(8).enumerate() {
                name_bytes[j] = b;
            }
            wad.extend_from_slice(&name_bytes); // name
        }

        wad
    }

    /// Cria um WAD de teste minimo e o salva em disco, retornando o caminho.
    /// Usa um nome unico por chamada para evitar conflitos entre testes paralelos.
    fn write_test_wad(wad_data: &[u8], test_name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("doom_map_test_{}.wad", test_name));
        std::fs::write(&path, wad_data).unwrap();
        path
    }

    /// Helper: monta os bytes de um vertice on-disk (4 bytes).
    fn vertex_bytes(x: i16, y: i16) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&x.to_le_bytes());
        v.extend_from_slice(&y.to_le_bytes());
        v
    }

    /// Helper: monta os bytes de um sector on-disk (26 bytes).
    fn sector_bytes(floor: i16, ceiling: i16, light: i16) -> Vec<u8> {
        let mut s = Vec::new();
        s.extend_from_slice(&floor.to_le_bytes());
        s.extend_from_slice(&ceiling.to_le_bytes());
        s.extend_from_slice(&[0u8; 8]); // floor_pic
        s.extend_from_slice(&[0u8; 8]); // ceiling_pic
        s.extend_from_slice(&light.to_le_bytes());
        s.extend_from_slice(&0i16.to_le_bytes()); // special
        s.extend_from_slice(&0i16.to_le_bytes()); // tag
        s
    }

    /// Helper: monta os bytes de um sidedef on-disk (30 bytes).
    fn sidedef_bytes(tex_offset: i16, row_offset: i16, sector: i16) -> Vec<u8> {
        let mut s = Vec::new();
        s.extend_from_slice(&tex_offset.to_le_bytes());
        s.extend_from_slice(&row_offset.to_le_bytes());
        s.extend_from_slice(&[0u8; 8]); // top_texture
        s.extend_from_slice(&[0u8; 8]); // bottom_texture
        s.extend_from_slice(&[0u8; 8]); // mid_texture
        s.extend_from_slice(&sector.to_le_bytes());
        s
    }

    /// Helper: monta os bytes de uma linedef on-disk (14 bytes).
    fn linedef_bytes(v1: i16, v2: i16, flags: i16, side0: i16, side1: i16) -> Vec<u8> {
        let mut l = Vec::new();
        l.extend_from_slice(&v1.to_le_bytes());
        l.extend_from_slice(&v2.to_le_bytes());
        l.extend_from_slice(&flags.to_le_bytes());
        l.extend_from_slice(&0i16.to_le_bytes()); // special
        l.extend_from_slice(&0i16.to_le_bytes()); // tag
        l.extend_from_slice(&side0.to_le_bytes());
        l.extend_from_slice(&side1.to_le_bytes());
        l
    }

    /// Helper: monta os bytes de um seg on-disk (12 bytes).
    fn seg_bytes(v1: i16, v2: i16, angle: i16, linedef: i16, side: i16, offset: i16) -> Vec<u8> {
        let mut s = Vec::new();
        s.extend_from_slice(&v1.to_le_bytes());
        s.extend_from_slice(&v2.to_le_bytes());
        s.extend_from_slice(&angle.to_le_bytes());
        s.extend_from_slice(&linedef.to_le_bytes());
        s.extend_from_slice(&side.to_le_bytes());
        s.extend_from_slice(&offset.to_le_bytes());
        s
    }

    /// Helper: monta os bytes de um subsector on-disk (4 bytes).
    fn subsector_bytes(num_segs: i16, first_seg: i16) -> Vec<u8> {
        let mut s = Vec::new();
        s.extend_from_slice(&num_segs.to_le_bytes());
        s.extend_from_slice(&first_seg.to_le_bytes());
        s
    }

    /// Helper: monta os bytes de um node on-disk (28 bytes).
    fn node_bytes(x: i16, y: i16, dx: i16, dy: i16, child0: u16, child1: u16) -> Vec<u8> {
        let mut n = Vec::new();
        n.extend_from_slice(&x.to_le_bytes());
        n.extend_from_slice(&y.to_le_bytes());
        n.extend_from_slice(&dx.to_le_bytes());
        n.extend_from_slice(&dy.to_le_bytes());
        n.extend_from_slice(&[0u8; 16]); // bbox (zeros)
        n.extend_from_slice(&child0.to_le_bytes());
        n.extend_from_slice(&child1.to_le_bytes());
        n
    }

    /// Helper: monta os bytes de um thing on-disk (10 bytes).
    fn thing_bytes(x: i16, y: i16, angle: i16, thing_type: i16, options: i16) -> Vec<u8> {
        let mut t = Vec::new();
        t.extend_from_slice(&x.to_le_bytes());
        t.extend_from_slice(&y.to_le_bytes());
        t.extend_from_slice(&angle.to_le_bytes());
        t.extend_from_slice(&thing_type.to_le_bytes());
        t.extend_from_slice(&options.to_le_bytes());
        t
    }

    /// Testa carregamento de vertices: coordenadas i16 -> fixed-point.
    #[test]
    fn load_vertexes_basic() {
        let mut verts = Vec::new();
        verts.extend(vertex_bytes(100, 200));
        verts.extend(vertex_bytes(-50, 300));

        let wad_data = create_test_wad("E1M1", &verts, &[], &[], &[], &[], &[], &[], &[]);
        let path = write_test_wad(&wad_data, "vertexes");

        let mut wad = WadSystem::new();
        wad.add_file(&path).unwrap();

        let lump_index = wad.find_lump("E1M1").unwrap();
        let vertexes = MapData::load_vertexes(&wad, lump_index + ML_VERTEXES).unwrap();

        assert_eq!(vertexes.len(), 2);
        assert_eq!(vertexes[0].x, Fixed::from_int(100));
        assert_eq!(vertexes[0].y, Fixed::from_int(200));
        assert_eq!(vertexes[1].x, Fixed::from_int(-50));
        assert_eq!(vertexes[1].y, Fixed::from_int(300));

        std::fs::remove_file(&path).ok();
    }

    /// Testa carregamento de sectors: alturas convertidas para fixed-point.
    #[test]
    fn load_sectors_basic() {
        let mut secs = Vec::new();
        secs.extend(sector_bytes(0, 128, 160));

        let wad_data = create_test_wad("E1M1", &[], &secs, &[], &[], &[], &[], &[], &[]);
        let path = write_test_wad(&wad_data, "sectors");

        let mut wad = WadSystem::new();
        wad.add_file(&path).unwrap();

        let lump_index = wad.find_lump("E1M1").unwrap();
        let sectors = MapData::load_sectors(&wad, lump_index + ML_SECTORS).unwrap();

        assert_eq!(sectors.len(), 1);
        assert_eq!(sectors[0].floor_height, Fixed::from_int(0));
        assert_eq!(sectors[0].ceiling_height, Fixed::from_int(128));
        assert_eq!(sectors[0].light_level, 160);

        std::fs::remove_file(&path).ok();
    }

    /// Testa carregamento de things: posicoes e tipo.
    #[test]
    fn load_things_basic() {
        let mut th = Vec::new();
        // Player 1 start: tipo 1, angulo 90 graus
        th.extend(thing_bytes(100, 200, 90, 1, 7));

        let wad_data = create_test_wad("E1M1", &[], &[], &[], &[], &[], &[], &[], &th);
        let path = write_test_wad(&wad_data, "things");

        let mut wad = WadSystem::new();
        wad.add_file(&path).unwrap();

        let lump_index = wad.find_lump("E1M1").unwrap();
        let things = MapData::load_things(&wad, lump_index + ML_THINGS).unwrap();

        assert_eq!(things.len(), 1);
        assert_eq!(things[0].x, Fixed::from_int(100));
        assert_eq!(things[0].y, Fixed::from_int(200));
        assert_eq!(things[0].angle, 90);
        assert_eq!(things[0].thing_type, 1);
        assert_eq!(things[0].options, 7);

        std::fs::remove_file(&path).ok();
    }

    /// Testa carregamento completo de um mapa minimo com todos os tipos.
    #[test]
    fn load_complete_map() {
        // Criar um mapa minimo: triangulo com 3 vertices, 1 sector,
        // 3 sidedefs, 3 linedefs, 1 seg, 1 subsector, 1 node, 1 thing.
        let mut verts = Vec::new();
        verts.extend(vertex_bytes(0, 0));     // v0
        verts.extend(vertex_bytes(100, 0));   // v1
        verts.extend(vertex_bytes(50, 100));  // v2

        let sectors_data = sector_bytes(0, 128, 200);

        let mut sides = Vec::new();
        sides.extend(sidedef_bytes(0, 0, 0)); // side 0 -> sector 0
        sides.extend(sidedef_bytes(0, 0, 0)); // side 1 -> sector 0
        sides.extend(sidedef_bytes(0, 0, 0)); // side 2 -> sector 0

        let mut lines = Vec::new();
        lines.extend(linedef_bytes(0, 1, 1, 0, -1)); // v0->v1, BLOCKING, one-sided
        lines.extend(linedef_bytes(1, 2, 1, 1, -1)); // v1->v2, BLOCKING, one-sided
        lines.extend(linedef_bytes(2, 0, 1, 2, -1)); // v2->v0, BLOCKING, one-sided

        let segs_data = seg_bytes(0, 1, 0, 0, 0, 0); // seg do linedef 0, lado 0

        let ssectors_data = subsector_bytes(1, 0); // 1 seg, comecando no seg 0

        let nodes_data = node_bytes(50, 0, 0, 100, 0x8000, 0x8000); // leaf nodes

        let things_data = thing_bytes(50, 50, 0, 1, 7); // player start

        let wad_data = create_test_wad(
            "E1M1",
            &verts,
            &sectors_data,
            &sides,
            &lines,
            &segs_data,
            &ssectors_data,
            &nodes_data,
            &things_data,
        );
        let path = write_test_wad(&wad_data, "complete");

        let mut wad = WadSystem::new();
        wad.add_file(&path).unwrap();

        let mut map = MapData::load("E1M1", &wad).unwrap();
        map.finalize();

        assert_eq!(map.vertexes.len(), 3);
        assert_eq!(map.sectors.len(), 1);
        assert_eq!(map.sidedefs.len(), 3);
        assert_eq!(map.linedefs.len(), 3);
        assert_eq!(map.segs.len(), 1);
        assert_eq!(map.subsectors.len(), 1);
        assert_eq!(map.nodes.len(), 1);
        assert_eq!(map.things.len(), 1);

        // Verificar dx/dy calculados na finalize()
        // linedef 0: v0(0,0) -> v1(100,0) => dx=100, dy=0 => Horizontal
        assert_eq!(map.linedefs[0].dx, Fixed::from_int(100));
        assert_eq!(map.linedefs[0].dy, Fixed::from_int(0));
        assert_eq!(map.linedefs[0].slope_type, SlopeType::Horizontal);

        // linedef 2: v2(50,100) -> v0(0,0) => dx=-50, dy=-100 => Positive (same sign)
        assert_eq!(map.linedefs[2].dx, Fixed::from_int(-50));
        assert_eq!(map.linedefs[2].dy, Fixed::from_int(-100));
        assert_eq!(map.linedefs[2].slope_type, SlopeType::Positive);

        // Verificar que subsector.sector foi preenchido
        assert_eq!(map.subsectors[0].sector, 0);

        std::fs::remove_file(&path).ok();
    }

    /// Testa que mapa inexistente retorna erro.
    #[test]
    fn load_map_not_found() {
        let wad_data = create_test_wad("E1M1", &[], &[], &[], &[], &[], &[], &[], &[]);
        let path = write_test_wad(&wad_data, "notfound");

        let mut wad = WadSystem::new();
        wad.add_file(&path).unwrap();

        let result = MapData::load("E2M1", &wad);
        assert!(matches!(result, Err(MapError::MapNotFound(_))));

        std::fs::remove_file(&path).ok();
    }

    /// Testa que lump com tamanho invalido retorna erro.
    #[test]
    fn load_vertexes_invalid_size() {
        // 5 bytes = nao e multiplo de 4
        let verts = vec![0u8; 5];
        let wad_data = create_test_wad("E1M1", &verts, &[], &[], &[], &[], &[], &[], &[]);
        let path = write_test_wad(&wad_data, "invalid_size");

        let mut wad = WadSystem::new();
        wad.add_file(&path).unwrap();

        let lump_index = wad.find_lump("E1M1").unwrap();
        let result = MapData::load_vertexes(&wad, lump_index + ML_VERTEXES);
        assert!(matches!(result, Err(MapError::InvalidLumpSize { .. })));

        std::fs::remove_file(&path).ok();
    }

    /// Testa linedef com TWO_SIDED: deve ter back_sector preenchido.
    #[test]
    fn linedef_two_sided() {
        let mut verts = Vec::new();
        verts.extend(vertex_bytes(0, 0));
        verts.extend(vertex_bytes(100, 0));

        let mut secs = Vec::new();
        secs.extend(sector_bytes(0, 128, 200));   // sector 0
        secs.extend(sector_bytes(0, 256, 160));   // sector 1

        let mut sides = Vec::new();
        sides.extend(sidedef_bytes(0, 0, 0)); // side 0 -> sector 0
        sides.extend(sidedef_bytes(0, 0, 1)); // side 1 -> sector 1

        // flags = TWO_SIDED (4), side0=0, side1=1
        let lines = linedef_bytes(0, 1, 4, 0, 1);

        let wad_data = create_test_wad("E1M1", &verts, &secs, &sides, &lines, &[], &[], &[], &[]);
        let path = write_test_wad(&wad_data, "two_sided");

        let mut wad = WadSystem::new();
        wad.add_file(&path).unwrap();

        let lump_index = wad.find_lump("E1M1").unwrap();
        let sidedefs = MapData::load_sidedefs(&wad, lump_index + ML_SIDEDEFS).unwrap();
        let linedefs = MapData::load_linedefs(&wad, lump_index + ML_LINEDEFS, &sidedefs).unwrap();

        assert_eq!(linedefs.len(), 1);
        assert!(linedefs[0].flags.contains(LineDefFlags::TWO_SIDED));
        assert_eq!(linedefs[0].front_sector, Some(0));
        assert_eq!(linedefs[0].back_sector, Some(1));

        std::fs::remove_file(&path).ok();
    }
}

//! # Dados de Textura e Patches
//!
//! Carrega e gerencia as texturas e sprites do WAD para uso no rendering.
//!
//! ## Hierarquia de dados visuais do DOOM
//!
//! - **Patches**: imagens individuais armazenadas como colunas de pixels
//!   com transparencia (posts). Ex: um pedaco de tijolo de uma parede.
//! - **Texturas**: composicoes de um ou mais patches organizados numa
//!   area retangular. Definidas nos lumps TEXTURE1/TEXTURE2.
//! - **Flats**: texturas de piso/teto, sempre 64x64, sem transparencia,
//!   armazenadas como array linear de bytes entre F_START e F_END.
//! - **Sprites**: patches de personagens/itens, entre S_START e S_END.
//! - **Colormaps**: tabelas de iluminacao (32 niveis x 256 cores).
//!
//! ## Formato de composicao de texturas
//!
//! O lump PNAMES lista os nomes dos patches disponiveis.
//! O lump TEXTURE1 (e opcionalmente TEXTURE2) define como esses
//! patches sao compostos em texturas de parede:
//!
//! ```text
//! TEXTURE1 lump:
//!   numtextures (i32)
//!   offsets[numtextures] (i32 cada) — offset de cada definicao
//!   Para cada textura:
//!     name (8 bytes), masked (i32), width (i16), height (i16)
//!     columndirectory (i32) — obsoleto
//!     patchcount (i16)
//!     patches[patchcount]: originx (i16), originy (i16),
//!                          patch (i16), stepdir (i16), colormap (i16)
//! ```
//!
//! ## Arquivo C original: `r_data.c` / `r_data.h`
//!
//! ## Conceitos que o leitor vai aprender
//! - Composicao de texturas a partir de patches
//! - Column-based storage para rendering vertical eficiente
//! - Lookup tables para acesso rapido a colunas de textura

use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

use crate::utils::fixed::{Fixed, FRACBITS};
use crate::wad::WadSystem;

/// Tipo para dimensoes de sprites: (widths, offsets, top_offsets).
type SpriteDimensions = (Vec<Fixed>, Vec<Fixed>, Vec<Fixed>);

/// Patch de textura dentro de uma definicao de textura composta.
///
/// Descreve onde um patch individual e posicionado dentro da textura.
///
/// C original: `texpatch_t` em `r_data.c`
#[derive(Debug, Clone)]
pub struct TexturePatch {
    /// Origem X do patch dentro da textura
    pub origin_x: i32,
    /// Origem Y do patch dentro da textura
    pub origin_y: i32,
    /// Indice do lump do patch no WAD
    pub patch_lump: usize,
}

/// Definicao de uma textura composta.
///
/// Uma textura e uma area retangular composta por um ou mais patches.
/// No rendering, a textura e acessada coluna por coluna.
///
/// C original: `texture_t` em `r_data.c`
#[derive(Debug, Clone)]
pub struct TextureDef {
    /// Nome da textura (ate 8 caracteres)
    pub name: [u8; 8],
    /// Largura em pixels
    pub width: i16,
    /// Altura em pixels
    pub height: i16,
    /// Lista de patches que compoem a textura
    pub patches: Vec<TexturePatch>,
}

/// Numero de niveis de iluminacao no DOOM.
///
/// O DOOM usa 32 niveis de luz (colormaps), do mais escuro (0)
/// ao mais claro (31). Cada nivel e uma tabela de 256 bytes que
/// mapeia indices de cor para versoes mais escuras.
///
/// C original: `NUMCOLORMAPS` nao e definido explicitamente, mas
/// o COLORMAP lump tem 34 tabelas (32 + fullbright + dark).
pub const NUMCOLORMAPS: usize = 32;

/// Dados de textura carregados do WAD.
///
/// Equivalente ao conjunto de globals em `r_data.c`:
/// - `textures[]`, `numtextures`
/// - `textureheight[]`, `texturewidthmask[]`
/// - `firstflat`, `lastflat`, `numflats`
/// - `firstspritelump`, `lastspritelump`
/// - `colormaps`
/// - `spritewidth[]`, `spriteoffset[]`, `spritetopoffset[]`
#[derive(Debug)]
pub struct TextureData {
    /// Definicoes de todas as texturas compostas
    pub textures: Vec<TextureDef>,
    /// Altura de cada textura em fixed-point (para pegging)
    pub texture_height: Vec<Fixed>,
    /// Mascara de largura (potencia de 2 - 1) para wrap horizontal
    pub texture_width_mask: Vec<i32>,
    /// Tabela de traducao de texturas (para animacao)
    pub texture_translation: Vec<usize>,

    /// Indice do primeiro lump de flat no WAD
    pub first_flat: usize,
    /// Numero total de flats
    pub num_flats: usize,
    /// Tabela de traducao de flats (para animacao)
    pub flat_translation: Vec<usize>,

    /// Indice do primeiro lump de sprite no WAD
    pub first_sprite_lump: usize,
    /// Numero total de lumps de sprite
    pub num_sprite_lumps: usize,
    /// Largura de cada sprite em fixed-point
    pub sprite_width: Vec<Fixed>,
    /// Offset horizontal de cada sprite em fixed-point
    pub sprite_offset: Vec<Fixed>,
    /// Offset vertical (topo) de cada sprite em fixed-point
    pub sprite_top_offset: Vec<Fixed>,

    /// Colormaps (tabelas de iluminacao).
    /// 34 tabelas de 256 bytes cada (32 niveis + fullbright + invulnerability).
    /// C original: `lighttable_t* colormaps` em `r_data.c`
    pub colormaps: Vec<u8>,
}

impl TextureData {
    /// Carrega todos os dados de textura do WAD.
    ///
    /// C original: `R_InitData()` em `r_data.c`, que chama:
    /// - `R_InitTextures()` — carrega PNAMES, TEXTURE1/2
    /// - `R_InitFlats()` — localiza F_START/F_END
    /// - `R_InitSpriteLumps()` — localiza S_START/S_END, le dimensoes
    pub fn load(wad: &WadSystem) -> Result<Self, Box<dyn std::error::Error>> {
        let textures = Self::load_textures(wad)?;
        let texture_height: Vec<Fixed> = textures
            .iter()
            .map(|t| Fixed(i32::from(t.height) << FRACBITS))
            .collect();
        let texture_width_mask: Vec<i32> = textures
            .iter()
            .map(|t| {
                let mut j = 1i32;
                while j * 2 <= i32::from(t.width) {
                    j <<= 1;
                }
                j - 1
            })
            .collect();
        let num_textures = textures.len();
        let texture_translation: Vec<usize> = (0..num_textures).collect();

        let (first_flat, num_flats) = Self::find_flat_range(wad)?;
        let flat_translation: Vec<usize> = (0..num_flats).collect();

        let (first_sprite_lump, num_sprite_lumps) = Self::find_sprite_range(wad)?;
        let (sprite_width, sprite_offset, sprite_top_offset) =
            Self::load_sprite_dimensions(wad, first_sprite_lump, num_sprite_lumps)?;

        let colormaps = Self::load_colormaps(wad)?;

        log::info!(
            "Dados de textura carregados: {} texturas, {} flats, {} sprites",
            num_textures,
            num_flats,
            num_sprite_lumps,
        );

        Ok(TextureData {
            textures,
            texture_height,
            texture_width_mask,
            texture_translation,
            first_flat,
            num_flats,
            flat_translation,
            first_sprite_lump,
            num_sprite_lumps,
            sprite_width,
            sprite_offset,
            sprite_top_offset,
            colormaps,
        })
    }

    /// Carrega definicoes de texturas dos lumps PNAMES e TEXTURE1/2.
    ///
    /// C original: `R_InitTextures()` em `r_data.c`
    fn load_textures(wad: &WadSystem) -> Result<Vec<TextureDef>, Box<dyn std::error::Error>> {
        // 1. Carregar PNAMES — tabela de nomes dos patches
        let pnames_data = wad.read_lump_by_name("PNAMES")?;
        let mut pnames_reader = Cursor::new(&pnames_data);
        let num_patches = pnames_reader.read_i32::<LittleEndian>()? as usize;

        let mut patch_lookup = Vec::with_capacity(num_patches);
        for _ in 0..num_patches {
            let mut name = [0u8; 8];
            std::io::Read::read_exact(&mut pnames_reader, &mut name)?;
            // Buscar o lump do patch pelo nome
            let end = name.iter().position(|&b| b == 0).unwrap_or(8);
            let name_str = std::str::from_utf8(&name[..end]).unwrap_or("");
            let lump_index = wad.find_lump(name_str).unwrap_or(0);
            patch_lookup.push(lump_index);
        }

        // 2. Carregar TEXTURE1 (e opcionalmente TEXTURE2)
        let mut textures = Vec::new();

        let tex1_data = wad.read_lump_by_name("TEXTURE1")?;
        Self::parse_texture_lump(&tex1_data, &patch_lookup, &mut textures)?;

        if let Ok(tex2_data) = wad.read_lump_by_name("TEXTURE2") {
            Self::parse_texture_lump(&tex2_data, &patch_lookup, &mut textures)?;
        }

        Ok(textures)
    }

    /// Faz o parse de um lump TEXTUREx (TEXTURE1 ou TEXTURE2).
    fn parse_texture_lump(
        data: &[u8],
        patch_lookup: &[usize],
        textures: &mut Vec<TextureDef>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut reader = Cursor::new(data);
        let num_textures = reader.read_i32::<LittleEndian>()? as usize;

        // Ler offsets de cada definicao de textura
        let mut offsets = Vec::with_capacity(num_textures);
        for _ in 0..num_textures {
            offsets.push(reader.read_i32::<LittleEndian>()? as usize);
        }

        // Parse cada definicao
        for &offset in &offsets {
            if offset + 22 > data.len() {
                continue;
            }
            let mut tex_reader = Cursor::new(&data[offset..]);

            let mut name = [0u8; 8];
            std::io::Read::read_exact(&mut tex_reader, &mut name)?;
            let _masked = tex_reader.read_i32::<LittleEndian>()?;
            let width = tex_reader.read_i16::<LittleEndian>()?;
            let height = tex_reader.read_i16::<LittleEndian>()?;
            let _columndirectory = tex_reader.read_i32::<LittleEndian>()?; // obsoleto
            let patchcount = tex_reader.read_i16::<LittleEndian>()? as usize;

            let mut patches = Vec::with_capacity(patchcount);
            for _ in 0..patchcount {
                let origin_x = tex_reader.read_i16::<LittleEndian>()? as i32;
                let origin_y = tex_reader.read_i16::<LittleEndian>()? as i32;
                let patch_idx = tex_reader.read_i16::<LittleEndian>()? as usize;
                let _stepdir = tex_reader.read_i16::<LittleEndian>()?;
                let _colormap = tex_reader.read_i16::<LittleEndian>()?;

                let patch_lump = if patch_idx < patch_lookup.len() {
                    patch_lookup[patch_idx]
                } else {
                    0
                };

                patches.push(TexturePatch {
                    origin_x,
                    origin_y,
                    patch_lump,
                });
            }

            textures.push(TextureDef {
                name,
                width,
                height,
                patches,
            });
        }

        Ok(())
    }

    /// Localiza o range de flats (entre F_START e F_END).
    ///
    /// C original: `R_InitFlats()` em `r_data.c`
    fn find_flat_range(wad: &WadSystem) -> Result<(usize, usize), Box<dyn std::error::Error>> {
        let first = wad.get_lump_index("F_START")? + 1;
        let last = wad.get_lump_index("F_END")? - 1;
        let count = if last >= first { last - first + 1 } else { 0 };
        Ok((first, count))
    }

    /// Localiza o range de sprites (entre S_START e S_END).
    ///
    /// C original: `R_InitSpriteLumps()` em `r_data.c`
    fn find_sprite_range(wad: &WadSystem) -> Result<(usize, usize), Box<dyn std::error::Error>> {
        let first = wad.get_lump_index("S_START")? + 1;
        let last = wad.get_lump_index("S_END")? - 1;
        let count = if last >= first { last - first + 1 } else { 0 };
        Ok((first, count))
    }

    /// Carrega dimensoes de todos os sprites.
    ///
    /// Para cada sprite, le o header do patch para extrair
    /// largura, leftoffset e topoffset, convertendo para fixed-point.
    ///
    /// C original: parte de `R_InitSpriteLumps()` em `r_data.c`
    fn load_sprite_dimensions(
        wad: &WadSystem,
        first_lump: usize,
        count: usize,
    ) -> Result<SpriteDimensions, Box<dyn std::error::Error>> {
        let mut widths = Vec::with_capacity(count);
        let mut offsets = Vec::with_capacity(count);
        let mut top_offsets = Vec::with_capacity(count);

        for i in 0..count {
            let data = wad.read_lump(first_lump + i)?;
            if data.len() >= 8 {
                let width = i16::from_le_bytes([data[0], data[1]]);
                // data[2..4] = height (nao precisamos)
                let left_offset = i16::from_le_bytes([data[4], data[5]]);
                let top_offset = i16::from_le_bytes([data[6], data[7]]);

                widths.push(Fixed(i32::from(width) << FRACBITS));
                offsets.push(Fixed(i32::from(left_offset) << FRACBITS));
                top_offsets.push(Fixed(i32::from(top_offset) << FRACBITS));
            } else {
                widths.push(Fixed::ZERO);
                offsets.push(Fixed::ZERO);
                top_offsets.push(Fixed::ZERO);
            }
        }

        Ok((widths, offsets, top_offsets))
    }

    /// Carrega o lump COLORMAP do WAD.
    ///
    /// O COLORMAP contem 34 tabelas de 256 bytes:
    /// - Tabelas 0-31: niveis de iluminacao (0 = fullbright, 31 = escuro total)
    /// - Tabela 32: full bright (sem atenuacao)
    /// - Tabela 33: invulnerability (inversao de cores)
    ///
    /// C original: parte de `R_InitColormaps()` chamada de `R_InitData()`
    fn load_colormaps(wad: &WadSystem) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let data = wad.read_lump_by_name("COLORMAP")?;
        Ok(data)
    }

    /// Retorna o nome de uma textura como string.
    pub fn texture_name(&self, index: usize) -> String {
        let name = &self.textures[index].name;
        let end = name.iter().position(|&b| b == 0).unwrap_or(8);
        String::from_utf8_lossy(&name[..end]).to_string()
    }

    /// Busca uma textura pelo nome, retornando seu indice.
    ///
    /// C original: `R_TextureNumForName()` em `r_data.c`
    pub fn texture_num_for_name(&self, name: &str) -> Option<usize> {
        let search = {
            let mut buf = [0u8; 8];
            for (i, b) in name.bytes().take(8).enumerate() {
                buf[i] = b.to_ascii_uppercase();
            }
            buf
        };

        self.textures.iter().position(|t| {
            let mut t_name = t.name;
            for b in &mut t_name {
                *b = b.to_ascii_uppercase();
            }
            t_name == search
        })
    }

    /// Retorna o indice do lump de um flat pelo nome.
    ///
    /// C original: `R_FlatNumForName()` em `r_data.c`
    pub fn flat_num_for_name(&self, name: &str, wad: &WadSystem) -> Option<usize> {
        let index = wad.find_lump(name)?;
        if index >= self.first_flat && index < self.first_flat + self.num_flats {
            Some(index - self.first_flat)
        } else {
            None
        }
    }

    /// Retorna o indice de textura traduzido (para animacao).
    pub fn translated_texture(&self, index: usize) -> usize {
        self.texture_translation[index]
    }

    /// Retorna o indice de flat traduzido (para animacao).
    pub fn translated_flat(&self, index: usize) -> usize {
        self.flat_translation[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifica parse de um lump TEXTURE1 sintetico.
    #[test]
    fn parse_texture_lump_basic() {
        // Criar um lump TEXTURE1 minimo com 1 textura, 1 patch
        let mut data = Vec::new();
        // numtextures = 1
        data.extend_from_slice(&1i32.to_le_bytes());
        // offsets[0] = 8 (logo apos header + offset array)
        data.extend_from_slice(&8i32.to_le_bytes());

        // Textura: name="WALL0001", masked=0, width=64, height=128,
        //          columndirectory=0, patchcount=1
        data.extend_from_slice(b"WALL0001");      // name
        data.extend_from_slice(&0i32.to_le_bytes());   // masked
        data.extend_from_slice(&64i16.to_le_bytes());  // width
        data.extend_from_slice(&128i16.to_le_bytes()); // height
        data.extend_from_slice(&0i32.to_le_bytes());   // columndirectory
        data.extend_from_slice(&1i16.to_le_bytes());   // patchcount

        // Patch: originx=0, originy=0, patch=0, stepdir=0, colormap=0
        data.extend_from_slice(&0i16.to_le_bytes());   // originx
        data.extend_from_slice(&0i16.to_le_bytes());   // originy
        data.extend_from_slice(&0i16.to_le_bytes());   // patch index
        data.extend_from_slice(&0i16.to_le_bytes());   // stepdir
        data.extend_from_slice(&0i16.to_le_bytes());   // colormap

        let patch_lookup = vec![42usize]; // patch 0 -> lump 42
        let mut textures = Vec::new();
        TextureData::parse_texture_lump(&data, &patch_lookup, &mut textures).unwrap();

        assert_eq!(textures.len(), 1);
        assert_eq!(&textures[0].name, b"WALL0001");
        assert_eq!(textures[0].width, 64);
        assert_eq!(textures[0].height, 128);
        assert_eq!(textures[0].patches.len(), 1);
        assert_eq!(textures[0].patches[0].origin_x, 0);
        assert_eq!(textures[0].patches[0].patch_lump, 42);
    }

    /// Verifica texture_num_for_name com busca case-insensitive.
    #[test]
    fn texture_lookup_by_name() {
        let tex = TextureDef {
            name: *b"STARTAN3",
            width: 64,
            height: 128,
            patches: vec![],
        };

        let td = TextureData {
            textures: vec![tex],
            texture_height: vec![Fixed(128 << FRACBITS)],
            texture_width_mask: vec![63],
            texture_translation: vec![0],
            first_flat: 0,
            num_flats: 0,
            flat_translation: vec![],
            first_sprite_lump: 0,
            num_sprite_lumps: 0,
            sprite_width: vec![],
            sprite_offset: vec![],
            sprite_top_offset: vec![],
            colormaps: vec![],
        };

        assert_eq!(td.texture_num_for_name("STARTAN3"), Some(0));
        assert_eq!(td.texture_num_for_name("startan3"), Some(0));
        assert_eq!(td.texture_num_for_name("NOEXIST"), None);
    }

    /// Verifica calculo de width mask (potencia de 2 - 1).
    #[test]
    fn width_mask_calculation() {
        // Largura 64: j=64, mask=63
        let mut j = 1i32;
        while j * 2 <= 64 {
            j <<= 1;
        }
        assert_eq!(j - 1, 63);

        // Largura 128: j=128, mask=127
        j = 1;
        while j * 2 <= 128 {
            j <<= 1;
        }
        assert_eq!(j - 1, 127);

        // Largura 72 (nao potencia de 2): j=64, mask=63
        j = 1;
        while j * 2 <= 72 {
            j <<= 1;
        }
        assert_eq!(j - 1, 63);
    }
}

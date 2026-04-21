//! # Primitivas de Desenho — Colunas e Spans
//!
//! Funcoes que escrevem pixels diretamente no framebuffer.
//! Todo o rendering do DOOM se reduz a duas operacoes basicas:
//!
//! - **Colunas** (R_DrawColumn): faixas verticais de textura de parede.
//!   Uma coluna e um slice vertical de uma textura, escalado e mapeado
//!   para uma faixa vertical na tela. Cada pixel da coluna passa por
//!   um colormap (light table) para simular iluminacao por distancia.
//!
//! - **Spans** (R_DrawSpan): faixas horizontais de textura de piso/teto.
//!   Um span e uma linha horizontal da tela preenchida com uma textura
//!   de piso (flat), mapeada com perspectiva e iluminacao.
//!
//! Variantes especiais:
//! - `R_DrawFuzzColumn`: efeito de invisibilidade (Spectre)
//! - `R_DrawTranslatedColumn`: sprites de jogador com cor remapeada
//!
//! ## Arquivo C original: `r_draw.c`
//!
//! ## Conceitos que o leitor vai aprender
//! - DDA (Digital Differential Analyzer) para escalamento de texturas
//! - Colormap como lookup table para iluminacao
//! - Flat textures (64x64) com mapeamento affine

use crate::utils::fixed::{Fixed, FRACBITS};
use crate::video::{SCREENHEIGHT, SCREENWIDTH};

/// Tamanho maximo da view window em pixels (largura).
const MAXWIDTH: usize = 1120;

/// Tamanho maximo da view window em pixels (altura).
const MAXHEIGHT: usize = 832;

/// Tamanho da tabela de fuzz effect (efeito de invisibilidade).
///
/// C original: `#define FUZZTABLE 50` em `r_draw.c`
const FUZZTABLE: usize = 50;

/// Tabela de offsets para o efeito fuzz (invisibilidade/Spectre).
///
/// Cada valor e +SCREENWIDTH ou -SCREENWIDTH, causando o efeito
/// de "borrão" ao ler pixels das linhas adjacentes.
///
/// C original: `int fuzzoffset[FUZZTABLE]` em `r_draw.c`
static FUZZ_OFFSET: [i32; FUZZTABLE] = [
    SCREENWIDTH as i32, -(SCREENWIDTH as i32),
    SCREENWIDTH as i32, -(SCREENWIDTH as i32),
    SCREENWIDTH as i32, SCREENWIDTH as i32,
    -(SCREENWIDTH as i32), SCREENWIDTH as i32,
    SCREENWIDTH as i32, -(SCREENWIDTH as i32),
    SCREENWIDTH as i32, SCREENWIDTH as i32,
    SCREENWIDTH as i32, -(SCREENWIDTH as i32),
    SCREENWIDTH as i32, SCREENWIDTH as i32,
    SCREENWIDTH as i32, -(SCREENWIDTH as i32),
    -(SCREENWIDTH as i32), -(SCREENWIDTH as i32),
    -(SCREENWIDTH as i32), SCREENWIDTH as i32,
    -(SCREENWIDTH as i32), -(SCREENWIDTH as i32),
    SCREENWIDTH as i32, SCREENWIDTH as i32,
    SCREENWIDTH as i32, SCREENWIDTH as i32,
    -(SCREENWIDTH as i32), SCREENWIDTH as i32,
    -(SCREENWIDTH as i32), SCREENWIDTH as i32,
    SCREENWIDTH as i32, -(SCREENWIDTH as i32),
    -(SCREENWIDTH as i32), SCREENWIDTH as i32,
    SCREENWIDTH as i32, -(SCREENWIDTH as i32),
    -(SCREENWIDTH as i32), -(SCREENWIDTH as i32),
    -(SCREENWIDTH as i32), SCREENWIDTH as i32,
    SCREENWIDTH as i32, SCREENWIDTH as i32,
    SCREENWIDTH as i32, -(SCREENWIDTH as i32),
    SCREENWIDTH as i32, SCREENWIDTH as i32,
    -(SCREENWIDTH as i32), SCREENWIDTH as i32,
];

/// Estado para desenhar uma coluna vertical de textura.
///
/// No C original, estes eram globals separados em `r_draw.c`.
/// Em Rust, agrupamos em uma struct para evitar estado global.
///
/// C original: `dc_colormap`, `dc_x`, `dc_yl`, `dc_yh`,
///             `dc_iscale`, `dc_texturemid`, `dc_source`
#[derive(Debug, Clone)]
pub struct ColumnDrawer {
    /// Tabela ylookup: offset do inicio de cada linha no framebuffer.
    /// C original: `byte* ylookup[MAXHEIGHT]` em `r_draw.c`
    pub ylookup: Vec<usize>,

    /// Tabela columnofs: offset de cada coluna (para view window).
    /// C original: `int columnofs[MAXWIDTH]` em `r_draw.c`
    pub columnofs: Vec<usize>,

    /// Largura da view window atual.
    pub view_width: usize,

    /// Altura da view window atual.
    pub view_height: usize,

    /// Offset X da view window no framebuffer.
    /// C original: `int viewwindowx` em `r_draw.c`
    pub view_window_x: usize,

    /// Offset Y da view window no framebuffer.
    /// C original: `int viewwindowy` em `r_draw.c`
    pub view_window_y: usize,

    /// Centro Y da tela em pixels (para calculo de perspectiva).
    pub center_y: i32,

    /// Posicao na tabela de fuzz.
    /// C original: `int fuzzpos` em `r_draw.c`
    fuzz_pos: usize,
}

impl ColumnDrawer {
    /// Cria um novo drawer com dimensoes padrao (320x200 fullscreen).
    pub fn new() -> Self {
        let mut drawer = ColumnDrawer {
            ylookup: vec![0; MAXHEIGHT],
            columnofs: vec![0; MAXWIDTH],
            view_width: SCREENWIDTH,
            view_height: SCREENHEIGHT,
            view_window_x: 0,
            view_window_y: 0,
            center_y: (SCREENHEIGHT / 2) as i32,
            fuzz_pos: 0,
        };
        drawer.init_buffer(SCREENWIDTH, SCREENHEIGHT);
        drawer
    }

    /// Inicializa as lookup tables de posicao no framebuffer.
    ///
    /// Cria tabelas que evitam multiplicacoes no inner loop do rendering:
    /// - `ylookup[y]` = offset do inicio da linha y no framebuffer
    /// - `columnofs[x]` = offset da coluna x (viewwindowx + x)
    ///
    /// C original: `R_InitBuffer()` em `r_draw.c`
    pub fn init_buffer(&mut self, width: usize, height: usize) {
        self.view_width = width;
        self.view_height = height;

        // Calcular offset X da view window (centralizada)
        self.view_window_x = (SCREENWIDTH - width) >> 1;

        // Column offsets
        for i in 0..width {
            self.columnofs[i] = self.view_window_x + i;
        }

        // Row offsets
        if width == SCREENWIDTH {
            self.view_window_y = 0;
        } else {
            self.view_window_y = (SCREENHEIGHT - 32 - height) >> 1; // 32 = SBARHEIGHT
        }

        for i in 0..height {
            self.ylookup[i] = (i + self.view_window_y) * SCREENWIDTH;
        }

        self.center_y = (height / 2) as i32;
    }

    /// Desenha uma coluna vertical de textura no framebuffer.
    ///
    /// Este e o inner loop mais critico do renderer — cada pixel visivel
    /// de parede passa por esta funcao. O DOOM original otimizava isso
    /// em assembly.
    ///
    /// Algoritmo DDA (Digital Differential Analyzer):
    /// 1. Calcular fracao inicial na textura
    /// 2. Para cada pixel vertical, buscar texel via fracao
    /// 3. Aplicar colormap (iluminacao) ao texel
    /// 4. Escrever pixel no framebuffer
    /// 5. Avancar fracao por iscale
    ///
    /// C original: `R_DrawColumn()` em `r_draw.c`
    ///
    /// Parametros:
    /// - `screen`: framebuffer de destino
    /// - `x`: coluna X na tela
    /// - `yl, yh`: range vertical (inclusive) a desenhar
    /// - `iscale`: inverse scale (avanco na textura por pixel)
    /// - `texturemid`: posicao central da textura
    /// - `source`: dados da textura (coluna de pixels)
    /// - `colormap`: tabela de iluminacao (256 bytes)
    #[allow(clippy::too_many_arguments)]
    pub fn draw_column(
        &self,
        screen: &mut [u8],
        x: usize,
        yl: i32,
        yh: i32,
        iscale: Fixed,
        texturemid: Fixed,
        source: &[u8],
        colormap: &[u8],
    ) {
        let count = yh - yl;
        if count < 0 {
            return;
        }

        let dest_start = self.ylookup[yl as usize] + self.columnofs[x];
        let fracstep = iscale.0;
        let mut frac = texturemid.0 + (yl - self.center_y) * fracstep;

        // Mascara para wrap da textura: source.len() e potencia de 2.
        // C original: usa `& 127` (texturas de parede sao 128 pixels de altura)
        // mas para texturas nao-potencia-de-2, usamos modulo para seguranca.
        let tex_len = source.len();
        let tex_mask = if tex_len.is_power_of_two() && tex_len > 0 {
            tex_len - 1
        } else {
            0 // sentinela: usar modulo ao inves de mascara
        };

        let mut dest_offset = dest_start;
        // Separar loops para evitar branch no inner loop (hot path)
        if tex_mask != 0 {
            // Fast path: potencia de 2 — bitmask (maioria das texturas)
            for _ in 0..=count {
                let texel_index = (frac >> FRACBITS) as usize & tex_mask;
                let texel = source[texel_index];
                screen[dest_offset] = colormap[texel as usize];
                dest_offset += SCREENWIDTH;
                frac += fracstep;
            }
        } else if tex_len > 0 {
            // Slow path: texturas nao-potencia-de-2 — modulo
            for _ in 0..=count {
                let texel_index = (frac >> FRACBITS) as usize % tex_len;
                let texel = source[texel_index];
                screen[dest_offset] = colormap[texel as usize];
                dest_offset += SCREENWIDTH;
                frac += fracstep;
            }
        }
    }

    /// Desenha uma coluna com efeito fuzz (invisibilidade/Spectre).
    ///
    /// Ao inves de ler uma textura, le pixels adjacentes do framebuffer
    /// e aplica um colormap escuro (indice 6 = ~19% de luz). O resultado
    /// e um efeito de "borrao escuro" que sugere transparencia.
    ///
    /// C original: `R_DrawFuzzColumn()` em `r_draw.c`
    pub fn draw_fuzz_column(
        &mut self,
        screen: &mut [u8],
        x: usize,
        mut yl: i32,
        mut yh: i32,
        colormaps: &[u8],
    ) {
        // Ajustar bordas para evitar ler fora do buffer
        if yl == 0 {
            yl = 1;
        }
        if yh == self.view_height as i32 - 1 {
            yh = self.view_height as i32 - 2;
        }

        let count = yh - yl;
        if count < 0 {
            return;
        }

        let dest_start = self.ylookup[yl as usize] + self.columnofs[x];
        let mut dest_offset = dest_start;

        // Colormap 6 (escura) para o efeito de sombra
        let colormap_offset = 6 * 256;

        for _ in 0..=count {
            // Ler pixel adjacente (acima ou abaixo, conforme fuzzoffset)
            let fuzz = FUZZ_OFFSET[self.fuzz_pos];
            let src_idx = (dest_offset as i32 + fuzz) as usize;
            let src_pixel = if src_idx < screen.len() {
                screen[src_idx]
            } else {
                0
            };

            screen[dest_offset] = colormaps[colormap_offset + src_pixel as usize];

            if self.fuzz_pos + 1 >= FUZZTABLE {
                self.fuzz_pos = 0;
            } else {
                self.fuzz_pos += 1;
            }

            dest_offset += SCREENWIDTH;
        }
    }

    /// Desenha uma coluna com translacao de cor (sprites de jogador).
    ///
    /// Primeiro traduz o indice de cor usando uma tabela de translacao
    /// (para mudar a cor do uniforme do jogador), depois aplica colormap.
    ///
    /// C original: `R_DrawTranslatedColumn()` em `r_draw.c`
    #[allow(clippy::too_many_arguments)]
    pub fn draw_translated_column(
        &self,
        screen: &mut [u8],
        x: usize,
        yl: i32,
        yh: i32,
        iscale: Fixed,
        texturemid: Fixed,
        source: &[u8],
        colormap: &[u8],
        translation: &[u8],
    ) {
        let count = yh - yl;
        if count < 0 {
            return;
        }

        let dest_start = self.ylookup[yl as usize] + self.columnofs[x];
        let fracstep = iscale.0;
        let mut frac = texturemid.0 + (yl - self.center_y) * fracstep;

        let mut dest_offset = dest_start;
        for _ in 0..=count {
            let texel = source[((frac >> FRACBITS) & 127) as usize];
            // Traduzir cor e aplicar iluminacao
            let translated = translation[texel as usize];
            screen[dest_offset] = colormap[translated as usize];

            dest_offset += SCREENWIDTH;
            frac += fracstep;
        }
    }

    /// Desenha um span horizontal de textura (piso/teto).
    ///
    /// Spans sao linhas horizontais preenchidas com uma textura de
    /// piso/teto (flat). Flats sao sempre 64x64 pixels, e o mapeamento
    /// e feito com stepping em U e V (coordenadas de textura).
    ///
    /// C original: `R_DrawSpan()` em `r_draw.c`
    ///
    /// Parametros:
    /// - `screen`: framebuffer de destino
    /// - `y`: linha Y na tela
    /// - `x1, x2`: range horizontal (inclusive) a desenhar
    /// - `xfrac, yfrac`: posicao inicial na textura (fixed-point)
    /// - `xstep, ystep`: avanco na textura por pixel (fixed-point)
    /// - `source`: dados da textura flat (64x64 = 4096 bytes)
    /// - `colormap`: tabela de iluminacao
    #[allow(clippy::too_many_arguments)]
    pub fn draw_span(
        &self,
        screen: &mut [u8],
        y: usize,
        x1: usize,
        x2: usize,
        mut xfrac: Fixed,
        mut yfrac: Fixed,
        xstep: Fixed,
        ystep: Fixed,
        source: &[u8],
        colormap: &[u8],
    ) {
        let dest_start = self.ylookup[y] + self.columnofs[x1];
        let count = x2 as i32 - x1 as i32;

        for (dest, _) in (dest_start..).zip(0..=count) {
            // Calcular indice na textura 64x64
            // spot = ((y >> 10) & 0xFC0) | ((x >> 16) & 63)
            let spot = (((yfrac.0 >> (16 - 6)) & (63 * 64)) + ((xfrac.0 >> 16) & 63)) as usize;

            screen[dest] = colormap[source[spot] as usize];

            xfrac.0 += xstep.0;
            yfrac.0 += ystep.0;
        }
    }
}

impl Default for ColumnDrawer {
    fn default() -> Self {
        Self::new()
    }
}

/// Inicializa as tabelas de translacao de cor para sprites de jogador.
///
/// Remapeia a rampa de cor verde (indices 0x70-0x7F) para outras cores:
/// - Tabela 0: verde -> cinza (0x60-0x6F)
/// - Tabela 1: verde -> marrom (0x40-0x4F)
/// - Tabela 2: verde -> vermelho (0x20-0x2F)
///
/// C original: `R_InitTranslationTables()` em `r_draw.c`
pub fn init_translation_tables() -> Vec<[u8; 256]> {
    let mut tables = vec![[0u8; 256]; 3];

    for i in 0u8..=255 {
        let idx = i as usize;
        if (0x70..=0x7F).contains(&i) {
            // Remapear rampa verde para outras cores
            tables[0][idx] = 0x60 + (i & 0x0F); // cinza
            tables[1][idx] = 0x40 + (i & 0x0F); // marrom
            tables[2][idx] = 0x20 + (i & 0x0F); // vermelho
        } else {
            // Manter todas as outras cores
            tables[0][idx] = i;
            tables[1][idx] = i;
            tables[2][idx] = i;
        }
    }

    tables
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::video::SCREEN_SIZE;

    /// Verifica que init_buffer calcula offsets corretos para fullscreen.
    #[test]
    fn init_buffer_fullscreen() {
        let drawer = ColumnDrawer::new();
        // Fullscreen: viewwindowx = 0
        assert_eq!(drawer.view_window_x, 0);
        assert_eq!(drawer.view_window_y, 0);
        assert_eq!(drawer.columnofs[0], 0);
        assert_eq!(drawer.columnofs[1], 1);
        // ylookup[0] = 0 * SCREENWIDTH = 0
        assert_eq!(drawer.ylookup[0], 0);
        // ylookup[1] = 1 * SCREENWIDTH = 320
        assert_eq!(drawer.ylookup[1], SCREENWIDTH);
    }

    /// Verifica que init_buffer centraliza view window menor.
    #[test]
    fn init_buffer_windowed() {
        let mut drawer = ColumnDrawer::new();
        drawer.init_buffer(256, 160);
        // viewwindowx = (320 - 256) / 2 = 32
        assert_eq!(drawer.view_window_x, 32);
        assert_eq!(drawer.columnofs[0], 32);
        assert_eq!(drawer.columnofs[1], 33);
    }

    /// Verifica que draw_column escreve pixels corretamente.
    #[test]
    fn draw_column_basic() {
        let drawer = ColumnDrawer::new();
        let mut screen = vec![0u8; SCREEN_SIZE];

        // Textura: 128 pixels de cor 5
        let source = vec![5u8; 128];
        // Colormap: identidade (indice = cor)
        let colormap: Vec<u8> = (0..=255).collect();

        drawer.draw_column(
            &mut screen,
            10,             // x = 10
            50,             // yl = 50
            52,             // yh = 52
            Fixed::UNIT,    // iscale = 1.0 (sem escala)
            Fixed::ZERO,    // texturemid = 0
            &source,
            &colormap,
        );

        // Pixels nas linhas 50, 51, 52, coluna 10 devem ter cor 5
        assert_eq!(screen[50 * SCREENWIDTH + 10], 5);
        assert_eq!(screen[51 * SCREENWIDTH + 10], 5);
        assert_eq!(screen[52 * SCREENWIDTH + 10], 5);
        // Pixel fora do range nao afetado
        assert_eq!(screen[49 * SCREENWIDTH + 10], 0);
        assert_eq!(screen[53 * SCREENWIDTH + 10], 0);
    }

    /// Verifica que draw_column aplica colormap (iluminacao).
    #[test]
    fn draw_column_with_colormap() {
        let drawer = ColumnDrawer::new();
        let mut screen = vec![0u8; SCREEN_SIZE];

        let source = vec![10u8; 128]; // texel = 10
        // Colormap que dobra o indice
        let mut colormap = vec![0u8; 256];
        colormap[10] = 20; // cor 10 -> 20

        drawer.draw_column(
            &mut screen, 0, 50, 50,
            Fixed::UNIT, Fixed::ZERO,
            &source, &colormap,
        );

        assert_eq!(screen[50 * SCREENWIDTH], 20); // colormap aplicado
    }

    /// Verifica que draw_span escreve pixels horizontais.
    #[test]
    fn draw_span_basic() {
        let drawer = ColumnDrawer::new();
        let mut screen = vec![0u8; SCREEN_SIZE];

        // Flat 64x64 preenchido com cor 7
        let source = vec![7u8; 64 * 64];
        let colormap: Vec<u8> = (0..=255).collect();

        drawer.draw_span(
            &mut screen,
            100,            // y = 100
            10,             // x1 = 10
            12,             // x2 = 12
            Fixed::ZERO,    // xfrac
            Fixed::ZERO,    // yfrac
            Fixed::UNIT,    // xstep = 1.0
            Fixed::ZERO,    // ystep = 0
            &source,
            &colormap,
        );

        // Pixels nas colunas 10, 11, 12 da linha 100
        assert_eq!(screen[100 * SCREENWIDTH + 10], 7);
        assert_eq!(screen[100 * SCREENWIDTH + 11], 7);
        assert_eq!(screen[100 * SCREENWIDTH + 12], 7);
        // Adjacentes nao afetados
        assert_eq!(screen[100 * SCREENWIDTH + 9], 0);
        assert_eq!(screen[100 * SCREENWIDTH + 13], 0);
    }

    /// Verifica tabelas de translacao de cor.
    #[test]
    fn translation_tables() {
        let tables = init_translation_tables();

        // Verde (0x70) -> cinza (0x60)
        assert_eq!(tables[0][0x70], 0x60);
        // Verde (0x7F) -> cinza (0x6F)
        assert_eq!(tables[0][0x7F], 0x6F);
        // Verde (0x70) -> marrom (0x40)
        assert_eq!(tables[1][0x70], 0x40);
        // Verde (0x70) -> vermelho (0x20)
        assert_eq!(tables[2][0x70], 0x20);

        // Cores fora do range verde nao sao alteradas
        assert_eq!(tables[0][0x00], 0x00);
        assert_eq!(tables[0][0xFF], 0xFF);
    }
}

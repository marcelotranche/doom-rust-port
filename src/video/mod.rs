//! # Modulo Video — Framebuffer e Paletas
//!
//! Interface de video: framebuffer 320x200 (8-bit indexed color),
//! paletas PLAYPAL, colormaps COLORMAP e funcoes de blit.
//!
//! O DOOM usa resolucao fixa de 320x200 com paleta de 256 cores.
//! O framebuffer e um array linear de bytes onde cada byte e um
//! indice na paleta. O rendering e feito em 5 screen buffers:
//!
//! - Screen 0: framebuffer principal (o que aparece na tela)
//! - Screen 1: background (borda da view window)
//! - Screen 2: buffer temporario (menus, wipe effect)
//! - Screen 3: status bar
//! - Screen 4: temporario
//!
//! ## Arquivos C originais
//! - `v_video.c` — Funcoes de desenho no framebuffer (V_DrawPatch, V_CopyRect)
//! - `v_video.h` — Declaracoes e gamma tables
//! - `i_video.c` — Interface com hardware de video (SDL no port Rust)
//!
//! ## Conceitos que o leitor vai aprender
//! - Framebuffer linear com paleta indexada
//! - Correcao gamma via lookup table
//! - Formato de patches do DOOM (column-based sprites)

/// Largura da tela do DOOM em pixels.
///
/// C original: `#define SCREENWIDTH 320` em `doomdef.h`
pub const SCREENWIDTH: usize = 320;

/// Altura da tela do DOOM em pixels.
///
/// C original: `#define SCREENHEIGHT 200` em `doomdef.h`
pub const SCREENHEIGHT: usize = 200;

/// Altura da status bar em pixels.
pub const SBARHEIGHT: usize = 32;

/// Numero de screen buffers.
///
/// C original: `byte* screens[5]` em `v_video.c`
pub const NUM_SCREENS: usize = 5;

/// Tamanho de um screen buffer em bytes (320 * 200 = 64000).
pub const SCREEN_SIZE: usize = SCREENWIDTH * SCREENHEIGHT;

/// Numero de niveis de gamma correction.
pub const NUM_GAMMA_LEVELS: usize = 5;

/// Sistema de framebuffer do DOOM.
///
/// Gerencia os 5 screen buffers e operacoes de blit.
/// Cada pixel e um indice na paleta (0-255).
///
/// C original: `byte* screens[5]` (globals em `v_video.c`)
/// Em Rust, ownership esta na struct ao inves de globals.
#[derive(Debug)]
pub struct VideoSystem {
    /// Os 5 screen buffers do DOOM.
    /// Screen 0 = framebuffer principal, 1 = background, etc.
    screens: Vec<Vec<u8>>,

    /// Nivel de gamma correction atual (0-4).
    /// C original: `int usegamma` em `v_video.c`
    pub use_gamma: usize,

    /// Dirty box para otimizar updates na tela.
    /// [top, bottom, left, right]
    /// C original: `int dirtybox[4]` em `v_video.c`
    dirty_box: [i32; 4],
}

impl VideoSystem {
    /// Inicializa o sistema de video com 5 screen buffers zerados.
    ///
    /// C original: `V_Init()` em `v_video.c`
    /// No C, alocava com `I_AllocLow()` (low DOS memory).
    /// Em Rust, simplesmente alocamos Vecs.
    pub fn new() -> Self {
        let mut screens = Vec::with_capacity(NUM_SCREENS);
        for _ in 0..NUM_SCREENS {
            screens.push(vec![0u8; SCREEN_SIZE]);
        }

        VideoSystem {
            screens,
            use_gamma: 0,
            dirty_box: [0, 0, 0, 0],
        }
    }

    /// Retorna referencia ao screen buffer especificado.
    pub fn screen(&self, index: usize) -> &[u8] {
        &self.screens[index]
    }

    /// Retorna referencia mutavel ao screen buffer especificado.
    pub fn screen_mut(&mut self, index: usize) -> &mut [u8] {
        &mut self.screens[index]
    }

    /// Marca uma regiao como suja (precisando redesenho).
    ///
    /// C original: `V_MarkRect()` em `v_video.c`
    pub fn mark_rect(&mut self, x: i32, y: i32, width: i32, height: i32) {
        // Expandir dirty box para incluir a regiao
        // top
        if y < self.dirty_box[0] {
            self.dirty_box[0] = y;
        }
        // bottom
        if y + height - 1 > self.dirty_box[1] {
            self.dirty_box[1] = y + height - 1;
        }
        // left
        if x < self.dirty_box[2] {
            self.dirty_box[2] = x;
        }
        // right
        if x + width - 1 > self.dirty_box[3] {
            self.dirty_box[3] = x + width - 1;
        }
    }

    /// Copia um retangulo de um screen buffer para outro.
    ///
    /// C original: `V_CopyRect()` em `v_video.c`
    ///
    /// Parametros:
    /// - `src_x, src_y`: posicao no buffer de origem
    /// - `src_screen`: indice do screen de origem (0-4)
    /// - `width, height`: dimensoes do retangulo
    /// - `dest_x, dest_y`: posicao no buffer de destino
    /// - `dest_screen`: indice do screen de destino (0-4)
    #[allow(clippy::too_many_arguments)]
    pub fn copy_rect(
        &mut self,
        src_x: usize,
        src_y: usize,
        src_screen: usize,
        width: usize,
        height: usize,
        dest_x: usize,
        dest_y: usize,
        dest_screen: usize,
    ) {
        self.mark_rect(dest_x as i32, dest_y as i32, width as i32, height as i32);

        // Precisamos copiar entre dois screens da mesma Vec,
        // entao usamos indices ao inves de split_at_mut
        for row in 0..height {
            let src_offset = (src_y + row) * SCREENWIDTH + src_x;
            let dest_offset = (dest_y + row) * SCREENWIDTH + dest_x;

            if src_screen == dest_screen {
                // Copia dentro do mesmo buffer
                let screen = &mut self.screens[src_screen];
                screen.copy_within(src_offset..src_offset + width, dest_offset);
            } else {
                // Copia entre buffers diferentes — precisamos de unsafe
                // para contornar borrow checker com dois indices na mesma Vec
                let (src_slice, dest_slice) = if src_screen < dest_screen {
                    let (left, right) = self.screens.split_at_mut(dest_screen);
                    (&left[src_screen], &mut right[0])
                } else {
                    let (left, right) = self.screens.split_at_mut(src_screen);
                    (&right[0], &mut left[dest_screen])
                };
                dest_slice[dest_offset..dest_offset + width]
                    .copy_from_slice(&src_slice[src_offset..src_offset + width]);
            }
        }
    }

    /// Desenha um bloco linear de pixels no screen buffer.
    ///
    /// C original: `V_DrawBlock()` em `v_video.c`
    pub fn draw_block(
        &mut self,
        x: usize,
        y: usize,
        screen: usize,
        width: usize,
        height: usize,
        src: &[u8],
    ) {
        self.mark_rect(x as i32, y as i32, width as i32, height as i32);

        let dest = &mut self.screens[screen];
        for row in 0..height {
            let dest_offset = (y + row) * SCREENWIDTH + x;
            let src_offset = row * width;
            dest[dest_offset..dest_offset + width]
                .copy_from_slice(&src[src_offset..src_offset + width]);
        }
    }

    /// Le um bloco linear de pixels do screen buffer.
    ///
    /// C original: `V_GetBlock()` em `v_video.c`
    pub fn get_block(
        &self,
        x: usize,
        y: usize,
        screen: usize,
        width: usize,
        height: usize,
        dest: &mut [u8],
    ) {
        let src = &self.screens[screen];
        for row in 0..height {
            let src_offset = (y + row) * SCREENWIDTH + x;
            let dest_offset = row * width;
            dest[dest_offset..dest_offset + width]
                .copy_from_slice(&src[src_offset..src_offset + width]);
        }
    }

    /// Desenha um patch (sprite column-based) no screen buffer.
    ///
    /// Patches sao o formato de imagem principal do DOOM. Cada coluna
    /// e armazenada como uma serie de "posts" (runs de pixels) para
    /// suportar transparencia.
    ///
    /// Formato de um patch na memoria:
    /// - Header: width (i16), height (i16), leftoffset (i16), topoffset (i16)
    /// - columnofs[width]: offsets para cada coluna (i32 cada)
    /// - Dados das colunas: sequencia de posts
    ///
    /// Formato de cada post:
    /// - topdelta (u8): offset vertical (0xFF = fim da coluna)
    /// - length (u8): numero de pixels
    /// - padding (u8): byte nao usado (artefato do formato)
    /// - pixels[length] (u8): indices na paleta
    /// - padding (u8): byte nao usado
    ///
    /// C original: `V_DrawPatch()` em `v_video.c`
    pub fn draw_patch(&mut self, x: i32, y: i32, screen: usize, patch_data: &[u8]) {
        if patch_data.len() < 8 {
            return;
        }

        // Ler header do patch
        let width = i16::from_le_bytes([patch_data[0], patch_data[1]]) as i32;
        let height = i16::from_le_bytes([patch_data[2], patch_data[3]]) as i32;
        let left_offset = i16::from_le_bytes([patch_data[4], patch_data[5]]) as i32;
        let top_offset = i16::from_le_bytes([patch_data[6], patch_data[7]]) as i32;

        let draw_x = x - left_offset;
        let draw_y = y - top_offset;

        // Range check
        if draw_x < 0
            || draw_x + width > SCREENWIDTH as i32
            || draw_y < 0
            || draw_y + height > SCREENHEIGHT as i32
        {
            return;
        }

        if screen == 0 {
            self.mark_rect(draw_x, draw_y, width, height);
        }

        let dest = &mut self.screens[screen];

        for col in 0..width {
            // Ler offset da coluna (i32 little-endian a partir do byte 8)
            let ofs_pos = 8 + (col as usize) * 4;
            if ofs_pos + 4 > patch_data.len() {
                break;
            }
            let col_offset = u32::from_le_bytes([
                patch_data[ofs_pos],
                patch_data[ofs_pos + 1],
                patch_data[ofs_pos + 2],
                patch_data[ofs_pos + 3],
            ]) as usize;

            // Percorrer posts da coluna
            let mut post_offset = col_offset;
            loop {
                if post_offset >= patch_data.len() {
                    break;
                }

                let top_delta = patch_data[post_offset];
                if top_delta == 0xFF {
                    break; // Fim da coluna
                }

                let length = patch_data[post_offset + 1] as usize;
                // post_offset + 2 = padding byte
                let pixel_start = post_offset + 3;

                // Desenhar pixels do post
                for i in 0..length {
                    let dest_y = draw_y + top_delta as i32 + i as i32;
                    if dest_y >= 0 && dest_y < SCREENHEIGHT as i32 {
                        let pixel_pos = pixel_start + i;
                        if pixel_pos < patch_data.len() {
                            let dest_idx =
                                dest_y as usize * SCREENWIDTH + (draw_x + col) as usize;
                            dest[dest_idx] = patch_data[pixel_pos];
                        }
                    }
                }

                // Proximo post: header (1) + length (1) + padding (1) + pixels + padding (1)
                post_offset = pixel_start + length + 1;
            }
        }
    }

    /// Copia pixels de screen 1 para screen 0 (para restaurar background).
    ///
    /// C original: `R_VideoErase()` em `r_draw.c`
    pub fn video_erase(&mut self, offset: usize, count: usize) {
        let (src, dest) = self.screens.split_at_mut(1);
        // src[0] = screen 0, dest[0] = screen 1 — invertidos no split
        // Na verdade: split_at_mut(1) -> left=[screen0], right=[screen1,..]
        let screen0 = &mut src[0];
        let screen1 = &dest[0];
        screen0[offset..offset + count].copy_from_slice(&screen1[offset..offset + count]);
    }

    /// Limpa o dirty box.
    pub fn clear_dirty_box(&mut self) {
        self.dirty_box = [SCREENHEIGHT as i32, 0, SCREENWIDTH as i32, 0];
    }

    /// Retorna o dirty box atual: [top, bottom, left, right].
    pub fn dirty_box(&self) -> [i32; 4] {
        self.dirty_box
    }

    /// Aplica correcao gamma a um valor de cor.
    ///
    /// C original: `gammatable[usegamma][color]` em `v_video.c`
    pub fn gamma_correct(&self, color: u8) -> u8 {
        GAMMA_TABLE[self.use_gamma][color as usize]
    }
}

impl Default for VideoSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Tabelas de correcao gamma do DOOM original.
///
/// 5 niveis de gamma (0 = escuro/normal, 4 = claro).
/// Cada tabela mapeia um indice de cor (0-255) para o valor corrigido.
///
/// C original: `byte gammatable[5][256]` em `v_video.c`
#[rustfmt::skip]
pub static GAMMA_TABLE: [[u8; 256]; NUM_GAMMA_LEVELS] = [
    // Gamma 0 — sem correcao (quase identidade, exceto 0->1)
    [
        1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,
        17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,
        33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,48,
        49,50,51,52,53,54,55,56,57,58,59,60,61,62,63,64,
        65,66,67,68,69,70,71,72,73,74,75,76,77,78,79,80,
        81,82,83,84,85,86,87,88,89,90,91,92,93,94,95,96,
        97,98,99,100,101,102,103,104,105,106,107,108,109,110,111,112,
        113,114,115,116,117,118,119,120,121,122,123,124,125,126,127,128,
        128,129,130,131,132,133,134,135,136,137,138,139,140,141,142,143,
        144,145,146,147,148,149,150,151,152,153,154,155,156,157,158,159,
        160,161,162,163,164,165,166,167,168,169,170,171,172,173,174,175,
        176,177,178,179,180,181,182,183,184,185,186,187,188,189,190,191,
        192,193,194,195,196,197,198,199,200,201,202,203,204,205,206,207,
        208,209,210,211,212,213,214,215,216,217,218,219,220,221,222,223,
        224,225,226,227,228,229,230,231,232,233,234,235,236,237,238,239,
        240,241,242,243,244,245,246,247,248,249,250,251,252,253,254,255,
    ],
    // Gamma 1
    [
        2,4,5,7,8,10,11,12,14,15,16,18,19,20,21,23,24,25,26,27,29,30,31,
        32,33,34,36,37,38,39,40,41,42,44,45,46,47,48,49,50,51,52,54,55,
        56,57,58,59,60,61,62,63,64,65,66,67,69,70,71,72,73,74,75,76,77,
        78,79,80,81,82,83,84,85,86,87,88,89,90,91,92,93,94,95,96,97,98,
        99,100,101,102,103,104,105,106,107,108,109,110,111,112,113,114,
        115,116,117,118,119,120,121,122,123,124,125,126,127,128,129,129,
        130,131,132,133,134,135,136,137,138,139,140,141,142,143,144,145,
        146,147,148,148,149,150,151,152,153,154,155,156,157,158,159,160,
        161,162,163,163,164,165,166,167,168,169,170,171,172,173,174,175,
        175,176,177,178,179,180,181,182,183,184,185,186,186,187,188,189,
        190,191,192,193,194,195,196,196,197,198,199,200,201,202,203,204,
        205,205,206,207,208,209,210,211,212,213,214,214,215,216,217,218,
        219,220,221,222,222,223,224,225,226,227,228,229,230,230,231,232,
        233,234,235,236,237,237,238,239,240,241,242,243,244,245,245,246,
        247,248,249,250,251,252,252,253,254,255,
    ],
    // Gamma 2
    [
        4,7,9,11,13,15,17,19,21,22,24,26,27,29,30,32,33,35,36,38,39,40,42,
        43,45,46,47,48,50,51,52,54,55,56,57,59,60,61,62,63,65,66,67,68,69,
        70,72,73,74,75,76,77,78,79,80,82,83,84,85,86,87,88,89,90,91,92,93,
        94,95,96,97,98,100,101,102,103,104,105,106,107,108,109,110,111,112,
        113,114,114,115,116,117,118,119,120,121,122,123,124,125,126,127,128,
        129,130,131,132,133,133,134,135,136,137,138,139,140,141,142,143,144,
        144,145,146,147,148,149,150,151,152,153,153,154,155,156,157,158,159,
        160,160,161,162,163,164,165,166,166,167,168,169,170,171,172,172,173,
        174,175,176,177,178,178,179,180,181,182,183,183,184,185,186,187,188,
        188,189,190,191,192,193,193,194,195,196,197,197,198,199,200,201,201,
        202,203,204,205,206,206,207,208,209,210,210,211,212,213,213,214,215,
        216,217,217,218,219,220,221,221,222,223,224,224,225,226,227,228,228,
        229,230,231,231,232,233,234,235,235,236,237,238,238,239,240,241,241,
        242,243,244,244,245,246,247,247,248,249,250,251,251,252,253,254,254,
        255,
    ],
    // Gamma 3
    [
        8,12,16,19,22,24,27,29,31,34,36,38,40,41,43,45,47,49,50,52,53,55,
        57,58,60,61,63,64,65,67,68,70,71,72,74,75,76,77,79,80,81,82,84,85,
        86,87,88,90,91,92,93,94,95,96,98,99,100,101,102,103,104,105,106,107,
        108,109,110,111,112,113,114,115,116,117,118,119,120,121,122,123,124,
        125,126,127,128,129,130,131,132,133,134,135,135,136,137,138,139,140,
        141,142,143,143,144,145,146,147,148,149,150,150,151,152,153,154,155,
        155,156,157,158,159,160,160,161,162,163,164,165,165,166,167,168,169,
        169,170,171,172,173,173,174,175,176,176,177,178,179,180,180,181,182,
        183,183,184,185,186,186,187,188,189,189,190,191,192,192,193,194,195,
        195,196,197,197,198,199,200,200,201,202,202,203,204,205,205,206,207,
        207,208,209,210,210,211,212,212,213,214,214,215,216,216,217,218,219,
        219,220,221,221,222,223,223,224,225,225,226,227,227,228,229,229,230,
        231,231,232,233,233,234,235,235,236,237,237,238,238,239,240,240,241,
        242,242,243,244,244,245,246,246,247,247,248,249,249,250,251,251,252,
        253,253,254,254,255,
    ],
    // Gamma 4
    [
        16,23,28,32,36,39,42,45,48,50,53,55,57,60,62,64,66,68,69,71,73,75,76,
        78,80,81,83,84,86,87,89,90,92,93,94,96,97,98,100,101,102,103,105,106,
        107,108,109,110,112,113,114,115,116,117,118,119,120,121,122,123,124,
        125,126,128,128,129,130,131,132,133,134,135,136,137,138,139,140,141,
        142,143,143,144,145,146,147,148,149,150,150,151,152,153,154,155,155,
        156,157,158,159,159,160,161,162,163,163,164,165,166,166,167,168,169,
        169,170,171,172,172,173,174,175,175,176,177,177,178,179,180,180,181,
        182,182,183,184,184,185,186,187,187,188,189,189,190,191,191,192,193,
        193,194,195,195,196,196,197,198,198,199,200,200,201,202,202,203,203,
        204,205,205,206,207,207,208,208,209,210,210,211,211,212,213,213,214,
        214,215,216,216,217,217,218,219,219,220,220,221,221,222,223,223,224,
        224,225,225,226,227,227,228,228,229,229,230,230,231,232,232,233,233,
        234,234,235,235,236,236,237,237,238,239,239,240,240,241,241,242,242,
        243,243,244,244,245,245,246,246,247,247,248,248,249,249,250,250,251,
        251,252,252,253,254,254,255,255,
    ],
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifica que o sistema de video inicializa com 5 buffers zerados.
    #[test]
    fn video_init() {
        let video = VideoSystem::new();
        assert_eq!(video.screens.len(), NUM_SCREENS);
        for screen in &video.screens {
            assert_eq!(screen.len(), SCREEN_SIZE);
            assert!(screen.iter().all(|&b| b == 0));
        }
    }

    /// Verifica que draw_block escreve pixels corretamente.
    #[test]
    fn draw_block_basic() {
        let mut video = VideoSystem::new();
        let pixels = vec![42u8; 4]; // 2x2 block
        video.draw_block(10, 20, 0, 2, 2, &pixels);

        let screen = video.screen(0);
        assert_eq!(screen[20 * SCREENWIDTH + 10], 42);
        assert_eq!(screen[20 * SCREENWIDTH + 11], 42);
        assert_eq!(screen[21 * SCREENWIDTH + 10], 42);
        assert_eq!(screen[21 * SCREENWIDTH + 11], 42);
        // Pixel adjacente nao afetado
        assert_eq!(screen[20 * SCREENWIDTH + 12], 0);
    }

    /// Verifica que copy_rect copia entre screens diferentes.
    #[test]
    fn copy_rect_between_screens() {
        let mut video = VideoSystem::new();
        // Escrever padrao no screen 1
        video.screens[1][0] = 100;
        video.screens[1][1] = 101;
        video.screens[1][SCREENWIDTH] = 110;
        video.screens[1][SCREENWIDTH + 1] = 111;

        // Copiar 2x2 do screen 1 para screen 0 na posicao (5, 5)
        video.copy_rect(0, 0, 1, 2, 2, 5, 5, 0);

        assert_eq!(video.screens[0][5 * SCREENWIDTH + 5], 100);
        assert_eq!(video.screens[0][5 * SCREENWIDTH + 6], 101);
        assert_eq!(video.screens[0][6 * SCREENWIDTH + 5], 110);
        assert_eq!(video.screens[0][6 * SCREENWIDTH + 6], 111);
    }

    /// Verifica correcao gamma no nivel 0 (quase identidade).
    #[test]
    fn gamma_correction() {
        let video = VideoSystem::new();
        // Gamma 0: 0 -> 1 (unica excecao), 255 -> 255
        assert_eq!(video.gamma_correct(0), 1);
        assert_eq!(video.gamma_correct(255), 255);
    }

    /// Verifica que a tabela gamma no nivel 4 clareia significativamente.
    #[test]
    fn gamma_level_4_brightens() {
        assert!(GAMMA_TABLE[4][0] > GAMMA_TABLE[0][0]);
        // Valor escuro (indice 10) no gamma 4 deve ser mais claro que no gamma 0
        assert!(GAMMA_TABLE[4][10] > GAMMA_TABLE[0][10]);
    }

    /// Verifica video_erase: copia de screen 1 para screen 0.
    #[test]
    fn video_erase_basic() {
        let mut video = VideoSystem::new();
        // Preencher screen 1 com padrao
        for i in 0..10 {
            video.screens[1][i] = (i + 1) as u8;
        }

        video.video_erase(0, 10);

        for i in 0..10 {
            assert_eq!(video.screens[0][i], (i + 1) as u8);
        }
    }

    /// Verifica draw_patch com um patch sintetico minimo.
    #[test]
    fn draw_patch_synthetic() {
        let mut video = VideoSystem::new();

        // Patch: 1 coluna, 2 pixels de altura, sem offset
        // Header: width=1, height=2, leftoffset=0, topoffset=0
        let mut patch = Vec::new();
        patch.extend_from_slice(&1i16.to_le_bytes());  // width
        patch.extend_from_slice(&2i16.to_le_bytes());  // height
        patch.extend_from_slice(&0i16.to_le_bytes());  // leftoffset
        patch.extend_from_slice(&0i16.to_le_bytes());  // topoffset

        // columnofs[0] = offset dos dados da coluna 0
        // Header = 8 bytes, columnofs = 4 bytes -> dados comecam em 12
        let col_data_offset = 12u32;
        patch.extend_from_slice(&col_data_offset.to_le_bytes());

        // Dados da coluna 0: um post com 2 pixels
        patch.push(0);    // topdelta = 0
        patch.push(2);    // length = 2
        patch.push(0);    // padding
        patch.push(42);   // pixel 0
        patch.push(43);   // pixel 1
        patch.push(0);    // padding
        patch.push(0xFF); // fim da coluna

        video.draw_patch(0, 0, 0, &patch);

        assert_eq!(video.screens[0][0], 42);
        assert_eq!(video.screens[0][SCREENWIDTH], 43);
    }
}

//! # Rendering de Sprites e Sky
//!
//! Sprites sao os objetos visiveis no mundo do DOOM: jogadores, monstros,
//! itens, decoracao. Sao renderizados apos paredes e pisos/tetos,
//! clippados contra drawsegs para oclusao correta.
//!
//! ## Pipeline de sprites
//!
//! 1. Durante a travessia BSP, para cada subsector visitado,
//!    `R_AddSprites()` coleta sprites (mobjs) no sector
//! 2. Cada sprite visivel e adicionado ao array de vissprites
//! 3. Apos a travessia BSP, `R_SortVisSprites()` ordena por profundidade
//! 4. `R_DrawSprites()` renderiza de tras para frente, clippando contra
//!    drawsegs acumulados durante o rendering de paredes
//!
//! ## Sky
//!
//! O ceu do DOOM e uma textura que envolve 360 graus. E desenhado
//! como colunas de textura que acompanham o angulo de visao horizontal.
//! O DOOM usa a textura do sky como se fosse uma parede muito distante.
//!
//! ## Arquivo C original: `r_things.c`, `r_sky.c`
//!
//! ## Conceitos que o leitor vai aprender
//! - Rendering de sprites com z-sorting
//! - Sprite clipping contra geometria de parede
//! - Sky rendering via texture wrapping

use crate::utils::fixed::Fixed;

/// Numero maximo de vissprites por frame.
///
/// C original: `#define MAXVISSPRITES 128` em `r_things.c`
pub const MAXVISSPRITES: usize = 128;

/// Vissprite — instancia de sprite projetada na tela, pronta para rendering.
///
/// C original: `vissprite_t` em `r_defs.h`
#[derive(Debug, Clone)]
pub struct VisSprite {
    /// Range de colunas X na tela
    pub x1: i32,
    pub x2: i32,

    /// Posicao global no mundo (para z-sorting)
    pub gx: Fixed,
    pub gy: Fixed,
    /// Posicao Z do piso e topo do sprite
    pub gz: Fixed,
    pub gzt: Fixed,

    /// Escala perspectiva (para determinar tamanho na tela)
    pub scale: Fixed,
    /// Inverso da escala X (para mapeamento de textura)
    pub x_iscale: Fixed,

    /// Offset vertical da textura
    pub texturemid: Fixed,

    /// Indice do lump do patch no WAD
    pub patch: usize,

    /// Se true, sprite e desenhado espelhado horizontalmente
    pub flip: bool,

    /// Indice do colormap para iluminacao
    pub colormap_index: Option<usize>,
}

impl VisSprite {
    /// Cria um vissprite vazio.
    pub fn new() -> Self {
        VisSprite {
            x1: 0,
            x2: 0,
            gx: Fixed::ZERO,
            gy: Fixed::ZERO,
            gz: Fixed::ZERO,
            gzt: Fixed::ZERO,
            scale: Fixed::ZERO,
            x_iscale: Fixed::ZERO,
            texturemid: Fixed::ZERO,
            patch: 0,
            flip: false,
            colormap_index: None,
        }
    }
}

impl Default for VisSprite {
    fn default() -> Self {
        Self::new()
    }
}

/// Sistema de rendering de sprites.
#[derive(Debug)]
pub struct SpriteRenderer {
    /// Array de vissprites acumulados neste frame.
    pub vissprites: Vec<VisSprite>,
}

impl SpriteRenderer {
    /// Cria um novo sistema de sprites.
    pub fn new() -> Self {
        SpriteRenderer {
            vissprites: Vec::with_capacity(MAXVISSPRITES),
        }
    }

    /// Limpa vissprites para um novo frame.
    ///
    /// C original: `R_ClearSprites()` em `r_things.c`
    pub fn clear(&mut self) {
        self.vissprites.clear();
    }

    /// Ordena vissprites por escala (profundidade) para rendering back-to-front.
    ///
    /// Sprites mais distantes (escala menor) sao desenhados primeiro,
    /// depois os mais proximos sao desenhados por cima.
    ///
    /// C original: `R_SortVisSprites()` em `r_things.c`
    pub fn sort(&mut self) {
        self.vissprites.sort_by_key(|a| a.scale.0);
    }
}

impl Default for SpriteRenderer {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Sky
// ---------------------------------------------------------------------------

/// Shift para converter angulo de visao em coluna de textura do sky.
///
/// C original: `#define ANGLETOSKYSHIFT 22` em `r_sky.c`
pub const ANGLETOSKYSHIFT: u32 = 22;

/// Estado do sky rendering.
///
/// C original: globals `skytexture`, `skytexturemid` em `r_sky.c`
#[derive(Debug, Clone)]
pub struct SkyState {
    /// Indice da textura do sky
    pub texture: usize,
    /// Offset vertical da textura do sky
    pub texture_mid: Fixed,
}

impl SkyState {
    /// Cria um novo estado de sky com valores padrao.
    pub fn new() -> Self {
        SkyState {
            texture: 0,
            texture_mid: Fixed::ZERO,
        }
    }
}

impl Default for SkyState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vissprite_init() {
        let vs = VisSprite::new();
        assert_eq!(vs.x1, 0);
        assert!(!vs.flip);
        assert!(vs.colormap_index.is_none());
    }

    #[test]
    fn sprite_renderer_sort() {
        let mut sr = SpriteRenderer::new();

        let mut vs1 = VisSprite::new();
        vs1.scale = Fixed(300);

        let mut vs2 = VisSprite::new();
        vs2.scale = Fixed(100);

        let mut vs3 = VisSprite::new();
        vs3.scale = Fixed(200);

        sr.vissprites.push(vs1);
        sr.vissprites.push(vs2);
        sr.vissprites.push(vs3);

        sr.sort();

        assert_eq!(sr.vissprites[0].scale, Fixed(100)); // mais distante
        assert_eq!(sr.vissprites[1].scale, Fixed(200));
        assert_eq!(sr.vissprites[2].scale, Fixed(300)); // mais proximo
    }

    #[test]
    fn sprite_renderer_clear() {
        let mut sr = SpriteRenderer::new();
        sr.vissprites.push(VisSprite::new());
        sr.clear();
        assert!(sr.vissprites.is_empty());
    }

    #[test]
    fn sky_state_init() {
        let sky = SkyState::new();
        assert_eq!(sky.texture, 0);
    }
}

//! # Bounding Box (Caixa Delimitadora)
//!
//! Bounding box alinhada aos eixos (AABB) usada pelo DOOM para
//! deteccao rapida de colisao e clipping no renderer.
//!
//! No DOOM original, um bounding box e representado como um array
//! de 4 `fixed_t` indexado por constantes TOP/BOTTOM/LEFT/RIGHT.
//!
//! ## Arquivo C original: `m_bbox.c` / `m_bbox.h`
//!
//! ## Conceitos que o leitor vai aprender
//! - AABB (Axis-Aligned Bounding Box) em game engines
//! - Como o DOOM usa bounding boxes para otimizar colisao

use super::fixed::Fixed;

/// Indices do bounding box no DOOM original.
/// C original: `#define BOXTOP 0` etc. em `m_bbox.h`
///
/// Em Rust, usamos uma struct com campos nomeados ao inves de
/// indices em array — mais legivel e seguro contra erros de indice.
///
/// Bounding box alinhada aos eixos (AABB) no espaco 2D do mapa.
///
/// No DOOM original: `fixed_t bbox[4]` com indices BOXTOP/BOTTOM/LEFT/RIGHT.
/// Em Rust: struct com campos nomeados para clareza.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BBox {
    /// Coordenada Y maxima (topo)
    /// C original: `bbox[BOXTOP]`
    pub top: Fixed,
    /// Coordenada Y minima (base)
    /// C original: `bbox[BOXBOTTOM]`
    pub bottom: Fixed,
    /// Coordenada X minima (esquerda)
    /// C original: `bbox[BOXLEFT]`
    pub left: Fixed,
    /// Coordenada X maxima (direita)
    /// C original: `bbox[BOXRIGHT]`
    pub right: Fixed,
}

impl BBox {
    /// Cria um bounding box "vazio" (invertido) pronto para receber pontos.
    ///
    /// Os valores sao invertidos propositalmente: top/right no minimo,
    /// bottom/left no maximo. Assim, o primeiro ponto adicionado com
    /// `add_point()` definira corretamente os limites.
    ///
    /// C original: `M_ClearBox()` em `m_bbox.c`
    pub fn cleared() -> Self {
        BBox {
            top: Fixed::MIN,
            bottom: Fixed::MAX,
            left: Fixed::MAX,
            right: Fixed::MIN,
        }
    }

    /// Cria um bounding box a partir de coordenadas explicitas.
    pub fn new(top: Fixed, bottom: Fixed, left: Fixed, right: Fixed) -> Self {
        BBox {
            top,
            bottom,
            left,
            right,
        }
    }

    /// Expande o bounding box para incluir o ponto (x, y).
    ///
    /// C original: `M_AddToBox()` em `m_bbox.c`
    /// ```c
    /// void M_AddToBox(fixed_t* box, fixed_t x, fixed_t y) {
    ///     if (x < box[BOXLEFT])   box[BOXLEFT] = x;
    ///     if (x > box[BOXRIGHT])  box[BOXRIGHT] = x;
    ///     if (y < box[BOXBOTTOM]) box[BOXBOTTOM] = y;
    ///     if (y > box[BOXTOP])    box[BOXTOP] = y;
    /// }
    /// ```
    pub fn add_point(&mut self, x: Fixed, y: Fixed) {
        if x < self.left {
            self.left = x;
        }
        if x > self.right {
            self.right = x;
        }
        if y < self.bottom {
            self.bottom = y;
        }
        if y > self.top {
            self.top = y;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifica que um bbox limpo expande corretamente ao adicionar pontos.
    #[test]
    fn cleared_then_add_points() {
        let mut bb = BBox::cleared();
        bb.add_point(Fixed::from_int(10), Fixed::from_int(20));
        bb.add_point(Fixed::from_int(-5), Fixed::from_int(-3));

        assert_eq!(bb.top, Fixed::from_int(20));
        assert_eq!(bb.bottom, Fixed::from_int(-3));
        assert_eq!(bb.left, Fixed::from_int(-5));
        assert_eq!(bb.right, Fixed::from_int(10));
    }

    /// Verifica que add_point com um unico ponto define todos os limites.
    #[test]
    fn single_point() {
        let mut bb = BBox::cleared();
        bb.add_point(Fixed::from_int(7), Fixed::from_int(3));

        assert_eq!(bb.top, Fixed::from_int(3));
        assert_eq!(bb.bottom, Fixed::from_int(3));
        assert_eq!(bb.left, Fixed::from_int(7));
        assert_eq!(bb.right, Fixed::from_int(7));
    }
}

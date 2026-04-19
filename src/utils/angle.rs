//! # Angulos e Tabelas Trigonometricas
//!
//! O DOOM usa angulos em formato inteiro de 32 bits (BAM — Binary Angle
//! Measurement) onde 0x00000000 = 0 graus e 0xFFFFFFFF = ~360 graus.
//! Isso permite que somas e subtracoes de angulos "deem a volta"
//! naturalmente usando overflow de inteiros sem sinal.
//!
//! ## Arquivo C original: `tables.c` / `tables.h`
//!
//! ## Conceitos que o leitor vai aprender
//! - Binary Angle Measurement (BAM) e por que e eficiente
//! - Lookup tables para trigonometria em game engines antigos
//! - Newtype pattern com operadores customizados

use std::ops::{Add, AddAssign, Sub, SubAssign};

/// Angulo no formato BAM (Binary Angle Measurement) de 32 bits.
///
/// No DOOM original: `typedef unsigned angle_t;` em `tables.h`
///
/// Valores especiais:
/// - `ANG0`   = 0x00000000 (0 graus)
/// - `ANG90`  = 0x40000000 (90 graus)
/// - `ANG180` = 0x80000000 (180 graus)
/// - `ANG270` = 0xC0000000 (270 graus)
///
/// A beleza deste sistema e que `ANG90 + ANG90 = ANG180` funciona
/// com aritmetica inteira normal, e o wraparound de u32 cuida da
/// volta de 360 -> 0 graus automaticamente.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Angle(pub u32);

/// Constantes de angulo, equivalentes aos #defines em `tables.h`
impl Angle {
    pub const ANG0: Angle = Angle(0x0000_0000);
    pub const ANG45: Angle = Angle(0x2000_0000);
    pub const ANG90: Angle = Angle(0x4000_0000);
    pub const ANG135: Angle = Angle(0x6000_0000);
    pub const ANG180: Angle = Angle(0x8000_0000);
    pub const ANG270: Angle = Angle(0xC000_0000);

    /// Cria um angulo a partir do valor bruto BAM.
    #[inline]
    pub const fn from_raw(raw: u32) -> Self {
        Angle(raw)
    }

    /// Retorna o valor bruto BAM.
    #[inline]
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Converte graus (0-359) para angulo BAM.
    /// Util para testes e debug — o engine nao usa graus internamente.
    #[inline]
    pub fn from_degrees(degrees: f64) -> Self {
        Angle(((degrees / 360.0) * (u32::MAX as f64 + 1.0)) as u32)
    }

    /// Converte angulo BAM para graus (0.0 - 360.0).
    /// Util para debug e display.
    #[inline]
    pub fn to_degrees(self) -> f64 {
        (self.0 as f64 / (u32::MAX as f64 + 1.0)) * 360.0
    }
}

impl Add for Angle {
    type Output = Self;
    /// Soma de angulos: usa wrapping_add para wraparound natural.
    /// No C original: simplesmente `a + b` com overflow de unsigned.
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Angle(self.0.wrapping_add(rhs.0))
    }
}

impl AddAssign for Angle {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.wrapping_add(rhs.0);
    }
}

impl Sub for Angle {
    type Output = Self;
    /// Subtracao de angulos: usa wrapping_sub para wraparound natural.
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Angle(self.0.wrapping_sub(rhs.0))
    }
}

impl SubAssign for Angle {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self.0.wrapping_sub(rhs.0);
    }
}

/// Tamanho da tabela de seno/cosseno do DOOM.
///
/// O DOOM usa uma tabela com 8192 entradas cobrindo 360 graus.
/// Como seno e cosseno sao periodicos, a tabela de cosseno e
/// simplesmente a tabela de seno deslocada em 90 graus (2048 entradas).
///
/// C original: `#define FINEANGLES 8192` em `tables.h`
pub const FINEANGLES: usize = 8192;

/// Mascara para indexar a tabela fine angles: 0x1FFF
pub const FINEMASK: usize = FINEANGLES - 1;

/// Bits de shift para converter BAM -> indice fine angle.
/// BAM tem 32 bits, fine angles tem 13 bits (8192 = 2^13),
/// entao o shift e 32 - 13 = 19.
///
/// C original: `#define ANGLETOFINESHIFT 19` em `tables.h`
pub const ANGLETOFINESHIFT: u32 = 19;

// As funcoes trigonometricas (fine_sine, fine_cosine, fine_tangent)
// e as tabelas hardcoded do DOOM original estao em `tables.rs`.

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifica que a soma de angulos faz wraparound corretamente.
    /// 270 + 180 = 450 = 90 graus (volta completa)
    #[test]
    fn angle_wraparound() {
        let result = Angle::ANG270 + Angle::ANG180;
        assert_eq!(result, Angle::ANG90);
    }

    /// Verifica que 90 - 90 = 0
    #[test]
    fn angle_sub() {
        assert_eq!(Angle::ANG90 - Angle::ANG90, Angle::ANG0);
    }
}

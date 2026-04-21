//! # Matematica em Ponto-Fixo (Fixed-Point)
//!
//! O DOOM nao usa numeros de ponto flutuante — toda a matematica
//! fracionaria e feita com inteiros de 32 bits no formato 16.16:
//! os 16 bits superiores sao a parte inteira, os 16 inferiores
//! sao a parte fracionaria.
//!
//! Exemplo: o valor 1.5 e representado como 0x00018000
//! - Parte inteira: 0x0001 = 1
//! - Parte fracionaria: 0x8000 = 0.5 (metade de 0x10000)
//!
//! ## Arquivo C original: `m_fixed.c` / `m_fixed.h`
//!
//! ## Conceitos que o leitor vai aprender
//! - Aritmetica em ponto-fixo e por que era usada em 1993
//! - Implementacao de operadores customizados em Rust (std::ops)
//! - Newtype pattern para seguranca de tipos

use std::fmt;
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// Numero em ponto-fixo 16.16, base da matematica do DOOM.
///
/// No DOOM original: `typedef int fixed_t;` em `m_fixed.h`
///
/// O formato 16.16 significa:
/// - Bits 31..16: parte inteira (com sinal, complemento de 2)
/// - Bits 15..0: parte fracionaria (1/65536 de precisao)
///
/// Faixa de valores: aproximadamente -32768.0 a +32767.99998
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Fixed(pub i32);

/// Numero de bits fracionarios no formato fixed-point do DOOM.
/// C original: `#define FRACBITS 16` em `m_fixed.h`
pub const FRACBITS: i32 = 16;

/// Mascara para a parte fracionaria: 0xFFFF
/// C original: `#define FRACUNIT (1 << FRACBITS)` em `m_fixed.h`
pub const FRACUNIT: i32 = 1 << FRACBITS;

impl Fixed {
    /// Valor zero (0.0)
    pub const ZERO: Fixed = Fixed(0);

    /// Valor unitario (1.0) — equivalente a FRACUNIT no C original
    pub const UNIT: Fixed = Fixed(FRACUNIT);

    /// Valor maximo representavel (~32767.99998)
    pub const MAX: Fixed = Fixed(i32::MAX);

    /// Valor minimo representavel (~-32768.0)
    pub const MIN: Fixed = Fixed(i32::MIN);

    /// Converte um inteiro para fixed-point.
    ///
    /// Exemplo: `Fixed::from_int(3)` produz 3.0 em fixed-point.
    ///
    /// C original: `n << FRACBITS`
    #[inline]
    pub const fn from_int(n: i32) -> Self {
        Fixed(n << FRACBITS)
    }

    /// Extrai a parte inteira (trunca a fracao).
    ///
    /// Exemplo: `Fixed(0x00028000).to_int()` retorna 2 (descarta o .5).
    ///
    /// C original: `n >> FRACBITS`
    #[inline]
    pub const fn to_int(self) -> i32 {
        self.0 >> FRACBITS
    }

    /// Retorna o valor bruto interno (i32 no formato 16.16).
    #[inline]
    pub const fn raw(self) -> i32 {
        self.0
    }

    /// Cria um Fixed a partir de um valor bruto (ja no formato 16.16).
    #[inline]
    pub const fn from_raw(raw: i32) -> Self {
        Fixed(raw)
    }

    /// Retorna o valor absoluto.
    ///
    /// C original: `abs(x)` aplicado a fixed_t
    #[inline]
    pub const fn abs(self) -> Self {
        Fixed(self.0.wrapping_abs())
    }
}

// --- Implementacao de operadores aritmeticos ---
// No C original, soma e subtracao sao operacoes diretas sobre i32.
// Multiplicacao e divisao precisam de casts para i64 para evitar overflow.

impl Add for Fixed {
    type Output = Self;
    /// Soma fixed-point: operacao direta sobre os i32 internos.
    /// C original: simplesmente `a + b` (ambos fixed_t = int)
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Fixed(self.0.wrapping_add(rhs.0))
    }
}

impl AddAssign for Fixed {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.wrapping_add(rhs.0);
    }
}

impl Sub for Fixed {
    type Output = Self;
    /// Subtracao fixed-point: operacao direta sobre os i32 internos.
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Fixed(self.0.wrapping_sub(rhs.0))
    }
}

impl SubAssign for Fixed {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self.0.wrapping_sub(rhs.0);
    }
}

impl Mul for Fixed {
    type Output = Self;
    /// Multiplicacao fixed-point: usa i64 intermediario para evitar overflow.
    ///
    /// A formula e: (a * b) >> FRACBITS
    /// Precisamos de 64 bits porque dois numeros 16.16 multiplicados
    /// produzem um resultado 32.32 — depois descartamos os 16 bits
    /// inferiores para voltar ao formato 16.16.
    ///
    /// C original: `FixedMul()` em `m_fixed.c`
    /// ```c
    /// fixed_t FixedMul(fixed_t a, fixed_t b) {
    ///     return ((long long)a * (long long)b) >> FRACBITS;
    /// }
    /// ```
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Fixed(((self.0 as i64 * rhs.0 as i64) >> FRACBITS) as i32)
    }
}

impl Div for Fixed {
    type Output = Self;
    /// Divisao fixed-point: desloca o dividendo para i64 antes de dividir.
    ///
    /// A formula e: (a << FRACBITS) / b
    /// Deslocamos o dividendo 16 bits para a esquerda (em 64 bits) para
    /// compensar a divisao, mantendo o resultado no formato 16.16.
    ///
    /// C original: `FixedDiv()` / `FixedDiv2()` em `m_fixed.c`
    /// Nota: o C original tem protecao contra overflow que nao
    /// reproduzimos aqui por simplicidade; pode causar panic em
    /// divisao por zero (comportamento Rust padrao).
    #[inline]
    fn div(self, rhs: Self) -> Self {
        if rhs.0 == 0 {
            // O DOOM original retornava um valor fixo em caso de overflow.
            if self.0 >= 0 {
                Fixed::MAX
            } else {
                Fixed::MIN
            }
        } else if (self.0.abs() >> 14) >= rhs.0.abs() {
            // Protecao contra overflow: se o resultado excederia ~30 bits,
            // retornar MIN/MAX com sinal correto.
            // C original: `if ((abs(a)>>14) >= abs(b)) return (a^b)<0 ? MININT : MAXINT;`
            if (self.0 ^ rhs.0) < 0 {
                Fixed::MIN
            } else {
                Fixed::MAX
            }
        } else {
            Fixed((((self.0 as i64) << FRACBITS) / rhs.0 as i64) as i32)
        }
    }
}

impl Neg for Fixed {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Fixed(-self.0)
    }
}

impl fmt::Debug for Fixed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Mostra tanto o valor decimal aproximado quanto o raw hex
        let float_val = self.0 as f64 / FRACUNIT as f64;
        write!(f, "Fixed({:.4} = 0x{:08X})", float_val, self.0 as u32)
    }
}

impl fmt::Display for Fixed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let float_val = self.0 as f64 / FRACUNIT as f64;
        write!(f, "{:.4}", float_val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifica que a conversao inteiro -> fixed -> inteiro preserva o valor.
    #[test]
    fn from_int_roundtrip() {
        for n in [-100, -1, 0, 1, 42, 1000] {
            assert_eq!(Fixed::from_int(n).to_int(), n);
        }
    }

    /// Verifica a soma: 1.0 + 2.0 = 3.0
    #[test]
    fn add_basic() {
        let a = Fixed::from_int(1);
        let b = Fixed::from_int(2);
        assert_eq!((a + b).to_int(), 3);
    }

    /// Verifica a subtracao: 5.0 - 3.0 = 2.0
    #[test]
    fn sub_basic() {
        let a = Fixed::from_int(5);
        let b = Fixed::from_int(3);
        assert_eq!((a - b).to_int(), 2);
    }

    /// Verifica a multiplicacao: 3.0 * 4.0 = 12.0
    /// Testa o caminho critico do FixedMul com i64 intermediario.
    #[test]
    fn mul_integers() {
        let a = Fixed::from_int(3);
        let b = Fixed::from_int(4);
        assert_eq!((a * b).to_int(), 12);
    }

    /// Verifica multiplicacao fracionaria: 1.5 * 2.0 = 3.0
    /// Este teste reproduz o comportamento do DOOM original.
    #[test]
    fn mul_fractional() {
        let a = Fixed(FRACUNIT + FRACUNIT / 2); // 1.5
        let b = Fixed::from_int(2); // 2.0
        assert_eq!((a * b).to_int(), 3);
    }

    /// Verifica a divisao: 10.0 / 2.0 = 5.0
    #[test]
    fn div_basic() {
        let a = Fixed::from_int(10);
        let b = Fixed::from_int(2);
        assert_eq!((a / b).to_int(), 5);
    }

    /// Verifica que divisao por zero retorna MAX/MIN ao inves de panic.
    #[test]
    fn div_by_zero() {
        assert_eq!(Fixed::from_int(1) / Fixed::ZERO, Fixed::MAX);
        assert_eq!(Fixed::from_int(-1) / Fixed::ZERO, Fixed::MIN);
    }

    /// Verifica a negacao: -3.0 = -3.0
    #[test]
    fn neg_basic() {
        let a = Fixed::from_int(3);
        assert_eq!((-a).to_int(), -3);
    }

    /// Verifica que o valor absoluto funciona para positivos e negativos.
    #[test]
    fn abs_basic() {
        assert_eq!(Fixed::from_int(-5).abs(), Fixed::from_int(5));
        assert_eq!(Fixed::from_int(5).abs(), Fixed::from_int(5));
    }
}

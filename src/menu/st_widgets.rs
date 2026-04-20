//! # Widgets da Status Bar — Numeros, Percentuais e Icones
//!
//! Biblioteca de widgets para a status bar do DOOM:
//! - `StNumber` — numero right-justified (ammo, health, armor)
//! - `StPercent` — numero + simbolo de percentual (health%, armor%)
//! - `StMultIcon` — icone selecionavel (armas, chaves)
//! - `StBinIcon` — icone booleano (on/off)
//!
//! ## Dirty-checking
//!
//! Todos os widgets usam "dirty checking" para evitar redraw
//! desnecessario: cada widget armazena o valor anterior (`old_value`)
//! e so redesenha quando o valor muda. Isso era crucial no hardware
//! da epoca, onde cada pixel era desenhado via CPU.
//!
//! ## Valor sentinela 1994
//!
//! O numero 1994 (ano em que o DOOM foi desenvolvido) e usado como
//! sentinela para indicar "nao desenhar este widget". Usado para
//! armas que nao consomem municao (fist, chainsaw).
//!
//! C original: `if (*n->num == 1994) return;` em `STlib_drawNum()`
//!
//! ## Arquivo C original: `st_lib.c`, `st_lib.h`
//!
//! ## Conceitos que o leitor vai aprender
//! - Dirty-checking para otimizacao de rendering
//! - Widgets compostos (StPercent contem StNumber)
//! - Valor sentinela (1994) para controle de visibilidade

/// Valor sentinela: "nao desenhar este widget".
///
/// Armas sem municao (fist, chainsaw) usam este valor.
///
/// C original: `if (*n->num == 1994) return;` em `st_lib.c`
pub const ST_DONT_DRAW: i32 = 1994;

/// Numero maximo de digitos em um StNumber.
pub const ST_MAX_DIGITS: usize = 3;

// ---------------------------------------------------------------------------
// StNumber — numero right-justified
// ---------------------------------------------------------------------------

/// Widget de numero right-justified — exibe municao, saude, etc.
///
/// Desenha ate `width` digitos alinhados a direita. Numeros negativos
/// (frags negativos em deathmatch) exibem um sinal de menos.
///
/// C original: `st_number_t` em `st_lib.h`
#[derive(Debug, Clone)]
pub struct StNumber {
    /// Posicao X na tela (canto direito do numero)
    pub x: i32,
    /// Posicao Y na tela
    pub y: i32,
    /// Numero de digitos a exibir
    pub width: usize,
    /// Valor atual
    pub value: i32,
    /// Valor anterior (para dirty-checking)
    pub old_value: i32,
    /// Se o widget esta visivel
    pub on: bool,
}

impl StNumber {
    /// Cria um novo widget de numero.
    ///
    /// C original: `STlib_initNum()` em `st_lib.c`
    pub fn new(x: i32, y: i32, width: usize) -> Self {
        StNumber {
            x,
            y,
            width: width.min(ST_MAX_DIGITS),
            value: 0,
            old_value: -1, // forcar primeiro draw
            on: true,
        }
    }

    /// Atualiza o widget.
    ///
    /// Retorna `true` se o widget precisa ser redesenhado
    /// (valor mudou desde o ultimo check).
    ///
    /// C original: `STlib_updateNum()` em `st_lib.c`
    pub fn update(&mut self, value: i32, refresh: bool) -> bool {
        self.value = value;

        if !self.on {
            return false;
        }

        // Sentinela: nao desenhar
        if self.value == ST_DONT_DRAW {
            return false;
        }

        if refresh || self.value != self.old_value {
            self.old_value = self.value;
            return true;
        }

        false
    }

    /// Retorna os digitos para renderizacao (right-justified).
    ///
    /// C original: logica de `STlib_drawNum()` em `st_lib.c`
    pub fn digits(&self) -> Vec<Option<u8>> {
        if self.value == ST_DONT_DRAW {
            return vec![None; self.width];
        }

        let mut result = vec![None; self.width];
        let abs_val = self.value.unsigned_abs();
        let mut remaining = abs_val;

        for i in (0..self.width).rev() {
            result[i] = Some((remaining % 10) as u8);
            remaining /= 10;
            if remaining == 0 {
                break;
            }
        }

        result
    }

    /// Verifica se o numero e negativo (para exibir sinal de menos).
    pub fn is_negative(&self) -> bool {
        self.value < 0
    }
}

// ---------------------------------------------------------------------------
// StPercent — numero + simbolo de percentual
// ---------------------------------------------------------------------------

/// Widget de percentual — numero seguido de '%'.
///
/// Usado para health e armor na status bar.
///
/// C original: `st_percent_t` em `st_lib.h`
#[derive(Debug, Clone)]
pub struct StPercent {
    /// Widget de numero subjacente
    pub number: StNumber,
}

impl StPercent {
    /// Cria um novo widget de percentual.
    ///
    /// C original: `STlib_initPercent()` em `st_lib.c`
    pub fn new(x: i32, y: i32) -> Self {
        StPercent {
            number: StNumber::new(x, y, 3),
        }
    }

    /// Atualiza o widget.
    ///
    /// C original: `STlib_updatePercent()` em `st_lib.c`
    pub fn update(&mut self, value: i32, refresh: bool) -> bool {
        self.number.update(value, refresh)
    }

    /// Retorna o valor atual.
    pub fn value(&self) -> i32 {
        self.number.value
    }
}

// ---------------------------------------------------------------------------
// StMultIcon — icone selecionavel
// ---------------------------------------------------------------------------

/// Widget de icone selecionavel — exibe uma de N imagens.
///
/// Usado para armas no inventario e chaves. O indice seleciona
/// qual icone exibir de uma lista de patches.
///
/// C original: `st_multicon_t` em `st_lib.h`
#[derive(Debug, Clone)]
pub struct StMultIcon {
    /// Posicao X na tela
    pub x: i32,
    /// Posicao Y na tela
    pub y: i32,
    /// Indice do icone atual (-1 = nenhum)
    pub icon_index: i32,
    /// Indice anterior (dirty-checking)
    pub old_index: i32,
    /// Se o widget esta visivel
    pub on: bool,
}

impl StMultIcon {
    /// Cria um novo widget de multi-icone.
    ///
    /// C original: `STlib_initMultIcon()` em `st_lib.c`
    pub fn new(x: i32, y: i32) -> Self {
        StMultIcon {
            x,
            y,
            icon_index: -1,
            old_index: -1,
            on: true,
        }
    }

    /// Atualiza o widget.
    ///
    /// Retorna `true` se precisa redesenhar.
    ///
    /// C original: `STlib_updateMultIcon()` em `st_lib.c`
    pub fn update(&mut self, index: i32, refresh: bool) -> bool {
        self.icon_index = index;

        if !self.on {
            return false;
        }

        if refresh || self.icon_index != self.old_index {
            self.old_index = self.icon_index;
            return true;
        }

        false
    }
}

// ---------------------------------------------------------------------------
// StBinIcon — icone booleano
// ---------------------------------------------------------------------------

/// Widget de icone booleano — visivel ou invisivel.
///
/// Usado para indicar powerups ativos ou armas disponiveis.
///
/// C original: `st_binicon_t` em `st_lib.h`
#[derive(Debug, Clone)]
pub struct StBinIcon {
    /// Posicao X na tela
    pub x: i32,
    /// Posicao Y na tela
    pub y: i32,
    /// Valor atual (true = exibir icone)
    pub value: bool,
    /// Valor anterior (dirty-checking)
    pub old_value: bool,
    /// Se o widget esta visivel
    pub on: bool,
}

impl StBinIcon {
    /// Cria um novo widget de icone booleano.
    ///
    /// C original: `STlib_initBinIcon()` em `st_lib.c`
    pub fn new(x: i32, y: i32) -> Self {
        StBinIcon {
            x,
            y,
            value: false,
            old_value: false,
            on: true,
        }
    }

    /// Atualiza o widget.
    ///
    /// Retorna `true` se precisa redesenhar.
    ///
    /// C original: `STlib_updateBinIcon()` em `st_lib.c`
    pub fn update(&mut self, value: bool, refresh: bool) -> bool {
        self.value = value;

        if !self.on {
            return false;
        }

        if refresh || self.value != self.old_value {
            self.old_value = self.value;
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn st_number_init() {
        let num = StNumber::new(100, 200, 3);
        assert_eq!(num.x, 100);
        assert_eq!(num.y, 200);
        assert_eq!(num.width, 3);
        assert_eq!(num.value, 0);
        assert!(num.on);
    }

    #[test]
    fn st_number_update_dirty() {
        let mut num = StNumber::new(0, 0, 3);
        // Primeiro update com valor diferente do old_value
        assert!(num.update(50, false));
        // Mesmo valor — nao precisa redesenhar
        assert!(!num.update(50, false));
        // Valor diferente — precisa redesenhar
        assert!(num.update(75, false));
        // Refresh forca redesenho
        assert!(num.update(75, true));
    }

    #[test]
    fn st_number_digits() {
        let mut num = StNumber::new(0, 0, 3);
        num.value = 42;
        let digits = num.digits();
        assert_eq!(digits, vec![None, Some(4), Some(2)]);

        num.value = 100;
        let digits = num.digits();
        assert_eq!(digits, vec![Some(1), Some(0), Some(0)]);

        num.value = 5;
        let digits = num.digits();
        assert_eq!(digits, vec![None, None, Some(5)]);
    }

    #[test]
    fn st_number_dont_draw() {
        let mut num = StNumber::new(0, 0, 3);
        assert!(!num.update(ST_DONT_DRAW, false));
        let digits = num.digits();
        assert_eq!(digits, vec![None, None, None]);
    }

    #[test]
    fn st_number_negative() {
        let mut num = StNumber::new(0, 0, 3);
        num.value = -3;
        assert!(num.is_negative());
        // Digitos sao do valor absoluto
        let digits = num.digits();
        assert_eq!(digits, vec![None, None, Some(3)]);
    }

    #[test]
    fn st_percent() {
        let mut pct = StPercent::new(0, 0);
        assert!(pct.update(100, false));
        assert_eq!(pct.value(), 100);
        assert!(!pct.update(100, false)); // dirty check
    }

    #[test]
    fn st_multi_icon() {
        let mut icon = StMultIcon::new(0, 0);
        assert!(icon.update(0, false)); // -1 -> 0
        assert!(!icon.update(0, false)); // mesmo
        assert!(icon.update(2, false)); // mudou
    }

    #[test]
    fn st_bin_icon() {
        let mut icon = StBinIcon::new(0, 0);
        assert!(!icon.update(false, false)); // false -> false
        assert!(icon.update(true, false)); // false -> true
        assert!(!icon.update(true, false)); // true -> true
        assert!(icon.update(false, false)); // true -> false
    }

    #[test]
    fn st_bin_icon_off() {
        let mut icon = StBinIcon::new(0, 0);
        icon.on = false;
        assert!(!icon.update(true, true)); // off — nunca redesenha
    }
}

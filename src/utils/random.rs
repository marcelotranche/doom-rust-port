//! # Gerador de Numeros Pseudo-Aleatorios
//!
//! O DOOM usa um gerador de numeros aleatorios extremamente simples:
//! uma tabela fixa de 256 valores (0-255) e dois indices que avancam
//! sequencialmente. Nao ha nenhuma formula matematica — apenas uma
//! lookup table pre-definida.
//!
//! Por que tao simples? Porque o DOOM precisa que o RNG seja
//! **deterministico** para que demos (gravacoes de partidas) possam
//! ser reproduzidas frame a frame. Se dois jogadores comecam com o
//! mesmo indice, os mesmos "aleatorios" serao gerados na mesma ordem.
//!
//! Existem dois indices independentes:
//! - `p_random`: usado pela simulacao de jogo (p_*.c) — deterministico
//! - `m_random`: usado por efeitos visuais e menu — nao afeta gameplay
//!
//! ## Arquivo C original: `m_random.c` / `m_random.h`
//!
//! ## Conceitos que o leitor vai aprender
//! - RNG deterministico em jogos e por que e essencial para netplay/demos
//! - Lookup table como gerador de numeros "aleatorios"

/// Tabela de 256 valores pseudo-aleatorios do DOOM original.
///
/// Esta tabela e identica byte a byte ao array `rndtable[256]` em
/// `m_random.c`. Os valores foram escolhidos por id Software para
/// ter boa distribuicao sem padroes obvios.
///
/// C original: `unsigned char rndtable[256]` em `m_random.c`
#[rustfmt::skip]
const RND_TABLE: [u8; 256] = [
      0,   8, 109, 220, 222, 241, 149, 107,  75, 248, 254, 140,  16,  66,
     74,  21, 211,  47,  80, 242, 154,  27, 205, 128, 161,  89,  77,  36,
     95, 110,  85,  48, 212, 140, 211, 249,  22,  79, 200,  50,  28, 188,
     52, 140, 202, 120,  68, 145,  62,  70, 184, 190,  91, 197, 152, 224,
    149, 104,  25, 178, 252, 182, 202, 182, 141, 197,   4,  81, 181, 242,
    145,  42,  39, 227, 156, 198, 225, 193, 219,  93, 122, 175, 249,   0,
    175, 143,  70, 239,  46, 246, 163,  53, 163, 109, 168, 135,   2, 235,
     25,  92,  20, 145, 138,  77,  69, 166,  78, 176, 173, 212, 166, 113,
     94, 161,  41,  50, 239,  49, 111, 164,  70,  60,   2,  37, 171,  75,
    136, 156,  11,  56,  42, 146, 138, 229,  73, 146,  77,  61,  98, 196,
    135, 106,  63, 197, 195,  86,  96, 203, 113, 101, 170, 247, 181, 113,
     80, 250, 108,   7, 255, 237, 129, 226,  79, 107, 112, 166, 103, 241,
     24, 223, 239, 120, 198,  58,  60,  82, 128,   3, 184,  66, 143, 224,
    145, 224,  81, 206, 163,  45,  63,  90, 168, 114,  59,  33, 159,  95,
     28, 139, 123,  98, 125, 196,  15,  70, 194, 253,  54,  14, 109, 226,
     71,  17, 161,  93, 186,  87, 244, 138,  20,  52, 123, 251,  26,  36,
     17,  46,  52, 231, 232,  76,  31, 221,  84,  37, 216, 165, 212, 106,
    197, 242,  98,  43,  39, 175, 254, 145, 190,  84, 118, 222, 187, 136,
    120, 163, 236, 249,
];

/// Gerador de numeros pseudo-aleatorios do DOOM.
///
/// Mantem dois indices independentes na mesma tabela:
/// - `p_index`: para a simulacao de jogo (deterministico, afeta gameplay)
/// - `m_index`: para efeitos visuais e menus (nao afeta gameplay)
///
/// No C original, esses eram globals `prndindex` e `rndindex` em `m_random.c`.
/// Em Rust, encapsulamos em uma struct para evitar estado global.
#[derive(Debug, Clone)]
pub struct DoomRandom {
    /// Indice para P_Random() — usado pela simulacao de jogo.
    /// C original: `int prndindex` em `m_random.c`
    p_index: u8,
    /// Indice para M_Random() — usado por efeitos visuais.
    /// C original: `int rndindex` em `m_random.c`
    m_index: u8,
}

impl DoomRandom {
    /// Cria um novo gerador com ambos os indices em 0.
    ///
    /// C original: estado inicial das globals `rndindex = prndindex = 0`
    pub fn new() -> Self {
        DoomRandom {
            p_index: 0,
            m_index: 0,
        }
    }

    /// Retorna o proximo numero pseudo-aleatorio para a simulacao de jogo.
    ///
    /// Este e o RNG deterministico — o mesmo indice sempre produz a mesma
    /// sequencia. Essencial para demos e netplay funcionarem corretamente.
    ///
    /// Retorna um valor de 0 a 255.
    ///
    /// C original: `P_Random()` em `m_random.c`
    /// ```c
    /// int P_Random(void) {
    ///     prndindex = (prndindex + 1) & 0xff;
    ///     return rndtable[prndindex];
    /// }
    /// ```
    pub fn p_random(&mut self) -> u8 {
        self.p_index = self.p_index.wrapping_add(1);
        RND_TABLE[self.p_index as usize]
    }

    /// Retorna o proximo numero pseudo-aleatorio para uso geral.
    ///
    /// Usado por efeitos visuais, menus, e outros sistemas que nao
    /// precisam ser deterministicos. Tem indice separado do P_Random
    /// para nao interferir na simulacao.
    ///
    /// Retorna um valor de 0 a 255.
    ///
    /// C original: `M_Random()` em `m_random.c`
    pub fn m_random(&mut self) -> u8 {
        self.m_index = self.m_index.wrapping_add(1);
        RND_TABLE[self.m_index as usize]
    }

    /// Reseta ambos os indices para 0.
    ///
    /// Chamado no inicio de um nivel ou demo para garantir que a
    /// sequencia de numeros aleatorios seja reproduzivel.
    ///
    /// C original: `M_ClearRandom()` em `m_random.c`
    pub fn clear(&mut self) {
        self.p_index = 0;
        self.m_index = 0;
    }

    /// Retorna o indice atual de P_Random (para save/load de demos).
    pub fn p_index(&self) -> u8 {
        self.p_index
    }

    /// Retorna o indice atual de M_Random.
    pub fn m_index(&self) -> u8 {
        self.m_index
    }
}

impl Default for DoomRandom {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifica que os primeiros valores de P_Random coincidem com o DOOM original.
    /// No C, o indice comeca em 0 e avanca para 1 antes de ler.
    /// Entao P_Random() retorna rndtable[1], rndtable[2], rndtable[3]...
    #[test]
    fn p_random_first_values() {
        let mut rng = DoomRandom::new();
        assert_eq!(rng.p_random(), 8);   // rndtable[1]
        assert_eq!(rng.p_random(), 109); // rndtable[2]
        assert_eq!(rng.p_random(), 220); // rndtable[3]
    }

    /// Verifica que P_Random e M_Random tem indices independentes.
    #[test]
    fn independent_indices() {
        let mut rng = DoomRandom::new();
        let p1 = rng.p_random();
        let m1 = rng.m_random();
        // Ambos devem retornar rndtable[1] pois seus indices sao independentes
        assert_eq!(p1, m1);
        assert_eq!(p1, 8); // rndtable[1]
    }

    /// Verifica que clear() reseta ambos os indices.
    #[test]
    fn clear_resets() {
        let mut rng = DoomRandom::new();
        rng.p_random();
        rng.p_random();
        rng.m_random();
        rng.clear();
        // Apos clear, deve voltar a produzir os mesmos valores
        assert_eq!(rng.p_random(), 8);
        assert_eq!(rng.m_random(), 8);
    }

    /// Verifica que apos 256 chamadas, o indice volta ao inicio (wraparound).
    #[test]
    fn wraparound_256() {
        let mut rng = DoomRandom::new();
        for _ in 0..256 {
            rng.p_random();
        }
        // Apos 256 chamadas, p_index voltou a 0, proxima chamada retorna rndtable[1]
        assert_eq!(rng.p_random(), 8);
    }

    /// Verifica que a tabela tem os valores corretos nos extremos.
    #[test]
    fn table_boundary_values() {
        assert_eq!(RND_TABLE[0], 0);     // primeiro valor
        assert_eq!(RND_TABLE[255], 249); // ultimo valor
    }
}

//! # Widgets de HUD — Texto, Mensagens e Input
//!
//! Biblioteca de widgets reutilizaveis para o heads-up display:
//! - `HudTextLine` — linha de texto renderizada com font do WAD
//! - `HudScrollText` — texto com scroll (mensagens do jogador)
//! - `HudInputText` — texto editavel (chat multiplayer)
//!
//! ## Font do DOOM
//!
//! O DOOM usa uma fonte bitmap armazenada no WAD como patches
//! individuais (STCFN033 a STCFN095). Cada patch corresponde a
//! um caractere ASCII de '!' (33) ate '_' (95). Espacos e
//! caracteres fora dessa faixa usam largura fixa de 4 pixels.
//!
//! ```text
//! ASCII:  ! " # $ % & ' ( ) * + , - . / 0 1 2 ... Z [ \ ] ^ _
//! Index:  0 1 2 3 4 5 6 7 8 9 ...                          62
//! Lump:  STCFN033 STCFN034 ... STCFN095
//! ```
//!
//! ## Scroll de mensagens
//!
//! O widget `HudScrollText` usa um ring buffer de N linhas.
//! Novas mensagens sao adicionadas na posicao `current_line`,
//! e ao atingir o limite, as linhas mais antigas sao sobrescritas.
//! Isso permite exibir as ultimas N mensagens sem alocacao dinamica.
//!
//! ## Arquivo C original: `hu_lib.c`, `hu_lib.h`
//!
//! ## Conceitos que o leitor vai aprender
//! - Widgets de texto com fonte bitmap (patch-based)
//! - Ring buffer para scroll de mensagens
//! - Separacao widget-library vs widget-user (hu_lib vs hu_stuff)

/// Primeiro caractere da fonte do HUD (ASCII '!').
///
/// C original: `#define HU_FONTSTART '!'` em `hu_lib.h`
pub const HU_FONTSTART: u8 = b'!';

/// Ultimo caractere da fonte do HUD (ASCII '_').
///
/// C original: `#define HU_FONTEND '_'` em `hu_lib.h`
pub const HU_FONTEND: u8 = b'_';

/// Numero de caracteres na fonte do HUD.
///
/// C original: `#define HU_FONTSIZE (HU_FONTEND - HU_FONTSTART + 1)`
pub const HU_FONTSIZE: usize = (HU_FONTEND - HU_FONTSTART + 1) as usize;

/// Numero maximo de linhas no widget de scroll.
///
/// C original: `#define HU_MAXLINES 4` em `hu_lib.h`
pub const HU_MAXLINES: usize = 4;

/// Comprimento maximo de uma linha de texto.
///
/// C original: `#define HU_MAXLINELENGTH 80` em `hu_lib.h`
pub const HU_MAXLINELENGTH: usize = 80;

/// Largura padrao de um espaco (quando o caractere nao tem glyph).
///
/// C original: largura fixa usada em `HUlib_drawTextLine()` para espacos
pub const HU_SPACE_WIDTH: i32 = 4;

// ---------------------------------------------------------------------------
// Informacao de fonte
// ---------------------------------------------------------------------------

/// Informacao de um glyph da fonte do HUD.
///
/// No DOOM completo, cada glyph e um `patch_t` do WAD.
/// Aqui armazenamos apenas as dimensoes para calculo de layout.
/// A renderizacao real sera feita pela camada de video.
#[derive(Debug, Clone, Copy)]
pub struct FontGlyph {
    /// Largura do glyph em pixels
    pub width: i32,
    /// Altura do glyph em pixels
    pub height: i32,
    /// Indice do lump no WAD (-1 = nao carregado)
    pub lumpnum: i32,
}

impl FontGlyph {
    /// Cria um glyph com dimensoes padrao.
    pub const fn new(width: i32, height: i32) -> Self {
        FontGlyph {
            width,
            height,
            lumpnum: -1,
        }
    }
}

impl Default for FontGlyph {
    fn default() -> Self {
        FontGlyph::new(8, 12) // tamanho tipico da fonte do DOOM
    }
}

/// Fonte do HUD — array de glyphs indexado por (char - HU_FONTSTART).
///
/// C original: `patch_t* hu_font[HU_FONTSIZE]` em `hu_stuff.c`
#[derive(Debug, Clone)]
pub struct HudFont {
    /// Glyphs da fonte (HU_FONTSIZE entradas)
    pub glyphs: Vec<FontGlyph>,
}

impl HudFont {
    /// Cria uma fonte com glyphs padrao.
    ///
    /// No DOOM completo, os glyphs sao carregados dos lumps
    /// STCFN033-STCFN095 do WAD. Aqui usamos dimensoes padrao.
    pub fn new() -> Self {
        HudFont {
            glyphs: vec![FontGlyph::default(); HU_FONTSIZE],
        }
    }

    /// Retorna a largura de um caractere.
    ///
    /// Caracteres fora da faixa HU_FONTSTART..HU_FONTEND
    /// retornam HU_SPACE_WIDTH (espaco).
    pub fn char_width(&self, ch: u8) -> i32 {
        if (HU_FONTSTART..=HU_FONTEND).contains(&ch) {
            let idx = (ch - HU_FONTSTART) as usize;
            self.glyphs[idx].width
        } else {
            HU_SPACE_WIDTH
        }
    }

    /// Calcula a largura total de uma string em pixels.
    ///
    /// C original: `M_StringWidth()` em `m_menu.c`
    pub fn string_width(&self, text: &str) -> i32 {
        text.bytes().map(|ch| self.char_width(ch)).sum()
    }

    /// Calcula a altura de uma string em pixels (considerando newlines).
    ///
    /// C original: `M_StringHeight()` em `m_menu.c`
    pub fn string_height(&self, text: &str) -> i32 {
        let line_height = if self.glyphs.is_empty() {
            12
        } else {
            self.glyphs[0].height
        };

        let lines = text.bytes().filter(|&ch| ch == b'\n').count() + 1;
        line_height * lines as i32
    }
}

impl Default for HudFont {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// HudTextLine — linha de texto simples
// ---------------------------------------------------------------------------

/// Linha de texto renderizavel — widget base do HUD.
///
/// Armazena uma string de ate HU_MAXLINELENGTH caracteres,
/// com posicao de tela e referencia a fonte para renderizacao.
///
/// C original: `hu_textline_t` em `hu_lib.h`
#[derive(Debug, Clone)]
pub struct HudTextLine {
    /// Posicao X na tela
    pub x: i32,
    /// Posicao Y na tela
    pub y: i32,
    /// Texto armazenado (ate HU_MAXLINELENGTH caracteres)
    pub text: String,
    /// Flag: precisa ser redesenhado
    pub needs_update: bool,
}

impl HudTextLine {
    /// Cria uma nova linha de texto vazia.
    ///
    /// C original: `HUlib_initTextLine()` em `hu_lib.c`
    pub fn new(x: i32, y: i32) -> Self {
        HudTextLine {
            x,
            y,
            text: String::with_capacity(HU_MAXLINELENGTH),
            needs_update: true,
        }
    }

    /// Adiciona um caractere ao final da linha.
    ///
    /// Retorna `true` se o caractere foi adicionado com sucesso.
    ///
    /// C original: `HUlib_addCharToTextLine()` em `hu_lib.c`
    pub fn add_char(&mut self, ch: char) -> bool {
        if self.text.len() >= HU_MAXLINELENGTH {
            return false;
        }
        self.text.push(ch);
        self.needs_update = true;
        true
    }

    /// Remove o ultimo caractere da linha.
    ///
    /// Retorna `true` se um caractere foi removido.
    ///
    /// C original: `HUlib_delCharFromTextLine()` em `hu_lib.c`
    pub fn del_char(&mut self) -> bool {
        if self.text.is_empty() {
            return false;
        }
        self.text.pop();
        self.needs_update = true;
        true
    }

    /// Limpa todo o texto da linha.
    ///
    /// C original: `HUlib_clearTextLine()` em `hu_lib.c`
    pub fn clear(&mut self) {
        self.text.clear();
        self.needs_update = true;
    }

    /// Retorna o comprimento da linha em caracteres.
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// Verifica se a linha esta vazia.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

// ---------------------------------------------------------------------------
// HudScrollText — texto com scroll (mensagens)
// ---------------------------------------------------------------------------

/// Texto com scroll — exibe as ultimas N linhas de mensagens.
///
/// Usa um ring buffer de HU_MAXLINES linhas. Quando uma nova
/// mensagem e adicionada, avanca o indice circular e sobrescreve
/// a linha mais antiga.
///
/// C original: `hu_stext_t` em `hu_lib.h`
#[derive(Debug, Clone)]
pub struct HudScrollText {
    /// Linhas de texto (ring buffer)
    pub lines: Vec<HudTextLine>,
    /// Numero de linhas visiveis
    pub height: usize,
    /// Indice da linha atual no ring buffer
    pub current_line: usize,
    /// Se o widget esta visivel
    pub visible: bool,
}

impl HudScrollText {
    /// Cria um novo widget de scroll text.
    ///
    /// C original: `HUlib_initSText()` em `hu_lib.c`
    pub fn new(x: i32, y: i32, height: usize) -> Self {
        let height = height.min(HU_MAXLINES);
        let lines = (0..height)
            .map(|i| {
                // Cada linha abaixo da anterior (assumindo altura 12)
                HudTextLine::new(x, y + (i as i32) * 12)
            })
            .collect();

        HudScrollText {
            lines,
            height,
            current_line: 0,
            visible: true,
        }
    }

    /// Adiciona uma mensagem ao scroll text.
    ///
    /// Avanca para a proxima linha no ring buffer, limpa-a,
    /// e copia a mensagem. Opcionalmente adiciona um prefixo.
    ///
    /// C original: `HUlib_addMessageToSText()` em `hu_lib.c`
    pub fn add_message(&mut self, prefix: Option<&str>, msg: &str) {
        // Avancar para proxima linha
        self.current_line = (self.current_line + 1) % self.height;
        self.lines[self.current_line].clear();

        // Adicionar prefixo se houver
        if let Some(pfx) = prefix {
            for ch in pfx.chars() {
                self.lines[self.current_line].add_char(ch);
            }
        }

        // Adicionar mensagem
        for ch in msg.chars() {
            self.lines[self.current_line].add_char(ch);
        }
    }

    /// Retorna as linhas na ordem de exibicao (mais antiga primeiro).
    pub fn visible_lines(&self) -> Vec<&HudTextLine> {
        let mut result = Vec::with_capacity(self.height);
        for i in 0..self.height {
            let idx = (self.current_line + 1 + i) % self.height;
            result.push(&self.lines[idx]);
        }
        result
    }
}

// ---------------------------------------------------------------------------
// HudInputText — texto editavel (chat)
// ---------------------------------------------------------------------------

/// Texto editavel — widget de input para chat multiplayer.
///
/// Extende `HudTextLine` com suporte a edicao: backspace
/// respeita a margem esquerda (prefixo nao-deletavel),
/// e Enter finaliza o input.
///
/// C original: `hu_itext_t` em `hu_lib.h`
#[derive(Debug, Clone)]
pub struct HudInputText {
    /// Linha de texto subjacente
    pub line: HudTextLine,
    /// Margem esquerda — caracteres antes desta posicao nao podem ser deletados.
    /// Usado para prefixos como "Player: ".
    pub left_margin: usize,
    /// Se o widget esta visivel/ativo
    pub visible: bool,
}

impl HudInputText {
    /// Cria um novo widget de input.
    ///
    /// C original: `HUlib_initIText()` em `hu_lib.c`
    pub fn new(x: i32, y: i32) -> Self {
        HudInputText {
            line: HudTextLine::new(x, y),
            left_margin: 0,
            visible: false,
        }
    }

    /// Processa um caractere de input.
    ///
    /// - Caracteres imprimiveis: adiciona ao texto
    /// - Backspace (8): remove ultimo caractere (respeitando margem)
    /// - Enter (13): retorna `true` (input completo)
    ///
    /// Retorna `true` se o input foi finalizado (Enter pressionado).
    ///
    /// C original: `HUlib_keyInIText()` em `hu_lib.c`
    pub fn key_input(&mut self, ch: u8) -> bool {
        match ch {
            // Enter — finalizar input
            13 => true,

            // Backspace — deletar (respeitando margem)
            8 => {
                if self.line.len() > self.left_margin {
                    self.line.del_char();
                }
                false
            }

            // Caractere imprimivel — adicionar
            32..=126 => {
                self.line.add_char(ch as char);
                false
            }

            // Outros — ignorar
            _ => false,
        }
    }

    /// Limpa o texto apos a margem esquerda.
    pub fn clear_input(&mut self) {
        let prefix: String = self.line.text.chars().take(self.left_margin).collect();
        self.line.clear();
        for ch in prefix.chars() {
            self.line.add_char(ch);
        }
    }

    /// Define o prefixo (margem esquerda) do input.
    pub fn set_prefix(&mut self, prefix: &str) {
        self.line.clear();
        for ch in prefix.chars() {
            self.line.add_char(ch);
        }
        self.left_margin = self.line.len();
    }

    /// Retorna o texto digitado (apos a margem).
    pub fn input_text(&self) -> &str {
        if self.left_margin < self.line.text.len() {
            &self.line.text[self.left_margin..]
        } else {
            ""
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_constants() {
        assert_eq!(HU_FONTSTART, b'!');
        assert_eq!(HU_FONTEND, b'_');
        assert_eq!(HU_FONTSIZE, 63);
    }

    #[test]
    fn hud_font_char_width() {
        let font = HudFont::new();
        // Caractere na faixa da fonte
        assert_eq!(font.char_width(b'A'), 8); // largura padrao
        // Espaco (fora da faixa)
        assert_eq!(font.char_width(b' '), HU_SPACE_WIDTH);
    }

    #[test]
    fn hud_font_string_width() {
        let font = HudFont::new();
        assert_eq!(font.string_width("AB"), 16); // 2 * 8
        assert_eq!(font.string_width("A B"), 20); // 8 + 4 + 8
    }

    #[test]
    fn hud_font_string_height() {
        let font = HudFont::new();
        assert_eq!(font.string_height("hello"), 12); // uma linha
        assert_eq!(font.string_height("a\nb"), 24); // duas linhas
        assert_eq!(font.string_height("a\nb\nc"), 36); // tres linhas
    }

    #[test]
    fn text_line_add_del() {
        let mut line = HudTextLine::new(0, 0);
        assert!(line.is_empty());

        line.add_char('H');
        line.add_char('I');
        assert_eq!(line.len(), 2);
        assert_eq!(line.text, "HI");

        line.del_char();
        assert_eq!(line.text, "H");

        line.clear();
        assert!(line.is_empty());
    }

    #[test]
    fn text_line_max_length() {
        let mut line = HudTextLine::new(0, 0);
        for _ in 0..HU_MAXLINELENGTH {
            assert!(line.add_char('X'));
        }
        // Proximo deve falhar
        assert!(!line.add_char('Y'));
        assert_eq!(line.len(), HU_MAXLINELENGTH);
    }

    #[test]
    fn scroll_text_messages() {
        let mut scroll = HudScrollText::new(0, 0, 3);
        assert_eq!(scroll.height, 3);

        scroll.add_message(None, "First");
        scroll.add_message(None, "Second");
        scroll.add_message(None, "Third");

        let visible = scroll.visible_lines();
        assert_eq!(visible.len(), 3);
        // A linha mais antiga e a primeira na lista visivel
        assert_eq!(visible[0].text, "First");
        assert_eq!(visible[1].text, "Second");
        assert_eq!(visible[2].text, "Third");
    }

    #[test]
    fn scroll_text_wraparound() {
        let mut scroll = HudScrollText::new(0, 0, 2);

        scroll.add_message(None, "First");
        scroll.add_message(None, "Second");
        // Terceira mensagem sobrescreve a primeira (ring buffer)
        scroll.add_message(None, "Third");

        let visible = scroll.visible_lines();
        assert_eq!(visible[0].text, "Second");
        assert_eq!(visible[1].text, "Third");
    }

    #[test]
    fn scroll_text_with_prefix() {
        let mut scroll = HudScrollText::new(0, 0, 2);
        scroll.add_message(Some("Player1: "), "hello");

        let _visible = scroll.visible_lines();
        // A mensagem mais recente
        let newest_idx = scroll.current_line;
        assert_eq!(scroll.lines[newest_idx].text, "Player1: hello");
    }

    #[test]
    fn input_text_basic() {
        let mut input = HudInputText::new(0, 0);
        input.set_prefix("Say: ");
        assert_eq!(input.left_margin, 5);

        // Digitar texto
        input.key_input(b'H');
        input.key_input(b'i');
        assert_eq!(input.input_text(), "Hi");

        // Backspace
        input.key_input(8); // backspace
        assert_eq!(input.input_text(), "H");

        // Nao pode deletar o prefixo
        input.key_input(8);
        assert_eq!(input.input_text(), "");
        input.key_input(8); // ja na margem
        assert_eq!(input.line.text, "Say: ");
    }

    #[test]
    fn input_text_enter() {
        let mut input = HudInputText::new(0, 0);
        input.key_input(b'Y');
        assert!(!input.key_input(b'o')); // nao finalizado
        assert!(input.key_input(13)); // Enter — finalizado
    }

    #[test]
    fn input_text_clear() {
        let mut input = HudInputText::new(0, 0);
        input.set_prefix(">> ");
        input.key_input(b'A');
        input.key_input(b'B');
        assert_eq!(input.input_text(), "AB");

        input.clear_input();
        assert_eq!(input.input_text(), "");
        assert_eq!(input.line.text, ">> "); // prefixo mantido
    }
}

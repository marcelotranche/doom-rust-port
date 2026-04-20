//! # Heads-Up Display — Mensagens, Titulo e Chat
//!
//! O HUD do DOOM exibe informacoes sobrepostas a vista do jogo:
//! - Mensagens do jogador ("Picked up a shotgun", "A secret is revealed!")
//! - Titulo do mapa (visivel no automap)
//! - Chat multiplayer (ativado com 't')
//!
//! ## Fluxo de mensagens
//!
//! ```text
//! player.message = "Picked up..."
//!       |
//!       v
//! HU_Ticker() detecta nova mensagem
//!       |
//!       v
//! Adiciona ao HudScrollText
//!       |
//!       v
//! message_counter = HU_MSGTIMEOUT (140 ticks = 4 segundos)
//!       |
//!       v
//! HU_Drawer() exibe enquanto counter > 0
//! ```
//!
//! ## Chat multiplayer
//!
//! Em partidas multiplayer, pressionar 't' abre o input de chat.
//! Os caracteres digitados sao armazenados no `HudInputText` e
//! enviados via `TicCmd.chatchar` a cada tick. Outros jogadores
//! recebem os caracteres e reconstroem a mensagem.
//!
//! ## Arquivo C original: `hu_stuff.c`, `hu_stuff.h`
//!
//! ## Conceitos que o leitor vai aprender
//! - Sistema de mensagens temporarias com countdown
//! - Input de chat multiplayer via tic commands
//! - Responder pattern: HU_Responder processa eventos

use super::hud_widgets::*;

/// Tempo de exibicao de mensagens (em ticks).
///
/// 4 segundos a 35 Hz = 140 ticks.
///
/// C original: `#define HU_MSGTIMEOUT (4*TICRATE)` em `hu_stuff.h`
pub const HU_MSGTIMEOUT: i32 = 4 * 35;

/// Posicao X das mensagens.
///
/// C original: `#define HU_MSGX 0` em `hu_stuff.h`
pub const HU_MSGX: i32 = 0;

/// Posicao Y das mensagens.
///
/// C original: `#define HU_MSGY 0` em `hu_stuff.h`
pub const HU_MSGY: i32 = 0;

/// Posicao Y do titulo do mapa.
///
/// C original: `#define HU_TITLEY (167 - SHORT(hu_font[0]->height))`
pub const HU_TITLEY: i32 = 167 - 12;

/// Tecla para ativar chat.
///
/// C original: `#define HU_INPUTTOGGLE 't'` em `hu_stuff.h`
pub const HU_INPUTTOGGLE: u8 = b't';

/// Numero maximo de jogadores para chat.
///
/// C original: `MAXPLAYERS` em `doomdef.h`
pub const HU_MAXPLAYERS: usize = 4;

/// Endereco de broadcast para chat (todos os jogadores).
///
/// C original: `#define HU_BROADCAST 5` em `hu_stuff.h`
pub const HU_BROADCAST: usize = 5;

// ---------------------------------------------------------------------------
// Nomes dos mapas
// ---------------------------------------------------------------------------

/// Nomes dos mapas de DOOM 1 (Episode 1).
///
/// C original: `mapnames[]` em `hu_stuff.c`
pub const E1_MAP_NAMES: [&str; 9] = [
    "E1M1: Hangar",
    "E1M2: Nuclear Plant",
    "E1M3: Toxin Refinery",
    "E1M4: Command Control",
    "E1M5: Phobos Lab",
    "E1M6: Central Processing",
    "E1M7: Computer Station",
    "E1M8: Phobos Anomaly",
    "E1M9: Military Base",
];

// ---------------------------------------------------------------------------
// HeadsUpDisplay
// ---------------------------------------------------------------------------

/// Heads-up display — gerencia mensagens, titulo e chat.
///
/// C original: globals em `hu_stuff.c` (`w_message`, `w_title`,
/// `w_chat`, `message_counter`, `chat_on`, etc.)
#[derive(Debug)]
pub struct HeadsUpDisplay {
    /// Widget de mensagens (scroll text)
    pub messages: HudScrollText,
    /// Widget do titulo do mapa
    pub title: HudTextLine,
    /// Widget de input de chat
    pub chat_input: HudInputText,
    /// Contador de tempo restante para exibir mensagem
    pub message_counter: i32,
    /// Se as mensagens estao habilitadas
    pub message_on: bool,
    /// Se o chat esta ativo (digitando)
    pub chat_on: bool,
    /// Se o HUD esta ativo (level em andamento)
    pub active: bool,
    /// Fonte do HUD
    pub font: HudFont,
    /// Fila de caracteres de chat para enviar via TicCmd
    chat_queue: Vec<u8>,
}

impl HeadsUpDisplay {
    /// Cria um novo HUD.
    ///
    /// C original: `HU_Init()` em `hu_stuff.c`
    pub fn new() -> Self {
        HeadsUpDisplay {
            messages: HudScrollText::new(HU_MSGX, HU_MSGY, HU_MAXLINES),
            title: HudTextLine::new(HU_MSGX, HU_TITLEY),
            chat_input: HudInputText::new(HU_MSGX, HU_TITLEY + 12),
            message_counter: 0,
            message_on: true,
            chat_on: false,
            active: false,
            font: HudFont::new(),
            chat_queue: Vec::with_capacity(128),
        }
    }

    /// Inicializa o HUD para um novo nivel.
    ///
    /// Define o titulo do mapa e reseta o estado.
    ///
    /// C original: `HU_Start()` em `hu_stuff.c`
    pub fn start(&mut self, map_name: &str) {
        self.active = true;
        self.message_counter = 0;
        self.chat_on = false;

        // Definir titulo do mapa
        self.title.clear();
        for ch in map_name.chars() {
            self.title.add_char(ch);
        }
    }

    /// Atualiza o HUD a cada tick.
    ///
    /// Decrementa o contador de mensagens e verifica se ha
    /// novas mensagens do jogador.
    ///
    /// C original: `HU_Ticker()` em `hu_stuff.c`
    pub fn ticker(&mut self, player_message: Option<&str>) {
        if !self.active {
            return;
        }

        // Verificar nova mensagem do jogador
        if let Some(msg) = player_message {
            self.display_message(msg);
        }

        // Decrementar contador de mensagem
        if self.message_counter > 0 {
            self.message_counter -= 1;
        }
    }

    /// Exibe uma mensagem no HUD.
    ///
    /// A mensagem sera visivel por HU_MSGTIMEOUT ticks.
    pub fn display_message(&mut self, msg: &str) {
        self.messages.add_message(None, msg);
        self.message_counter = HU_MSGTIMEOUT;
        self.message_on = true;
    }

    /// Processa um evento de input.
    ///
    /// Retorna `true` se o evento foi consumido pelo HUD.
    ///
    /// C original: `HU_Responder()` em `hu_stuff.c`
    pub fn responder(&mut self, key: u8, key_down: bool) -> bool {
        if !self.active {
            return false;
        }

        if self.chat_on {
            if !key_down {
                return false;
            }

            // Processar input de chat
            if self.chat_input.key_input(key) {
                // Enter pressionado — enviar mensagem
                let msg = self.chat_input.input_text().to_string();
                if !msg.is_empty() {
                    self.display_message(&msg);
                    // Enfileirar caracteres para envio via TicCmd
                    for ch in msg.bytes() {
                        self.chat_queue.push(ch);
                    }
                }
                self.chat_on = false;
                self.chat_input.clear_input();
                return true;
            }

            // Escape cancela chat
            if key == 27 {
                self.chat_on = false;
                self.chat_input.clear_input();
                return true;
            }

            return true; // consumir todas as teclas durante chat
        }

        // Verificar tecla de chat
        if key_down && key == HU_INPUTTOGGLE {
            self.chat_on = true;
            self.chat_input.visible = true;
            self.chat_input.set_prefix("");
            return true;
        }

        false
    }

    /// Desenfileira um caractere de chat para enviar via TicCmd.
    ///
    /// C original: `HU_dequeueChatChar()` em `hu_stuff.c`
    pub fn dequeue_chat_char(&mut self) -> Option<u8> {
        if self.chat_queue.is_empty() {
            None
        } else {
            Some(self.chat_queue.remove(0))
        }
    }

    /// Verifica se ha mensagem visivel para exibir.
    pub fn has_visible_message(&self) -> bool {
        self.message_on && self.message_counter > 0
    }
}

impl Default for HeadsUpDisplay {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hud_init() {
        let hud = HeadsUpDisplay::new();
        assert!(!hud.active);
        assert!(!hud.chat_on);
        assert_eq!(hud.message_counter, 0);
    }

    #[test]
    fn hud_start_level() {
        let mut hud = HeadsUpDisplay::new();
        hud.start("E1M1: Hangar");
        assert!(hud.active);
        assert_eq!(hud.title.text, "E1M1: Hangar");
    }

    #[test]
    fn hud_message_display() {
        let mut hud = HeadsUpDisplay::new();
        hud.start("E1M1");

        hud.display_message("Picked up a shotgun.");
        assert!(hud.has_visible_message());
        assert_eq!(hud.message_counter, HU_MSGTIMEOUT);

        // Ticker decrementa
        hud.ticker(None);
        assert_eq!(hud.message_counter, HU_MSGTIMEOUT - 1);
    }

    #[test]
    fn hud_message_timeout() {
        let mut hud = HeadsUpDisplay::new();
        hud.start("E1M1");

        hud.display_message("Test");
        for _ in 0..HU_MSGTIMEOUT {
            hud.ticker(None);
        }
        assert!(!hud.has_visible_message());
    }

    #[test]
    fn hud_ticker_receives_message() {
        let mut hud = HeadsUpDisplay::new();
        hud.start("E1M1");

        hud.ticker(Some("A secret is revealed!"));
        assert!(hud.has_visible_message());
    }

    #[test]
    fn hud_chat_toggle() {
        let mut hud = HeadsUpDisplay::new();
        hud.start("E1M1");

        // 't' abre chat
        assert!(hud.responder(HU_INPUTTOGGLE, true));
        assert!(hud.chat_on);

        // Escape cancela
        assert!(hud.responder(27, true));
        assert!(!hud.chat_on);
    }

    #[test]
    fn hud_chat_send() {
        let mut hud = HeadsUpDisplay::new();
        hud.start("E1M1");

        // Abrir chat
        hud.responder(HU_INPUTTOGGLE, true);

        // Digitar "Hi"
        hud.responder(b'H', true);
        hud.responder(b'i', true);
        assert_eq!(hud.chat_input.input_text(), "Hi");

        // Enter envia
        hud.responder(13, true);
        assert!(!hud.chat_on);
        assert!(hud.has_visible_message());

        // Caracteres enfileirados
        assert_eq!(hud.dequeue_chat_char(), Some(b'H'));
        assert_eq!(hud.dequeue_chat_char(), Some(b'i'));
        assert_eq!(hud.dequeue_chat_char(), None);
    }

    #[test]
    fn hud_responder_inactive() {
        let mut hud = HeadsUpDisplay::new();
        // HUD nao ativo — nao consome eventos
        assert!(!hud.responder(HU_INPUTTOGGLE, true));
    }
}

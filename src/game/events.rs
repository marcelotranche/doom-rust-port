//! # Sistema de Eventos e Comandos de Tick
//!
//! Define os tipos fundamentais de entrada do DOOM: eventos de input
//! (teclado, mouse, joystick) e comandos de tick (ticcmd) que
//! representam as acoes do jogador em cada tick logico.
//!
//! ## Fluxo de eventos
//!
//! ```text
//! Plataforma (SDL2)
//!   → I_StartTic()           — le input do hardware
//!   → D_PostEvent()          — enfileira event_t no ring buffer
//!   → D_ProcessEvents()      — despacha para cadeia de responders
//!     → M_Responder()        — menu consome evento?
//!     → G_Responder()        — jogo consome evento?
//!   → G_BuildTiccmd()        — converte estado de teclas em ticcmd_t
//! ```
//!
//! ## Ticcmd
//!
//! O `TicCmd` e a unidade atomica de input do DOOM. A cada tick (1/35s),
//! o estado das teclas/mouse e convertido em um ticcmd que contem:
//! movimento frente/lado, rotacao, e botoes de acao.
//!
//! Em multiplayer, ticcmds sao transmitidos entre peers para
//! manter o jogo sincronizado (lockstep deterministic).
//!
//! ## Arquivos C originais: `d_event.h`, `d_ticcmd.h`
//!
//! ## Conceitos que o leitor vai aprender
//! - Event-driven input com ring buffer
//! - Cadeia de responders (menu > jogo > automap)
//! - Separacao entre input bruto (eventos) e input logico (ticcmd)
//! - Lockstep networking via ticcmds deterministicos

/// Tamanho maximo da fila de eventos (ring buffer).
///
/// C original: `#define MAXEVENTS 64` em `d_event.h`
pub const MAXEVENTS: usize = 64;

// ---------------------------------------------------------------------------
// Tipos de evento
// ---------------------------------------------------------------------------

/// Tipo de evento de input.
///
/// C original: `evtype_t` em `d_event.h`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    /// Tecla pressionada
    KeyDown,
    /// Tecla liberada
    KeyUp,
    /// Movimento/botao de mouse (data1=botoes, data2=dx, data3=dy)
    Mouse,
    /// Movimento/botao de joystick (data1=botoes, data2=dx, data3=dy)
    Joystick,
}

/// Evento de input do sistema.
///
/// Estrutura generica que carrega ate 3 dados inteiros alem do tipo.
/// A interpretacao dos dados depende do tipo de evento.
///
/// C original: `event_t` em `d_event.h`
#[derive(Debug, Clone, Copy)]
pub struct Event {
    /// Tipo do evento
    pub event_type: EventType,
    /// Dados do evento: tecla, ou botoes de mouse/joystick
    pub data1: i32,
    /// Dados do evento: movimento X de mouse/joystick
    pub data2: i32,
    /// Dados do evento: movimento Y de mouse/joystick
    pub data3: i32,
}

impl Event {
    /// Cria um evento de tecla pressionada.
    pub fn key_down(key: i32) -> Self {
        Event {
            event_type: EventType::KeyDown,
            data1: key,
            data2: 0,
            data3: 0,
        }
    }

    /// Cria um evento de tecla liberada.
    pub fn key_up(key: i32) -> Self {
        Event {
            event_type: EventType::KeyUp,
            data1: key,
            data2: 0,
            data3: 0,
        }
    }

    /// Cria um evento de mouse.
    pub fn mouse(buttons: i32, dx: i32, dy: i32) -> Self {
        Event {
            event_type: EventType::Mouse,
            data1: buttons,
            data2: dx,
            data3: dy,
        }
    }

    /// Cria um evento de joystick.
    pub fn joystick(buttons: i32, dx: i32, dy: i32) -> Self {
        Event {
            event_type: EventType::Joystick,
            data1: buttons,
            data2: dx,
            data3: dy,
        }
    }
}

// ---------------------------------------------------------------------------
// Fila de eventos (ring buffer)
// ---------------------------------------------------------------------------

/// Fila circular de eventos de input.
///
/// Eventos sao enfileirados por `post()` (chamado pela camada de plataforma)
/// e consumidos por `drain()` (chamado por `D_ProcessEvents`).
///
/// C original: `events[MAXEVENTS]`, `eventhead`, `eventtail` em `d_main.c`
#[derive(Debug)]
pub struct EventQueue {
    /// Ring buffer de eventos
    events: [Event; MAXEVENTS],
    /// Indice de escrita (proximo slot livre)
    head: usize,
    /// Indice de leitura (proximo evento a consumir)
    tail: usize,
}

impl EventQueue {
    /// Cria uma fila de eventos vazia.
    pub fn new() -> Self {
        let empty = Event {
            event_type: EventType::KeyDown,
            data1: 0,
            data2: 0,
            data3: 0,
        };
        EventQueue {
            events: [empty; MAXEVENTS],
            head: 0,
            tail: 0,
        }
    }

    /// Enfileira um evento.
    ///
    /// C original: `D_PostEvent()` em `d_main.c`
    pub fn post(&mut self, ev: Event) {
        self.events[self.head] = ev;
        self.head = (self.head + 1) & (MAXEVENTS - 1);
    }

    /// Retorna o proximo evento pendente, ou None se a fila estiver vazia.
    pub fn poll(&mut self) -> Option<Event> {
        if self.tail == self.head {
            return None;
        }
        let ev = self.events[self.tail];
        self.tail = (self.tail + 1) & (MAXEVENTS - 1);
        Some(ev)
    }

    /// Verifica se a fila esta vazia.
    pub fn is_empty(&self) -> bool {
        self.tail == self.head
    }
}

impl Default for EventQueue {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Ticcmd — comando de tick
// ---------------------------------------------------------------------------

/// Comando de tick — acoes do jogador em um unico tick logico.
///
/// Cada tick (1/35 de segundo), o estado das teclas e mouse e
/// convertido em um TicCmd. Em multiplayer, TicCmds sao
/// transmitidos entre os peers para sincronizacao.
///
/// C original: `ticcmd_t` em `d_ticcmd.h`
#[derive(Debug, Clone, Copy, Default)]
pub struct TicCmd {
    /// Movimento frente/tras (-forwardmove[1]..+forwardmove[1]).
    /// Multiplicado por 2048 para a velocidade real do mobj.
    pub forwardmove: i8,
    /// Movimento lateral (-sidemove[1]..+sidemove[1]).
    /// Multiplicado por 2048 para a velocidade real do mobj.
    pub sidemove: i8,
    /// Rotacao horizontal. Deslocado <<16 para o angulo delta real.
    pub angleturn: i16,
    /// Checksum de consistencia para verificacao em netgame.
    pub consistancy: i16,
    /// Caractere de chat para multiplayer.
    pub chatchar: u8,
    /// Botoes de acao (BT_ATTACK, BT_USE, BT_CHANGE, etc.)
    pub buttons: u8,
}

impl TicCmd {
    /// Cria um ticcmd vazio (sem input).
    pub fn new() -> Self {
        TicCmd::default()
    }

    /// Limpa todos os campos do ticcmd.
    pub fn clear(&mut self) {
        *self = TicCmd::default();
    }
}

// ---------------------------------------------------------------------------
// Constantes de botoes (buttoncode_t)
// ---------------------------------------------------------------------------

/// Botao de ataque (disparar arma).
///
/// C original: `BT_ATTACK = 1` em `d_event.h`
pub const BT_ATTACK: u8 = 1;

/// Botao de uso (abrir portas, ativar switches).
///
/// C original: `BT_USE = 2` em `d_event.h`
pub const BT_USE: u8 = 2;

/// Flag de troca de arma pendente.
///
/// C original: `BT_CHANGE = 4` em `d_event.h`
pub const BT_CHANGE: u8 = 4;

/// Mascara para extrair o numero da arma (3 bits).
///
/// C original: `BT_WEAPONMASK = (8+16+32)` em `d_event.h`
pub const BT_WEAPONMASK: u8 = 8 + 16 + 32;

/// Shift para posicionar o numero da arma nos bits corretos.
///
/// C original: `BT_WEAPONSHIFT = 3` em `d_event.h`
pub const BT_WEAPONSHIFT: u8 = 3;

/// Flag: evento especial (nao e botao real do jogador).
///
/// C original: `BT_SPECIAL = 128` em `d_event.h`
pub const BT_SPECIAL: u8 = 128;

/// Mascara para tipo de evento especial.
///
/// C original: `BT_SPECIALMASK = 3` em `d_event.h`
pub const BT_SPECIALMASK: u8 = 3;

/// Evento especial: pausar o jogo.
///
/// C original: `BTS_PAUSE = 1` em `d_event.h`
pub const BTS_PAUSE: u8 = 1;

/// Evento especial: salvar o jogo.
///
/// C original: `BTS_SAVEGAME = 2` em `d_event.h`
pub const BTS_SAVEGAME: u8 = 2;

/// Mascara para slot de savegame.
///
/// C original: `BTS_SAVEMASK = (4+8+16)` em `d_event.h`
pub const BTS_SAVEMASK: u8 = 4 + 8 + 16;

/// Shift para posicionar o slot de savegame.
///
/// C original: `BTS_SAVESHIFT = 2` em `d_event.h`
pub const BTS_SAVESHIFT: u8 = 2;

// ---------------------------------------------------------------------------
// Codigos de teclas do DOOM
// ---------------------------------------------------------------------------

/// Seta direita.
pub const KEY_RIGHTARROW: i32 = 0xae;
/// Seta esquerda.
pub const KEY_LEFTARROW: i32 = 0xac;
/// Seta cima.
pub const KEY_UPARROW: i32 = 0xad;
/// Seta baixo.
pub const KEY_DOWNARROW: i32 = 0xaf;
/// Escape.
pub const KEY_ESCAPE: i32 = 27;
/// Enter.
pub const KEY_ENTER: i32 = 13;
/// Tab.
pub const KEY_TAB: i32 = 9;

/// Teclas de funcao F1-F12.
pub const KEY_F1: i32 = 0x80 + 0x3b;
pub const KEY_F2: i32 = 0x80 + 0x3c;
pub const KEY_F3: i32 = 0x80 + 0x3d;
pub const KEY_F4: i32 = 0x80 + 0x3e;
pub const KEY_F5: i32 = 0x80 + 0x3f;
pub const KEY_F6: i32 = 0x80 + 0x40;
pub const KEY_F7: i32 = 0x80 + 0x41;
pub const KEY_F8: i32 = 0x80 + 0x42;
pub const KEY_F9: i32 = 0x80 + 0x43;
pub const KEY_F10: i32 = 0x80 + 0x44;
pub const KEY_F11: i32 = 0x80 + 0x57;
pub const KEY_F12: i32 = 0x80 + 0x58;

/// Backspace.
pub const KEY_BACKSPACE: i32 = 127;
/// Pause.
pub const KEY_PAUSE: i32 = 0xff;
/// Equals (=).
pub const KEY_EQUALS: i32 = 0x3d;
/// Minus (-).
pub const KEY_MINUS: i32 = 0x2d;

/// Right Shift.
pub const KEY_RSHIFT: i32 = 0x80 + 0x36;
/// Right Control (usado como "fire" padrao).
pub const KEY_RCTRL: i32 = 0x80 + 0x1d;
/// Right Alt (usado como "strafe" padrao).
pub const KEY_RALT: i32 = 0x80 + 0x38;

/// Numero de teclas rastreadas.
///
/// C original: `#define NUMKEYS 256` em `g_game.c`
pub const NUMKEYS: usize = 256;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_constructors() {
        let ev = Event::key_down(KEY_UPARROW);
        assert_eq!(ev.event_type, EventType::KeyDown);
        assert_eq!(ev.data1, KEY_UPARROW);

        let ev = Event::mouse(1, 10, -5);
        assert_eq!(ev.event_type, EventType::Mouse);
        assert_eq!(ev.data1, 1);
        assert_eq!(ev.data2, 10);
        assert_eq!(ev.data3, -5);
    }

    #[test]
    fn event_queue_post_poll() {
        let mut q = EventQueue::new();
        assert!(q.is_empty());

        q.post(Event::key_down(KEY_ENTER));
        q.post(Event::key_up(KEY_ENTER));
        assert!(!q.is_empty());

        let ev1 = q.poll().unwrap();
        assert_eq!(ev1.event_type, EventType::KeyDown);
        assert_eq!(ev1.data1, KEY_ENTER);

        let ev2 = q.poll().unwrap();
        assert_eq!(ev2.event_type, EventType::KeyUp);

        assert!(q.poll().is_none());
        assert!(q.is_empty());
    }

    #[test]
    fn event_queue_wraparound() {
        let mut q = EventQueue::new();
        // Enfileirar mais que MAXEVENTS eventos para testar wraparound
        for i in 0..(MAXEVENTS + 5) {
            q.post(Event::key_down(i as i32));
        }
        // O ring buffer sobreescreveu os primeiros 5 eventos,
        // mas tail ainda aponta para onde estava — vamos consumir o que resta
        // (o comportamento C original nao protege contra overflow)
        let mut count = 0;
        while q.poll().is_some() {
            count += 1;
        }
        // Ring buffer com head que ultrapassou tail: depende da semantica
        // O importante e que nao ha panic
        assert!(count <= MAXEVENTS);
    }

    #[test]
    fn ticcmd_default() {
        let cmd = TicCmd::new();
        assert_eq!(cmd.forwardmove, 0);
        assert_eq!(cmd.sidemove, 0);
        assert_eq!(cmd.angleturn, 0);
        assert_eq!(cmd.buttons, 0);
    }

    #[test]
    fn ticcmd_clear() {
        let mut cmd = TicCmd::new();
        cmd.forwardmove = 25;
        cmd.buttons = BT_ATTACK | BT_USE;
        cmd.clear();
        assert_eq!(cmd.forwardmove, 0);
        assert_eq!(cmd.buttons, 0);
    }

    #[test]
    fn button_constants() {
        // Verificar que as constantes nao se sobrepoe incorretamente
        assert_eq!(BT_ATTACK, 1);
        assert_eq!(BT_USE, 2);
        assert_eq!(BT_CHANGE, 4);
        assert_eq!(BT_WEAPONMASK, 56); // 8+16+32
        assert_eq!(BT_SPECIAL, 128);

        // Testar encoding de troca de arma: arma 3
        let weapon = 3u8;
        let buttons = BT_CHANGE | (weapon << BT_WEAPONSHIFT);
        assert_eq!(buttons & BT_CHANGE, BT_CHANGE);
        assert_eq!((buttons & BT_WEAPONMASK) >> BT_WEAPONSHIFT, 3);
    }

    #[test]
    fn key_constants() {
        // Teclas especiais estao acima de 0x80
        assert!(KEY_F1 > 0x80);
        assert!(KEY_RSHIFT > 0x80);
        assert!(KEY_RCTRL > 0x80);
        // Teclas ASCII sao valores diretos
        assert_eq!(KEY_ESCAPE, 27);
        assert_eq!(KEY_ENTER, 13);
        assert_eq!(KEY_TAB, 9);
    }
}

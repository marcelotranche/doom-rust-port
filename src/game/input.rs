//! # Construcao de TicCmd e Mapeamento de Input
//!
//! Converte o estado das teclas, mouse e joystick em um `TicCmd`
//! que representa as acoes do jogador em um unico tick logico.
//!
//! ## G_BuildTiccmd — o coracao do input
//!
//! A cada tick (1/35s), `build_ticcmd()` le o estado atual das teclas
//! e produz um TicCmd com:
//!
//! - `forwardmove`: frente/tras (W/S ou setas cima/baixo)
//! - `sidemove`: strafe esquerda/direita
//! - `angleturn`: rotacao horizontal (setas ou mouse)
//! - `buttons`: botoes de acao (atirar, usar, trocar arma)
//!
//! ## Velocidades de movimento
//!
//! O DOOM tem duas velocidades para cada tipo de movimento:
//! - Normal: `forwardmove[0]=0x19`, `sidemove[0]=0x18`
//! - Run (segurando Shift): `forwardmove[1]=0x32`, `sidemove[1]=0x28`
//!
//! A rotacao tem tres velocidades: normal, run, e slow turn
//! (primeiros 6 ticks ao pressionar a tecla de rotacao).
//!
//! ## Responders
//!
//! Eventos de input passam por uma cadeia de "responders":
//! 1. Menu (consome se o menu esta ativo)
//! 2. Game (G_Responder — processa teclas de jogo)
//! 3. Automap, HUD, Status bar
//!
//! Se um responder "consome" o evento, os seguintes nao o recebem.
//!
//! ## Arquivo C original: `g_game.c` (G_BuildTiccmd, G_Responder)
//!
//! ## Conceitos que o leitor vai aprender
//! - Conversao de input bruto para comandos logicos
//! - Aceleracao de rotacao (slow turn → fast turn)
//! - Sistema de key bindings configuravel
//! - Cadeia de responders para despacho de eventos

use super::events::*;

// ---------------------------------------------------------------------------
// Tabelas de velocidade de movimento
// ---------------------------------------------------------------------------

/// Velocidades de movimento frente/tras: [normal, run].
///
/// Valores sao escalados por 2048 quando aplicados ao mobj.
///
/// C original: `fixed_t forwardmove[2] = {0x19, 0x32}` em `g_game.c`
pub const FORWARDMOVE: [i8; 2] = [0x19, 0x32];

/// Velocidades de movimento lateral: [normal, run].
///
/// C original: `fixed_t sidemove[2] = {0x18, 0x28}` em `g_game.c`
pub const SIDEMOVE: [i8; 2] = [0x18, 0x28];

/// Velocidades de rotacao: [normal, fast, slow].
///
/// Valores sao deslocados <<16 para o angulo delta real.
///
/// C original: `fixed_t angleturn[3] = {640, 1280, 320}` em `g_game.c`
pub const ANGLETURN: [i16; 3] = [640, 1280, 320];

/// Numero de ticks com rotacao lenta antes de acelerar.
///
/// C original: `#define SLOWTURNTICS 6` em `g_game.c`
pub const SLOWTURNTICS: i32 = 6;

/// Limite de velocidade para detectar "turbo" (movimento rapido demais).
///
/// C original: `#define TURBOTHRESHOLD 0x32` em `g_game.c`
pub const TURBOTHRESHOLD: i8 = 0x32;

// ---------------------------------------------------------------------------
// Key bindings
// ---------------------------------------------------------------------------

/// Configuracao de key bindings do jogador.
///
/// C original: globals `key_right`, `key_left`, etc. em `g_game.c`
#[derive(Debug, Clone)]
pub struct KeyBindings {
    /// Tecla para rotacionar direita
    pub key_right: i32,
    /// Tecla para rotacionar esquerda
    pub key_left: i32,
    /// Tecla para mover frente
    pub key_up: i32,
    /// Tecla para mover tras
    pub key_down: i32,
    /// Tecla para strafe esquerda
    pub key_strafeleft: i32,
    /// Tecla para strafe direita
    pub key_straferight: i32,
    /// Tecla para atirar
    pub key_fire: i32,
    /// Tecla para usar (abrir portas, switches)
    pub key_use: i32,
    /// Tecla para ativar strafe com setas
    pub key_strafe: i32,
    /// Tecla para correr (run)
    pub key_speed: i32,

    /// Botao do mouse para atirar (indice, -1 = nenhum)
    pub mouseb_fire: i32,
    /// Botao do mouse para strafe
    pub mouseb_strafe: i32,
    /// Botao do mouse para mover frente
    pub mouseb_forward: i32,

    /// Botao do joystick para atirar
    pub joyb_fire: i32,
    /// Botao do joystick para strafe
    pub joyb_strafe: i32,
    /// Botao do joystick para usar
    pub joyb_use: i32,
    /// Botao do joystick para correr
    pub joyb_speed: i32,
}

impl KeyBindings {
    /// Cria bindings padrao do DOOM.
    ///
    /// C original: `M_SetDefaultBindings()` e defaults em `m_misc.c`
    pub fn default_bindings() -> Self {
        KeyBindings {
            key_right: KEY_RIGHTARROW,
            key_left: KEY_LEFTARROW,
            key_up: KEY_UPARROW,
            key_down: KEY_DOWNARROW,
            key_strafeleft: b',' as i32,
            key_straferight: b'.' as i32,
            key_fire: KEY_RCTRL,
            key_use: b' ' as i32,
            key_strafe: KEY_RALT,
            key_speed: KEY_RSHIFT,
            mouseb_fire: 0,
            mouseb_strafe: 1,
            mouseb_forward: 2,
            joyb_fire: 0,
            joyb_strafe: 1,
            joyb_use: 3,
            joyb_speed: 2,
        }
    }
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self::default_bindings()
    }
}

// ---------------------------------------------------------------------------
// Estado de input
// ---------------------------------------------------------------------------

/// Estado do sistema de input.
///
/// Rastreia quais teclas estao pressionadas, posicao do mouse/joystick,
/// e o estado de aceleracao de rotacao.
///
/// C original: globals `gamekeydown[]`, `mousebuttons[]`, etc. em `g_game.c`
#[derive(Debug)]
pub struct InputState {
    /// Bindings de teclas
    pub bindings: KeyBindings,
    /// Estado de cada tecla (true = pressionada)
    pub gamekeydown: [bool; NUMKEYS],
    /// Estado dos botoes do mouse
    pub mousebuttons: [bool; 4],
    /// Movimento X do mouse (consumido a cada tick)
    pub mousex: i32,
    /// Movimento Y do mouse (consumido a cada tick)
    pub mousey: i32,
    /// Estado dos botoes do joystick
    pub joybuttons: [bool; 5],
    /// Movimento X do joystick
    pub joyxmove: i32,
    /// Movimento Y do joystick
    pub joyymove: i32,
    /// Contador de ticks com tecla de rotacao pressionada
    /// (para aceleracao de rotacao)
    pub turnheld: i32,
}

impl InputState {
    /// Cria um novo estado de input com tudo zerado.
    pub fn new() -> Self {
        InputState {
            bindings: KeyBindings::default(),
            gamekeydown: [false; NUMKEYS],
            mousebuttons: [false; 4],
            mousex: 0,
            mousey: 0,
            joybuttons: [false; 5],
            joyxmove: 0,
            joyymove: 0,
            turnheld: 0,
        }
    }

    /// Processa um evento de input, atualizando o estado de teclas/mouse.
    ///
    /// Retorna true se o evento foi consumido (para a cadeia de responders).
    ///
    /// C original: parte de `G_Responder()` em `g_game.c`
    pub fn handle_event(&mut self, ev: &Event) -> bool {
        match ev.event_type {
            EventType::KeyDown => {
                let key = ev.data1 as usize;
                if key < NUMKEYS {
                    self.gamekeydown[key] = true;
                }
                true
            }
            EventType::KeyUp => {
                let key = ev.data1 as usize;
                if key < NUMKEYS {
                    self.gamekeydown[key] = false;
                }
                true
            }
            EventType::Mouse => {
                // data1 = botoes (bitmask)
                self.mousebuttons[0] = ev.data1 & 1 != 0;
                self.mousebuttons[1] = ev.data1 & 2 != 0;
                self.mousebuttons[2] = ev.data1 & 4 != 0;
                // data2, data3 = movimento delta
                self.mousex = ev.data2;
                self.mousey = ev.data3;
                true
            }
            EventType::Joystick => {
                self.joybuttons[0] = ev.data1 & 1 != 0;
                self.joybuttons[1] = ev.data1 & 2 != 0;
                self.joybuttons[2] = ev.data1 & 4 != 0;
                self.joybuttons[3] = ev.data1 & 8 != 0;
                self.joyxmove = ev.data2;
                self.joyymove = ev.data3;
                true
            }
        }
    }

    /// Constroi um ticcmd a partir do estado atual de input.
    ///
    /// Le o estado de teclas, mouse e joystick e preenche um TicCmd
    /// com os valores de movimento, rotacao e botoes correspondentes.
    ///
    /// C original: `G_BuildTiccmd()` em `g_game.c`
    pub fn build_ticcmd(&mut self, _maketic: i32, consistancy: i16) -> TicCmd {
        let mut cmd = TicCmd::new();
        cmd.consistancy = consistancy;

        let b = &self.bindings;

        // Detectar modificadores
        let strafe = self.gamekeydown[b.key_strafe as usize]
            || self.mousebuttons.get(b.mouseb_strafe as usize).copied().unwrap_or(false)
            || self.joybuttons.get(b.joyb_strafe as usize).copied().unwrap_or(false);

        let speed = if self.gamekeydown[b.key_speed as usize]
            || self.joybuttons.get(b.joyb_speed as usize).copied().unwrap_or(false)
        {
            1usize
        } else {
            0usize
        };

        let mut forward: i32 = 0;
        let mut side: i32 = 0;

        // Aceleracao de rotacao: nos primeiros SLOWTURNTICS ticks,
        // rotacao e mais lenta para permitir ajuste fino
        if self.joyxmove != 0
            || self.gamekeydown[b.key_right as usize]
            || self.gamekeydown[b.key_left as usize]
        {
            self.turnheld += 1;
        } else {
            self.turnheld = 0;
        }

        let tspeed = if self.turnheld < SLOWTURNTICS {
            2 // slow turn
        } else {
            speed as i32
        };

        // Rotacao vs strafe (se segurando a tecla strafe, setas fazem strafe)
        if strafe {
            if self.gamekeydown[b.key_right as usize] {
                side += SIDEMOVE[speed] as i32;
            }
            if self.gamekeydown[b.key_left as usize] {
                side -= SIDEMOVE[speed] as i32;
            }
            if self.joyxmove > 0 {
                side += SIDEMOVE[speed] as i32;
            }
            if self.joyxmove < 0 {
                side -= SIDEMOVE[speed] as i32;
            }
        } else {
            if self.gamekeydown[b.key_right as usize] {
                cmd.angleturn -= ANGLETURN[tspeed as usize];
            }
            if self.gamekeydown[b.key_left as usize] {
                cmd.angleturn += ANGLETURN[tspeed as usize];
            }
            if self.joyxmove > 0 {
                cmd.angleturn -= ANGLETURN[tspeed as usize];
            }
            if self.joyxmove < 0 {
                cmd.angleturn += ANGLETURN[tspeed as usize];
            }
        }

        // Movimento frente/tras
        if self.gamekeydown[b.key_up as usize] {
            forward += FORWARDMOVE[speed] as i32;
        }
        if self.gamekeydown[b.key_down as usize] {
            forward -= FORWARDMOVE[speed] as i32;
        }
        if self.joyymove < 0 {
            forward += FORWARDMOVE[speed] as i32;
        }
        if self.joyymove > 0 {
            forward -= FORWARDMOVE[speed] as i32;
        }

        // Strafe dedicado
        if self.gamekeydown[b.key_straferight as usize] {
            side += SIDEMOVE[speed] as i32;
        }
        if self.gamekeydown[b.key_strafeleft as usize] {
            side -= SIDEMOVE[speed] as i32;
        }

        // Botao de mouse para mover frente
        if self.mousebuttons.get(b.mouseb_forward as usize).copied().unwrap_or(false) {
            forward += FORWARDMOVE[speed] as i32;
        }

        // Botoes de acao
        if self.gamekeydown[b.key_fire as usize]
            || self.mousebuttons.get(b.mouseb_fire as usize).copied().unwrap_or(false)
            || self.joybuttons.get(b.joyb_fire as usize).copied().unwrap_or(false)
        {
            cmd.buttons |= BT_ATTACK;
        }

        if self.gamekeydown[b.key_use as usize]
            || self.joybuttons.get(b.joyb_use as usize).copied().unwrap_or(false)
        {
            cmd.buttons |= BT_USE;
        }

        // Troca de arma via teclas numericas 1-7
        for i in 0..7u8 {
            if self.gamekeydown[(b'1' + i) as usize] {
                cmd.buttons |= BT_CHANGE;
                cmd.buttons |= i << BT_WEAPONSHIFT;
                break;
            }
        }

        // Clampar valores de movimento
        let max_forward = FORWARDMOVE[1] as i32;
        forward = forward.clamp(-max_forward, max_forward);
        let max_side = SIDEMOVE[1] as i32;
        side = side.clamp(-max_side, max_side);

        cmd.forwardmove = forward as i8;
        cmd.sidemove = side as i8;

        // Consumir movimento do mouse (valores sao "one-shot")
        self.mousex = 0;
        self.mousey = 0;

        cmd
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

/// Numero de armas no jogo.
///
/// C original: `NUMWEAPONS` em `doomdef.h`
pub const NUMWEAPONS: usize = 9;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bindings() {
        let kb = KeyBindings::default_bindings();
        assert_eq!(kb.key_up, KEY_UPARROW);
        assert_eq!(kb.key_down, KEY_DOWNARROW);
        assert_eq!(kb.key_fire, KEY_RCTRL);
        assert_eq!(kb.key_use, b' ' as i32);
        assert_eq!(kb.key_speed, KEY_RSHIFT);
    }

    #[test]
    fn input_handle_key_events() {
        let mut input = InputState::new();

        // Pressionar seta cima
        input.handle_event(&Event::key_down(KEY_UPARROW));
        assert!(input.gamekeydown[KEY_UPARROW as usize]);

        // Liberar seta cima
        input.handle_event(&Event::key_up(KEY_UPARROW));
        assert!(!input.gamekeydown[KEY_UPARROW as usize]);
    }

    #[test]
    fn input_handle_mouse_event() {
        let mut input = InputState::new();

        // Mouse com botao 0 pressionado, movendo direita e cima
        input.handle_event(&Event::mouse(1, 10, -5));
        assert!(input.mousebuttons[0]);
        assert!(!input.mousebuttons[1]);
        assert_eq!(input.mousex, 10);
        assert_eq!(input.mousey, -5);
    }

    #[test]
    fn build_ticcmd_forward() {
        let mut input = InputState::new();
        input.gamekeydown[KEY_UPARROW as usize] = true;

        let cmd = input.build_ticcmd(0, 0);
        assert_eq!(cmd.forwardmove, FORWARDMOVE[0]);
        assert_eq!(cmd.sidemove, 0);
    }

    #[test]
    fn build_ticcmd_backward() {
        let mut input = InputState::new();
        input.gamekeydown[KEY_DOWNARROW as usize] = true;

        let cmd = input.build_ticcmd(0, 0);
        assert_eq!(cmd.forwardmove, -FORWARDMOVE[0]);
    }

    #[test]
    fn build_ticcmd_run() {
        let mut input = InputState::new();
        input.gamekeydown[KEY_UPARROW as usize] = true;
        input.gamekeydown[KEY_RSHIFT as usize] = true; // speed/run

        let cmd = input.build_ticcmd(0, 0);
        assert_eq!(cmd.forwardmove, FORWARDMOVE[1]); // velocidade de corrida
    }

    #[test]
    fn build_ticcmd_turn() {
        let mut input = InputState::new();

        // Primeiros ticks tem rotacao lenta
        input.gamekeydown[KEY_RIGHTARROW as usize] = true;
        let cmd = input.build_ticcmd(0, 0);
        assert_eq!(cmd.angleturn, -ANGLETURN[2]); // slow turn

        // Apos SLOWTURNTICS, rotacao acelera
        for _ in 0..SLOWTURNTICS {
            input.build_ticcmd(0, 0);
        }
        let cmd = input.build_ticcmd(0, 0);
        assert_eq!(cmd.angleturn, -ANGLETURN[0]); // normal speed
    }

    #[test]
    fn build_ticcmd_strafe() {
        let mut input = InputState::new();
        // Segurar ALT (strafe) + seta direita = strafe right
        input.gamekeydown[KEY_RALT as usize] = true;
        input.gamekeydown[KEY_RIGHTARROW as usize] = true;

        let cmd = input.build_ticcmd(0, 0);
        assert_eq!(cmd.angleturn, 0); // sem rotacao
        assert_eq!(cmd.sidemove, SIDEMOVE[0]); // strafe right
    }

    #[test]
    fn build_ticcmd_attack() {
        let mut input = InputState::new();
        input.gamekeydown[KEY_RCTRL as usize] = true; // fire

        let cmd = input.build_ticcmd(0, 0);
        assert!(cmd.buttons & BT_ATTACK != 0);
    }

    #[test]
    fn build_ticcmd_use() {
        let mut input = InputState::new();
        input.gamekeydown[b' ' as usize] = true; // space = use

        let cmd = input.build_ticcmd(0, 0);
        assert!(cmd.buttons & BT_USE != 0);
    }

    #[test]
    fn build_ticcmd_weapon_change() {
        let mut input = InputState::new();
        input.gamekeydown[b'3' as usize] = true; // arma 3 (shotgun)

        let cmd = input.build_ticcmd(0, 0);
        assert!(cmd.buttons & BT_CHANGE != 0);
        let weapon = (cmd.buttons & BT_WEAPONMASK) >> BT_WEAPONSHIFT;
        assert_eq!(weapon, 2); // indice 2 = tecla '3'
    }

    #[test]
    fn movement_tables() {
        // Verificar que os valores correspondem ao C original
        assert_eq!(FORWARDMOVE[0], 0x19); // 25 normal
        assert_eq!(FORWARDMOVE[1], 0x32); // 50 run
        assert_eq!(SIDEMOVE[0], 0x18);    // 24 normal
        assert_eq!(SIDEMOVE[1], 0x28);    // 40 run
        assert_eq!(ANGLETURN[0], 640);    // normal turn
        assert_eq!(ANGLETURN[1], 1280);   // fast turn
        assert_eq!(ANGLETURN[2], 320);    // slow turn
    }
}

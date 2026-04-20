//! # Interface SDL2 — Janela, Rendering e Input
//!
//! Conecta o framebuffer 320x200 do DOOM a uma janela SDL2 real,
//! convertendo pixels indexados (paleta) para RGB32 e escalando 2x.
//! Tambem converte eventos SDL2 para o formato de eventos do DOOM.
//!
//! ## Pipeline de video
//!
//! ```text
//! VideoSystem (320x200, 8-bit indexed)
//!   → paleta PLAYPAL (256 cores RGB)
//!   → buffer RGB32 (320x200x4 bytes)
//!   → SDL2 Texture (streaming)
//!   → SDL2 Window (640x400, escalado 2x)
//! ```
//!
//! ## Pipeline de input
//!
//! ```text
//! SDL2 Event Loop
//!   → SDL_PollEvent()
//!   → mapear Scancode → KEY_* do DOOM
//!   → Event::key_down / key_up / mouse
//!   → EventQueue::post()
//! ```
//!
//! ## Arquivo C original: `i_video.c`, `i_system.c`
//!
//! ## Conceitos que o leitor vai aprender
//! - Conversao de paleta indexada para RGB32
//! - SDL2 streaming textures para rendering por software
//! - Mapeamento de scancodes para keycodes de jogo
//! - Frame pacing com timing fixo

use sdl2::event::Event as SdlEvent;
use sdl2::keyboard::Scancode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::{Canvas, TextureCreator};
use sdl2::video::{Window, WindowContext};
use sdl2::EventPump;

use super::{SCREENHEIGHT, SCREENWIDTH};
use crate::game::events;

/// Fator de escala da janela (2x = 640x400).
const SCALE: u32 = 2;

/// Largura da janela SDL2.
const WINDOW_WIDTH: u32 = SCREENWIDTH as u32 * SCALE;

/// Altura da janela SDL2.
const WINDOW_HEIGHT: u32 = SCREENHEIGHT as u32 * SCALE;

/// Paleta de cores: 256 entradas RGB (768 bytes no WAD).
///
/// C original: `byte* palette` carregada de PLAYPAL em `i_video.c`
pub type Palette = [[u8; 3]; 256];

/// Paleta padrao (grayscale) usada quando PLAYPAL nao esta disponivel.
fn default_palette() -> Palette {
    let mut pal = [[0u8; 3]; 256];
    for (i, entry) in pal.iter_mut().enumerate() {
        let v = i as u8;
        *entry = [v, v, v];
    }
    pal
}

/// Carrega a paleta PLAYPAL do WAD.
///
/// PLAYPAL contem 14 paletas de 768 bytes cada (256 * RGB).
/// Usamos a primeira paleta (indice 0) como paleta base.
///
/// C original: `W_CacheLumpName("PLAYPAL")` em `i_video.c`
pub fn load_palette(wad: &crate::wad::WadSystem) -> Palette {
    match wad.read_lump_by_name("PLAYPAL") {
        Ok(data) => {
            if data.len() < 768 {
                log::warn!("PLAYPAL muito pequeno ({} bytes), usando paleta padrao", data.len());
                return default_palette();
            }
            let mut pal = [[0u8; 3]; 256];
            for (i, entry) in pal.iter_mut().enumerate() {
                let offset = i * 3;
                entry[0] = data[offset];     // R
                entry[1] = data[offset + 1]; // G
                entry[2] = data[offset + 2]; // B
            }
            pal
        }
        Err(_) => {
            log::warn!("PLAYPAL nao encontrado, usando paleta padrao");
            default_palette()
        }
    }
}

/// Janela SDL2 do DOOM.
///
/// Gerencia a janela, o canvas SDL2, a textura de streaming,
/// e o event pump para input.
///
/// C original: `I_InitGraphics()` em `i_video.c`
pub struct SdlWindow {
    /// Canvas SDL2 para rendering
    canvas: Canvas<Window>,
    /// Texture creator (precisa viver tanto quanto as textures)
    _texture_creator: TextureCreator<WindowContext>,
    /// Event pump para polling de eventos
    event_pump: EventPump,
    /// Buffer RGB32 intermediario (320*200*4 bytes)
    rgb_buffer: Vec<u8>,
    /// Paleta atual (256 cores RGB)
    palette: Palette,
    /// Indice da paleta ativa (0-13 para efeitos de dano/bonus)
    palette_index: usize,
}

impl SdlWindow {
    /// Cria e abre a janela SDL2.
    ///
    /// C original: `I_InitGraphics()` em `i_video.c`
    pub fn new(title: &str) -> Result<Self, String> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;

        let window = video_subsystem
            .window(title, WINDOW_WIDTH, WINDOW_HEIGHT)
            .position_centered()
            .build()
            .map_err(|e| e.to_string())?;

        let canvas = window
            .into_canvas()
            .present_vsync()
            .build()
            .map_err(|e| e.to_string())?;

        let event_pump = sdl_context.event_pump()?;

        let texture_creator = canvas.texture_creator();

        Ok(SdlWindow {
            canvas,
            _texture_creator: texture_creator,
            event_pump,
            rgb_buffer: vec![0u8; SCREENWIDTH * SCREENHEIGHT * 4],
            palette: default_palette(),
            palette_index: 0,
        })
    }

    /// Define a paleta de cores.
    pub fn set_palette(&mut self, palette: Palette) {
        self.palette = palette;
    }

    /// Define o indice da paleta ativa (para efeitos de dano/bonus).
    pub fn set_palette_index(&mut self, index: usize) {
        self.palette_index = index;
    }

    /// Converte o framebuffer indexed (320x200) para RGB32 e apresenta na janela.
    ///
    /// Pipeline:
    /// 1. Para cada pixel do framebuffer, lookup na paleta → RGB
    /// 2. Escrever no buffer RGB32
    /// 3. Upload para SDL texture
    /// 4. Render texture escalada na janela
    ///
    /// C original: `I_FinishUpdate()` em `i_video.c`
    pub fn finish_update(&mut self, framebuffer: &[u8]) -> Result<(), String> {
        // Converter indexed → RGB32
        for (i, &color_index) in framebuffer.iter().enumerate() {
            let rgb = &self.palette[color_index as usize];
            let offset = i * 4;
            self.rgb_buffer[offset] = rgb[2];     // B (SDL ARGB8888)
            self.rgb_buffer[offset + 1] = rgb[1]; // G
            self.rgb_buffer[offset + 2] = rgb[0]; // R
            self.rgb_buffer[offset + 3] = 255;    // A
        }

        // Criar texture temporaria e fazer blit
        // (recriamos a cada frame porque texture_creator lifetime e complicado)
        let texture_creator = self.canvas.texture_creator();
        let mut texture = texture_creator
            .create_texture_streaming(
                PixelFormatEnum::ARGB8888,
                SCREENWIDTH as u32,
                SCREENHEIGHT as u32,
            )
            .map_err(|e| e.to_string())?;

        texture
            .update(None, &self.rgb_buffer, SCREENWIDTH * 4)
            .map_err(|e| e.to_string())?;

        self.canvas.clear();
        self.canvas
            .copy(&texture, None, None)
            .map_err(|e| e.to_string())?;
        self.canvas.present();

        Ok(())
    }

    /// Processa eventos SDL2 e converte para eventos do DOOM.
    ///
    /// Retorna `false` se o usuario pediu para fechar a janela (quit).
    ///
    /// C original: `I_StartTic()` em `i_video.c`
    pub fn pump_events(&mut self, event_queue: &mut events::EventQueue) -> bool {
        for sdl_event in self.event_pump.poll_iter() {
            match sdl_event {
                SdlEvent::Quit { .. } => {
                    return false;
                }

                SdlEvent::KeyDown {
                    scancode: Some(sc),
                    repeat: false,
                    ..
                } => {
                    if let Some(key) = scancode_to_doom(sc) {
                        event_queue.post(events::Event::key_down(key));
                    }
                }

                SdlEvent::KeyUp {
                    scancode: Some(sc),
                    ..
                } => {
                    if let Some(key) = scancode_to_doom(sc) {
                        event_queue.post(events::Event::key_up(key));
                    }
                }

                SdlEvent::MouseMotion {
                    xrel, yrel, ..
                } if xrel != 0 || yrel != 0 => {
                    event_queue.post(events::Event::mouse(0, xrel, -yrel));
                }

                SdlEvent::MouseButtonDown { mouse_btn, .. } => {
                    let btn = mouse_button_mask(mouse_btn);
                    if btn != 0 {
                        event_queue.post(events::Event::mouse(btn, 0, 0));
                    }
                }

                SdlEvent::MouseButtonUp { mouse_btn, .. } => {
                    // Enviar com botao zerado para indicar release
                    let _ = mouse_btn;
                    event_queue.post(events::Event::mouse(0, 0, 0));
                }

                _ => {}
            }
        }
        true
    }

    /// Retorna referencia ao canvas (para debug/info).
    pub fn window_title(&self) -> &str {
        self.canvas.window().title()
    }
}

// Nao podemos derivar Debug por causa dos tipos SDL2
impl std::fmt::Debug for SdlWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SdlWindow")
            .field("palette_index", &self.palette_index)
            .finish()
    }
}

/// Converte um SDL2 Scancode para um keycode do DOOM.
///
/// C original: `xlatekey()` em `i_video.c`
fn scancode_to_doom(sc: Scancode) -> Option<i32> {
    match sc {
        // Setas
        Scancode::Up => Some(events::KEY_UPARROW),
        Scancode::Down => Some(events::KEY_DOWNARROW),
        Scancode::Left => Some(events::KEY_LEFTARROW),
        Scancode::Right => Some(events::KEY_RIGHTARROW),

        // Teclas de acao
        Scancode::Escape => Some(events::KEY_ESCAPE),
        Scancode::Return => Some(events::KEY_ENTER),
        Scancode::Tab => Some(events::KEY_TAB),
        Scancode::Backspace => Some(events::KEY_BACKSPACE),
        Scancode::Pause => Some(events::KEY_PAUSE),
        Scancode::Minus => Some(events::KEY_MINUS),
        Scancode::Equals => Some(events::KEY_EQUALS),
        Scancode::Space => Some(b' ' as i32),

        // Modificadores
        Scancode::RShift | Scancode::LShift => Some(events::KEY_RSHIFT),
        Scancode::RCtrl | Scancode::LCtrl => Some(events::KEY_RCTRL),
        Scancode::RAlt | Scancode::LAlt => Some(events::KEY_RALT),

        // Teclas de funcao
        Scancode::F1 => Some(events::KEY_F1),
        Scancode::F2 => Some(events::KEY_F2),
        Scancode::F3 => Some(events::KEY_F3),
        Scancode::F4 => Some(events::KEY_F4),
        Scancode::F5 => Some(events::KEY_F5),
        Scancode::F6 => Some(events::KEY_F6),
        Scancode::F7 => Some(events::KEY_F7),
        Scancode::F8 => Some(events::KEY_F8),
        Scancode::F9 => Some(events::KEY_F9),
        Scancode::F10 => Some(events::KEY_F10),
        Scancode::F11 => Some(events::KEY_F11),
        Scancode::F12 => Some(events::KEY_F12),

        // Letras (DOOM usa ASCII lowercase)
        Scancode::A => Some(b'a' as i32),
        Scancode::B => Some(b'b' as i32),
        Scancode::C => Some(b'c' as i32),
        Scancode::D => Some(b'd' as i32),
        Scancode::E => Some(b'e' as i32),
        Scancode::F => Some(b'f' as i32),
        Scancode::G => Some(b'g' as i32),
        Scancode::H => Some(b'h' as i32),
        Scancode::I => Some(b'i' as i32),
        Scancode::J => Some(b'j' as i32),
        Scancode::K => Some(b'k' as i32),
        Scancode::L => Some(b'l' as i32),
        Scancode::M => Some(b'm' as i32),
        Scancode::N => Some(b'n' as i32),
        Scancode::O => Some(b'o' as i32),
        Scancode::P => Some(b'p' as i32),
        Scancode::Q => Some(b'q' as i32),
        Scancode::R => Some(b'r' as i32),
        Scancode::S => Some(b's' as i32),
        Scancode::T => Some(b't' as i32),
        Scancode::U => Some(b'u' as i32),
        Scancode::V => Some(b'v' as i32),
        Scancode::W => Some(b'w' as i32),
        Scancode::X => Some(b'x' as i32),
        Scancode::Y => Some(b'y' as i32),
        Scancode::Z => Some(b'z' as i32),

        // Numeros
        Scancode::Num0 => Some(b'0' as i32),
        Scancode::Num1 => Some(b'1' as i32),
        Scancode::Num2 => Some(b'2' as i32),
        Scancode::Num3 => Some(b'3' as i32),
        Scancode::Num4 => Some(b'4' as i32),
        Scancode::Num5 => Some(b'5' as i32),
        Scancode::Num6 => Some(b'6' as i32),
        Scancode::Num7 => Some(b'7' as i32),
        Scancode::Num8 => Some(b'8' as i32),
        Scancode::Num9 => Some(b'9' as i32),

        // Pontuacao usada pelo DOOM
        Scancode::Comma => Some(b',' as i32),
        Scancode::Period => Some(b'.' as i32),

        _ => None,
    }
}

/// Converte botao de mouse SDL2 para bitmask do DOOM.
///
/// C original: botoes em `I_StartTic()` em `i_video.c`
fn mouse_button_mask(btn: sdl2::mouse::MouseButton) -> i32 {
    match btn {
        sdl2::mouse::MouseButton::Left => 1,
        sdl2::mouse::MouseButton::Right => 2,
        sdl2::mouse::MouseButton::Middle => 4,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_palette_grayscale() {
        let pal = default_palette();
        assert_eq!(pal[0], [0, 0, 0]);
        assert_eq!(pal[128], [128, 128, 128]);
        assert_eq!(pal[255], [255, 255, 255]);
    }

    #[test]
    fn scancode_arrows() {
        assert_eq!(scancode_to_doom(Scancode::Up), Some(events::KEY_UPARROW));
        assert_eq!(scancode_to_doom(Scancode::Down), Some(events::KEY_DOWNARROW));
        assert_eq!(scancode_to_doom(Scancode::Left), Some(events::KEY_LEFTARROW));
        assert_eq!(scancode_to_doom(Scancode::Right), Some(events::KEY_RIGHTARROW));
    }

    #[test]
    fn scancode_modifiers() {
        assert_eq!(scancode_to_doom(Scancode::LShift), Some(events::KEY_RSHIFT));
        assert_eq!(scancode_to_doom(Scancode::RShift), Some(events::KEY_RSHIFT));
        assert_eq!(scancode_to_doom(Scancode::LCtrl), Some(events::KEY_RCTRL));
        assert_eq!(scancode_to_doom(Scancode::LAlt), Some(events::KEY_RALT));
    }

    #[test]
    fn scancode_letters() {
        assert_eq!(scancode_to_doom(Scancode::A), Some(b'a' as i32));
        assert_eq!(scancode_to_doom(Scancode::Z), Some(b'z' as i32));
    }

    #[test]
    fn scancode_numbers() {
        assert_eq!(scancode_to_doom(Scancode::Num1), Some(b'1' as i32));
        assert_eq!(scancode_to_doom(Scancode::Num0), Some(b'0' as i32));
    }

    #[test]
    fn scancode_function_keys() {
        assert_eq!(scancode_to_doom(Scancode::F1), Some(events::KEY_F1));
        assert_eq!(scancode_to_doom(Scancode::F12), Some(events::KEY_F12));
    }

    #[test]
    fn scancode_special() {
        assert_eq!(scancode_to_doom(Scancode::Escape), Some(events::KEY_ESCAPE));
        assert_eq!(scancode_to_doom(Scancode::Return), Some(events::KEY_ENTER));
        assert_eq!(scancode_to_doom(Scancode::Space), Some(b' ' as i32));
    }

    #[test]
    fn scancode_unknown() {
        // Teclas nao mapeadas retornam None
        assert_eq!(scancode_to_doom(Scancode::CapsLock), None);
    }

    #[test]
    fn mouse_buttons() {
        assert_eq!(mouse_button_mask(sdl2::mouse::MouseButton::Left), 1);
        assert_eq!(mouse_button_mask(sdl2::mouse::MouseButton::Right), 2);
        assert_eq!(mouse_button_mask(sdl2::mouse::MouseButton::Middle), 4);
    }
}

//! # Sistema de Musica — Reproducao e Controle de MUS/MIDI
//!
//! Gerencia a reproducao de musica no DOOM:
//! - Troca de faixas por nivel (`S_ChangeMusic`)
//! - Pausa e retomada (`S_PauseSound`, `S_ResumeSound`)
//! - Controle de volume global de musica
//! - Registro e liberacao de faixas (interface com plataforma)
//!
//! ## Formato MUS
//!
//! O DOOM usa o formato MUS (Music), uma variante compacta de MIDI.
//! Faixas sao armazenadas como lumps no WAD com prefixo "D_"
//! (ex: "D_E1M1" para E1M1 — At Doom's Gate).
//!
//! ```text
//! MUS Header (fixo):
//! - bytes 0-3:  "MUS\x1a" (magic)
//! - bytes 4-5:  tamanho dos dados de musica
//! - bytes 6-7:  offset dos dados de musica
//! - bytes 8-9:  numero de canais primarios
//! - bytes 10-11: numero de canais secundarios
//! - bytes 12-13: numero de instrumentos
//! - bytes 14-15: reservado
//! - bytes 16+:  lista de instrumentos (2 bytes cada)
//! ```
//!
//! ## Camada de plataforma
//!
//! No DOOM original para Linux, as funcoes I_* de musica eram stubs
//! vazios — a musica nunca foi implementada no port Linux. Em ports
//! modernos (Chocolate Doom), usa-se SDL_mixer com conversao MUS→MIDI.
//!
//! ## Arquivo C original: `s_sound.c` (S_*), `i_sound.c` (I_*)
//!
//! ## Conceitos que o leitor vai aprender
//! - Gerenciamento de estado de reproducao (play/pause/stop)
//! - Interface game-layer vs platform-layer para audio
//! - Formato MUS do DOOM (variante compacta de MIDI)

use super::types::*;

/// Magic bytes do formato MUS: "MUS\x1a"
///
/// C original: verificacao em `I_RegisterSong()` (ports modernos)
pub const MUS_MAGIC: [u8; 4] = [b'M', b'U', b'S', 0x1a];

/// Header do formato MUS.
///
/// Descreve a estrutura do lump de musica no WAD.
///
/// C original: nao existe struct explicita no linuxdoom
/// (ports modernos definem `mus_header_t`)
#[derive(Debug, Clone)]
pub struct MusHeader {
    /// Tamanho dos dados de musica (em bytes)
    pub score_len: u16,
    /// Offset dos dados de musica a partir do inicio do lump
    pub score_start: u16,
    /// Numero de canais primarios usados
    pub primary_channels: u16,
    /// Numero de canais secundarios
    pub secondary_channels: u16,
    /// Numero de instrumentos
    pub instrument_count: u16,
}

impl MusHeader {
    /// Tenta parsear um header MUS a partir de bytes raw.
    ///
    /// Retorna `None` se os dados sao curtos demais ou o magic esta errado.
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 16 {
            return None;
        }

        // Verificar magic
        if data[0..4] != MUS_MAGIC {
            return None;
        }

        let score_len = u16::from_le_bytes([data[4], data[5]]);
        let score_start = u16::from_le_bytes([data[6], data[7]]);
        let primary_channels = u16::from_le_bytes([data[8], data[9]]);
        let secondary_channels = u16::from_le_bytes([data[10], data[11]]);
        let instrument_count = u16::from_le_bytes([data[12], data[13]]);

        Some(MusHeader {
            score_len,
            score_start,
            primary_channels,
            secondary_channels,
            instrument_count,
        })
    }
}

// ---------------------------------------------------------------------------
// Sistema de musica
// ---------------------------------------------------------------------------

/// Sistema de musica do DOOM — controla reproducao de faixas MUS/MIDI.
///
/// C original: globals `mus_playing`, `mus_paused`, `snd_MusicVolume`
/// e funcoes `S_ChangeMusic()`, `S_StartMusic()`, etc. em `s_sound.c`
#[derive(Debug)]
pub struct MusicSystem {
    /// Volume global de musica (0-127)
    pub music_volume: i32,
    /// Indice da musica tocando atualmente (None = nenhuma)
    pub current_music: Option<MusicId>,
    /// Se a musica esta pausada
    pub paused: bool,
    /// Se a musica atual esta em loop
    pub looping: bool,
    /// Tabela de metadados de musica
    pub music_table: Vec<MusicInfo>,
}

impl MusicSystem {
    /// Cria um novo sistema de musica.
    ///
    /// C original: inicializacao em `S_Init()` / `S_SetMusicVolume()`
    pub fn new(volume: i32) -> Self {
        MusicSystem {
            music_volume: volume.clamp(0, S_MAX_VOLUME),
            current_music: None,
            paused: false,
            looping: false,
            music_table: music_table(),
        }
    }

    /// Inicia uma musica (sem loop).
    ///
    /// Wrapper simples para `change_music()` com `looping = false`.
    ///
    /// C original: `S_StartMusic(int m_id)` em `s_sound.c`
    pub fn start_music(&mut self, music_id: MusicId) {
        self.change_music(music_id, false);
    }

    /// Troca a musica atual.
    ///
    /// Para a musica anterior (se houver), registra a nova faixa,
    /// e inicia a reproducao. Se `looping` e `true`, a musica
    /// reinicia automaticamente ao terminar.
    ///
    /// C original: `S_ChangeMusic(int musicnum, int looping)` em `s_sound.c`
    pub fn change_music(&mut self, music_id: MusicId, looping: bool) {
        if music_id == MUS_NONE || music_id >= self.music_table.len() {
            return;
        }

        // Se ja esta tocando a mesma musica, nao reiniciar
        if self.current_music == Some(music_id) {
            return;
        }

        // Parar musica anterior
        self.stop_music();

        // Iniciar nova musica
        self.current_music = Some(music_id);
        self.looping = looping;
        self.paused = false;

        // TODO: chamar I_RegisterSong() e I_PlaySong() da camada de plataforma
    }

    /// Para a musica atual.
    ///
    /// Libera os recursos da faixa e limpa o estado de reproducao.
    ///
    /// C original: `S_StopMusic()` em `s_sound.c`
    pub fn stop_music(&mut self) {
        if self.current_music.is_none() {
            return;
        }

        // Se estava pausada, retomar antes de parar (como no C original)
        if self.paused {
            // TODO: I_ResumeSong()
            self.paused = false;
        }

        // TODO: I_StopSong(), I_UnRegisterSong()
        self.current_music = None;
        self.looping = false;
    }

    /// Pausa a musica atual.
    ///
    /// C original: `S_PauseSound()` em `s_sound.c`
    pub fn pause(&mut self) {
        if self.current_music.is_some() && !self.paused {
            // TODO: I_PauseSong()
            self.paused = true;
        }
    }

    /// Retoma a musica pausada.
    ///
    /// C original: `S_ResumeSound()` em `s_sound.c`
    pub fn resume(&mut self) {
        if self.current_music.is_some() && self.paused {
            // TODO: I_ResumeSong()
            self.paused = false;
        }
    }

    /// Altera o volume global de musica.
    ///
    /// C original: `S_SetMusicVolume(int volume)` em `s_sound.c`
    pub fn set_volume(&mut self, volume: i32) {
        self.music_volume = volume.clamp(0, S_MAX_VOLUME);
        // TODO: I_SetMusicVolume()
    }

    /// Verifica se ha musica tocando (nao pausada).
    pub fn is_playing(&self) -> bool {
        self.current_music.is_some() && !self.paused
    }
}

impl Default for MusicSystem {
    fn default() -> Self {
        Self::new(S_MAX_VOLUME)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn music_system_init() {
        let ms = MusicSystem::new(100);
        assert_eq!(ms.music_volume, 100);
        assert!(ms.current_music.is_none());
        assert!(!ms.paused);
        assert!(!ms.is_playing());
    }

    #[test]
    fn change_music() {
        let mut ms = MusicSystem::new(S_MAX_VOLUME);
        ms.change_music(MUS_E1M1, true);
        assert_eq!(ms.current_music, Some(MUS_E1M1));
        assert!(ms.looping);
        assert!(ms.is_playing());

        // Trocar para outra musica
        ms.change_music(MUS_E1M2, false);
        assert_eq!(ms.current_music, Some(MUS_E1M2));
        assert!(!ms.looping);
    }

    #[test]
    fn change_music_same_no_restart() {
        let mut ms = MusicSystem::new(S_MAX_VOLUME);
        ms.change_music(MUS_E1M1, true);
        assert_eq!(ms.current_music, Some(MUS_E1M1));
        assert!(ms.looping);

        // Mesma musica — nao deve reiniciar (looping permanece true)
        ms.change_music(MUS_E1M1, false);
        assert_eq!(ms.current_music, Some(MUS_E1M1));
        assert!(ms.looping); // manteve o estado original
    }

    #[test]
    fn pause_and_resume() {
        let mut ms = MusicSystem::new(S_MAX_VOLUME);
        ms.change_music(MUS_E1M1, true);
        assert!(ms.is_playing());

        ms.pause();
        assert!(ms.paused);
        assert!(!ms.is_playing());

        ms.resume();
        assert!(!ms.paused);
        assert!(ms.is_playing());
    }

    #[test]
    fn pause_without_music() {
        let mut ms = MusicSystem::new(S_MAX_VOLUME);
        ms.pause(); // nada tocando — deve ser no-op
        assert!(!ms.paused);
    }

    #[test]
    fn stop_music() {
        let mut ms = MusicSystem::new(S_MAX_VOLUME);
        ms.change_music(MUS_E1M1, true);
        assert!(ms.is_playing());

        ms.stop_music();
        assert!(ms.current_music.is_none());
        assert!(!ms.is_playing());
        assert!(!ms.looping);
    }

    #[test]
    fn stop_while_paused() {
        let mut ms = MusicSystem::new(S_MAX_VOLUME);
        ms.change_music(MUS_E1M1, true);
        ms.pause();
        assert!(ms.paused);

        ms.stop_music();
        assert!(ms.current_music.is_none());
        assert!(!ms.paused); // pausa limpa ao parar
    }

    #[test]
    fn set_volume_clamp() {
        let mut ms = MusicSystem::new(100);
        ms.set_volume(200);
        assert_eq!(ms.music_volume, S_MAX_VOLUME);
        ms.set_volume(-10);
        assert_eq!(ms.music_volume, 0);
    }

    #[test]
    fn start_music_shortcut() {
        let mut ms = MusicSystem::new(S_MAX_VOLUME);
        ms.start_music(MUS_E1M1);
        assert_eq!(ms.current_music, Some(MUS_E1M1));
        assert!(!ms.looping); // start_music nao faz loop
    }

    #[test]
    fn invalid_music_id() {
        let mut ms = MusicSystem::new(S_MAX_VOLUME);
        ms.change_music(MUS_NONE, true); // SFX_NONE — ignorado
        assert!(ms.current_music.is_none());

        ms.change_music(999, true); // fora do range — ignorado
        assert!(ms.current_music.is_none());
    }

    #[test]
    fn mus_header_parse() {
        let mut data = vec![0u8; 16];
        data[0..4].copy_from_slice(&MUS_MAGIC);
        data[4] = 100; // score_len = 100
        data[6] = 16;  // score_start = 16
        data[8] = 9;   // primary_channels = 9
        data[12] = 15; // instrument_count = 15

        let header = MusHeader::parse(&data).unwrap();
        assert_eq!(header.score_len, 100);
        assert_eq!(header.score_start, 16);
        assert_eq!(header.primary_channels, 9);
        assert_eq!(header.instrument_count, 15);
    }

    #[test]
    fn mus_header_invalid() {
        // Too short
        assert!(MusHeader::parse(&[0; 4]).is_none());

        // Wrong magic
        let data = vec![0u8; 16];
        assert!(MusHeader::parse(&data).is_none());
    }
}

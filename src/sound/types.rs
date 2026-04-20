//! # Tipos e Metadados de Audio
//!
//! Define os tipos fundamentais do sistema de audio do DOOM:
//! informacoes de efeitos sonoros (SFX), informacoes de musica,
//! e os enums que identificam cada som e musica do jogo.
//!
//! ## Formato de som no WAD
//!
//! Efeitos sonoros sao armazenados como lumps com prefixo "DS"
//! (ex: "DSPISTOL"). O formato e PCM 8-bit unsigned, mono,
//! com header de 8 bytes:
//! - bytes 0-1: format (3 = PCM)
//! - bytes 2-3: sample rate (tipicamente 11025 Hz)
//! - bytes 4-7: numero de samples
//! - bytes 8+: dados PCM
//!
//! Musicas usam o formato MUS (variante MIDI do DOOM),
//! armazenadas como lumps com prefixo "D_" (ex: "D_E1M1").
//!
//! ## Arquivo C original: `sounds.h`, `sounds.c`
//!
//! ## Conceitos que o leitor vai aprender
//! - Metadados de som (singularidade, prioridade, links)
//! - Sistema de cache com contagem de utilidade
//! - Formato de audio do WAD (PCM 8-bit)

/// Volume maximo de efeitos sonoros (0-127).
///
/// C original: `#define S_MAX_VOLUME 127` em `s_sound.h`
pub const S_MAX_VOLUME: i32 = 127;

/// Pitch padrao (centro da faixa).
///
/// C original: `#define NORM_PITCH 128` em `s_sound.c`
pub const NORM_PITCH: i32 = 128;

/// Separacao estereo padrao (centro).
///
/// C original: `#define NORM_SEP 128` em `s_sound.c`
pub const NORM_SEP: i32 = 128;

/// Prioridade padrao.
///
/// C original: `#define NORM_PRIORITY 64` em `s_sound.c`
pub const NORM_PRIORITY: i32 = 64;

/// Distancia de clipping — sons alem desta distancia sao inaudiveis.
///
/// C original: `#define S_CLIPPING_DIST (1200*0x10000)` em `s_sound.c`
pub const S_CLIPPING_DIST: i32 = 1200 * 0x10000;

/// Distancia proxima — volume maximo dentro desta distancia.
///
/// C original: `#define S_CLOSE_DIST (160*0x10000)` em `s_sound.c`
pub const S_CLOSE_DIST: i32 = 160 * 0x10000;

/// Fator de atenuacao para calculo de volume por distancia.
///
/// C original: `#define S_ATTENUATOR ((S_CLIPPING_DIST-S_CLOSE_DIST)>>FRACBITS)`
pub const S_ATTENUATOR: i32 = (S_CLIPPING_DIST - S_CLOSE_DIST) >> 16;

/// Numero de canais de mixing de hardware.
///
/// C original: `#define NUM_CHANNELS 8` em `i_sound.c`
pub const NUM_CHANNELS: usize = 8;

// ---------------------------------------------------------------------------
// SFX info
// ---------------------------------------------------------------------------

/// Informacoes de um efeito sonoro.
///
/// Descreve as propriedades de um SFX: nome, prioridade, se e
/// singular (apenas uma instancia por vez), e se e um link para
/// outro som (para variantes com pitch/volume diferente).
///
/// C original: `sfxinfo_t` em `sounds.h`
#[derive(Debug, Clone)]
pub struct SfxInfo {
    /// Nome do som (ate 6 caracteres, ex: "pistol")
    pub name: &'static str,
    /// Se true, apenas uma instancia deste som pode tocar por vez
    pub singularity: bool,
    /// Prioridade (0-127). Sons de maior prioridade preemptam os menores
    pub priority: i32,
    /// Indice de som linkado (para aliases). None = som independente.
    pub link: Option<usize>,
    /// Pitch override se linkado (0-255)
    pub pitch: i32,
    /// Volume override se linkado
    pub volume: i32,
    /// Indice do lump no WAD (-1 = nao carregado)
    pub lumpnum: i32,
    /// Contagem de utilidade para cache (-1 = nao cached, 0 = descartar)
    pub usefulness: i32,
}

impl SfxInfo {
    /// Cria uma entrada de SFX.
    pub const fn new(name: &'static str, singularity: bool, priority: i32) -> Self {
        SfxInfo {
            name,
            singularity,
            priority,
            link: None,
            pitch: -1,
            volume: -1,
            lumpnum: -1,
            usefulness: -1,
        }
    }

    /// Cria uma entrada de SFX linkada a outro som.
    pub const fn linked(name: &'static str, priority: i32, link: usize, pitch: i32, volume: i32) -> Self {
        SfxInfo {
            name,
            singularity: false,
            priority,
            link: Some(link),
            pitch,
            volume,
            lumpnum: -1,
            usefulness: -1,
        }
    }
}

// ---------------------------------------------------------------------------
// Identificadores de SFX
// ---------------------------------------------------------------------------

/// Identificador de efeito sonoro.
///
/// C original: `sfxenum_t` em `sounds.h`
pub type SfxId = usize;

/// Nenhum som.
pub const SFX_NONE: SfxId = 0;
/// Tiro de pistola.
pub const SFX_PISTOL: SfxId = 1;
/// Tiro de shotgun.
pub const SFX_SHOTGN: SfxId = 2;
/// Shotgun recarregando (pump).
pub const SFX_SGCOCK: SfxId = 3;
/// Tiro de chaingun.
pub const SFX_DSHTGN: SfxId = 4;
/// Tiro de plasma.
pub const SFX_PLASMA: SfxId = 5;
/// Tiro de BFG.
pub const SFX_BFG: SfxId = 6;
/// Motosserra (idle).
pub const SFX_SAWUP: SfxId = 7;
/// Motosserra (ataque).
pub const SFX_SAWIDL: SfxId = 8;
/// Motosserra (hit).
pub const SFX_SAWFUL: SfxId = 9;
/// Motosserra (acertou).
pub const SFX_SAWHIT: SfxId = 10;
/// Rocket (lancamento).
pub const SFX_RLAUNC: SfxId = 11;
/// Explosao.
pub const SFX_RXPLOD: SfxId = 12;
/// Imp ataque.
pub const SFX_FIRSHT: SfxId = 13;
/// Explosao de fireball.
pub const SFX_FIRXPL: SfxId = 14;
/// Pegar item.
pub const SFX_ITEMUP: SfxId = 15;
/// Pegar arma.
pub const SFX_WPNUP: SfxId = 16;
/// Dor do jogador.
pub const SFX_OOF: SfxId = 17;
/// Teleporte.
pub const SFX_TELEPT: SfxId = 18;
/// Porta abrindo.
pub const SFX_DOROPN: SfxId = 19;
/// Porta fechando.
pub const SFX_DORCLS: SfxId = 20;
/// Switch ativado.
pub const SFX_SWTCHN: SfxId = 21;
/// Pegar chave.
pub const SFX_SWTCHX: SfxId = 22;

/// Numero total de SFX no DOOM.
///
/// C original: `NUMSFX` em `sounds.h`
pub const NUMSFX: usize = 109;

// ---------------------------------------------------------------------------
// Music info
// ---------------------------------------------------------------------------

/// Informacoes de uma musica.
///
/// C original: `musicinfo_t` em `sounds.h`
#[derive(Debug, Clone)]
pub struct MusicInfo {
    /// Nome da musica (ate 6 caracteres, ex: "e1m1")
    pub name: &'static str,
    /// Indice do lump no WAD (-1 = nao carregado)
    pub lumpnum: i32,
    /// Handle de reproducao (-1 = nao tocando)
    pub handle: i32,
}

impl MusicInfo {
    /// Cria uma entrada de musica.
    pub const fn new(name: &'static str) -> Self {
        MusicInfo {
            name,
            lumpnum: -1,
            handle: -1,
        }
    }
}

// ---------------------------------------------------------------------------
// Identificadores de musica
// ---------------------------------------------------------------------------

/// Identificador de musica.
///
/// C original: `musicenum_t` em `sounds.h`
pub type MusicId = usize;

/// Nenhuma musica.
pub const MUS_NONE: MusicId = 0;
/// E1M1 — At Doom's Gate.
pub const MUS_E1M1: MusicId = 1;
/// E1M2 — The Imp's Song.
pub const MUS_E1M2: MusicId = 2;
/// E1M3 — Dark Halls.
pub const MUS_E1M3: MusicId = 3;
/// E1M4 — Kitchen Ace.
pub const MUS_E1M4: MusicId = 4;
/// E1M5 — Suspense.
pub const MUS_E1M5: MusicId = 5;
/// E1M6 — On the Hunt.
pub const MUS_E1M6: MusicId = 6;
/// E1M7 — Demons on the Prey.
pub const MUS_E1M7: MusicId = 7;
/// E1M8 — Sign of Evil.
pub const MUS_E1M8: MusicId = 8;
/// E1M9 — Hiding the Secrets.
pub const MUS_E1M9: MusicId = 9;
/// Intermission.
pub const MUS_INTER: MusicId = 10;
/// Title screen.
pub const MUS_INTRO: MusicId = 11;

/// Numero total de musicas no DOOM.
///
/// C original: `NUMMUSIC` em `sounds.h`
pub const NUMMUSIC: usize = 68;

/// Tabela de metadados de SFX.
///
/// Selecao representativa dos efeitos sonoros do DOOM.
/// A tabela completa tem NUMSFX entradas.
///
/// C original: `S_sfx[]` em `sounds.c`
pub fn sfx_table() -> Vec<SfxInfo> {
    vec![
        SfxInfo::new("none",   false, 0),     // SFX_NONE
        SfxInfo::new("pistol", false, 64),     // SFX_PISTOL
        SfxInfo::new("shotgn", false, 64),     // SFX_SHOTGN
        SfxInfo::new("sgcock", false, 64),     // SFX_SGCOCK
        SfxInfo::new("dshtgn", false, 64),     // SFX_DSHTGN
        SfxInfo::new("plasma", false, 64),     // SFX_PLASMA
        SfxInfo::new("bfg",    false, 64),     // SFX_BFG
        SfxInfo::new("sawup",  true,  64),     // SFX_SAWUP (singular)
        SfxInfo::new("sawidl", true,  118),    // SFX_SAWIDL
        SfxInfo::new("sawful", true,  64),     // SFX_SAWFUL
        SfxInfo::new("sawhit", true,  64),     // SFX_SAWHIT
        SfxInfo::new("rlaunc", false, 64),     // SFX_RLAUNC
        SfxInfo::new("rxplod", false, 70),     // SFX_RXPLOD
        SfxInfo::new("firsht", false, 70),     // SFX_FIRSHT
        SfxInfo::new("firxpl", false, 70),     // SFX_FIRXPL
        SfxInfo::new("itemup", true,  78),     // SFX_ITEMUP
        SfxInfo::new("wpnup",  true,  78),     // SFX_WPNUP
        SfxInfo::new("oof",    false, 96),     // SFX_OOF
        SfxInfo::new("telept", false, 32),     // SFX_TELEPT
        SfxInfo::new("doropn", false, 72),     // SFX_DOROPN
        SfxInfo::new("dorcls", false, 72),     // SFX_DORCLS
        SfxInfo::new("swtchn", false, 78),     // SFX_SWTCHN
        SfxInfo::new("swtchx", false, 78),     // SFX_SWTCHX
    ]
}

/// Tabela de metadados de musica.
///
/// C original: `S_music[]` em `sounds.c`
pub fn music_table() -> Vec<MusicInfo> {
    vec![
        MusicInfo::new(""),      // MUS_NONE
        MusicInfo::new("e1m1"),  // MUS_E1M1
        MusicInfo::new("e1m2"),  // MUS_E1M2
        MusicInfo::new("e1m3"),  // MUS_E1M3
        MusicInfo::new("e1m4"),  // MUS_E1M4
        MusicInfo::new("e1m5"),  // MUS_E1M5
        MusicInfo::new("e1m6"),  // MUS_E1M6
        MusicInfo::new("e1m7"),  // MUS_E1M7
        MusicInfo::new("e1m8"),  // MUS_E1M8
        MusicInfo::new("e1m9"),  // MUS_E1M9
        MusicInfo::new("inter"), // MUS_INTER
        MusicInfo::new("intro"), // MUS_INTRO
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sfx_table_populated() {
        let table = sfx_table();
        assert!(table.len() >= 23);
        assert_eq!(table[SFX_NONE].name, "none");
        assert_eq!(table[SFX_PISTOL].name, "pistol");
        assert_eq!(table[SFX_SHOTGN].name, "shotgn");
    }

    #[test]
    fn sfx_singularity() {
        let table = sfx_table();
        assert!(!table[SFX_PISTOL].singularity); // pistol: multiple instances OK
        assert!(table[SFX_SAWUP].singularity);    // chainsaw: singular
        assert!(table[SFX_ITEMUP].singularity);   // item pickup: singular
    }

    #[test]
    fn music_table_populated() {
        let table = music_table();
        assert!(table.len() >= 12);
        assert_eq!(table[MUS_E1M1].name, "e1m1");
        assert_eq!(table[MUS_INTRO].name, "intro");
    }

    #[test]
    fn sfx_info_linked() {
        let linked = SfxInfo::linked("test", 64, SFX_PISTOL, 128, 100);
        assert_eq!(linked.link, Some(SFX_PISTOL));
        assert_eq!(linked.pitch, 128);
        assert_eq!(linked.volume, 100);
    }

    #[test]
    fn audio_constants() {
        assert_eq!(S_MAX_VOLUME, 127);
        assert_eq!(NORM_PITCH, 128);
        assert_eq!(NORM_SEP, 128);
        assert_eq!(NUM_CHANNELS, 8);
        // S_ATTENUATOR = (1200 - 160) = 1040
        assert_eq!(S_ATTENUATOR, 1040);
    }
}

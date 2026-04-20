//! # Sistema de Som — Canais, Espacializacao e Mixer
//!
//! Gerencia a reproducao de efeitos sonoros com:
//! - Pool de canais de audio (NUM_CHANNELS = 8)
//! - Posicionamento 3D com atenuacao por distancia
//! - Separacao estereo baseada no angulo de visao
//! - Sistema de prioridade para alocacao de canais
//! - Singularidade (apenas uma instancia de certos sons)
//!
//! ## Atenuacao por distancia
//!
//! ```text
//! Volume
//! |===|         S_CLOSE_DIST (160 unidades)
//! |   \         volume maximo
//! |    \
//! |     \       atenuacao linear
//! |      \
//! |       |     S_CLIPPING_DIST (1200 unidades)
//! +-------+---> inaudivel
//! ```
//!
//! ## Separacao estereo
//!
//! A separacao estereo e calculada usando o angulo entre o listener
//! (jogador) e a fonte de som. Sons a esquerda do jogador saem
//! mais no speaker esquerdo e vice-versa.
//!
//! ## Arquivo C original: `s_sound.c`, `i_sound.c`
//!
//! ## Conceitos que o leitor vai aprender
//! - Pool de canais com alocacao por prioridade
//! - Atenuacao de som por distancia (linear)
//! - Espacializacao estereo via angulo
//! - Interface game-layer (S_*) vs platform-layer (I_*)

use crate::utils::fixed::Fixed;

use super::types::*;

/// Swing estereo — amplitude da separacao.
///
/// C original: `#define S_STEREO_SWING (96*0x10000)` em `s_sound.c`
pub const S_STEREO_SWING: i32 = 96 * 0x10000;

// ---------------------------------------------------------------------------
// Canal de som
// ---------------------------------------------------------------------------

/// Canal de reproducao de som — um slot no mixer.
///
/// C original: `channel_t` (struct local em `s_sound.c`)
#[derive(Debug, Clone)]
pub struct SoundChannel {
    /// Indice do SFX tocando (None = canal livre)
    pub sfx_id: Option<SfxId>,
    /// Origem do som (indice do mobj, None = som do jogador/UI)
    pub origin: Option<usize>,
    /// Handle retornado pela camada de plataforma
    pub handle: i32,
    /// Volume atual (0-127)
    pub volume: i32,
    /// Separacao estereo atual (0=esquerda, 128=centro, 255=direita)
    pub separation: i32,
    /// Pitch atual (0-255)
    pub pitch: i32,
    /// Prioridade do som neste canal
    pub priority: i32,
}

impl SoundChannel {
    /// Cria um canal vazio (livre).
    pub fn new() -> Self {
        SoundChannel {
            sfx_id: None,
            origin: None,
            handle: -1,
            volume: 0,
            separation: NORM_SEP,
            pitch: NORM_PITCH,
            priority: 0,
        }
    }

    /// Verifica se o canal esta livre.
    pub fn is_free(&self) -> bool {
        self.sfx_id.is_none()
    }

    /// Libera o canal.
    pub fn stop(&mut self) {
        self.sfx_id = None;
        self.origin = None;
        self.handle = -1;
    }
}

impl Default for SoundChannel {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Sistema de som
// ---------------------------------------------------------------------------

/// Sistema de som do DOOM — gerencia canais e espacializacao.
///
/// C original: logica em `s_sound.c` com globals
/// `channels[]`, `snd_SfxVolume`, `snd_MusicVolume`
#[derive(Debug)]
pub struct SoundSystem {
    /// Canais de reproducao
    pub channels: Vec<SoundChannel>,
    /// Volume global de SFX (0-127)
    pub sfx_volume: i32,
    /// Tabela de metadados de SFX
    pub sfx_table: Vec<SfxInfo>,
}

impl SoundSystem {
    /// Cria um novo sistema de som.
    ///
    /// C original: `S_Init()` em `s_sound.c`
    pub fn new(sfx_volume: i32) -> Self {
        SoundSystem {
            channels: (0..NUM_CHANNELS).map(|_| SoundChannel::new()).collect(),
            sfx_volume: sfx_volume.clamp(0, S_MAX_VOLUME),
            sfx_table: sfx_table(),
        }
    }

    /// Inicia um som a partir de uma posicao no mundo.
    ///
    /// Calcula volume e separacao estereo baseados na distancia
    /// e angulo entre o listener e a origem, aloca um canal,
    /// e inicia a reproducao.
    ///
    /// C original: `S_StartSoundAtVolume()` em `s_sound.c`
    #[allow(clippy::too_many_arguments)]
    pub fn start_sound(
        &mut self,
        sfx_id: SfxId,
        origin: Option<usize>,
        listener_x: Fixed,
        listener_y: Fixed,
        listener_angle: u32,
        origin_x: Fixed,
        origin_y: Fixed,
    ) {
        if sfx_id == SFX_NONE || sfx_id >= self.sfx_table.len() {
            return;
        }

        let sfx = &self.sfx_table[sfx_id];
        let priority = sfx.priority;

        // Calcular parametros espaciais
        let params = if origin.is_some() {
            self.adjust_sound_params(
                listener_x,
                listener_y,
                listener_angle,
                origin_x,
                origin_y,
            )
        } else {
            // Som do jogador: volume maximo, centro
            Some((self.sfx_volume, NORM_SEP))
        };

        let (volume, sep) = match params {
            Some(p) => p,
            None => return, // fora da distancia de clipping
        };

        // Se e singular, parar instancias anteriores
        if sfx.singularity {
            self.stop_sfx(sfx_id);
        }

        // Alocar canal
        if let Some(chan_idx) = self.get_channel(sfx_id, priority, origin) {
            let chan = &mut self.channels[chan_idx];
            chan.sfx_id = Some(sfx_id);
            chan.origin = origin;
            chan.volume = volume;
            chan.separation = sep;
            chan.pitch = NORM_PITCH;
            chan.priority = priority;
            // TODO: chamar I_StartSound() da camada de plataforma
            chan.handle = chan_idx as i32;
        }
    }

    /// Calcula volume e separacao estereo baseados na distancia.
    ///
    /// Usa distancia Manhattan aproximada (com correcao octante)
    /// para atenuacao linear entre S_CLOSE_DIST e S_CLIPPING_DIST.
    ///
    /// C original: `S_AdjustSoundParams()` em `s_sound.c`
    pub fn adjust_sound_params(
        &self,
        listener_x: Fixed,
        listener_y: Fixed,
        listener_angle: u32,
        source_x: Fixed,
        source_y: Fixed,
    ) -> Option<(i32, i32)> {
        // Calcular distancia Manhattan aproximada
        let adx = (source_x - listener_x).0.abs();
        let ady = (source_y - listener_y).0.abs();

        // Aproximacao: dist ≈ max(adx,ady) + min(adx,ady)/2
        let approx_dist = adx + ady - (adx.min(ady) >> 1);

        if approx_dist > S_CLIPPING_DIST {
            return None; // Muito longe — inaudivel
        }

        // Calcular volume com atenuacao linear
        let volume = if approx_dist < S_CLOSE_DIST {
            self.sfx_volume
        } else {
            let dist_factor = S_CLIPPING_DIST - approx_dist;
            (self.sfx_volume * dist_factor / S_ATTENUATOR) >> 16
        };

        if volume <= 0 {
            return None;
        }

        // Calcular separacao estereo
        // Angulo entre listener e fonte
        let _angle_to_source = {
            let dx = source_x - listener_x;
            let dy = source_y - listener_y;
            // Simplificacao: usar angulo baseado em dx/dy
            // No DOOM completo, usa-se R_PointToAngle2
            let _ = (dx, dy);
            listener_angle // placeholder — sera refinado em fases futuras
        };

        // Separacao centro por enquanto (sera implementado com tabelas de seno)
        let sep = NORM_SEP;

        Some((volume.clamp(0, S_MAX_VOLUME), sep))
    }

    /// Para todas as instancias de um SFX.
    ///
    /// C original: parte de `S_StartSoundAtVolume()` para singularidade
    pub fn stop_sfx(&mut self, sfx_id: SfxId) {
        for chan in &mut self.channels {
            if chan.sfx_id == Some(sfx_id) {
                chan.stop();
            }
        }
    }

    /// Para todos os sons em todos os canais.
    ///
    /// C original: `S_StopSound()` para cada canal
    pub fn stop_all(&mut self) {
        for chan in &mut self.channels {
            chan.stop();
        }
    }

    /// Aloca um canal para um novo som.
    ///
    /// Procura um canal livre. Se nao ha canais livres,
    /// tenta preemptar um som de menor prioridade.
    ///
    /// C original: `S_getChannel()` em `s_sound.c`
    fn get_channel(
        &mut self,
        _sfx_id: SfxId,
        priority: i32,
        origin: Option<usize>,
    ) -> Option<usize> {
        // Se a origem ja tem um som tocando, reusar o canal
        if let Some(orig) = origin {
            for (i, chan) in self.channels.iter().enumerate() {
                if chan.origin == Some(orig) {
                    return Some(i);
                }
            }
        }

        // Procurar canal livre
        for (i, chan) in self.channels.iter().enumerate() {
            if chan.is_free() {
                return Some(i);
            }
        }

        // Preemptar canal de menor prioridade
        let mut lowest_priority = priority;
        let mut lowest_idx = None;

        for (i, chan) in self.channels.iter().enumerate() {
            if chan.priority < lowest_priority {
                lowest_priority = chan.priority;
                lowest_idx = Some(i);
            }
        }

        if let Some(idx) = lowest_idx {
            self.channels[idx].stop();
            return Some(idx);
        }

        None // Todos os canais tem prioridade igual ou maior
    }

    /// Altera o volume global de SFX.
    ///
    /// C original: `S_SetSfxVolume()` em `s_sound.c`
    pub fn set_sfx_volume(&mut self, volume: i32) {
        self.sfx_volume = volume.clamp(0, S_MAX_VOLUME);
    }

    /// Retorna o numero de canais ativos.
    pub fn active_channels(&self) -> usize {
        self.channels.iter().filter(|c| !c.is_free()).count()
    }
}

impl Default for SoundSystem {
    fn default() -> Self {
        Self::new(S_MAX_VOLUME)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_system_init() {
        let ss = SoundSystem::new(100);
        assert_eq!(ss.sfx_volume, 100);
        assert_eq!(ss.channels.len(), NUM_CHANNELS);
        assert_eq!(ss.active_channels(), 0);
    }

    #[test]
    fn start_sound_local() {
        let mut ss = SoundSystem::new(S_MAX_VOLUME);
        ss.start_sound(
            SFX_PISTOL,
            None, // som local (jogador)
            Fixed::ZERO,
            Fixed::ZERO,
            0,
            Fixed::ZERO,
            Fixed::ZERO,
        );
        assert_eq!(ss.active_channels(), 1);
        assert_eq!(ss.channels[0].sfx_id, Some(SFX_PISTOL));
        assert_eq!(ss.channels[0].volume, S_MAX_VOLUME);
    }

    #[test]
    fn start_sound_distant() {
        let mut ss = SoundSystem::new(S_MAX_VOLUME);
        // Som muito longe — deve ser ignorado
        ss.start_sound(
            SFX_PISTOL,
            Some(1),
            Fixed::ZERO,
            Fixed::ZERO,
            0,
            Fixed::from_int(2000), // muito alem de S_CLIPPING_DIST/FRACUNIT=1200
            Fixed::ZERO,
        );
        assert_eq!(ss.active_channels(), 0);
    }

    #[test]
    fn start_sound_close() {
        let mut ss = SoundSystem::new(S_MAX_VOLUME);
        // Som proximo — volume maximo
        ss.start_sound(
            SFX_SHOTGN,
            Some(1),
            Fixed::ZERO,
            Fixed::ZERO,
            0,
            Fixed::from_int(50), // dentro de S_CLOSE_DIST/FRACUNIT=160
            Fixed::ZERO,
        );
        assert_eq!(ss.active_channels(), 1);
        assert_eq!(ss.channels[0].volume, S_MAX_VOLUME);
    }

    #[test]
    fn singularity_stops_previous() {
        let mut ss = SoundSystem::new(S_MAX_VOLUME);
        // Chainsaw e singular
        ss.start_sound(SFX_SAWUP, None, Fixed::ZERO, Fixed::ZERO, 0, Fixed::ZERO, Fixed::ZERO);
        assert_eq!(ss.active_channels(), 1);
        assert_eq!(ss.channels[0].sfx_id, Some(SFX_SAWUP));

        // Segundo chainsaw deve parar o primeiro e reusar
        ss.start_sound(SFX_SAWUP, None, Fixed::ZERO, Fixed::ZERO, 0, Fixed::ZERO, Fixed::ZERO);
        // Canal 0 foi parado pela singularidade, novo som em canal 0
        assert_eq!(ss.active_channels(), 1);
    }

    #[test]
    fn channel_priority_preempt() {
        let mut ss = SoundSystem::new(S_MAX_VOLUME);
        // Preencher todos os canais com som de baixa prioridade
        for i in 0..NUM_CHANNELS {
            ss.channels[i].sfx_id = Some(SFX_TELEPT); // prioridade 32
            ss.channels[i].priority = 32;
            ss.channels[i].handle = i as i32;
        }
        assert_eq!(ss.active_channels(), NUM_CHANNELS);

        // Som de alta prioridade deve preemptar
        ss.start_sound(SFX_PISTOL, None, Fixed::ZERO, Fixed::ZERO, 0, Fixed::ZERO, Fixed::ZERO);
        // Deve ter preemptado um canal
        assert_eq!(ss.active_channels(), NUM_CHANNELS);
        assert!(ss.channels.iter().any(|c| c.sfx_id == Some(SFX_PISTOL)));
    }

    #[test]
    fn stop_all() {
        let mut ss = SoundSystem::new(S_MAX_VOLUME);
        ss.start_sound(SFX_PISTOL, None, Fixed::ZERO, Fixed::ZERO, 0, Fixed::ZERO, Fixed::ZERO);
        ss.start_sound(SFX_SHOTGN, None, Fixed::ZERO, Fixed::ZERO, 0, Fixed::ZERO, Fixed::ZERO);
        assert_eq!(ss.active_channels(), 2);
        ss.stop_all();
        assert_eq!(ss.active_channels(), 0);
    }

    #[test]
    fn set_sfx_volume_clamp() {
        let mut ss = SoundSystem::new(100);
        ss.set_sfx_volume(200);
        assert_eq!(ss.sfx_volume, S_MAX_VOLUME);
        ss.set_sfx_volume(-10);
        assert_eq!(ss.sfx_volume, 0);
    }

    #[test]
    fn adjust_params_clipping() {
        let ss = SoundSystem::new(S_MAX_VOLUME);
        // Muito longe
        let result = ss.adjust_sound_params(
            Fixed::ZERO, Fixed::ZERO, 0,
            Fixed::from_int(2000), Fixed::ZERO,
        );
        assert!(result.is_none());
    }

    #[test]
    fn adjust_params_close() {
        let ss = SoundSystem::new(S_MAX_VOLUME);
        // Muito perto
        let result = ss.adjust_sound_params(
            Fixed::ZERO, Fixed::ZERO, 0,
            Fixed::from_int(10), Fixed::ZERO,
        );
        assert!(result.is_some());
        let (vol, _sep) = result.unwrap();
        assert_eq!(vol, S_MAX_VOLUME);
    }
}

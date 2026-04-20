//! # Tipos e Pacotes de Rede
//!
//! Define os tipos fundamentais do protocolo de rede do DOOM:
//! - `DoomData` — payload dos pacotes de rede (ticcmds)
//! - `DoomCom` — buffer de controle entre game e driver de rede
//! - Constantes de protocolo (flags, limites, portas)
//!
//! ## Protocolo de rede do DOOM
//!
//! O DOOM usa um modelo **lockstep determinístico**: todos os
//! jogadores executam a mesma sequencia de tics com os mesmos
//! inputs. A rede sincroniza apenas os inputs (ticcmds),
//! nao o estado do jogo.
//!
//! ```text
//! Jogador A                    Jogador B
//! +----------+                 +----------+
//! | ticcmd_A | ---(UDP)------> | ticcmd_A |
//! | ticcmd_B | <---(UDP)------ | ticcmd_B |
//! +----------+                 +----------+
//!      |                            |
//!   game tick                    game tick
//!   (deterministico)           (deterministico)
//!      |                            |
//!   mesmo estado              mesmo estado
//! ```
//!
//! ## Formato do pacote
//!
//! ```text
//! DoomData (payload UDP):
//! +----------+------+------+------+------+
//! | checksum | start| ntics|player|retrans|
//! | 4 bytes  | 1b   | 1b   | 1b   | 1b   |
//! +----------+------+------+------+------+
//! | cmds[0]  | cmds[1] | ... | cmds[n-1] |
//! | ticcmd_t | ticcmd_t| ... | ticcmd_t  |
//! +-------------------------------------------+
//! ```
//!
//! ## Arquivo C original: `d_net.h`, `doomdef.h`
//!
//! ## Conceitos que o leitor vai aprender
//! - Lockstep determinístico para multiplayer
//! - Formato de pacote com checksum e flags
//! - Ring buffer de comandos (BACKUPTICS)

/// Numero maximo de nos (jogadores) na rede.
///
/// C original: `#define MAXNETNODES 8` em `d_net.h`
pub const MAXNETNODES: usize = 8;

/// Numero maximo de jogadores.
///
/// C original: `#define MAXPLAYERS 4` em `doomdef.h`
pub const MAXPLAYERS: usize = 4;

/// Numero de tics armazenados para retransmissao.
///
/// Ring buffer de comandos: permite reenviar ate 12 tics
/// anteriores caso um pacote se perca.
///
/// C original: `#define BACKUPTICS 12` em `doomdef.h`
pub const BACKUPTICS: usize = 12;

/// Porta UDP padrao do DOOM.
///
/// C original: `DOOMPORT = IPPORT_USERRESERVED + 0x1d` em `i_net.c`
/// IPPORT_USERRESERVED = 5000, entao DOOMPORT = 5029
/// (No linuxdoom original era 1024 + 0x1d = 1053)
pub const DOOMPORT: u16 = 5029;

// ---------------------------------------------------------------------------
// Flags de pacote (codificadas no checksum)
// ---------------------------------------------------------------------------

/// Flag: jogador saindo do jogo.
///
/// C original: `#define NCMD_EXIT 0x80000000l`
pub const NCMD_EXIT: u32 = 0x80000000;

/// Flag: pedido de retransmissao.
///
/// Quando setada, `retransmit_from` indica o tic a partir
/// do qual o remetente deve reenviar.
///
/// C original: `#define NCMD_RETRANSMIT 0x40000000l`
pub const NCMD_RETRANSMIT: u32 = 0x40000000;

/// Flag: pacote de setup (configuracao inicial).
///
/// C original: `#define NCMD_SETUP 0x20000000l`
pub const NCMD_SETUP: u32 = 0x20000000;

/// Flag: driver de rede matou o jogo.
///
/// C original: `#define NCMD_KILL 0x10000000l`
pub const NCMD_KILL: u32 = 0x10000000;

/// Mascara para extrair o checksum real (28 bits).
///
/// C original: `#define NCMD_CHECKSUM 0x0fffffffl`
pub const NCMD_CHECKSUM: u32 = 0x0FFFFFFF;

/// Flag de drone (jogador observador, sem controle).
///
/// C original: `#define PL_DRONE 0x80`
pub const PL_DRONE: u8 = 0x80;

/// Numero de ticks antes de solicitar retransmissao.
///
/// C original: `#define RESENDCOUNT 10`
pub const RESENDCOUNT: i32 = 10;

// ---------------------------------------------------------------------------
// Comandos do driver de rede
// ---------------------------------------------------------------------------

/// Comando: enviar pacote.
///
/// C original: `#define CMD_SEND 1`
pub const CMD_SEND: i16 = 1;

/// Comando: receber pacote.
///
/// C original: `#define CMD_GET 2`
pub const CMD_GET: i16 = 2;

/// ID magico para validacao do doomcom.
///
/// C original: `#define DOOMCOM_ID 0x12345678l`
pub const DOOMCOM_ID: u32 = 0x12345678;

// ---------------------------------------------------------------------------
// NetTicCmd — ticcmd serializado para rede
// ---------------------------------------------------------------------------

/// Comando de tic serializado para transmissao em rede.
///
/// Versao compacta do `TicCmd` otimizada para o wire format.
/// No DOOM, `ticcmd_t` e a mesma struct usada localmente e na rede,
/// mas os campos multi-byte (angleturn, consistancy) precisam
/// de byte-swapping.
///
/// C original: `ticcmd_t` em `d_ticcmd.h`
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NetTicCmd {
    /// Movimento frontal (-/+ = tras/frente)
    pub forwardmove: i8,
    /// Movimento lateral (-/+ = esquerda/direita)
    pub sidemove: i8,
    /// Rotacao (angulo, 16-bit)
    pub angleturn: i16,
    /// Verificacao de consistencia (para detectar dessincronizacao)
    pub consistancy: i16,
    /// Caractere de chat
    pub chatchar: u8,
    /// Botoes (BT_ATTACK, BT_USE, BT_CHANGE, BT_SPECIAL)
    pub buttons: u8,
}

impl NetTicCmd {
    /// Cria um comando vazio.
    pub fn new() -> Self {
        Self::default()
    }

    /// Calcula o tamanho em bytes para serializacao.
    pub const fn wire_size() -> usize {
        // forwardmove(1) + sidemove(1) + angleturn(2) +
        // consistancy(2) + chatchar(1) + buttons(1) = 8 bytes
        8
    }

    /// Serializa o comando para bytes (network byte order).
    ///
    /// C original: byte-swapping em `PacketSend()` de `i_net.c`
    pub fn to_bytes(&self) -> [u8; 8] {
        let mut buf = [0u8; 8];
        buf[0] = self.forwardmove as u8;
        buf[1] = self.sidemove as u8;
        let turn = self.angleturn.to_be_bytes();
        buf[2] = turn[0];
        buf[3] = turn[1];
        let cons = self.consistancy.to_be_bytes();
        buf[4] = cons[0];
        buf[5] = cons[1];
        buf[6] = self.chatchar;
        buf[7] = self.buttons;
        buf
    }

    /// Desserializa o comando a partir de bytes (network byte order).
    ///
    /// C original: byte-swapping em `PacketGet()` de `i_net.c`
    pub fn from_bytes(buf: &[u8; 8]) -> Self {
        NetTicCmd {
            forwardmove: buf[0] as i8,
            sidemove: buf[1] as i8,
            angleturn: i16::from_be_bytes([buf[2], buf[3]]),
            consistancy: i16::from_be_bytes([buf[4], buf[5]]),
            chatchar: buf[6],
            buttons: buf[7],
        }
    }
}

// ---------------------------------------------------------------------------
// DoomData — payload do pacote de rede
// ---------------------------------------------------------------------------

/// Payload de um pacote de rede do DOOM.
///
/// Contem o checksum (com flags nos bits altos), metadados do pacote,
/// e ate BACKUPTICS ticcmds do jogador.
///
/// C original: `doomdata_t` em `d_net.h`
#[derive(Debug, Clone)]
pub struct DoomData {
    /// Checksum com flags nos 4 bits mais altos.
    ///
    /// Bits 31-28: NCMD_EXIT, NCMD_RETRANSMIT, NCMD_SETUP, NCMD_KILL
    /// Bits 27-0: checksum real do pacote
    pub checksum: u32,

    /// Tic de retransmissao (quando flag NCMD_RETRANSMIT esta setada).
    /// Apenas o byte baixo e transmitido.
    pub retransmit_from: u8,

    /// Primeiro tic contido neste pacote (byte baixo).
    ///
    /// O valor completo e reconstruido via `expand_tics()`.
    pub starttic: u8,

    /// Jogador que enviou este pacote.
    ///
    /// Bit 7 (PL_DRONE) indica jogador observador.
    pub player: u8,

    /// Numero de tics contidos no pacote.
    pub numtics: u8,

    /// Comandos de tic (ate BACKUPTICS).
    pub cmds: Vec<NetTicCmd>,
}

impl DoomData {
    /// Cria um pacote vazio.
    pub fn new() -> Self {
        DoomData {
            checksum: 0,
            retransmit_from: 0,
            starttic: 0,
            player: 0,
            numtics: 0,
            cmds: Vec::new(),
        }
    }

    /// Verifica se a flag de saida esta setada.
    pub fn is_exit(&self) -> bool {
        self.checksum & NCMD_EXIT != 0
    }

    /// Verifica se e um pedido de retransmissao.
    pub fn is_retransmit(&self) -> bool {
        self.checksum & NCMD_RETRANSMIT != 0
    }

    /// Verifica se e um pacote de setup.
    pub fn is_setup(&self) -> bool {
        self.checksum & NCMD_SETUP != 0
    }

    /// Retorna o checksum real (28 bits, sem flags).
    pub fn real_checksum(&self) -> u32 {
        self.checksum & NCMD_CHECKSUM
    }

    /// Define as flags no checksum.
    pub fn set_flags(&mut self, flags: u32) {
        self.checksum = (self.checksum & NCMD_CHECKSUM) | flags;
    }

    /// Calcula o checksum a partir dos ticcmds.
    ///
    /// C original: `NetbufferChecksum()` em `d_net.c`
    pub fn calculate_checksum(&self) -> u32 {
        let mut sum: u32 = 0;
        for cmd in &self.cmds {
            sum = sum.wrapping_add(cmd.forwardmove as u32);
            sum = sum.wrapping_add(cmd.sidemove as u32);
            sum = sum.wrapping_add(cmd.angleturn as u32);
            sum = sum.wrapping_add(cmd.buttons as u32);
        }
        sum & NCMD_CHECKSUM
    }
}

impl Default for DoomData {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// DoomCom — buffer de controle game <-> driver de rede
// ---------------------------------------------------------------------------

/// Buffer de controle entre o jogo e o driver de rede.
///
/// No DOOM original, o driver de rede (IPXSETUP, SERSETUP)
/// comunicava com o jogo via esta struct em memoria compartilhada.
/// No nosso port, serve como configuracao e estado da sessao.
///
/// C original: `doomcom_t` em `d_net.h`
#[derive(Debug, Clone)]
pub struct DoomCom {
    /// ID magico para validacao (DOOMCOM_ID = 0x12345678)
    pub id: u32,

    /// Numero de nos (jogadores) na sessao
    pub numnodes: i16,
    /// Numero de jogadores ativos
    pub numplayers: i16,
    /// Indice do jogador local (consoleplayer)
    pub consoleplayer: i16,

    /// Fator de duplicacao de tics (1-9).
    ///
    /// Em redes lentas, cada tic e repetido N vezes para
    /// reduzir a quantidade de pacotes necessarios.
    pub ticdup: i16,
    /// Tics extras a enviar em cada pacote (para redundancia)
    pub extratics: i16,

    /// Tipo de deathmatch (0=coop, 1=deathmatch, 2=altdeath)
    pub deathmatch: i16,
    /// Episodio selecionado (1-based)
    pub episode: i16,
    /// Mapa selecionado (1-based)
    pub map: i16,
    /// Dificuldade (0-4)
    pub skill: i16,
}

impl DoomCom {
    /// Cria uma configuracao para jogo single-player.
    pub fn single_player() -> Self {
        DoomCom {
            id: DOOMCOM_ID,
            numnodes: 1,
            numplayers: 1,
            consoleplayer: 0,
            ticdup: 1,
            extratics: 1,
            deathmatch: 0,
            episode: 1,
            map: 1,
            skill: 2, // Medium
        }
    }

    /// Cria uma configuracao para jogo multiplayer.
    pub fn multiplayer(num_players: i16, consoleplayer: i16) -> Self {
        DoomCom {
            id: DOOMCOM_ID,
            numnodes: num_players,
            numplayers: num_players,
            consoleplayer,
            ticdup: 1,
            extratics: 1,
            deathmatch: 0,
            episode: 1,
            map: 1,
            skill: 2,
        }
    }

    /// Verifica se e uma sessao multiplayer.
    pub fn is_multiplayer(&self) -> bool {
        self.numplayers > 1
    }

    /// Verifica se e deathmatch.
    pub fn is_deathmatch(&self) -> bool {
        self.deathmatch != 0
    }

    /// Valida o ID magico.
    pub fn is_valid(&self) -> bool {
        self.id == DOOMCOM_ID
    }
}

impl Default for DoomCom {
    fn default() -> Self {
        Self::single_player()
    }
}

// ---------------------------------------------------------------------------
// Expansao de tics
// ---------------------------------------------------------------------------

/// Expande um numero de tic de 8 bits para o valor completo.
///
/// O campo `starttic` no pacote tem apenas 8 bits. Para
/// reconstruir o tic completo, usamos o tic atual como
/// referencia e ajustamos para o valor mais proximo.
///
/// C original: `ExpandTics()` em `d_net.c`
pub fn expand_tics(low_byte: u8, current_tic: i32) -> i32 {
    let delta = (low_byte as i32) - (current_tic & 0xFF);
    // Ajustar para o valor mais proximo (wraparound de 256)
    if delta > 128 {
        current_tic + delta - 256
    } else if delta < -128 {
        current_tic + delta + 256
    } else {
        current_tic + delta
    }
}

/// Calcula o checksum de consistencia para um tic.
///
/// Usado para detectar dessincronizacao entre jogadores.
/// No DOOM, a consistencia e baseada na posicao X do jogador.
///
/// C original: parte de `G_Ticker()` em `g_game.c`
pub fn consistency_check(player_x: i32) -> i16 {
    (player_x & 0xFFFF) as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants() {
        assert_eq!(MAXNETNODES, 8);
        assert_eq!(MAXPLAYERS, 4);
        assert_eq!(BACKUPTICS, 12);
        assert_eq!(DOOMCOM_ID, 0x12345678);
    }

    #[test]
    fn net_tic_cmd_roundtrip() {
        let cmd = NetTicCmd {
            forwardmove: 25,
            sidemove: -10,
            angleturn: 640,
            consistancy: 12345,
            chatchar: b'H',
            buttons: 3,
        };

        let bytes = cmd.to_bytes();
        assert_eq!(bytes.len(), 8);

        let restored = NetTicCmd::from_bytes(&bytes);
        assert_eq!(restored, cmd);
    }

    #[test]
    fn net_tic_cmd_negative_values() {
        let cmd = NetTicCmd {
            forwardmove: -50,
            sidemove: -24,
            angleturn: -1280,
            consistancy: -1,
            chatchar: 0,
            buttons: 0,
        };

        let bytes = cmd.to_bytes();
        let restored = NetTicCmd::from_bytes(&bytes);
        assert_eq!(restored, cmd);
    }

    #[test]
    fn doom_data_flags() {
        let mut data = DoomData::new();
        assert!(!data.is_exit());
        assert!(!data.is_retransmit());
        assert!(!data.is_setup());

        data.set_flags(NCMD_EXIT);
        assert!(data.is_exit());
        assert!(!data.is_retransmit());

        data.set_flags(NCMD_RETRANSMIT | NCMD_EXIT);
        assert!(data.is_exit());
        assert!(data.is_retransmit());
    }

    #[test]
    fn doom_data_checksum() {
        let mut data = DoomData::new();
        data.cmds.push(NetTicCmd {
            forwardmove: 10,
            sidemove: 5,
            angleturn: 100,
            consistancy: 0,
            chatchar: 0,
            buttons: 1,
        });

        let checksum = data.calculate_checksum();
        assert!(checksum > 0);
        assert_eq!(checksum & !NCMD_CHECKSUM, 0); // sem flags
    }

    #[test]
    fn doom_com_single_player() {
        let com = DoomCom::single_player();
        assert!(com.is_valid());
        assert!(!com.is_multiplayer());
        assert!(!com.is_deathmatch());
        assert_eq!(com.consoleplayer, 0);
    }

    #[test]
    fn doom_com_multiplayer() {
        let com = DoomCom::multiplayer(4, 1);
        assert!(com.is_multiplayer());
        assert_eq!(com.numplayers, 4);
        assert_eq!(com.consoleplayer, 1);
    }

    #[test]
    fn expand_tics_normal() {
        // Byte baixo 10, tic atual 5 → delta = 5 → resultado 10
        assert_eq!(expand_tics(10, 5), 10);

        // Byte baixo 100, tic atual 256+90 → delta = 100-90=10 → 256+100
        assert_eq!(expand_tics(100, 346), 356);
    }

    #[test]
    fn expand_tics_wraparound() {
        // Wraparound: byte baixo 5, tic atual 250 → delta = 5-250 = -245 → +256 = 11
        // Resultado: 250 + 11 = 261
        assert_eq!(expand_tics(5, 250), 261);

        // Outro lado: byte baixo 250, tic atual 260 (byte=4) → delta = 250-4 = 246 → -256 = -10
        // Resultado: 260 + (-10) = 250
        assert_eq!(expand_tics(250, 260), 250);
    }

    #[test]
    fn consistency_check_basic() {
        let check = consistency_check(0x12345678);
        assert_eq!(check, 0x5678u16 as i16);
    }

    #[test]
    fn packet_flags_mask() {
        assert_eq!(NCMD_EXIT | NCMD_RETRANSMIT | NCMD_SETUP | NCMD_KILL, 0xF0000000);
        assert_eq!(NCMD_CHECKSUM, 0x0FFFFFFF);
    }
}

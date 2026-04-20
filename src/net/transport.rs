//! # Camada de Transporte UDP
//!
//! Interface de rede de baixo nivel para transmissao de pacotes:
//! - Socket UDP non-blocking
//! - Serializacao/desserializacao de pacotes
//! - Resolucao de enderecos de jogadores
//!
//! ## Arquitetura de rede do DOOM
//!
//! No DOOM original, a rede era dividida em duas camadas:
//!
//! ```text
//! Game Layer (d_net.c)         Transport Layer (i_net.c)
//! +-------------------+       +--------------------+
//! | NetUpdate()       |       | I_InitNetwork()    |
//! | TryRunTics()      | ----> | PacketSend()       |
//! | GetPackets()      | <---- | PacketGet()        |
//! +-------------------+       +--------------------+
//!       |                            |
//!  sincronizacao              UDP sockets
//!  lockstep                   byte-swapping
//!  retransmissao              non-blocking I/O
//! ```
//!
//! ## Driver externo
//!
//! No linuxdoom original, o driver de rede (IPXSETUP/SERSETUP)
//! era um processo separado que se comunicava via `doomcom_t`
//! em memoria compartilhada. No nosso port, a comunicacao
//! e feita diretamente via sockets UDP.
//!
//! ## Arquivo C original: `i_net.c`, `i_net.h`
//!
//! ## Conceitos que o leitor vai aprender
//! - Serializacao de structs para wire format
//! - Non-blocking UDP para game networking
//! - Byte order (host vs network) em protocolos binarios

use super::types::*;

// ---------------------------------------------------------------------------
// Serializacao de pacotes
// ---------------------------------------------------------------------------

/// Tamanho maximo de um pacote UDP do DOOM.
///
/// Header (8 bytes) + BACKUPTICS * ticcmd (8 bytes cada) = 104 bytes
pub const MAX_PACKET_SIZE: usize = 8 + BACKUPTICS * NetTicCmd::wire_size();

/// Serializa um DoomData para bytes (wire format).
///
/// Formato:
/// ```text
/// [0..4]  checksum (big-endian)
/// [4]     retransmit_from
/// [5]     starttic
/// [6]     player
/// [7]     numtics
/// [8..]   cmds (8 bytes cada)
/// ```
///
/// C original: byte-swapping em `PacketSend()` de `i_net.c`
pub fn serialize_packet(data: &DoomData) -> Vec<u8> {
    let header_size = 8;
    let cmds_size = data.numtics as usize * NetTicCmd::wire_size();
    let mut buf = Vec::with_capacity(header_size + cmds_size);

    // Checksum (big-endian)
    buf.extend_from_slice(&data.checksum.to_be_bytes());

    // Header fields
    buf.push(data.retransmit_from);
    buf.push(data.starttic);
    buf.push(data.player);
    buf.push(data.numtics);

    // Ticcmds
    for i in 0..data.numtics as usize {
        if i < data.cmds.len() {
            buf.extend_from_slice(&data.cmds[i].to_bytes());
        } else {
            buf.extend_from_slice(&[0u8; 8]);
        }
    }

    buf
}

/// Desserializa bytes (wire format) para DoomData.
///
/// C original: byte-swapping em `PacketGet()` de `i_net.c`
pub fn deserialize_packet(buf: &[u8]) -> Option<DoomData> {
    if buf.len() < 8 {
        return None;
    }

    let checksum = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
    let retransmit_from = buf[4];
    let starttic = buf[5];
    let player = buf[6];
    let numtics = buf[7];

    // Verificar se temos bytes suficientes para os ticcmds
    let expected_size = 8 + numtics as usize * NetTicCmd::wire_size();
    if buf.len() < expected_size {
        return None;
    }

    let mut cmds = Vec::with_capacity(numtics as usize);
    for i in 0..numtics as usize {
        let offset = 8 + i * 8;
        let cmd_bytes: [u8; 8] = [
            buf[offset],
            buf[offset + 1],
            buf[offset + 2],
            buf[offset + 3],
            buf[offset + 4],
            buf[offset + 5],
            buf[offset + 6],
            buf[offset + 7],
        ];
        cmds.push(NetTicCmd::from_bytes(&cmd_bytes));
    }

    Some(DoomData {
        checksum,
        retransmit_from,
        starttic,
        player,
        numtics,
        cmds,
    })
}

// ---------------------------------------------------------------------------
// Endereco de no de rede
// ---------------------------------------------------------------------------

/// Endereco de um no na rede.
///
/// C original: `struct sockaddr_in sendaddress[MAXNETNODES]` em `i_net.c`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeAddress {
    /// Endereco IP (4 bytes, ex: [192, 168, 1, 1])
    pub ip: [u8; 4],
    /// Porta UDP
    pub port: u16,
}

impl NodeAddress {
    /// Cria um endereco de no.
    pub fn new(ip: [u8; 4], port: u16) -> Self {
        NodeAddress { ip, port }
    }

    /// Cria um endereco localhost.
    pub fn localhost(port: u16) -> Self {
        NodeAddress::new([127, 0, 0, 1], port)
    }
}

impl std::fmt::Display for NodeAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}:{}",
            self.ip[0], self.ip[1], self.ip[2], self.ip[3], self.port
        )
    }
}

impl Default for NodeAddress {
    fn default() -> Self {
        Self::localhost(DOOMPORT)
    }
}

// ---------------------------------------------------------------------------
// Trait de transporte
// ---------------------------------------------------------------------------

/// Interface de transporte de rede.
///
/// Abstrai a camada de sockets para permitir implementacoes
/// diferentes (UDP real, loopback para testes, etc.).
///
/// C original: `I_NetCmd()` dispatch em `i_net.c`
pub trait NetTransport: std::fmt::Debug {
    /// Envia um pacote para o no especificado.
    ///
    /// C original: `PacketSend()` em `i_net.c`
    fn send(&mut self, node: usize, data: &[u8]) -> bool;

    /// Tenta receber um pacote (non-blocking).
    ///
    /// Retorna `Some((node, data))` se um pacote foi recebido,
    /// `None` se nao ha pacotes pendentes.
    ///
    /// C original: `PacketGet()` em `i_net.c`
    fn receive(&mut self) -> Option<(usize, Vec<u8>)>;
}

// ---------------------------------------------------------------------------
// LoopbackTransport — para single-player e testes
// ---------------------------------------------------------------------------

/// Transporte loopback — para single-player e testes unitarios.
///
/// Pacotes enviados sao armazenados em uma fila interna e
/// podem ser recebidos imediatamente. Util para testar a
/// logica de sincronizacao sem sockets reais.
#[derive(Debug)]
pub struct LoopbackTransport {
    /// Fila de pacotes pendentes (node, data)
    queue: Vec<(usize, Vec<u8>)>,
}

impl LoopbackTransport {
    /// Cria um novo transporte loopback.
    pub fn new() -> Self {
        LoopbackTransport { queue: Vec::new() }
    }
}

impl Default for LoopbackTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl NetTransport for LoopbackTransport {
    fn send(&mut self, node: usize, data: &[u8]) -> bool {
        self.queue.push((node, data.to_vec()));
        true
    }

    fn receive(&mut self) -> Option<(usize, Vec<u8>)> {
        if self.queue.is_empty() {
            None
        } else {
            Some(self.queue.remove(0))
        }
    }
}

// ---------------------------------------------------------------------------
// PairTransport — para testes multiplayer local
// ---------------------------------------------------------------------------

/// Par de transportes conectados — para testes de 2 jogadores.
///
/// Simula dois endpoints de rede conectados diretamente.
/// Pacotes enviados por um lado aparecem no receive do outro.
#[derive(Debug)]
pub struct TransportPair {
    /// Fila de pacotes do lado A → B
    a_to_b: Vec<Vec<u8>>,
    /// Fila de pacotes do lado B → A
    b_to_a: Vec<Vec<u8>>,
}

impl TransportPair {
    /// Cria um par de transportes conectados.
    pub fn new() -> Self {
        TransportPair {
            a_to_b: Vec::new(),
            b_to_a: Vec::new(),
        }
    }

    /// Envia um pacote do lado A para B.
    pub fn send_a(&mut self, data: &[u8]) {
        self.a_to_b.push(data.to_vec());
    }

    /// Envia um pacote do lado B para A.
    pub fn send_b(&mut self, data: &[u8]) {
        self.b_to_a.push(data.to_vec());
    }

    /// Recebe um pacote no lado A (enviado por B).
    pub fn receive_a(&mut self) -> Option<Vec<u8>> {
        if self.b_to_a.is_empty() {
            None
        } else {
            Some(self.b_to_a.remove(0))
        }
    }

    /// Recebe um pacote no lado B (enviado por A).
    pub fn receive_b(&mut self) -> Option<Vec<u8>> {
        if self.a_to_b.is_empty() {
            None
        } else {
            Some(self.a_to_b.remove(0))
        }
    }

    /// Verifica se ha pacotes pendentes em qualquer direcao.
    pub fn has_pending(&self) -> bool {
        !self.a_to_b.is_empty() || !self.b_to_a.is_empty()
    }
}

impl Default for TransportPair {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_deserialize_roundtrip() {
        let mut data = DoomData::new();
        data.checksum = 0x12345678;
        data.starttic = 42;
        data.player = 1;
        data.numtics = 2;
        data.cmds.push(NetTicCmd {
            forwardmove: 25,
            sidemove: -10,
            angleturn: 640,
            consistancy: 100,
            chatchar: 0,
            buttons: 3,
        });
        data.cmds.push(NetTicCmd {
            forwardmove: -50,
            sidemove: 24,
            angleturn: -1280,
            consistancy: 200,
            chatchar: b'H',
            buttons: 0,
        });

        let bytes = serialize_packet(&data);
        assert_eq!(bytes.len(), 8 + 2 * 8); // header + 2 cmds

        let restored = deserialize_packet(&bytes).unwrap();
        assert_eq!(restored.checksum, data.checksum);
        assert_eq!(restored.starttic, data.starttic);
        assert_eq!(restored.player, data.player);
        assert_eq!(restored.numtics, data.numtics);
        assert_eq!(restored.cmds.len(), 2);
        assert_eq!(restored.cmds[0], data.cmds[0]);
        assert_eq!(restored.cmds[1], data.cmds[1]);
    }

    #[test]
    fn deserialize_too_short() {
        assert!(deserialize_packet(&[0; 4]).is_none());
    }

    #[test]
    fn deserialize_truncated_cmds() {
        // Header diz 2 tics mas dados insuficientes
        let mut buf = [0u8; 12]; // header(8) + only 4 bytes of cmd
        buf[7] = 2; // numtics = 2, needs 24 bytes total
        assert!(deserialize_packet(&buf).is_none());
    }

    #[test]
    fn node_address() {
        let addr = NodeAddress::new([192, 168, 1, 100], 5029);
        assert_eq!(addr.to_string(), "192.168.1.100:5029");

        let local = NodeAddress::localhost(DOOMPORT);
        assert_eq!(local.ip, [127, 0, 0, 1]);
        assert_eq!(local.port, DOOMPORT);
    }

    #[test]
    fn loopback_transport() {
        let mut transport = LoopbackTransport::new();

        // Sem pacotes
        assert!(transport.receive().is_none());

        // Enviar e receber
        assert!(transport.send(0, &[1, 2, 3]));
        let (node, data) = transport.receive().unwrap();
        assert_eq!(node, 0);
        assert_eq!(data, vec![1, 2, 3]);

        // Fila vazia de novo
        assert!(transport.receive().is_none());
    }

    #[test]
    fn transport_pair() {
        let mut pair = TransportPair::new();

        // A envia para B
        pair.send_a(&[10, 20, 30]);
        assert!(pair.has_pending());

        // B recebe
        let data = pair.receive_b().unwrap();
        assert_eq!(data, vec![10, 20, 30]);

        // B envia para A
        pair.send_b(&[40, 50]);
        let data = pair.receive_a().unwrap();
        assert_eq!(data, vec![40, 50]);

        assert!(!pair.has_pending());
    }

    #[test]
    fn max_packet_size() {
        assert_eq!(MAX_PACKET_SIZE, 8 + 12 * 8); // 104 bytes
    }

    #[test]
    fn serialize_empty_packet() {
        let data = DoomData::new();
        let bytes = serialize_packet(&data);
        assert_eq!(bytes.len(), 8); // header only

        let restored = deserialize_packet(&bytes).unwrap();
        assert_eq!(restored.numtics, 0);
        assert!(restored.cmds.is_empty());
    }

    #[test]
    fn serialize_with_flags() {
        let mut data = DoomData::new();
        data.set_flags(NCMD_EXIT | NCMD_RETRANSMIT);
        data.retransmit_from = 42;

        let bytes = serialize_packet(&data);
        let restored = deserialize_packet(&bytes).unwrap();

        assert!(restored.is_exit());
        assert!(restored.is_retransmit());
        assert_eq!(restored.retransmit_from, 42);
    }
}

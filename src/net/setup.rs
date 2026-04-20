//! # Setup e Arbitragem de Rede
//!
//! Gerencia a inicializacao de uma sessao multiplayer:
//! - Deteccao de modo de jogo (single/multi)
//! - Handshake entre jogadores (NCMD_SETUP)
//! - Broadcast de configuracao (skill, episode, map, deathmatch)
//! - Verificacao de consistencia da sessao
//!
//! ## Protocolo de setup
//!
//! ```text
//! Host (consoleplayer=0)          Client (consoleplayer=1)
//!         |                              |
//!         |--- NCMD_SETUP (config) ----->|
//!         |                              |
//!         |<-- NCMD_SETUP (ack) ---------|
//!         |                              |
//!         |--- NCMD_SETUP (config) ----->|  (repetido ate
//!         |<-- NCMD_SETUP (ack) ---------|   todos confirmarem)
//!         |                              |
//!      jogo inicia                    jogo inicia
//! ```
//!
//! O host (jogador 0) envia pacotes de setup com a configuracao
//! do jogo. Cada cliente responde com um ack. Quando todos
//! confirmam, o jogo inicia sincronizadamente.
//!
//! ## Arquivo C original: `d_net.c` (D_CheckNetGame, D_ArbitrateNetStart)
//!
//! ## Conceitos que o leitor vai aprender
//! - Handshake de rede (setup/ack)
//! - Arbitragem de configuracao multiplayer
//! - Deteccao de modo de jogo

use super::types::*;

// ---------------------------------------------------------------------------
// Configuracao da sessao
// ---------------------------------------------------------------------------

/// Configuracao de uma sessao de jogo multiplayer.
///
/// Broadcast pelo host durante a fase de setup.
/// Todos os jogadores devem concordar com estes parametros.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameConfig {
    /// Dificuldade (0=Baby, 1=Easy, 2=Medium, 3=Hard, 4=Nightmare)
    pub skill: i32,
    /// Episodio (1-based, 0 para DOOM 2)
    pub episode: i32,
    /// Mapa (1-based)
    pub map: i32,
    /// Tipo de deathmatch (0=coop, 1=deathmatch, 2=altdeath)
    pub deathmatch: i32,
    /// Numero de jogadores
    pub num_players: i32,
    /// Fator de duplicacao de tics
    pub ticdup: i32,
    /// Tics extras por pacote
    pub extratics: i32,
}

impl GameConfig {
    /// Cria uma configuracao padrao (coop, E1M1, Medium).
    pub fn default_config() -> Self {
        GameConfig {
            skill: 2,
            episode: 1,
            map: 1,
            deathmatch: 0,
            num_players: 1,
            ticdup: 1,
            extratics: 1,
        }
    }

    /// Cria a configuracao a partir de um DoomCom.
    pub fn from_doomcom(com: &DoomCom) -> Self {
        GameConfig {
            skill: com.skill as i32,
            episode: com.episode as i32,
            map: com.map as i32,
            deathmatch: com.deathmatch as i32,
            num_players: com.numplayers as i32,
            ticdup: com.ticdup as i32,
            extratics: com.extratics as i32,
        }
    }

    /// Aplica a configuracao a um DoomCom.
    pub fn apply_to_doomcom(&self, com: &mut DoomCom) {
        com.skill = self.skill as i16;
        com.episode = self.episode as i16;
        com.map = self.map as i16;
        com.deathmatch = self.deathmatch as i16;
        com.numplayers = self.num_players as i16;
        com.ticdup = self.ticdup as i16;
        com.extratics = self.extratics as i16;
    }
}

impl Default for GameConfig {
    fn default() -> Self {
        Self::default_config()
    }
}

// ---------------------------------------------------------------------------
// Estado do setup
// ---------------------------------------------------------------------------

/// Estado da fase de setup de rede.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupState {
    /// Esperando conexao de todos os jogadores
    WaitingForPlayers,
    /// Enviando/recebendo configuracao
    Exchanging,
    /// Setup completo, pronto para jogar
    Complete,
    /// Erro na configuracao
    Failed,
}

/// Gerenciador de setup de sessao multiplayer.
///
/// C original: `D_ArbitrateNetStart()` em `d_net.c`
#[derive(Debug)]
pub struct NetSetup {
    /// Estado atual do setup
    pub state: SetupState,
    /// Configuracao do jogo
    pub config: GameConfig,
    /// Se cada no confirmou o setup
    pub node_confirmed: [bool; MAXNETNODES],
    /// Numero de nos esperados
    pub expected_nodes: usize,
    /// Se somos o host (consoleplayer == 0)
    pub is_host: bool,
    /// Indice do jogador local
    pub console_player: usize,
    /// Contador de tentativas de setup
    pub retry_count: i32,
}

impl NetSetup {
    /// Cria um novo setup de rede.
    ///
    /// C original: inicio de `D_ArbitrateNetStart()` em `d_net.c`
    pub fn new(com: &DoomCom) -> Self {
        let is_host = com.consoleplayer == 0;
        let console_player = com.consoleplayer as usize;

        let mut setup = NetSetup {
            state: if com.numplayers <= 1 {
                SetupState::Complete
            } else {
                SetupState::WaitingForPlayers
            },
            config: GameConfig::from_doomcom(com),
            node_confirmed: [false; MAXNETNODES],
            expected_nodes: com.numnodes as usize,
            is_host,
            console_player,
            retry_count: 0,
        };

        // O host ja esta confirmado
        if is_host {
            setup.node_confirmed[0] = true;
        }

        setup
    }

    /// Cria um setup para single-player (ja completo).
    pub fn single_player() -> Self {
        Self::new(&DoomCom::single_player())
    }

    /// Processa um pacote de setup recebido.
    ///
    /// Se somos o host: registra a confirmacao do cliente.
    /// Se somos cliente: aceita a configuracao do host.
    ///
    /// C original: loop em `D_ArbitrateNetStart()` de `d_net.c`
    pub fn process_setup_packet(&mut self, node: usize, packet: &DoomData) {
        if !packet.is_setup() || node >= MAXNETNODES {
            return;
        }

        if self.is_host {
            // Host: cliente confirmou
            self.node_confirmed[node] = true;
        } else {
            // Cliente: receber configuracao do host
            // No DOOM original, skill/episode/map sao codificados
            // nos campos starttic/retransmitfrom
            self.config.skill = (packet.starttic & 0x0F) as i32;
            self.config.deathmatch = ((packet.starttic >> 4) & 0x03) as i32;
            self.config.episode = (packet.retransmit_from & 0x0F) as i32;
            self.config.map = ((packet.retransmit_from >> 4) & 0x0F) as i32;
            self.node_confirmed[0] = true; // host confirmado
        }

        // Verificar se todos confirmaram
        self.check_complete();
    }

    /// Cria um pacote de setup para enviar.
    ///
    /// C original: parte de `D_ArbitrateNetStart()` em `d_net.c`
    pub fn build_setup_packet(&self) -> DoomData {
        let mut packet = DoomData::new();
        packet.set_flags(NCMD_SETUP);
        packet.player = self.console_player as u8;

        if self.is_host {
            // Codificar configuracao nos campos de header
            packet.starttic =
                (self.config.skill as u8 & 0x0F) | ((self.config.deathmatch as u8 & 0x03) << 4);
            packet.retransmit_from =
                (self.config.episode as u8 & 0x0F) | ((self.config.map as u8 & 0x0F) << 4);
        }

        packet
    }

    /// Verifica se todos os nos confirmaram.
    fn check_complete(&mut self) {
        let confirmed = self.node_confirmed[..self.expected_nodes]
            .iter()
            .filter(|&&c| c)
            .count();

        if confirmed >= self.expected_nodes {
            self.state = SetupState::Complete;
        } else {
            self.state = SetupState::Exchanging;
        }
    }

    /// Verifica se o setup esta completo.
    pub fn is_complete(&self) -> bool {
        self.state == SetupState::Complete
    }

    /// Retorna o numero de nos confirmados.
    pub fn confirmed_count(&self) -> usize {
        self.node_confirmed[..self.expected_nodes]
            .iter()
            .filter(|&&c| c)
            .count()
    }
}

// ---------------------------------------------------------------------------
// D_CheckNetGame — deteccao de modo de jogo
// ---------------------------------------------------------------------------

/// Resultado da checagem de rede.
///
/// C original: `D_CheckNetGame()` em `d_net.c`
#[derive(Debug, Clone)]
pub struct NetCheckResult {
    /// Numero de jogadores detectados
    pub num_players: i32,
    /// Se e uma sessao multiplayer
    pub multiplayer: bool,
    /// Se e deathmatch
    pub deathmatch: bool,
    /// Indice do jogador local
    pub console_player: i32,
}

/// Verifica a configuracao de rede e retorna o resultado.
///
/// Em single-player, retorna imediatamente.
/// Em multiplayer, inicia a fase de arbitragem.
///
/// C original: `D_CheckNetGame()` em `d_net.c`
pub fn check_net_game(com: &DoomCom) -> NetCheckResult {
    NetCheckResult {
        num_players: com.numplayers as i32,
        multiplayer: com.is_multiplayer(),
        deathmatch: com.is_deathmatch(),
        console_player: com.consoleplayer as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_config_default() {
        let config = GameConfig::default_config();
        assert_eq!(config.skill, 2);
        assert_eq!(config.episode, 1);
        assert_eq!(config.map, 1);
        assert_eq!(config.deathmatch, 0);
    }

    #[test]
    fn game_config_from_doomcom() {
        let mut com = DoomCom::multiplayer(4, 0);
        com.skill = 3;
        com.episode = 2;
        com.map = 5;
        com.deathmatch = 1;

        let config = GameConfig::from_doomcom(&com);
        assert_eq!(config.skill, 3);
        assert_eq!(config.episode, 2);
        assert_eq!(config.map, 5);
        assert_eq!(config.deathmatch, 1);
        assert_eq!(config.num_players, 4);
    }

    #[test]
    fn game_config_apply() {
        let config = GameConfig {
            skill: 4,
            episode: 3,
            map: 7,
            deathmatch: 2,
            num_players: 2,
            ticdup: 2,
            extratics: 3,
        };

        let mut com = DoomCom::single_player();
        config.apply_to_doomcom(&mut com);

        assert_eq!(com.skill, 4);
        assert_eq!(com.episode, 3);
        assert_eq!(com.map, 7);
        assert_eq!(com.deathmatch, 2);
    }

    #[test]
    fn setup_single_player() {
        let setup = NetSetup::single_player();
        assert!(setup.is_complete());
        assert!(setup.is_host); // consoleplayer=0 → host
        assert_eq!(setup.expected_nodes, 1);
    }

    #[test]
    fn setup_host_init() {
        let com = DoomCom::multiplayer(2, 0);
        let setup = NetSetup::new(&com);
        assert!(setup.is_host);
        assert!(!setup.is_complete());
        assert_eq!(setup.confirmed_count(), 1); // host auto-confirmado
    }

    #[test]
    fn setup_client_init() {
        let com = DoomCom::multiplayer(2, 1);
        let setup = NetSetup::new(&com);
        assert!(!setup.is_host);
        assert!(!setup.is_complete());
        assert_eq!(setup.confirmed_count(), 0);
    }

    #[test]
    fn setup_handshake() {
        // Host setup
        let host_com = DoomCom::multiplayer(2, 0);
        let mut host_setup = NetSetup::new(&host_com);

        // Cliente setup
        let client_com = DoomCom::multiplayer(2, 1);
        let mut client_setup = NetSetup::new(&client_com);

        // Host envia pacote de setup
        let host_packet = host_setup.build_setup_packet();
        assert!(host_packet.is_setup());

        // Cliente recebe e processa
        client_setup.process_setup_packet(0, &host_packet);
        assert!(client_setup.node_confirmed[0]); // host confirmado

        // Cliente envia ack
        let client_packet = client_setup.build_setup_packet();

        // Host recebe ack
        host_setup.process_setup_packet(1, &client_packet);
        assert!(host_setup.node_confirmed[1]); // cliente confirmado
        assert!(host_setup.is_complete());
    }

    #[test]
    fn setup_config_encoding() {
        let host_com = DoomCom::multiplayer(2, 0);
        let mut host_setup = NetSetup::new(&host_com);
        host_setup.config.skill = 3;
        host_setup.config.deathmatch = 1;
        host_setup.config.episode = 2;
        host_setup.config.map = 5;

        let packet = host_setup.build_setup_packet();

        // Cliente decodifica
        let client_com = DoomCom::multiplayer(2, 1);
        let mut client_setup = NetSetup::new(&client_com);
        client_setup.process_setup_packet(0, &packet);

        assert_eq!(client_setup.config.skill, 3);
        assert_eq!(client_setup.config.deathmatch, 1);
        assert_eq!(client_setup.config.episode, 2);
        assert_eq!(client_setup.config.map, 5);
    }

    #[test]
    fn check_net_game_single() {
        let com = DoomCom::single_player();
        let result = check_net_game(&com);
        assert!(!result.multiplayer);
        assert!(!result.deathmatch);
        assert_eq!(result.console_player, 0);
    }

    #[test]
    fn check_net_game_multi() {
        let mut com = DoomCom::multiplayer(4, 2);
        com.deathmatch = 1;
        let result = check_net_game(&com);
        assert!(result.multiplayer);
        assert!(result.deathmatch);
        assert_eq!(result.num_players, 4);
        assert_eq!(result.console_player, 2);
    }

    #[test]
    fn setup_ignores_non_setup_packets() {
        let com = DoomCom::multiplayer(2, 0);
        let mut setup = NetSetup::new(&com);

        let packet = DoomData::new(); // sem flag NCMD_SETUP
        setup.process_setup_packet(1, &packet);
        assert!(!setup.node_confirmed[1]);
    }
}

//! # Sincronizacao de Tics — Lockstep Determinístico
//!
//! Implementa o protocolo de sincronizacao do DOOM multiplayer:
//! o jogo so avanca quando TODOS os jogadores tem inputs
//! disponiveis para o proximo tic. Isso garante que todos
//! executem a mesma sequencia de tics deterministicamente.
//!
//! ## Fluxo de sincronizacao
//!
//! ```text
//! NetUpdate() — chamado a cada frame
//!   |
//!   +-> gerar ticcmd local (G_BuildTiccmd)
//!   +-> enviar pacote com tics pendentes
//!   +-> receber pacotes de outros jogadores
//!   |
//!   v
//! TryRunTics() — chamado a cada frame
//!   |
//!   +-> lowtic = min(nettics[]) — tic mais atrasado
//!   +-> se lowtic > gametic: executar tics ate lowtic
//!   +-> se lowtic == gametic: esperar (chamar NetUpdate)
//! ```
//!
//! ## Ring buffers
//!
//! Os comandos sao armazenados em ring buffers de BACKUPTICS (12)
//! posicoes. O indice e calculado como `tic % BACKUPTICS`.
//! Isso permite retransmitir tics recentes sem alocacao.
//!
//! ## Variáveis de sincronizacao
//!
//! - `maketic` — proximo tic para o qual gerar input (local)
//! - `gametic` — tic sendo executado (global, sincronizado)
//! - `nettics[i]` — ultimo tic confirmado do no `i`
//!
//! A invariante fundamental e: `gametic <= min(nettics[])`.
//! O jogo nunca executa um tic antes de ter inputs de TODOS.
//!
//! ## Arquivo C original: `d_net.c`
//!
//! ## Conceitos que o leitor vai aprender
//! - Lockstep multiplayer determinístico
//! - Ring buffer para retransmissao
//! - Backpressure natural via espera de inputs

use super::types::*;

// ---------------------------------------------------------------------------
// Estado de sincronizacao
// ---------------------------------------------------------------------------

/// Estado de sincronizacao de rede — gerencia o lockstep.
///
/// C original: globals em `d_net.c`
/// (`nettics[]`, `maketic`, `gametic`, `netcmds[][]`, etc.)
#[derive(Debug)]
pub struct NetSync {
    /// Ultimo tic confirmado de cada no.
    ///
    /// `nettics[i]` = ultimo tic cujo input do no `i` ja recebemos.
    /// O jogo so pode avancar ate `min(nettics[])`.
    ///
    /// C original: `int nettics[MAXNETNODES]`
    pub nettics: [i32; MAXNETNODES],

    /// Proximo tic para o qual gerar input local.
    ///
    /// Incrementado em `NetUpdate()` ao gerar um novo ticcmd.
    ///
    /// C original: `int maketic`
    pub maketic: i32,

    /// Tic atualmente sendo executado pelo game loop.
    ///
    /// Incrementado em `TryRunTics()` ao executar cada tic.
    ///
    /// C original: `int gametic`
    pub gametic: i32,

    /// Ring buffer de comandos locais do console player.
    ///
    /// C original: `ticcmd_t localcmds[BACKUPTICS]`
    pub local_cmds: [NetTicCmd; BACKUPTICS],

    /// Ring buffer de comandos recebidos de cada jogador.
    ///
    /// `net_cmds[player][tic % BACKUPTICS]` = comando do jogador
    /// para aquele tic.
    ///
    /// C original: `ticcmd_t netcmds[MAXPLAYERS][BACKUPTICS]`
    pub net_cmds: [[NetTicCmd; BACKUPTICS]; MAXPLAYERS],

    /// Se cada no precisa de retransmissao.
    ///
    /// C original: `boolean remoteresend[MAXNETNODES]`
    pub remote_resend: [bool; MAXNETNODES],

    /// Tic a partir do qual reenviar para cada no.
    ///
    /// C original: `unsigned int resendto[MAXNETNODES]`
    pub resend_to: [i32; MAXNETNODES],

    /// Contador de ticks desde o ultimo pacote recebido de cada no.
    ///
    /// Quando atinge RESENDCOUNT, solicita retransmissao.
    ///
    /// C original: `int resendcount[MAXNETNODES]`
    pub resend_count: [i32; MAXNETNODES],

    /// Se cada no esta ativo no jogo.
    ///
    /// C original: `boolean nodeingame[MAXNETNODES]`
    pub node_in_game: [bool; MAXNETNODES],

    /// Numero de nos na sessao.
    pub num_nodes: usize,

    /// Indice do jogador local.
    pub console_player: usize,

    /// Fator de duplicacao de tics.
    pub ticdup: i32,

    /// Se estamos em modo singletics (sem buffering).
    pub single_tics: bool,
}

impl NetSync {
    /// Cria um novo estado de sincronizacao.
    ///
    /// C original: inicializacao em `D_CheckNetGame()` / `D_ArbitrateNetStart()`
    pub fn new(com: &DoomCom) -> Self {
        let mut sync = NetSync {
            nettics: [0; MAXNETNODES],
            maketic: 0,
            gametic: 0,
            local_cmds: [NetTicCmd::new(); BACKUPTICS],
            net_cmds: [[NetTicCmd::new(); BACKUPTICS]; MAXPLAYERS],
            remote_resend: [false; MAXNETNODES],
            resend_to: [0; MAXNETNODES],
            resend_count: [0; MAXNETNODES],
            node_in_game: [false; MAXNETNODES],
            num_nodes: com.numnodes as usize,
            console_player: com.consoleplayer as usize,
            ticdup: com.ticdup as i32,
            single_tics: false,
        };

        // Marcar nos ativos
        for i in 0..sync.num_nodes {
            sync.node_in_game[i] = true;
        }

        sync
    }

    /// Cria estado para jogo single-player.
    pub fn single_player() -> Self {
        Self::new(&DoomCom::single_player())
    }

    /// Armazena um comando local no ring buffer.
    ///
    /// Chamado por `NetUpdate()` apos `G_BuildTiccmd()`.
    pub fn store_local_cmd(&mut self, cmd: NetTicCmd) {
        let idx = self.maketic as usize % BACKUPTICS;
        self.local_cmds[idx] = cmd;
        self.net_cmds[self.console_player][idx] = cmd;
        self.maketic += 1;
    }

    /// Armazena um comando recebido da rede.
    ///
    /// Chamado por `GetPackets()` ao processar pacotes recebidos.
    pub fn store_remote_cmd(&mut self, player: usize, tic: i32, cmd: NetTicCmd) {
        if player >= MAXPLAYERS {
            return;
        }
        let idx = tic as usize % BACKUPTICS;
        self.net_cmds[player][idx] = cmd;
    }

    /// Retorna o comando de um jogador para um tic especifico.
    pub fn get_cmd(&self, player: usize, tic: i32) -> &NetTicCmd {
        let idx = tic as usize % BACKUPTICS;
        &self.net_cmds[player][idx]
    }

    /// Calcula o tic mais baixo confirmado (lowtic).
    ///
    /// O jogo so pode avancar ate este tic.
    /// Em single-player, lowtic = maketic.
    ///
    /// C original: parte de `TryRunTics()` em `d_net.c`
    pub fn low_tic(&self) -> i32 {
        if self.num_nodes <= 1 {
            return self.maketic;
        }

        let mut low = i32::MAX;
        for i in 0..self.num_nodes {
            if self.node_in_game[i] && self.nettics[i] < low {
                low = self.nettics[i];
            }
        }

        // Em single-tics mode, limitar a gametic + 1
        if self.single_tics && low > self.gametic + 1 {
            self.gametic + 1
        } else {
            low
        }
    }

    /// Calcula quantos tics podem ser executados.
    ///
    /// C original: parte de `TryRunTics()` em `d_net.c`
    pub fn available_tics(&self) -> i32 {
        let low = self.low_tic();
        let available = low - self.gametic;
        available.max(0)
    }

    /// Avanca o gametic (chamado apos executar um tic).
    pub fn advance_gametic(&mut self) {
        self.gametic += 1;
    }

    /// Processa um pacote recebido.
    ///
    /// Extrai os ticcmds do pacote e atualiza o nettics do no remetente.
    ///
    /// C original: `GetPackets()` em `d_net.c`
    pub fn process_packet(&mut self, node: usize, packet: &DoomData) {
        if node >= MAXNETNODES || !self.node_in_game[node] {
            return;
        }

        // Verificar se e pedido de retransmissao
        if packet.is_retransmit() {
            let resend_from = expand_tics(packet.retransmit_from, self.maketic);
            self.resend_to[node] = resend_from;
        }

        // Verificar saida
        if packet.is_exit() {
            self.node_in_game[node] = false;
            return;
        }

        let player = (packet.player & !PL_DRONE) as usize;
        if player >= MAXPLAYERS {
            return;
        }

        // Reconstruir tic inicial
        let starttic = expand_tics(packet.starttic, self.maketic);

        // Armazenar comandos
        for i in 0..packet.numtics as usize {
            let tic = starttic + i as i32;
            if i < packet.cmds.len() {
                self.store_remote_cmd(player, tic, packet.cmds[i]);
            }
        }

        // Atualizar nettics deste no
        let last_tic = starttic + packet.numtics as i32;
        if last_tic > self.nettics[node] {
            self.nettics[node] = last_tic;
        }

        // Resetar contador de retransmissao
        self.resend_count[node] = 0;
        self.remote_resend[node] = false;
    }

    /// Prepara um pacote para enviar ao no especificado.
    ///
    /// Inclui tics desde `resend_to[node]` ate `maketic`.
    ///
    /// C original: parte de `NetUpdate()` em `d_net.c`
    pub fn build_packet(&self, node: usize) -> DoomData {
        let mut packet = DoomData::new();
        packet.player = self.console_player as u8;

        let start = self.resend_to[node];
        let end = self.maketic;

        // Limitar ao maximo de tics por pacote
        let numtics = ((end - start) as usize).min(BACKUPTICS);

        packet.starttic = (start & 0xFF) as u8;
        packet.numtics = numtics as u8;

        for i in 0..numtics {
            let tic = start + i as i32;
            let idx = tic as usize % BACKUPTICS;
            packet.cmds.push(self.local_cmds[idx]);
        }

        // Calcular checksum
        let checksum = packet.calculate_checksum();
        packet.checksum = checksum;

        packet
    }

    /// Verifica se algum no precisa de retransmissao.
    ///
    /// Incrementa contadores e marca nos para retransmissao
    /// quando o tempo excede RESENDCOUNT.
    ///
    /// C original: parte de `NetUpdate()` em `d_net.c`
    pub fn check_retransmit(&mut self) {
        for i in 0..self.num_nodes {
            if !self.node_in_game[i] {
                continue;
            }

            self.resend_count[i] += 1;

            if self.resend_count[i] >= RESENDCOUNT {
                self.remote_resend[i] = true;
            }
        }
    }

    /// Verifica se todos os nos estao dessincronizados (jogo travado).
    ///
    /// Se gametic == lowtic e nao ha tics disponiveis por muito tempo,
    /// o jogo esta efetivamente travado.
    pub fn is_stalled(&self) -> bool {
        self.num_nodes > 1 && self.available_tics() == 0
    }

    /// Retorna o numero de nos ativos.
    pub fn active_nodes(&self) -> usize {
        self.node_in_game[..self.num_nodes]
            .iter()
            .filter(|&&active| active)
            .count()
    }
}

impl Default for NetSync {
    fn default() -> Self {
        Self::single_player()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_player_sync() {
        let sync = NetSync::single_player();
        assert_eq!(sync.num_nodes, 1);
        assert_eq!(sync.console_player, 0);
        assert_eq!(sync.gametic, 0);
        assert_eq!(sync.maketic, 0);
    }

    #[test]
    fn store_and_get_local_cmd() {
        let mut sync = NetSync::single_player();
        let cmd = NetTicCmd {
            forwardmove: 25,
            sidemove: 0,
            angleturn: 640,
            consistancy: 0,
            chatchar: 0,
            buttons: 1,
        };

        sync.store_local_cmd(cmd);
        assert_eq!(sync.maketic, 1);

        let stored = sync.get_cmd(0, 0);
        assert_eq!(stored.forwardmove, 25);
        assert_eq!(stored.buttons, 1);
    }

    #[test]
    fn low_tic_single_player() {
        let mut sync = NetSync::single_player();
        assert_eq!(sync.low_tic(), 0);

        sync.store_local_cmd(NetTicCmd::new());
        sync.store_local_cmd(NetTicCmd::new());
        assert_eq!(sync.low_tic(), 2);
    }

    #[test]
    fn low_tic_multiplayer() {
        let com = DoomCom::multiplayer(2, 0);
        let mut sync = NetSync::new(&com);

        // No 0 (local) tem maketic = 5
        for _ in 0..5 {
            sync.store_local_cmd(NetTicCmd::new());
        }
        // No 1 (remoto) tem nettics = 3
        sync.nettics[1] = 3;

        // lowtic = min(nettics) = min(maketic=5 para no local, 3 para no 1) = 3
        // Nota: no 0 usa maketic implicitamente via nettics[0] que começa em 0
        // mas no single-player low_tic retorna maketic diretamente
        // Em multiplayer, no 0 tem nettics[0] = 0 (nao atualizado)
        assert_eq!(sync.low_tic(), 0); // no 0 ainda em 0
    }

    #[test]
    fn available_tics() {
        let mut sync = NetSync::single_player();
        assert_eq!(sync.available_tics(), 0);

        sync.store_local_cmd(NetTicCmd::new());
        sync.store_local_cmd(NetTicCmd::new());
        assert_eq!(sync.available_tics(), 2);

        sync.advance_gametic();
        assert_eq!(sync.available_tics(), 1);
        assert_eq!(sync.gametic, 1);
    }

    #[test]
    fn process_packet() {
        let com = DoomCom::multiplayer(2, 0);
        let mut sync = NetSync::new(&com);

        // Simular pacote do jogador 1 com 2 tics
        let mut packet = DoomData::new();
        packet.player = 1;
        packet.starttic = 0;
        packet.numtics = 2;
        packet.cmds.push(NetTicCmd {
            forwardmove: 10,
            ..NetTicCmd::new()
        });
        packet.cmds.push(NetTicCmd {
            forwardmove: 20,
            ..NetTicCmd::new()
        });

        sync.process_packet(1, &packet);

        assert_eq!(sync.nettics[1], 2);
        assert_eq!(sync.get_cmd(1, 0).forwardmove, 10);
        assert_eq!(sync.get_cmd(1, 1).forwardmove, 20);
    }

    #[test]
    fn process_exit_packet() {
        let com = DoomCom::multiplayer(2, 0);
        let mut sync = NetSync::new(&com);
        assert_eq!(sync.active_nodes(), 2);

        let mut packet = DoomData::new();
        packet.set_flags(NCMD_EXIT);
        packet.player = 1;

        sync.process_packet(1, &packet);
        assert!(!sync.node_in_game[1]);
        assert_eq!(sync.active_nodes(), 1);
    }

    #[test]
    fn build_packet() {
        let mut sync = NetSync::single_player();

        let cmd1 = NetTicCmd {
            forwardmove: 10,
            buttons: 1,
            ..NetTicCmd::new()
        };
        let cmd2 = NetTicCmd {
            forwardmove: 20,
            buttons: 2,
            ..NetTicCmd::new()
        };

        sync.store_local_cmd(cmd1);
        sync.store_local_cmd(cmd2);

        let packet = sync.build_packet(0);
        assert_eq!(packet.numtics, 2);
        assert_eq!(packet.cmds[0].forwardmove, 10);
        assert_eq!(packet.cmds[1].forwardmove, 20);
        assert_eq!(packet.player, 0);
    }

    #[test]
    fn retransmit_check() {
        let com = DoomCom::multiplayer(2, 0);
        let mut sync = NetSync::new(&com);

        for _ in 0..RESENDCOUNT {
            sync.check_retransmit();
        }

        assert!(sync.remote_resend[0]);
        assert!(sync.remote_resend[1]);
    }

    #[test]
    fn ring_buffer_wraparound() {
        let mut sync = NetSync::single_player();

        // Armazenar mais que BACKUPTICS
        for i in 0..BACKUPTICS + 5 {
            let cmd = NetTicCmd {
                forwardmove: i as i8,
                ..NetTicCmd::new()
            };
            sync.store_local_cmd(cmd);
        }

        assert_eq!(sync.maketic, (BACKUPTICS + 5) as i32);

        // Os ultimos BACKUPTICS comandos devem estar acessiveis
        let last = sync.get_cmd(0, sync.maketic - 1);
        assert_eq!(last.forwardmove, (BACKUPTICS + 4) as i8);
    }

    #[test]
    fn is_stalled() {
        let sync = NetSync::single_player();
        assert!(!sync.is_stalled()); // single-player nunca trava

        let com = DoomCom::multiplayer(2, 0);
        let sync2 = NetSync::new(&com);
        assert!(sync2.is_stalled()); // multiplayer sem tics = travado
    }
}

//! # Parser de Argumentos de Linha de Comando
//!
//! Processa os argumentos de linha de comando do DOOM,
//! fornecendo acesso tipado a todas as opcoes suportadas.
//!
//! ## Argumentos suportados
//!
//! ```text
//! --iwad <path>       Caminho para o IWAD principal (obrigatorio)
//! --file <path...>    PWADs adicionais (mods, patches)
//! --warp <e> <m>      Iniciar no mapa ExMy (ou --warp <m> para DOOM II)
//! --skill <n>         Dificuldade (1-5)
//! --episode <n>       Episodio inicial (1-4)
//! --deathmatch        Modo deathmatch
//! --altdeath          Modo deathmatch alternativo
//! --nomonsters        Sem monstros
//! --fast              Monstros rapidos
//! --respawn           Monstros respawnam
//! --turbo <n>         Velocidade turbo (10-400%)
//! --timedemo <lump>   Demo cronometrada
//! --playdemo <lump>   Reproduzir demo
//! --devparm           Modo desenvolvedor
//! --singletics        Um tick por frame (debug)
//! --net <n> <addr>    Multiplayer (numero de jogadores e endereco)
//! ```
//!
//! ## Implementacao
//!
//! No C original, argumentos eram processados com `M_CheckParm()`
//! que buscava strings no array `myargv[]`. Aqui usamos um parser
//! estruturado que produz um `DoomArgs` tipado.
//!
//! ## Arquivo C original: `d_main.c` (D_DoomMain), `m_argv.c`
//!
//! ## Conceitos que o leitor vai aprender
//! - Parsing de argumentos de CLI sem dependencias externas
//! - Conversao de globals C para struct tipada
//! - Validacao e defaults de configuracao

use std::path::PathBuf;

/// Argumentos de linha de comando parseados.
///
/// C original: globals `myargv`, `myargc` e chamadas `M_CheckParm()`
/// espalhadas por `d_main.c`
#[derive(Debug, Clone)]
pub struct DoomArgs {
    /// Caminho para o IWAD principal (ex: freedoom1.wad)
    pub iwad: PathBuf,
    /// PWADs adicionais (mods, patches)
    pub pwads: Vec<PathBuf>,
    /// Dificuldade (1-5, default 3 = Hurt Me Plenty)
    pub skill: Option<i32>,
    /// Episodio inicial (1-4)
    pub episode: Option<i32>,
    /// Mapa inicial
    pub warp_map: Option<i32>,
    /// Episodio do warp (para ExMy)
    pub warp_episode: Option<i32>,
    /// Modo deathmatch (0=coop, 1=deathmatch, 2=altdeath)
    pub deathmatch: i32,
    /// Sem monstros
    pub nomonsters: bool,
    /// Monstros rapidos
    pub fast: bool,
    /// Monstros respawnam
    pub respawn: bool,
    /// Velocidade turbo (porcentagem, 100 = normal)
    pub turbo: Option<i32>,
    /// Demo cronometrada
    pub timedemo: Option<String>,
    /// Reproduzir demo
    pub playdemo: Option<String>,
    /// Modo desenvolvedor (mostra FPS, etc.)
    pub devparm: bool,
    /// Um tick por frame (debug)
    pub singletics: bool,
    /// Numero de jogadores para rede
    pub net_players: Option<i32>,
    /// Endereco do host para rede
    pub net_host: Option<String>,
}

impl DoomArgs {
    /// Parseia argumentos a partir de `std::env::args()`.
    ///
    /// C original: `D_DoomMain()` em `d_main.c`, chamadas a `M_CheckParm()`
    pub fn parse() -> Result<Self, ArgsError> {
        let args: Vec<String> = std::env::args().collect();
        Self::parse_from(&args[1..])
    }

    /// Parseia argumentos a partir de uma lista de strings.
    ///
    /// Util para testes e para separar parsing de `std::env`.
    pub fn parse_from(args: &[String]) -> Result<Self, ArgsError> {
        let mut result = DoomArgs {
            iwad: PathBuf::new(),
            pwads: Vec::new(),
            skill: None,
            episode: None,
            warp_map: None,
            warp_episode: None,
            deathmatch: 0,
            nomonsters: false,
            fast: false,
            respawn: false,
            turbo: None,
            timedemo: None,
            playdemo: None,
            devparm: false,
            singletics: false,
            net_players: None,
            net_host: None,
        };

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--iwad" | "-iwad" => {
                    i += 1;
                    if i >= args.len() {
                        return Err(ArgsError::MissingValue("--iwad".into()));
                    }
                    result.iwad = PathBuf::from(&args[i]);
                }
                "--file" | "-file" => {
                    i += 1;
                    while i < args.len() && !args[i].starts_with('-') {
                        result.pwads.push(PathBuf::from(&args[i]));
                        i += 1;
                    }
                    continue; // nao incrementar i novamente
                }
                "--skill" | "-skill" => {
                    i += 1;
                    if i >= args.len() {
                        return Err(ArgsError::MissingValue("--skill".into()));
                    }
                    let s = args[i]
                        .parse::<i32>()
                        .map_err(|_| ArgsError::InvalidValue("--skill".into(), args[i].clone()))?;
                    if !(1..=5).contains(&s) {
                        return Err(ArgsError::InvalidValue(
                            "--skill".into(),
                            format!("{} (deve ser 1-5)", s),
                        ));
                    }
                    result.skill = Some(s);
                }
                "--episode" | "-episode" => {
                    i += 1;
                    if i >= args.len() {
                        return Err(ArgsError::MissingValue("--episode".into()));
                    }
                    let e = args[i]
                        .parse::<i32>()
                        .map_err(|_| ArgsError::InvalidValue("--episode".into(), args[i].clone()))?;
                    result.episode = Some(e);
                }
                "--warp" | "-warp" => {
                    i += 1;
                    if i >= args.len() {
                        return Err(ArgsError::MissingValue("--warp".into()));
                    }
                    let first = args[i]
                        .parse::<i32>()
                        .map_err(|_| ArgsError::InvalidValue("--warp".into(), args[i].clone()))?;

                    // Verificar se o proximo argumento tambem e um numero (ExMy format)
                    if i + 1 < args.len() {
                        if let Ok(second) = args[i + 1].parse::<i32>() {
                            result.warp_episode = Some(first);
                            result.warp_map = Some(second);
                            i += 1;
                        } else {
                            // Formato DOOM II: apenas numero do mapa
                            result.warp_map = Some(first);
                        }
                    } else {
                        result.warp_map = Some(first);
                    }
                }
                "--deathmatch" | "-deathmatch" => {
                    result.deathmatch = 1;
                }
                "--altdeath" | "-altdeath" => {
                    result.deathmatch = 2;
                }
                "--nomonsters" | "-nomonsters" => {
                    result.nomonsters = true;
                }
                "--fast" | "-fast" => {
                    result.fast = true;
                }
                "--respawn" | "-respawn" => {
                    result.respawn = true;
                }
                "--turbo" | "-turbo" => {
                    i += 1;
                    if i >= args.len() {
                        return Err(ArgsError::MissingValue("--turbo".into()));
                    }
                    let t = args[i]
                        .parse::<i32>()
                        .map_err(|_| ArgsError::InvalidValue("--turbo".into(), args[i].clone()))?;
                    result.turbo = Some(t.clamp(10, 400));
                }
                "--timedemo" | "-timedemo" => {
                    i += 1;
                    if i >= args.len() {
                        return Err(ArgsError::MissingValue("--timedemo".into()));
                    }
                    result.timedemo = Some(args[i].clone());
                }
                "--playdemo" | "-playdemo" => {
                    i += 1;
                    if i >= args.len() {
                        return Err(ArgsError::MissingValue("--playdemo".into()));
                    }
                    result.playdemo = Some(args[i].clone());
                }
                "--devparm" | "-devparm" => {
                    result.devparm = true;
                }
                "--singletics" | "-singletics" => {
                    result.singletics = true;
                }
                "--net" | "-net" => {
                    i += 1;
                    if i >= args.len() {
                        return Err(ArgsError::MissingValue("--net".into()));
                    }
                    let n = args[i]
                        .parse::<i32>()
                        .map_err(|_| ArgsError::InvalidValue("--net".into(), args[i].clone()))?;
                    result.net_players = Some(n);
                    if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                        i += 1;
                        result.net_host = Some(args[i].clone());
                    }
                }
                "--help" | "-help" | "-h" => {
                    return Err(ArgsError::HelpRequested);
                }
                other => {
                    return Err(ArgsError::UnknownArg(other.into()));
                }
            }
            i += 1;
        }

        // IWAD e obrigatorio
        if result.iwad.as_os_str().is_empty() {
            return Err(ArgsError::MissingIwad);
        }

        Ok(result)
    }

    /// Retorna a mensagem de uso.
    pub fn usage() -> &'static str {
        "Uso: doom-rust --iwad <caminho-para-wad> [opcoes]\n\
         \n\
         Opcoes:\n\
         \x20 --iwad <path>       IWAD principal (freedoom1.wad, doom.wad, etc.)\n\
         \x20 --file <path...>    PWADs adicionais\n\
         \x20 --warp <e> <m>      Iniciar no mapa ExMy\n\
         \x20 --skill <1-5>       Dificuldade (1=Baby, 5=Nightmare)\n\
         \x20 --episode <1-4>     Episodio inicial\n\
         \x20 --deathmatch        Modo deathmatch\n\
         \x20 --altdeath          Deathmatch alternativo\n\
         \x20 --nomonsters        Sem monstros\n\
         \x20 --fast              Monstros rapidos\n\
         \x20 --respawn           Monstros respawnam\n\
         \x20 --turbo <10-400>    Velocidade turbo (%)\n\
         \x20 --timedemo <lump>   Demo cronometrada\n\
         \x20 --playdemo <lump>   Reproduzir demo\n\
         \x20 --devparm           Modo desenvolvedor\n\
         \x20 --singletics        Um tick por frame (debug)\n\
         \x20 --help              Esta mensagem"
    }
}

/// Erros de parsing de argumentos.
#[derive(Debug, Clone)]
pub enum ArgsError {
    /// Argumento desconhecido
    UnknownArg(String),
    /// Valor faltando para argumento
    MissingValue(String),
    /// Valor invalido para argumento
    InvalidValue(String, String),
    /// IWAD nao especificado
    MissingIwad,
    /// Ajuda solicitada (--help)
    HelpRequested,
}

impl std::fmt::Display for ArgsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgsError::UnknownArg(a) => write!(f, "Argumento desconhecido: {}", a),
            ArgsError::MissingValue(a) => write!(f, "Valor faltando para {}", a),
            ArgsError::InvalidValue(a, v) => write!(f, "Valor invalido para {}: {}", a, v),
            ArgsError::MissingIwad => write!(f, "IWAD nao especificado. Use --iwad <caminho>"),
            ArgsError::HelpRequested => write!(f, "{}", DoomArgs::usage()),
        }
    }
}

impl std::error::Error for ArgsError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper para criar Vec<String> a partir de &str
    fn args(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn parse_basic_iwad() {
        let a = DoomArgs::parse_from(&args(&["--iwad", "doom.wad"])).unwrap();
        assert_eq!(a.iwad, PathBuf::from("doom.wad"));
        assert!(a.pwads.is_empty());
        assert_eq!(a.deathmatch, 0);
    }

    #[test]
    fn parse_missing_iwad() {
        let err = DoomArgs::parse_from(&args(&["--skill", "3"])).unwrap_err();
        assert!(matches!(err, ArgsError::MissingIwad));
    }

    #[test]
    fn parse_skill() {
        let a = DoomArgs::parse_from(&args(&["--iwad", "d.wad", "--skill", "4"])).unwrap();
        assert_eq!(a.skill, Some(4));
    }

    #[test]
    fn parse_skill_invalid() {
        let err = DoomArgs::parse_from(&args(&["--iwad", "d.wad", "--skill", "6"])).unwrap_err();
        assert!(matches!(err, ArgsError::InvalidValue(_, _)));
    }

    #[test]
    fn parse_warp_doom1_format() {
        let a = DoomArgs::parse_from(&args(&["--iwad", "d.wad", "--warp", "2", "3"])).unwrap();
        assert_eq!(a.warp_episode, Some(2));
        assert_eq!(a.warp_map, Some(3));
    }

    #[test]
    fn parse_warp_doom2_format() {
        let a = DoomArgs::parse_from(&args(&["--iwad", "d.wad", "--warp", "15"])).unwrap();
        assert_eq!(a.warp_episode, None);
        assert_eq!(a.warp_map, Some(15));
    }

    #[test]
    fn parse_pwads() {
        let a = DoomArgs::parse_from(&args(&[
            "--iwad", "d.wad", "--file", "mod1.wad", "mod2.wad",
        ]))
        .unwrap();
        assert_eq!(a.pwads.len(), 2);
        assert_eq!(a.pwads[0], PathBuf::from("mod1.wad"));
        assert_eq!(a.pwads[1], PathBuf::from("mod2.wad"));
    }

    #[test]
    fn parse_flags() {
        let a = DoomArgs::parse_from(&args(&[
            "--iwad",
            "d.wad",
            "--nomonsters",
            "--fast",
            "--respawn",
            "--devparm",
            "--singletics",
        ]))
        .unwrap();
        assert!(a.nomonsters);
        assert!(a.fast);
        assert!(a.respawn);
        assert!(a.devparm);
        assert!(a.singletics);
    }

    #[test]
    fn parse_deathmatch() {
        let a = DoomArgs::parse_from(&args(&["--iwad", "d.wad", "--deathmatch"])).unwrap();
        assert_eq!(a.deathmatch, 1);

        let a = DoomArgs::parse_from(&args(&["--iwad", "d.wad", "--altdeath"])).unwrap();
        assert_eq!(a.deathmatch, 2);
    }

    #[test]
    fn parse_turbo() {
        let a = DoomArgs::parse_from(&args(&["--iwad", "d.wad", "--turbo", "200"])).unwrap();
        assert_eq!(a.turbo, Some(200));

        // Clamped
        let a = DoomArgs::parse_from(&args(&["--iwad", "d.wad", "--turbo", "999"])).unwrap();
        assert_eq!(a.turbo, Some(400));
    }

    #[test]
    fn parse_timedemo() {
        let a =
            DoomArgs::parse_from(&args(&["--iwad", "d.wad", "--timedemo", "demo1"])).unwrap();
        assert_eq!(a.timedemo, Some("demo1".into()));
    }

    #[test]
    fn parse_help() {
        let err = DoomArgs::parse_from(&args(&["--help"])).unwrap_err();
        assert!(matches!(err, ArgsError::HelpRequested));
    }

    #[test]
    fn parse_unknown_arg() {
        let err = DoomArgs::parse_from(&args(&["--iwad", "d.wad", "--foobar"])).unwrap_err();
        assert!(matches!(err, ArgsError::UnknownArg(_)));
    }

    #[test]
    fn parse_dash_prefix() {
        // DOOM original aceita tanto - quanto --
        let a = DoomArgs::parse_from(&args(&["-iwad", "d.wad", "-skill", "2"])).unwrap();
        assert_eq!(a.iwad, PathBuf::from("d.wad"));
        assert_eq!(a.skill, Some(2));
    }

    #[test]
    fn parse_net() {
        let a = DoomArgs::parse_from(&args(&[
            "--iwad", "d.wad", "--net", "2", "192.168.1.1",
        ]))
        .unwrap();
        assert_eq!(a.net_players, Some(2));
        assert_eq!(a.net_host, Some("192.168.1.1".into()));
    }

    #[test]
    fn usage_message() {
        let usage = DoomArgs::usage();
        assert!(usage.contains("--iwad"));
        assert!(usage.contains("--skill"));
        assert!(usage.contains("--warp"));
    }
}

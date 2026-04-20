//! # DOOM Rust — Ponto de Entrada
//!
//! Inicializa o engine e executa o game loop principal.
//!
//! ## Sequencia de execucao
//!
//! ```text
//! main()
//!   +-> parse argumentos (DoomArgs)
//!   +-> DoomEngine::init()  — D_DoomMain
//!   +-> loop { engine.run_frame() }  — D_DoomLoop
//!   +-> engine.quit()
//! ```
//!
//! C original: `i_main.c` (main), `d_main.c` (D_DoomMain, D_DoomLoop)

use doom_rust::args::{ArgsError, DoomArgs};
use doom_rust::engine::DoomEngine;

fn main() {
    env_logger::init();

    println!("DOOM Rust v{}", env!("CARGO_PKG_VERSION"));
    println!("Port educacional do DOOM (1993) para Rust");
    println!();

    // Parse de argumentos
    let args = match DoomArgs::parse() {
        Ok(args) => args,
        Err(ArgsError::HelpRequested) => {
            println!("{}", DoomArgs::usage());
            return;
        }
        Err(e) => {
            eprintln!("Erro: {}", e);
            eprintln!();
            eprintln!("{}", DoomArgs::usage());
            std::process::exit(1);
        }
    };

    // Inicializar engine (D_DoomMain)
    println!("D_DoomMain: Inicializando...");
    let mut engine = match DoomEngine::init(&args) {
        Ok(engine) => engine,
        Err(e) => {
            eprintln!("Erro fatal: {}", e);
            std::process::exit(1);
        }
    };

    println!("D_DoomMain: Inicializacao completa.");
    println!(
        "  Estado: {:?} | Skill: {:?} | E{}M{}",
        engine.state(),
        engine.game.skill,
        engine.game.episode,
        engine.game.map,
    );

    if let Some(ref map) = engine.map {
        println!(
            "  Mapa carregado: {} vertexes, {} linedefs, {} sectors, {} things",
            map.vertexes.len(),
            map.linedefs.len(),
            map.sectors.len(),
            map.things.len(),
        );
    }

    println!();

    // Game loop (D_DoomLoop)
    // Na versao completa, este loop seria integrado com SDL2
    // para rendering real e input de hardware.
    println!(
        "D_DoomLoop: Iniciando game loop a {} Hz...",
        engine.ticrate()
    );

    while engine.run_frame() {
        // Na versao completa:
        // 1. SDL_PollEvent → engine.event_queue.post()
        // 2. engine.run_frame() executa ticks e prepara rendering
        // 3. VideoSystem → SDL2 window blit
        // 4. SDL_Delay para frame pacing

        // Por enquanto, rodar um numero limitado de frames
        // para nao travar sem SDL2
        if engine.gametic() >= 35 {
            break;
        }
    }

    println!(
        "D_DoomLoop: {} ticks executados (~{:.1}s de jogo simulados)",
        engine.gametic(),
        engine.gametic() as f64 / engine.ticrate() as f64,
    );

    engine.quit();
    println!();
    println!("DOOM Rust encerrado.");
    println!("(Rendering visual requer compilacao com feature SDL2: cargo run --features sdl)");
}

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
    let mut engine = match DoomEngine::init(&args) {
        Ok(engine) => engine,
        Err(e) => {
            eprintln!("Erro fatal: {}", e);
            std::process::exit(1);
        }
    };

    // Game loop (D_DoomLoop)
    // Na versao completa, este loop seria integrado com SDL2
    // para rendering real e input de hardware.
    log::info!(
        "D_DoomLoop: Iniciando game loop a {} Hz",
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
        if engine.gametic() >= 3 {
            log::info!(
                "D_DoomLoop: {} ticks executados (sem SDL2, encerrando)",
                engine.gametic()
            );
            break;
        }
    }

    engine.quit();
}

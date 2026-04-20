//! # DOOM Rust — Ponto de Entrada
//!
//! Inicializa o engine e executa o game loop principal.
//! Com a feature `sdl`, abre uma janela grafica real.
//! Sem ela, roda em modo headless (simulacao sem video).
//!
//! ## Sequencia de execucao
//!
//! ```text
//! main()
//!   +-> parse argumentos (DoomArgs)
//!   +-> DoomEngine::init()    — D_DoomMain
//!   +-> SdlWindow::new()      — I_InitGraphics (se feature sdl)
//!   +-> loop {
//!   |     pump_events()       — I_StartTic
//!   |     engine.run_frame()  — D_DoomLoop
//!   |     finish_update()     — I_FinishUpdate
//!   |   }
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

    // Escolher modo de execucao: SDL (grafico) ou headless
    #[cfg(feature = "sdl")]
    {
        run_sdl(&mut engine);
    }

    #[cfg(not(feature = "sdl"))]
    {
        run_headless(&mut engine);
    }

    engine.quit();
    println!();
    println!("DOOM Rust encerrado.");
}

/// Game loop com janela SDL2 (modo grafico).
///
/// Abre janela 640x400, converte framebuffer via paleta PLAYPAL,
/// e processa input real do teclado/mouse.
#[cfg(feature = "sdl")]
fn run_sdl(engine: &mut DoomEngine) {
    use doom_rust::video::sdl::{load_palette, SdlWindow};

    println!("I_InitGraphics: Abrindo janela SDL2 (640x400)...");

    let mut window = match SdlWindow::new("DOOM Rust") {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Erro ao inicializar SDL2: {}", e);
            eprintln!("Executando em modo headless...");
            run_headless(engine);
            return;
        }
    };

    // Carregar paleta PLAYPAL do WAD
    let palette = load_palette(&engine.wad);
    window.set_palette(palette);

    println!(
        "D_DoomLoop: Iniciando game loop a {} Hz...",
        engine.ticrate()
    );

    loop {
        // I_StartTic — processar eventos SDL2
        if !window.pump_events(&mut engine.event_queue) {
            break; // Janela fechada
        }

        // Executar tick(s) e preparar frame
        if !engine.run_frame() {
            break;
        }

        // I_FinishUpdate — blit framebuffer para a janela
        if let Err(e) = window.finish_update(engine.framebuffer()) {
            eprintln!("Erro de rendering: {}", e);
            break;
        }
    }

    println!(
        "D_DoomLoop: {} ticks executados (~{:.1}s de jogo)",
        engine.gametic(),
        engine.gametic() as f64 / engine.ticrate() as f64,
    );
}

/// Funcao auxiliar para fallback headless quando SDL falha.
#[cfg(feature = "sdl")]
fn run_headless(engine: &mut DoomEngine) {
    run_headless_impl(engine);
}

/// Game loop headless (sem video).
///
/// Roda 35 ticks (1 segundo de simulacao) e encerra.
/// Util para testes e CI.
#[cfg(not(feature = "sdl"))]
fn run_headless(engine: &mut DoomEngine) {
    run_headless_impl(engine);
}

/// Implementacao do game loop headless.
fn run_headless_impl(engine: &mut DoomEngine) {
    println!("D_DoomLoop: Modo headless (sem janela)...");
    println!(
        "D_DoomLoop: Iniciando game loop a {} Hz...",
        engine.ticrate()
    );

    while engine.run_frame() {
        if engine.gametic() >= 35 {
            break;
        }
    }

    println!(
        "D_DoomLoop: {} ticks executados (~{:.1}s de jogo simulados)",
        engine.gametic(),
        engine.gametic() as f64 / engine.ticrate() as f64,
    );
    println!("(Para janela grafica: cargo run --features sdl -- --iwad <wad>)");
}

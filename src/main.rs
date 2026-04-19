//! # DOOM Rust — Ponto de Entrada
//!
//! Inicializa o engine e inicia o game loop.
//!
//! C original: `i_main.c` / `d_main.c`

fn main() {
    env_logger::init();
    log::info!("DOOM Rust v{}", env!("CARGO_PKG_VERSION"));
    log::info!("Port educacional do DOOM (1993) para Rust");

    // TODO: Parse de argumentos (--iwad, --pwad, etc.)
    // TODO: Inicializar subsistemas (video, audio, input)
    // TODO: Carregar WAD
    // TODO: Iniciar game loop

    println!("DOOM Rust - Port educacional");
    println!("Uso: doom-rust --iwad <caminho-para-wad>");
    println!("(Implementacao em andamento - Fase 0: Setup)");
}

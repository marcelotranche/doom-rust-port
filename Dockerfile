# =============================================================================
# DOOM Rust — Dockerfile multi-stage
#
# Estagio 1 (builder): compila o projeto em release
# Estagio 2 (runtime): imagem minima com o binario e assets
#
# Uso:
#   docker build -t doom-rust .
#   docker run --rm -v /caminho/para/freedoom1.wad:/data/freedoom1.wad doom-rust \
#       --iwad /data/freedoom1.wad --warp 1 1
# =============================================================================

# ---------------------------------------------------------------------------
# Estagio 1: Build
# ---------------------------------------------------------------------------
FROM rust:1.78-bookworm AS builder

WORKDIR /build

# Copiar manifesto primeiro para cache de dependencias
COPY Cargo.toml Cargo.lock* ./

# Criar projeto dummy para compilar dependencias (cache layer)
RUN mkdir src && \
    echo 'fn main() {}' > src/main.rs && \
    echo '' > src/lib.rs && \
    cargo build --release 2>/dev/null || true && \
    rm -rf src

# Copiar codigo-fonte real
COPY src/ src/

# Compilar em release (sem feature sdl — headless/CLI mode)
RUN cargo build --release && \
    strip target/release/doom-rust

# ---------------------------------------------------------------------------
# Estagio 2: Runtime
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# Metadados
LABEL maintainer="doom-rust"
LABEL description="DOOM Rust — Port educacional do DOOM (1993) para Rust"

# Dependencias minimas de runtime
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Criar usuario nao-root
RUN useradd --create-home --shell /bin/bash doomguy
USER doomguy
WORKDIR /home/doomguy

# Copiar binario do estagio de build
COPY --from=builder /build/target/release/doom-rust /usr/local/bin/doom-rust

# Diretorio para WADs montados via volume
RUN mkdir -p /home/doomguy/data

# Porta UDP do DOOM (para multiplayer futuro)
EXPOSE 5029/udp

# Entrypoint: o binario do DOOM
ENTRYPOINT ["doom-rust"]

# Default: mostrar ajuda se nenhum argumento for passado
CMD ["--help"]

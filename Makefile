# =============================================================================
# DOOM Rust — Makefile
#
# Alvos principais:
#   make build       — compilar com cargo (nativo)
#   make test        — rodar testes
#   make clippy      — lint
#   make run         — executar com freedoom (nativo)
#   make docker      — build da imagem Docker
#   make docker-run  — rodar via Docker com freedoom
#   make freedoom    — baixar Freedoom automaticamente
#   make clean       — limpar artefatos
#
# Variaveis:
#   IWAD=caminho     — WAD a usar (default: assets/freedoom1.wad)
#   WARP="1 1"       — mapa inicial (default: E1M1)
#   SKILL=3          — dificuldade 1-5 (default: 3)
# =============================================================================

# ---------------------------------------------------------------------------
# Variaveis
# ---------------------------------------------------------------------------

# Nome da imagem Docker
IMAGE_NAME    := doom-rust
# Tag da imagem
IMAGE_TAG     := latest
# Imagem completa
IMAGE         := $(IMAGE_NAME):$(IMAGE_TAG)

# Caminho do WAD (pode ser sobrescrito: make run IWAD=doom.wad)
IWAD          ?= assets/freedoom1.wad
# Mapa inicial (episodio e mapa)
WARP          ?= 1 1
# Dificuldade (1=Baby, 2=Easy, 3=Medium, 4=Hard, 5=Nightmare)
SKILL         ?= 3
# Argumentos extras
ARGS          ?=

# Freedoom
FREEDOOM_VER  := 0.13.0
FREEDOOM_URL  := https://github.com/freedoom/freedoom/releases/download/v$(FREEDOOM_VER)/freedoom-$(FREEDOOM_VER).zip
FREEDOOM_WAD  := assets/freedoom1.wad

# Diretorio de assets dentro do container
CONTAINER_DATA := /data
CONTAINER_IWAD := $(CONTAINER_DATA)/$(notdir $(IWAD))

# ---------------------------------------------------------------------------
# Alvos nativos (cargo)
# ---------------------------------------------------------------------------

.PHONY: build test clippy run clean help

## Compilar o projeto (release)
build:
	cargo build --release

## Rodar todos os testes
test:
	cargo test

## Lint com clippy (warnings = erro)
clippy:
	cargo clippy -- -D warnings

## Executar com WAD nativo
run: $(IWAD)
	cargo run --release -- --iwad $(IWAD) --warp $(WARP) --skill $(SKILL) $(ARGS)

# ---------------------------------------------------------------------------
# Alvos Docker
# ---------------------------------------------------------------------------

.PHONY: docker docker-run docker-test docker-shell docker-clean

## Build da imagem Docker
docker:
	docker build -t $(IMAGE) .

## Rodar o jogo via Docker
docker-run: $(IWAD) docker
	docker run --rm \
		-v $(CURDIR)/assets:$(CONTAINER_DATA):ro \
		$(IMAGE) \
		--iwad $(CONTAINER_IWAD) --warp $(WARP) --skill $(SKILL) $(ARGS)

## Rodar testes dentro do Docker
docker-test:
	docker run --rm \
		-v $(CURDIR):/build \
		-w /build \
		rust:1.78-bookworm \
		sh -c "cargo clippy -- -D warnings && cargo test"

## Shell interativo no container de desenvolvimento
docker-shell:
	docker run --rm -it \
		-v $(CURDIR):/build \
		-v $(CURDIR)/assets:$(CONTAINER_DATA):ro \
		-w /build \
		rust:1.78-bookworm \
		bash

## Ver ajuda do binario via Docker
docker-help: docker
	docker run --rm $(IMAGE) --help

## Remover imagem Docker
docker-clean:
	docker rmi $(IMAGE) 2>/dev/null || true

# ---------------------------------------------------------------------------
# Freedoom (download automatico)
# ---------------------------------------------------------------------------

.PHONY: freedoom

## Baixar Freedoom automaticamente
freedoom: $(FREEDOOM_WAD)

$(FREEDOOM_WAD):
	@echo "Baixando Freedoom $(FREEDOOM_VER)..."
	@mkdir -p assets
	@curl -L -o /tmp/freedoom.zip $(FREEDOOM_URL)
	@python3 -c "\
	import zipfile, shutil, os; \
	z = zipfile.ZipFile('/tmp/freedoom.zip'); \
	src = 'freedoom-$(FREEDOOM_VER)/freedoom1.wad'; \
	f = z.open(src); \
	out = open('assets/freedoom1.wad', 'wb'); \
	shutil.copyfileobj(f, out); \
	f.close(); out.close(); z.close()"
	@rm -f /tmp/freedoom.zip
	@echo "Freedoom instalado em $(FREEDOOM_WAD)"

# ---------------------------------------------------------------------------
# Limpeza
# ---------------------------------------------------------------------------

## Limpar artefatos de build
clean:
	cargo clean

## Limpar tudo (build + docker + freedoom)
distclean: clean docker-clean
	rm -f $(FREEDOOM_WAD)

# ---------------------------------------------------------------------------
# Compose
# ---------------------------------------------------------------------------

.PHONY: compose-build compose-run compose-test

## Build via Docker Compose
compose-build:
	docker compose build

## Rodar via Docker Compose
compose-run: $(IWAD)
	docker compose run --rm doom \
		--iwad $(CONTAINER_IWAD) --warp $(WARP) --skill $(SKILL) $(ARGS)

## Teste rapido via Docker Compose
compose-test:
	docker compose run --rm test

# ---------------------------------------------------------------------------
# Ajuda
# ---------------------------------------------------------------------------

## Mostrar esta ajuda
help:
	@echo "DOOM Rust — Makefile"
	@echo ""
	@echo "Uso: make <alvo> [VARIAVEL=valor]"
	@echo ""
	@echo "Alvos nativos:"
	@echo "  build           Compilar com cargo (release)"
	@echo "  test            Rodar testes"
	@echo "  clippy          Lint (warnings = erro)"
	@echo "  run             Executar com WAD nativo"
	@echo ""
	@echo "Alvos Docker:"
	@echo "  docker          Build da imagem Docker"
	@echo "  docker-run      Rodar o jogo via Docker"
	@echo "  docker-test     Rodar testes dentro do Docker"
	@echo "  docker-shell    Shell interativo no container"
	@echo "  docker-help     Ver ajuda do binario"
	@echo "  docker-clean    Remover imagem Docker"
	@echo ""
	@echo "Freedoom:"
	@echo "  freedoom        Baixar Freedoom automaticamente"
	@echo ""
	@echo "Compose:"
	@echo "  compose-build   Build via Docker Compose"
	@echo "  compose-run     Rodar via Docker Compose"
	@echo "  compose-test    Teste rapido via Compose"
	@echo ""
	@echo "Limpeza:"
	@echo "  clean           Limpar artefatos de build"
	@echo "  distclean       Limpar tudo (build + docker + wad)"
	@echo ""
	@echo "Variaveis:"
	@echo "  IWAD=path       WAD a usar (default: assets/freedoom1.wad)"
	@echo "  WARP=\"e m\"      Mapa inicial (default: 1 1)"
	@echo "  SKILL=n         Dificuldade 1-5 (default: 3)"
	@echo "  ARGS=\"...\"      Argumentos extras"
	@echo ""
	@echo "Exemplos:"
	@echo "  make freedoom docker-run"
	@echo "  make docker-run WARP=\"2 3\" SKILL=5"
	@echo "  make run IWAD=assets/doom.wad"

.DEFAULT_GOAL := help

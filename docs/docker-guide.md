# Guia: Rodar DOOM Rust em Docker

Este guia explica como compilar e executar o DOOM Rust port usando Docker,
sem precisar instalar Rust ou dependencias na maquina host.

## Pre-requisitos

- [Docker](https://docs.docker.com/get-docker/) instalado (20.10+)
- Um arquivo IWAD do DOOM (ver secao "Obtendo o WAD")

## Obtendo o WAD

O DOOM Rust precisa de um arquivo WAD com os assets do jogo.
Voce tem duas opcoes:

### Opcao 1: Freedoom (gratuito e livre)

Baixe o Freedoom, um projeto de assets livres compativeis com o DOOM:

```bash
# Via Makefile (recomendado — baixa e extrai automaticamente)
make freedoom

# Ou manualmente:
mkdir -p assets
curl -L -o /tmp/freedoom.zip \
    https://github.com/freedoom/freedoom/releases/download/v0.13.0/freedoom-0.13.0.zip
python3 -c "
import zipfile, shutil
z = zipfile.ZipFile('/tmp/freedoom.zip')
with z.open('freedoom-0.13.0/freedoom1.wad') as src, \
     open('assets/freedoom1.wad', 'wb') as dst:
    shutil.copyfileobj(src, dst)
z.close()
"
rm -f /tmp/freedoom.zip
```

### Opcao 2: DOOM original (se voce possui)

Se voce tem o DOOM original (comprado no Steam, GOG, etc.),
copie o arquivo `doom.wad` para o diretorio `assets/`:

```bash
cp /caminho/para/doom.wad assets/
```

## Build da imagem Docker

### Com Docker direto

```bash
# Na raiz do projeto
docker build -t doom-rust .
```

O build usa multi-stage:
1. **builder**: compila o projeto em release com `rust:1.78-bookworm`
2. **runtime**: imagem minima `debian:bookworm-slim` (~80MB) com o binario

### Com Docker Compose

```bash
docker compose build
```

## Executando

### Ver ajuda

```bash
docker run --rm doom-rust
# ou
docker compose run --rm doom --help
```

Saida esperada:

```
Uso: doom-rust --iwad <caminho-para-wad> [opcoes]

Opcoes:
  --iwad <path>       IWAD principal (freedoom1.wad, doom.wad, etc.)
  --file <path...>    PWADs adicionais
  --warp <e> <m>      Iniciar no mapa ExMy
  --skill <1-5>       Dificuldade (1=Baby, 5=Nightmare)
  ...
```

### Rodar com um WAD

Monte o diretorio `assets/` como volume em `/data`:

```bash
# Com Freedoom
docker run --rm \
    -v $(pwd)/assets:/data:ro \
    doom-rust \
    --iwad /data/freedoom1.wad

# Ir direto para E1M1
docker run --rm \
    -v $(pwd)/assets:/data:ro \
    doom-rust \
    --iwad /data/freedoom1.wad --warp 1 1

# Com dificuldade Nightmare
docker run --rm \
    -v $(pwd)/assets:/data:ro \
    doom-rust \
    --iwad /data/freedoom1.wad --warp 1 1 --skill 5
```

### Com Docker Compose

Coloque o WAD em `assets/` e execute:

```bash
# Rodar com argumentos
docker compose run --rm doom \
    --iwad /data/freedoom1.wad --warp 1 1

# Apenas testar se compila
docker compose run --rm test
```

### Rodar testes dentro do container

```bash
# Build e testes em um unico comando
docker run --rm \
    -v $(pwd):/build \
    -w /build \
    rust:1.78-bookworm \
    cargo test

# Testes com clippy
docker run --rm \
    -v $(pwd):/build \
    -w /build \
    rust:1.78-bookworm \
    sh -c "cargo clippy -- -D warnings && cargo test"
```

## Modo desenvolvedor

Para desenvolvimento iterativo sem rebuild a cada mudanca:

```bash
# Montar o codigo-fonte e compilar dentro do container
docker run --rm -it \
    -v $(pwd):/build \
    -w /build \
    rust:1.78-bookworm \
    bash

# Dentro do container:
cargo build
cargo test
cargo run -- --iwad assets/freedoom1.wad --warp 1 1
```

## Multiplayer (futuro)

A porta UDP 5029 esta exposta para multiplayer:

```bash
# Host
docker run --rm \
    -v $(pwd)/assets:/data:ro \
    -p 5029:5029/udp \
    doom-rust \
    --iwad /data/freedoom1.wad --net 2

# Cliente (em outra maquina)
docker run --rm \
    -v $(pwd)/assets:/data:ro \
    doom-rust \
    --iwad /data/freedoom1.wad --net 2 <ip-do-host>
```

## Troubleshooting

### "IWAD nao especificado"

Voce esqueceu o argumento `--iwad`. Certifique-se de montar o volume
e passar o caminho correto:

```bash
docker run --rm -v $(pwd)/assets:/data:ro doom-rust --iwad /data/freedoom1.wad
```

### "Erro ao carregar WAD: arquivo nao encontrado"

Verifique se:
1. O arquivo WAD existe em `assets/`
2. O volume esta montado corretamente (`-v $(pwd)/assets:/data:ro`)
3. O nome do arquivo corresponde ao passado em `--iwad`

```bash
# Verificar conteudo do volume
docker run --rm -v $(pwd)/assets:/data:ro doom-rust ls /data/
```

### Build lento na primeira vez

O primeiro build baixa a imagem Rust (~1.5GB) e compila todas
as dependencias. Builds subsequentes usam cache do Docker e
sao muito mais rapidos (apenas recompila o codigo que mudou).

### Permissao negada no volume

No Linux, o container roda como usuario `doomguy` (UID 1000).
Se seus arquivos tem permissoes restritivas:

```bash
chmod 644 assets/*.wad
```

## Estrutura dos arquivos Docker

```
doom-rust/
├── Dockerfile          # Build multi-stage (builder + runtime)
├── .dockerignore       # Exclui target/, references/, .git/
├── docker-compose.yml  # Compose com servicos doom e test
└── assets/             # Coloque WADs aqui
    └── freedoom1.wad   # (nao versionado)
```

## Tamanho das imagens

| Imagem | Tamanho aprox. |
|--------|---------------|
| Builder (rust:1.78) | ~1.5 GB (cache, nao fica no host) |
| Runtime (bookworm-slim) | ~80 MB |
| Binario doom-rust | ~5 MB |

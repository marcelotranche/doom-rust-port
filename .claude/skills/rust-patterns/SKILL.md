---
name: rust-patterns
description: >
  Padroes Rust especificos para este port de DOOM. Carrega quando
  o contexto envolve implementacao de codigo Rust para o port,
  decisoes de tipo, ownership, conversao de padroes C para Rust.
---

## Padroes Rust para o Port

### Fixed-Point Math
```rust
/// Numero em ponto-fixo 16.16, base da matematica do DOOM.
/// No DOOM original: `typedef int fixed_t;` em m_fixed.h
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fixed(pub i32);

impl Fixed {
    pub const FRACBITS: i32 = 16;
    pub const UNIT: Fixed = Fixed(1 << 16);  // 1.0 em fixed-point
    pub const ZERO: Fixed = Fixed(0);

    /// Converte inteiro para fixed-point
    pub fn from_int(n: i32) -> Self { Fixed(n << Self::FRACBITS) }

    /// Parte inteira do valor
    pub fn to_int(self) -> i32 { self.0 >> Self::FRACBITS }
}

// Implementar Add, Sub, Mul, Div via std::ops
```

### Eliminando Globals com Context Structs
```rust
// C original: variaveis globais em r_main.c
// int viewwidth, viewheight;
// fixed_t viewx, viewy, viewz;
// angle_t viewangle;

// Rust: struct de contexto passada por referencia
/// Contexto de camera para o frame atual de rendering.
/// Equivalente as globals viewx/viewy/viewz/viewangle de r_main.c
pub struct ViewContext {
    pub x: Fixed,
    pub y: Fixed,
    pub z: Fixed,
    pub angle: Angle,
    pub width: usize,
    pub height: usize,
}
```

### Thinkers como Trait
```rust
// C original: thinker_t com function pointer e linked list
// Rust: trait + enum dispatch

/// Um Thinker e qualquer entidade que "pensa" a cada tic.
/// No DOOM original: struct thinker_t em p_tick.h
pub trait Thinker {
    /// Atualiza o estado deste thinker por um tic.
    /// Retorna false se o thinker deve ser removido.
    fn think(&mut self, world: &mut World) -> bool;
}
```

### Enums para State Machines
```rust
// C original: #define S_PLAY_RUN1 46 (info.h)
// Rust: enum tipado

/// Estado de animacao de um Map Object.
/// Mapeado a partir das constantes S_* em info.h
#[derive(Clone, Copy, Debug)]
pub struct StateNum(pub usize);

/// Definicao de um frame de animacao.
/// Equivalente a `state_t` em info.h
pub struct StateDef {
    pub sprite: SpriteNum,
    pub frame: u32,
    pub tics: i32,
    pub next_state: StateNum,
    pub action: Option<fn(&mut Mobj, &mut World)>,
}
```

### Error Handling para I/O
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WadError {
    #[error("Arquivo WAD nao encontrado: {0}")]
    FileNotFound(String),
    #[error("Header WAD invalido: magic esperado IWAD/PWAD, encontrado {0}")]
    InvalidHeader(String),
    #[error("Lump '{0}' nao encontrado no WAD")]
    LumpNotFound(String),
    #[error("Erro de I/O ao ler WAD: {0}")]
    Io(#[from] std::io::Error),
}
```

### Comentarios Didaticos — Estilo
```rust
//! # Modulo WAD (Where's All the Data)
//!
//! O WAD e o formato de arquivo container do DOOM. Todo o conteudo
//! do jogo — mapas, texturas, sons, sprites — vive dentro de um
//! unico arquivo .wad.
//!
//! ## Estrutura do arquivo
//! ```text
//! +------------------+
//! |  Header (12 bytes)| <- magic ("IWAD"/"PWAD") + contagem + offset
//! +------------------+
//! |  Dados dos lumps  | <- blocos de dados brutos, sem estrutura fixa
//! +------------------+
//! |  Diretorio        | <- lista de (offset, tamanho, nome) por lump
//! +------------------+
//! ```
//!
//! ## Arquivo C original: `w_wad.c`
```

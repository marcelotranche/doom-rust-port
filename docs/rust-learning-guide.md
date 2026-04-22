# Mini-Guia de Aprendizado Rust

Roteiro enxuto para quem quer **aprender Rust** usando este
repositório como material de estudo prático. Pensado para
programadores com experiência prévia em C, C++ ou linguagens
similares — mas acessível a quem vem de Python/JavaScript com
paciência extra nas primeiras etapas.

Documentos complementares:
- [`rust-idiomatic.md`](rust-idiomatic.md) — os quatro pilares
  em detalhe.
- [`rust-glossary.md`](rust-glossary.md) — definições curtas.
- [`glossary.md`](glossary.md) — termos do DOOM.

---

## Antes de começar

### Instalação

```bash
# Linux / macOS / WSL
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Windows: baixar rustup-init.exe em https://rustup.rs/
```

Após instalar, valide:

```bash
rustc --version      # >= 1.75
cargo --version
```

### Editor recomendado

- **VS Code** + extensão [`rust-analyzer`](https://rust-analyzer.github.io/)
  (feedback inline, go-to-definition, inlay hints).
- Alternativas: RustRover (JetBrains), Zed, Neovim com LSP.

### Mentalidade

Três expectativas importantes:

1. **O compilador é seu parceiro**, não seu adversário. Ele
   apontará erros que em C você só descobriria em runtime —
   leia as mensagens, elas são famosas por serem úteis.
2. **"Fighting the borrow checker"** é uma fase que todo
   iniciante passa. Conforme você internaliza o modelo de
   ownership, ela desaparece.
3. **Não tente escrever C em Rust.** Quando o compilador
   rejeita seu código, pergunte: "qual é a forma Rust de
   expressar isso?" — costuma ser um enum, um trait ou
   restruturação de ownership.

---

## Trilha de aprendizado sugerida

### Etapa 1 — Fundamentos (≈ 1 semana)

**Objetivo:** escrever programas imperativos simples, entender tipos
básicos e controle de fluxo.

Conceitos a dominar:
- Variáveis (`let` / `let mut`), shadowing.
- Tipos primitivos (`i32`, `u32`, `f64`, `bool`, `char`, tuples,
  arrays).
- `String` vs `&str`.
- `if` / `else` como expressão (retorna valor).
- Loops: `loop`, `while`, `for` com ranges.
- Funções, closures básicas.
- `println!`, `format!`, `dbg!`.

Recursos:
- [*The Rust Book*](https://doc.rust-lang.org/book/) — capítulos 1–3.
- [Rustlings](https://github.com/rust-lang/rustlings) — exercícios
  interativos, comece aqui: `variables`, `functions`, `if`.

Exercício prático: escreva uma calculadora de linha de comando
que leia dois números e uma operação (`+`, `-`, `*`, `/`).

---

### Etapa 2 — Ownership e Borrowing (≈ 1 semana)

**Objetivo:** parar de lutar com o borrow checker.

Conceitos:
- Move vs Copy.
- `&T` (empréstimo imutável) vs `&mut T` (empréstimo mutável).
- Regra: "N leitores OU 1 escritor, nunca ambos".
- Lifetimes básicas (`'a`); lifetime elision.
- Slices (`&[T]`, `&str`).
- `Vec<T>` e `String`.
- `Drop` e RAII.

Recursos:
- [*The Rust Book*](https://doc.rust-lang.org/book/) — capítulo 4
  (Understanding Ownership).
- [Rustlings](https://github.com/rust-lang/rustlings): `move_semantics`,
  `primitive_types`, `vecs`, `strings`.
- Neste repo: [`rust-idiomatic.md`](rust-idiomatic.md), seções 1 e 2.

Exercício prático: escreva uma função que receba `&mut Vec<i32>`
e remova todos os números pares. Veja quantos erros de borrow
checker aparecem e por quê.

**Checkpoint:** quando você consegue explicar *por que* o
compilador rejeita código como este, você superou a etapa:

```rust
let mut v = vec![1, 2, 3];
let first = &v[0];
v.push(4);               // ❌ por que falha?
println!("{}", first);
```

---

### Etapa 3 — Tipos ricos: Enums, Option, Result (≈ 3–5 dias)

**Objetivo:** modelar dados de forma que estados inválidos sejam
inexpressáveis.

Conceitos:
- Structs (nomeadas, tuple, unit).
- Enums com dados em cada variante.
- `match` exaustivo.
- `if let`, `while let`.
- `Option<T>` — substituto para null.
- `Result<T, E>` — substituto para códigos de erro.
- Operador `?` para propagar erro.
- Padrões comuns: `map`, `and_then`, `unwrap_or`, `?`.

Recursos:
- [*The Rust Book*](https://doc.rust-lang.org/book/) — capítulos 5, 6, 9.
- [Rustlings](https://github.com/rust-lang/rustlings): `structs`,
  `enums`, `options`, `error_handling`.
- Neste repo: [`rust-idiomatic.md`](rust-idiomatic.md), seção 3.

Exercício prático: modele um `Pickup` do DOOM como enum
(`Health(i32)`, `Ammo { kind, amount }`, `Key(KeyColor)`, `None`)
e escreva uma função `apply(pickup, &mut player)`.

---

### Etapa 4 — Traits e Generics (≈ 1 semana)

**Objetivo:** escrever código polimórfico sem `void*`.

Conceitos:
- Definir e implementar traits (`trait Thinker { fn think(&mut self); }`).
- Traits derivados (`#[derive(Debug, Clone, PartialEq)]`).
- Generics com trait bounds (`fn foo<T: Display>(x: T)`).
- `impl Trait` em posições de argumento e retorno.
- Static dispatch (generics) vs dynamic dispatch (`dyn Trait`).
- `Box<dyn Trait>` — substituto direto para `void*` + ponteiros
  de função.
- Traits comuns: `Iterator`, `Debug`, `Display`, `From`/`Into`,
  `Default`, `Clone`, `Copy`.

Recursos:
- [*The Rust Book*](https://doc.rust-lang.org/book/) — capítulo 10.
- [Rustlings](https://github.com/rust-lang/rustlings): `generics`,
  `traits`, `iterators`.
- Neste repo: [`rust-idiomatic.md`](rust-idiomatic.md), seção 4.

Exercício prático: defina um trait `Shape` com método `area()` e
implemente para `Circle`, `Rectangle`, `Triangle`. Armazene vários
em um `Vec<Box<dyn Shape>>` e some as áreas.

---

### Etapa 5 — Coleções, Iteradores e Error Handling avançado (≈ 1 semana)

**Objetivo:** escrever código idiomático e conciso.

Conceitos:
- `Vec`, `HashMap`, `HashSet`, `BTreeMap`.
- Cadeias de iteradores: `iter().map().filter().collect()`.
- `iter()` vs `iter_mut()` vs `into_iter()`.
- Custom error types com `thiserror` ou enum manual.
- Conversão entre erros via `From`.
- Quando usar `panic!` vs `Result`.

Recursos:
- [*The Rust Book*](https://doc.rust-lang.org/book/) — capítulos 8, 9.
- [Rustlings](https://github.com/rust-lang/rustlings): `hashmaps`,
  `iterators`, `error_handling`.

Exercício prático: leia um arquivo texto linha a linha, conte a
frequência de cada palavra e imprima as 10 mais comuns. Trate
erros de I/O com `Result` e `?`.

---

### Etapa 6 — Módulos, Cargo, Testing (≈ 3–5 dias)

**Objetivo:** organizar um projeto real.

Conceitos:
- `mod`, `pub`, `use`.
- Hierarquia de arquivos (`mod.rs`, submódulos).
- `Cargo.toml`: `dependencies`, `dev-dependencies`, `features`.
- `#[test]` e `#[cfg(test)]`.
- Testes unitários (no mesmo arquivo) vs integração (pasta `tests/`).
- Doc-tests em comentários `///`.
- `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt`.

Recursos:
- [*The Rust Book*](https://doc.rust-lang.org/book/) — capítulos 7, 11, 14.
- [The Cargo Book](https://doc.rust-lang.org/cargo/).

---

### Etapa 7 — Tópicos avançados (sob demanda)

Estudar conforme a necessidade, não todos de uma vez:

| Tópico | Quando estudar |
|--------|----------------|
| **Lifetimes explícitas** | Quando escrever structs que contêm referências. |
| **`Rc`, `Arc`, `RefCell`** | Quando ownership único não modela o problema (ex: grafos). |
| **`unsafe`** | Ao interfacear com C (FFI) ou otimizar hot paths. |
| **Macros (`macro_rules!`)** | Quando notar duplicação que nem closures resolvem. |
| **Procedural macros** | Raramente — para DSLs ou `#[derive]` customizado. |
| **`async` / `await`** | Para I/O concorrente (servidores, APIs). |
| **FFI com C** | Para usar bibliotecas C ou expor Rust a outras linguagens. |

---

## Usando este repositório como material de estudo

Depois das etapas 1–4, você pode usar o código deste port como
estudo de caso de **C → Rust** em cenário real. Sugestão de
percurso:

1. Leia [`rust-idiomatic.md`](rust-idiomatic.md) em conjunto com
   [`architecture.md`](architecture.md).
2. Escolha um módulo pequeno para começar:
   - [`src/utils/fixed.rs`](../src/utils/fixed.rs) — newtype
     `Fixed(i32)` e operadores. Mostra Etapa 4 (traits).
   - [`src/utils/angle.rs`](../src/utils/angle.rs) — newtype
     sobre `u32` com overflow intencional (`wrapping_add`).
   - [`src/wad/`](../src/wad/) — parsing de binário com
     `byteorder`. Mostra Etapa 5 (Result, iterators).
3. Compare com o arquivo C original em
   `references/DOOM-master/linuxdoom-1.10/` (ex: `m_fixed.c`).
   Note as invariantes que em C viviam em comentários e em
   Rust viraram tipos.
4. Leia os testes unitários (`#[cfg(test)] mod tests`) — eles
   documentam o comportamento esperado.
5. Progrida para módulos maiores: [`src/map/`](../src/map/),
   [`src/game/`](../src/game/), [`src/renderer/`](../src/renderer/).

### Tarefas de estudo sugeridas

- **Fácil:** adicione um método à `Fixed` (ex: `saturating_mul`)
  com teste unitário.
- **Médio:** implemente um `Debug` customizado para `Angle`
  que mostre graus em vez do raw u32.
- **Difícil:** adicione uma nova variante a `MapObjectKind` e
  rastreie pelo compilador quais lugares precisam atualizar
  (o `match` exaustivo fará o trabalho).

---

## Recursos externos

### Oficiais / gratuitos

- [*The Rust Programming Language*](https://doc.rust-lang.org/book/)
  — "O Livro". Referência canônica.
- [*Rust by Example*](https://doc.rust-lang.org/rust-by-example/)
  — mesma matéria, com exemplos executáveis.
- [Rustlings](https://github.com/rust-lang/rustlings) — exercícios
  pequenos e guiados, altamente recomendados.
- [Rust Playground](https://play.rust-lang.org/) — compilador no
  navegador, ótimo para testar snippets.
- [Rust Standard Library Docs](https://doc.rust-lang.org/std/)
  — documentação da biblioteca padrão.

### Para programadores vindos de C / C++

- [*The Rustonomicon*](https://doc.rust-lang.org/nomicon/) —
  leitura avançada sobre `unsafe` e invariantes de memória.
- [*Learn Rust With Entirely Too Many Linked Lists*](https://rust-unofficial.github.io/too-many-lists/)
  — estudo de caso perfeito para quem vem de C: porquê listas
  ligadas são "difíceis" em Rust.
- [Rust Cheat Sheet](https://cheats.rs/) — uma única página
  com toda a sintaxe.

### Livros impressos

- **Programming Rust** (Blandy, Orendorff, Tindall) — clássico
  para quem vem de C/C++.
- **Rust for Rustaceans** (Jon Gjengset) — intermediário/avançado,
  excelente depois de dominar o básico.
- **Zero to Production in Rust** (Luca Palmieri) — projeto real
  (API web), ótimo para tópicos de sistemas.

### Comunidade

- [r/rust](https://reddit.com/r/rust) — discussão ativa.
- [Rust Users Forum](https://users.rust-lang.org/) — suporte
  técnico, ótimo para perguntas iniciantes.
- [This Week in Rust](https://this-week-in-rust.org/) —
  newsletter semanal com novidades e crates.
- [Jon Gjengset no YouTube](https://www.youtube.com/@jonhoo) —
  streams longos implementando coisas reais em Rust.

---

## Indicadores de progresso

Você pode considerar que passou do nível iniciante quando:

- [ ] Lê uma mensagem de erro do compilador e entende a correção
      sem copiar-colar.
- [ ] Escolhe entre `Vec<T>`, `&[T]` e `[T; N]` conscientemente.
- [ ] Usa `Option<T>` e `Result<T, E>` naturalmente (sem
      `unwrap()` por todo lado).
- [ ] Entende a diferença entre `String` e `&str` e quando usar
      cada um.
- [ ] Consegue projetar um enum com variantes de dados heterogêneos
      para modelar estado.
- [ ] Escreve um `impl` de trait sem consultar a sintaxe.
- [ ] Explica o que `Box<dyn Trait>` faz e quando é necessário.
- [ ] `cargo clippy` do seu código passa sem avisos.

Se marcou todos: está pronto para contribuir com este repo —
comece por um módulo da [Etapa avançada](#etapa-7--tópicos-avançados-sob-demanda).

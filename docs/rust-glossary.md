# Mini-Glossário Rust

Glossário enxuto dos termos de **linguagem Rust** que aparecem no
código e na documentação deste port. Para termos específicos do
engine DOOM, veja [`glossary.md`](glossary.md). Para a explicação
aprofundada dos quatro pilares, veja
[`rust-idiomatic.md`](rust-idiomatic.md).

---

## Modelo de memória

| Termo | Definição curta |
|-------|-----------------|
| **Ownership** | Regra de que cada valor tem exatamente um dono; quando o dono sai de escopo, o valor é destruído. |
| **Move** | Transferência de posse — após mover, o original não pode mais ser usado. |
| **Borrow** | Empréstimo temporário de um valor via referência, sem transferir posse. |
| **Reference (`&T`, `&mut T`)** | Ponteiro seguro — imutável (`&T`) ou mutável (`&mut T`). |
| **Borrow checker** | Componente do compilador que verifica as regras de empréstimo em tempo de compilação. |
| **Lifetime (`'a`)** | Anotação que expressa quanto tempo uma referência vive; garante que nunca aponte para dado liberado. |
| **`'static`** | Lifetime especial: dura por toda a execução do programa (ex: string literals). |
| **Drop** | Trait chamado automaticamente quando o valor sai de escopo (equivalente a destrutor + `free()`). |
| **RAII** | Resource Acquisition Is Initialization — padrão em que recursos são liberados automaticamente via `Drop`. |

---

## Tipos e sistema de tipos

| Termo | Definição curta |
|-------|-----------------|
| **Primitive** | Tipos built-in: `i8..i64`, `u8..u64`, `isize`, `usize`, `f32`, `f64`, `bool`, `char`. |
| **`str`** | String imutável em UTF-8 (fatia, sem dono). Normalmente usada como `&str`. |
| **`String`** | String mutável com dono, alocada no heap. |
| **`Vec<T>`** | Array dinâmico no heap (equivalente a `std::vector` do C++). |
| **`[T; N]`** | Array de tamanho fixo `N` na stack. |
| **`&[T]`** | Slice — empréstimo de uma sequência contígua. |
| **`Box<T>`** | Ponteiro único para o heap (posse exclusiva). |
| **`Rc<T>` / `Arc<T>`** | Contagem de referências (single-thread / thread-safe). |
| **`RefCell<T>` / `Cell<T>`** | Interior mutability — permite mutar através de `&T` com checagem em runtime. |
| **`Option<T>`** | Enum: `Some(T)` ou `None`. Substitui o ponteiro nulo do C. |
| **`Result<T, E>`** | Enum: `Ok(T)` ou `Err(E)`. Substitui códigos de erro. |
| **Struct** | Agregado nomeado de campos (equivalente a `struct` do C). |
| **Tuple struct** | Struct com campos posicionais: `struct Fixed(i32);`. |
| **Newtype** | Tuple struct de um único campo, usado para segurança de tipos (ex: `Fixed`, `Angle`). |
| **Enum** | Tipo-soma — pode ser uma de várias variantes, cada uma com dados próprios. |
| **Variant** | Uma das possibilidades de um enum (ex: `Pickup::Health(i32)`). |
| **Generic (`<T>`)** | Parametrização por tipo — `Vec<T>`, `Option<T>` são genéricos. |
| **Trait bound (`T: Debug`)** | Restrição de que um tipo genérico implemente certos traits. |
| **Phantom type** | Tipo usado apenas para distinguir no sistema de tipos, sem representação em runtime. |

---

## Traits e polimorfismo

| Termo | Definição curta |
|-------|-----------------|
| **Trait** | Contrato/interface — conjunto de métodos que um tipo pode implementar. |
| **`impl Trait for Type`** | Implementação de um trait para um tipo específico. |
| **Derive (`#[derive(Debug)]`)** | Implementação automática de traits padrão pelo compilador. |
| **Trait object (`dyn Trait`)** | Ponteiro com vtable — permite polimorfismo dinâmico em runtime. |
| **`Box<dyn Trait>`** | Trait object alocado no heap; substituto Rust direto para `void*` + função virtual do C. |
| **Monomorfização** | Geração de código especializado para cada tipo concreto usado num genérico (zero-cost). |
| **Static dispatch** | Chamada resolvida em compile-time (via generics). |
| **Dynamic dispatch** | Chamada resolvida em runtime (via `dyn`). |
| **Blanket impl** | Implementação de um trait para todos os tipos que satisfazem certa condição. |
| **Supertrait** | Trait que depende de outro trait (`trait Eq: PartialEq`). |

### Traits mais usados

| Trait | Uso |
|-------|-----|
| `Debug` | Formatação com `{:?}` (debug). |
| `Display` | Formatação com `{}` (user-facing). |
| `Clone` / `Copy` | Duplicação explícita / implícita. |
| `PartialEq` / `Eq` | Comparação com `==`. |
| `Hash` | Uso como chave em `HashMap`/`HashSet`. |
| `Default` | Valor padrão via `Type::default()`. |
| `From` / `Into` | Conversão entre tipos. |
| `Iterator` | Iteração preguiçosa com `.next()`. |
| `Drop` | Cleanup automático ao sair de escopo. |
| `Send` / `Sync` | Markers para uso em threads. |

---

## Sintaxe e controle de fluxo

| Termo | Definição curta |
|-------|-----------------|
| **`match`** | Pattern matching exaustivo — o compilador exige tratar todas as variantes. |
| **`if let` / `while let`** | Forma curta de `match` para um único padrão. |
| **Pattern** | Forma de desestruturar valores (`Some(x)`, `(a, b)`, `Foo { field, .. }`). |
| **`?` operator** | Propaga erro: em `expr?`, retorna cedo se `expr` for `Err`/`None`. |
| **Closure** | Função anônima: `\|x\| x + 1`. |
| **`move` closure** | Closure que toma posse das variáveis capturadas. |
| **Turbofish (`::<T>`)** | Sintaxe para passar parâmetro de tipo explícito: `parse::<u32>()`. |
| **`impl` block** | Bloco onde métodos de um tipo são definidos. |
| **`self` / `&self` / `&mut self`** | Receiver de método: consome, empresta ou empresta mutavelmente. |
| **`Self`** | Alias para o tipo onde o método é definido. |

---

## Erros e segurança

| Termo | Definição curta |
|-------|-----------------|
| **`panic!`** | Aborta o programa (ou unwinding) — uso para bugs, nunca para erros esperados. |
| **`unwrap()`** | Extrai o valor de `Option`/`Result` ou entra em pânico. Use apenas quando *certeza*. |
| **`expect("msg")`** | Como `unwrap()` mas com mensagem customizada. |
| **`?`** | Propagação ergonômica de erro — preferido sobre `unwrap()` em código real. |
| **`unsafe`** | Bloco/função onde o programador garante invariantes que o compilador não pode verificar. |
| **UB (Undefined Behavior)** | Comportamento indefinido — só possível em `unsafe` mal escrito. |
| **Safe abstraction** | API segura construída sobre `unsafe` interno (ex: `Vec`, `Mutex`). |

---

## Concorrência

| Termo | Definição curta |
|-------|-----------------|
| **`Send`** | Tipo pode ser movido entre threads. |
| **`Sync`** | Tipo pode ser acessado por múltiplas threads simultaneamente (`&T` é `Send`). |
| **`Mutex<T>`** | Mutual exclusion — acesso exclusivo via `.lock()`. |
| **`RwLock<T>`** | Read-write lock — múltiplos leitores ou um escritor. |
| **`Arc<T>`** | Atomic reference count — `Rc` thread-safe. |
| **`async` / `.await`** | Primitivas de programação assíncrona (coroutines). |
| **Channel (`mpsc`)** | Comunicação entre threads por envio de mensagens. |
| **Data race** | Dois acessos concorrentes, ao menos um escrita — **impossível** em Rust seguro. |

---

## Macros

| Termo | Definição curta |
|-------|-----------------|
| **`println!` / `vec!` / `format!`** | Macros declarativas — sintaxe com `!`. |
| **Declarative macro (`macro_rules!`)** | Macro definida por padrões (substituição de tokens). |
| **Procedural macro** | Macro que processa tokens como código Rust (ex: `#[derive(...)]`). |
| **Attribute macro** | Macro aplicada com `#[...]` sobre itens. |
| **`#[cfg(...)]`** | Compilação condicional (ex: `#[cfg(test)]`). |

---

## Módulos e pacotes

| Termo | Definição curta |
|-------|-----------------|
| **Crate** | Unidade de compilação Rust — biblioteca ou binário. |
| **Package** | Um ou mais crates gerenciados por um `Cargo.toml`. |
| **Module (`mod`)** | Namespace dentro de um crate, hierárquico. |
| **`pub`** | Visibilidade pública. Sem `pub`, o item é privado ao módulo. |
| **`use`** | Traz um item ao escopo atual (equivalente a `import` / `using`). |
| **Workspace** | Múltiplos pacotes compartilhando `Cargo.lock` e diretório `target/`. |

---

## Ferramentas

| Ferramenta | Uso |
|------------|-----|
| **`rustc`** | Compilador Rust. |
| **`cargo`** | Build system + package manager (build, test, publish). |
| **`rustup`** | Gerenciador de versões do Rust (toolchain). |
| **`clippy`** | Lint — detecta anti-padrões e sugestões (`cargo clippy`). |
| **`rustfmt`** | Formatador oficial (`cargo fmt`). |
| **`rustdoc`** | Gera documentação HTML a partir de comentários `///`. |
| **`miri`** | Interpretador que detecta UB em código `unsafe`. |
| **`crates.io`** | Registro público de pacotes Rust. |
| **`docs.rs`** | Hospedagem de documentação de todos os crates publicados. |

---

## Jargão da comunidade

| Termo | Significado |
|-------|-------------|
| **Rustacean** | Membro da comunidade Rust. |
| **Ferris** | Mascote (caranguejo) da linguagem. |
| **Idiomatic** | Código que segue os padrões esperados pela comunidade. |
| **Zero-cost abstraction** | Abstração sem overhead em runtime vs. escrever o código "na mão". |
| **Fearless concurrency** | Slogan: concorrência sem medo de data races, graças ao sistema de tipos. |
| **"Fighting the borrow checker"** | Fase inicial do aprendizado em que o compilador parece adversário. |
| **"If it compiles, it works"** | Exagero bem-humorado — com tipos fortes, muitos bugs somem após compilar. |
| **MSRV** | Minimum Supported Rust Version — versão mínima do compilador suportada. |
| **Nightly / Stable / Beta** | Três canais de release do Rust. |
| **Edition (2015 / 2018 / 2021)** | "Versão dialetal" do Rust — mudanças de sintaxe sem quebrar compatibilidade. |

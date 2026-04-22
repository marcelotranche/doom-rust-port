# Rust Idiomático — Os Quatro Pilares

Este documento explica os quatro conceitos centrais que tornam o Rust
**Rust**, e como eles aparecem neste port do DOOM. É leitura
recomendada antes de mergulhar no código-fonte: entendê-los é a
diferença entre *ler* o port e *entender por que ele foi escrito assim*.

Os quatro pilares são:

1. [Ownership](#1-ownership--quem-é-dono-desta-memória) — quem é dono
   desta memória?
2. [Borrow checker](#2-borrow-checker--posso-olhar-sem-roubar) — posso
   olhar sem roubar?
3. [Enums algébricos](#3-enums-algébricos--este-valor-é-exatamente-uma-de-várias-possibilidades)
   — este valor é exatamente uma de várias possibilidades?
4. [Traits](#4-traits--estes-tipos-diferentes-compartilham-um-comportamento)
   — estes tipos diferentes compartilham um comportamento?

E no final, [como os quatro se amarram](#como-os-quatro-se-amarram).

---

## 1. Ownership — "quem é dono desta memória?"

### Problema em C

Em C, qualquer ponteiro pode liberar qualquer memória. Isso gera
três classes clássicas de bug:

- **Use-after-free:** um ponteiro ainda é usado depois que `free()`
  foi chamado.
- **Double-free:** `free()` é chamado duas vezes no mesmo bloco.
- **Memory leak:** ninguém chama `free()`.

O DOOM original convive com isso via `Z_Malloc` / `Z_Free` e uma
disciplina rigorosa de convenções: "o chamador é dono", "o callee
é dono", "este buffer é estático". Essas regras vivem em
comentários e na cabeça de quem programa.

### Regra em Rust

> Cada valor tem **exatamente um dono**. Quando o dono sai de
> escopo, o valor é destruído automaticamente.

Atribuir ou passar um valor **transfere** a posse (*move*), a
menos que o tipo implemente `Copy` (tipos pequenos, como `i32`,
`bool`, `Fixed`).

```rust
fn carrega_wad() -> WadFile {
    let arquivo = WadFile::open("doom.wad");  // 'arquivo' é dono
    arquivo                                    // posse transferida ao chamador
}  // se não retornasse, 'arquivo' seria destruído aqui — Drop fecharia o fd

fn main() {
    let wad = carrega_wad();     // 'wad' é o dono agora
    processa(wad);               // posse movida para 'processa'
    // usar 'wad' aqui seria ERRO DE COMPILAÇÃO — já foi movido
}
```

### Por que isso mata bugs

- `drop()` é inserido **automaticamente** no ponto certo pelo
  compilador. É impossível esquecer.
- É **impossível** chamar `drop()` duas vezes: depois que um valor
  é movido, o dono original não pode mais acessá-lo.
- Vazamentos são possíveis (ex: ciclos em `Rc<RefCell<_>>`), mas
  raros e detectáveis.

### No port DOOM-Rust

- `WadFile` é dono dos bytes do WAD. Ao carregar o mapa, ele
  empresta fatias (`&[u8]`) aos parsers. Quando o `WadFile` é
  destruído, tudo que dependia dele também é invalidado — o
  compilador **obriga** que nada use memória liberada.
- `MapData` é dono de `Vec<Sector>`, `Vec<Line>`, `Vec<Thing>` etc.
  Uma única função (`carrega_mapa`) cria tudo e transfere a posse
  inteira. Não há array global mutável à espreita.

---

## 2. Borrow checker — "posso olhar sem roubar?"

### Problema

Mover posse toda hora é inconveniente. Frequentemente você só quer
**ler** ou **modificar temporariamente**, sem transferir. Em C isso
é um ponteiro — mas dois ponteiros mutáveis para a mesma coisa
causam bugs sutis (*iterator invalidation*, data races).

### Regra em Rust

> A qualquer momento, para qualquer valor, você tem **ou**
> **N referências imutáveis** (`&T`) **ou** **uma única referência
> mutável** (`&mut T`). **Nunca as duas coisas simultaneamente.**

```rust
let mut sectors: Vec<Sector> = carrega_setores();

// OK: muitos leitores ao mesmo tempo
let a = &sectors[0];
let b = &sectors[1];
println!("{} {}", a.floor_height, b.floor_height);

// OK: um único escritor (depois que 'a' e 'b' saíram de escopo)
let s = &mut sectors[0];
s.floor_height = 64;

// ERRO: misturar leitor e escritor na mesma janela de vida
let leitor = &sectors[0];
let escritor = &mut sectors[0];  // ❌ não compila
println!("{}", leitor.floor_height);
```

### Lifetimes

Quando uma função recebe ou retorna referências, o compilador
precisa saber quanto tempo elas vivem. Na maioria dos casos
consegue inferir (*lifetime elision*); quando não, exige
anotações:

```rust
fn sector_de_thing<'a>(map: &'a MapData, thing: &Thing) -> &'a Sector {
    &map.sectors[thing.sector_idx]
}
// A referência retornada vive tanto quanto o 'map' emprestado.
```

### Por que isso mata bugs

- **Zero data races** em código seguro multithread — dois escritores
  concorrentes são impossíveis de construir.
- **Zero iterator invalidation** — não dá para mutar um `Vec`
  enquanto outro empréstimo segura uma referência para dentro dele.
- **Dangling references** impossíveis — a referência não pode
  sobreviver ao dono.

### No port

- `MapData` é passado como `&mut` para os `think()` dos thinkers
  (portas, plataformas precisam mover setores).
- Durante o *rendering*, o renderer recebe `&MapData` imutável —
  o compilador **garante** que ninguém tente mutar o mapa no meio
  de um frame.
- Quando um thinker precisa modificar um setor específico, pega
  `&mut map.sectors[idx]` pelo tempo mínimo necessário.

---

## 3. Enums algébricos — "este valor é exatamente uma de várias possibilidades"

### Problema

Em C, representar "uma de várias coisas" exige um `struct` com
`int type` + `union` (ou `void*`) com os dados específicos. É
propenso a erro:

- Esquecer um `case` no `switch` sobre o `type`.
- Castar para o tipo errado dentro da `union`.
- Nada impede ler `.ammo.amount` quando o `type` é `HEALTH`.

```c
// C — tudo depende de disciplina do programador
struct pickup_t {
    int type;  // 0=health, 1=ammo, 2=key
    union { int health; struct { int kind; int amount; } ammo; int key; } data;
};
```

### Enums do Rust

Enums do Rust são **tagged unions** de verdade (também chamados
*sum types* em linguagens funcionais). Cada variante carrega
**dados diferentes**, e o acesso só é possível via `match`, que
é **exaustivo**:

```rust
enum Pickup {
    Health(i32),                                  // quanto cura
    Ammo { kind: AmmoKind, amount: u32 },         // tipo e quantidade
    Key(KeyColor),                                // cor da chave
    None,                                         // nada a pegar
}

fn aplica(pickup: Pickup, player: &mut Player) {
    match pickup {
        Pickup::Health(n)             => player.health += n,
        Pickup::Ammo { kind, amount } => player.ammo[kind] += amount,
        Pickup::Key(cor)              => player.keys.insert(cor),
        Pickup::None                  => {}
    }
    // Se você esquecer uma variante, o compilador REJEITA o código.
}
```

### Tipos fundamentais construídos assim

```rust
enum Option<T> { Some(T), None }           // substitui ponteiro nulo
enum Result<T, E> { Ok(T), Err(E) }        // substitui -1/errno
```

Essas duas definições **eliminam** classes inteiras de bug:

- Não há como "esquecer de checar `NULL`" — para usar o valor
  dentro de um `Option`, você é **obrigado** a tratar o `None`.
- Não há como ignorar um erro silenciosamente — `Result` força
  tratamento explícito (ou propagação com `?`).

### Por que isso mata bugs

- **Exaustividade do `match`:** adicionar `Pickup::Armor` depois
  quebra a compilação em **todos** os lugares que precisam
  atualizar. Impossível esquecer de tratar.
- **Dados só no lugar certo:** não existe "acessar `.amount` de
  uma `Health`" — o tipo `Pickup::Health` nem tem esse campo.
- **Estados impossíveis são inexpressáveis:** um `Option<T>`
  nunca carrega um valor inválido; `None` é uma variante distinta.

### No port

- Estados do player: `PlayerState::Alive`, `Dead`, `Reborn`.
- Resultados de parsing WAD: `Result<MapData, WadError>`.
- Classificação de coisas: `MapObjectKind` como enum com dezenas
  de variantes, cada uma com `radius`, `height`, `spawnstate`,
  etc. — o compilador garante que nunca se acesse campos
  incompatíveis.
- Eventos de input: `Event::KeyDown(key)`, `Event::MouseMove(dx, dy)`,
  `Event::Quit`.

---

## 4. Traits — "estes tipos diferentes compartilham um comportamento"

### Problema

Em C, polimorfismo é feito com ponteiros de função dentro de
structs:

```c
// C — mecanismo do DOOM original
typedef struct thinker_s {
    struct thinker_s *prev, *next;
    void (*function)(struct thinker_s*);  // ponteiro de função
} thinker_t;
```

É flexível mas **sem verificação de tipos** — nada impede
registrar um thinker com `function` apontando para uma função
com assinatura errada. O bug só aparece em runtime, como *crash*.

### Traits no Rust

Um **trait** é um **contrato**. Quem implementa declara: "eu
suporto essas operações".

```rust
trait Thinker {
    fn think(&mut self, map: &mut MapData);
    fn is_removable(&self) -> bool;
}

struct DoorThinker { sector: usize, speed: Fixed, /* ... */ }
struct PlatformThinker { sector: usize, low: Fixed, high: Fixed, /* ... */ }

impl Thinker for DoorThinker {
    fn think(&mut self, map: &mut MapData) { /* move porta */ }
    fn is_removable(&self) -> bool { /* chegou ao topo? */ }
}

impl Thinker for PlatformThinker {
    fn think(&mut self, map: &mut MapData) { /* move plataforma */ }
    fn is_removable(&self) -> bool { /* terminou o ciclo? */ }
}
```

### Duas formas de uso

**Estático (monomorfização)** — o compilador gera código
especializado para cada tipo. **Zero custo de runtime**:

```rust
fn processa<T: Thinker>(t: &mut T, map: &mut MapData) {
    t.think(map);
}
```

Comparável, em performance, a chamar diretamente `DoorThinker::think`.

**Dinâmico (vtable)** — decidido em runtime, como herança virtual
em C++. Necessário quando a coleção é **heterogênea**:

```rust
let mut thinkers: Vec<Box<dyn Thinker>> = vec![
    Box::new(DoorThinker { /* ... */ }),
    Box::new(PlatformThinker { /* ... */ }),
];
for t in &mut thinkers {
    t.think(&mut map);  // despacho dinâmico, mas type-safe
}
```

A sintaxe `Box<dyn Thinker>` indica: "um ponteiro para algo que
implementa `Thinker`, alocado no heap, com vtable". É o
equivalente Rust direto ao `thinker_t*` do DOOM.

### Traits que o compilador implementa sozinho

Muitos traits têm `#[derive(...)]`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Fixed(i32);
```

- `Debug` — formatação para `println!("{:?}", ...)`.
- `Clone` / `Copy` — duplicação (posse dupla, para tipos
  pequenos).
- `PartialEq` / `Eq` — comparação com `==`.
- `Hash` — uso como chave de `HashMap`.

### Por que isso mata bugs

- O compilador **verifica** que todo `Thinker` implementa `think`
  e `is_removable`. Impossível registrar um pseudo-thinker com
  função faltando (como era fácil em C, onde um `function = NULL`
  causa SIGSEGV em runtime).
- A assinatura dos métodos é **checada** — não dá para registrar
  uma função com parâmetros trocados.

### No port

- A linked list de `thinker_t` do DOOM original virou
  `Vec<Box<dyn Thinker>>`. Portas, plataformas, elevadores e
  projéteis ativos são todos `Thinker`.
- `trait Drawable` para sprites e paredes no renderer.
- `trait SoundSource` para emissores de áudio posicional.
- Traits da standard library (`Read`, `Write`, `Iterator`) usados
  amplamente no WAD loader.

---

## Como os quatro se amarram

Os pilares não são ortogonais — eles se reforçam:

| Combinação | O que habilita |
|------------|----------------|
| Ownership + `Drop` | **RAII automático** — `WadFile` fecha sozinho, `Vec` libera memória sozinho, `Mutex` destranca no fim do escopo. |
| Ownership + Borrow checker | **Segurança sem garbage collector** — zero custo de runtime, garantias em compile-time. |
| Enums + `match` exaustivo | **Modelagem de estado impossível de representar errado** — estados válidos são os únicos expressáveis. |
| Traits + Generics | **Abstração sem custo** — `impl Iterator` compila tão rápido quanto um laço `for` cru. |
| Traits + `Box<dyn T>` | **Polimorfismo dinâmico seguro** — o substituto direto para `void*` + ponteiros de função do C. |
| `Option<T>` + Borrow checker | **Null safety** — não dá para "esquecer de checar `NULL`". |
| `Result<T, E>` + `?` | **Propagação de erro explícita e ergonômica** — sem perder a informação de erro. |

### A tese central

> **Invariantes que em C viveriam em comentários e disciplina do
> programador passam a ser checadas pelo compilador.**

Exemplos concretos desse deslocamento:

- "Este ponteiro é dono; aquele é apenas visualiza" — em C é
  convenção, em Rust é o tipo (`Box<T>` vs `&T`).
- "Este array não pode ser modificado enquanto itera" — em C é
  disciplina, em Rust o borrow checker impede.
- "Este `void*` é do tipo X quando `type == 3`" — em C é
  convenção, em Rust é `enum`.
- "Este campo pode ser `NULL`" — em C é suposição, em Rust é
  `Option<T>`.
- "Esta função pode falhar com errno=EIO" — em C é documentação,
  em Rust é `Result<T, IoError>`.

O custo é uma curva de aprendizado íngreme: o compilador rejeita
código que "pareceria funcionar" em C. O ganho é uma classe
inteira de bugs eliminada antes de rodar.

Este é o motivo de o README deste projeto usar a expressão
**"Rust idiomático"**: o port não é transcrição 1-para-1 do C —
é uma **retradução** que aproveita ownership, enums algébricos,
traits e o borrow checker para expressar as mesmas invariantes
que em C viviam em convenções e comentários.

---

## Leituras complementares

- [*The Rust Programming Language*](https://doc.rust-lang.org/book/)
  (livro oficial) — capítulos 4 (Ownership), 6 (Enums e Match),
  10 (Generics, Traits, Lifetimes).
- [*Rust by Example*](https://doc.rust-lang.org/rust-by-example/)
  — exemplos curtos de cada conceito.
- [`docs/glossary.md`](glossary.md) — glossário do port com
  termos de Rust aplicados ao DOOM.
- [`docs/architecture.md`](architecture.md) — arquitetura do
  DOOM original em C; compare com os módulos Rust em `src/`.

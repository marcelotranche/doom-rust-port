---
name: didactic-code
description: >
  Regras para produzir codigo didatico e legivel. Carrega em
  qualquer contexto de implementacao de codigo Rust neste projeto.
  Aplica-se a todo codigo produzido para o port.
---

## Regras de Codigo Didatico

### Estrutura de Arquivo
Todo arquivo .rs deve seguir esta ordem:
1. `//!` Module-level docstring em portugues explicando:
   - O que este modulo faz no contexto do DOOM
   - Qual arquivo C original corresponde a este modulo
   - Conceitos-chave que o leitor vai aprender
2. `use` imports organizados (std, external, internal)
3. Constantes e tipos auxiliares
4. Structs e enums principais (com `///` docstrings)
5. Implementacoes (`impl`)
6. Testes (`#[cfg(test)] mod tests`)

### Comentarios — Quando e Como
- **Sempre**: antes de blocos de codigo que implementam algoritmos
  do engine (BSP, rendering, colisao)
- **Sempre**: na declaracao de structs que mapeiam structs C
- **Sempre**: quando usar `unsafe`, explicar a razao de seguranca
- **Nunca**: comentarios obvios como "incrementa o contador"
- **Formato**: frases curtas e diretas, em portugues

### Exemplo de Comentario Bom
```rust
/// Percorre a BSP tree para determinar quais subsectors sao visiveis.
///
/// O DOOM usa uma BSP tree (Binary Space Partition) para dividir o mapa
/// em regioes convexas. A travessia comeca pela raiz e desce recursivamente
/// pelo lado da particao onde a camera esta, garantindo que paredes mais
/// proximas sejam desenhadas primeiro (painter's algorithm inverso).
///
/// C original: `R_RenderBSPNode()` em `r_bsp.c`, linha ~200
fn render_bsp_node(&mut self, node_id: usize) {
```

### Exemplo de Comentario Ruim
```rust
// Renderiza o no BSP  <- apenas repete o nome da funcao
fn render_bsp_node(&mut self, node_id: usize) {
```

### Referencias ao C Original
Ao portar uma funcao, incluir no docstring:
- Nome da funcao C original
- Arquivo e numero de linha aproximado
- Se o comportamento Rust difere do C, explicar por que

### Testes como Documentacao
```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Verifica que a multiplicacao fixed-point reproduz o
    /// comportamento do DOOM original: 1.5 * 2.0 = 3.0
    #[test]
    fn fixed_multiply_basic() {
        let a = Fixed::from_int(1) + Fixed(1 << 15); // 1.5
        let b = Fixed::from_int(2);                    // 2.0
        assert_eq!((a * b).to_int(), 3);
    }
}
```

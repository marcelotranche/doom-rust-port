//! # Sistema de Thinkers — Objetos que "Pensam"
//!
//! O DOOM usa um sistema de "thinkers" para gerenciar todos os objetos
//! ativos no mundo: monstros, projeteis, portas, elevadores, etc.
//!
//! No C original, thinkers formam uma lista duplamente encadeada circular.
//! Cada thinker tem um ponteiro de funcao (`think_t`) que e chamado
//! a cada tick. A remocao e "lazy": o ponteiro de funcao e setado
//! para -1, e o thinker e efetivamente removido na proxima iteracao.
//!
//! ## Equivalencia em Rust
//!
//! Em vez de linked list com function pointers (C), usamos:
//! - `trait Thinker` — define a interface de "pensar"
//! - `Vec<Box<dyn Thinker>>` — lista de thinkers com ownership
//! - Flag `active` para remocao lazy
//!
//! ## Arquivo C original: `p_tick.c`, `d_think.h`
//!
//! ## Conceitos que o leitor vai aprender
//! - Trait objects como alternativa a function pointers
//! - Padroes de iteracao com remocao lazy
//! - Game loop tick: como o DOOM atualiza todos os objetos

/// Trait para objetos que executam logica a cada tick.
///
/// Equivalente a `think_t` (function pointer) no C original.
/// Qualquer coisa que precisa ser atualizada a cada tick
/// implementa este trait: mobjs, portas, elevadores, etc.
///
/// C original: `think_t` / `actionf_t` em `d_think.h`
pub trait Thinker: std::fmt::Debug {
    /// Executa a logica de um tick para este thinker.
    ///
    /// Retorna `true` se o thinker deve continuar ativo,
    /// `false` se deve ser removido na proxima limpeza.
    ///
    /// C original: `thinker->function.acp1(thinker)` em `p_tick.c`
    fn think(&mut self) -> bool;
}

/// Lista de thinkers ativos — gerencia todos os objetos "pensantes".
///
/// C original: `thinker_t thinkercap` (lista circular) em `p_tick.c`
#[derive(Debug)]
pub struct ThinkerList {
    /// Thinkers ativos. `None` = slot marcado para remocao.
    thinkers: Vec<Option<Box<dyn Thinker>>>,
}

impl ThinkerList {
    /// Cria uma nova lista de thinkers vazia.
    ///
    /// C original: `P_InitThinkers()` em `p_tick.c`
    pub fn new() -> Self {
        ThinkerList {
            thinkers: Vec::new(),
        }
    }

    /// Adiciona um thinker ao final da lista.
    ///
    /// C original: `P_AddThinker()` em `p_tick.c`
    pub fn add(&mut self, thinker: Box<dyn Thinker>) {
        self.thinkers.push(Some(thinker));
    }

    /// Marca um thinker para remocao (lazy deletion).
    ///
    /// O thinker nao e removido imediatamente — sera limpo
    /// na proxima chamada a `run()`.
    ///
    /// C original: `P_RemoveThinker()` em `p_tick.c`
    /// (seta function.acv = -1)
    pub fn remove(&mut self, index: usize) {
        if index < self.thinkers.len() {
            self.thinkers[index] = None;
        }
    }

    /// Executa todos os thinkers ativos e remove os marcados.
    ///
    /// Itera pela lista, chama `think()` para cada thinker ativo.
    /// Se `think()` retorna `false`, marca para remocao.
    /// Ao final, remove todos os slots marcados.
    ///
    /// C original: `P_RunThinkers()` em `p_tick.c`
    pub fn run(&mut self) {
        // Executar cada thinker ativo
        for i in 0..self.thinkers.len() {
            if let Some(thinker) = &mut self.thinkers[i] {
                if !thinker.think() {
                    self.thinkers[i] = None;
                }
            }
        }

        // Remover slots vazios (compactar a lista)
        self.thinkers.retain(|t| t.is_some());
    }

    /// Limpa todos os thinkers (para novo nivel).
    ///
    /// C original: `P_InitThinkers()` em `p_tick.c`
    pub fn clear(&mut self) {
        self.thinkers.clear();
    }

    /// Retorna o numero de thinkers ativos.
    pub fn count(&self) -> usize {
        self.thinkers.iter().filter(|t| t.is_some()).count()
    }

    /// Verifica se a lista esta vazia.
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }
}

impl Default for ThinkerList {
    fn default() -> Self {
        Self::new()
    }
}

/// Tempo decorrido no nivel atual (em ticks).
///
/// Incrementado a cada tick por `P_Ticker`.
///
/// C original: `int leveltime` em `p_tick.c`
pub type LevelTime = i32;

#[cfg(test)]
mod tests {
    use super::*;

    /// Thinker de teste que conta quantas vezes foi chamado.
    #[derive(Debug)]
    struct CounterThinker {
        count: i32,
        max: i32,
    }

    impl CounterThinker {
        fn new(max: i32) -> Self {
            CounterThinker { count: 0, max }
        }
    }

    impl Thinker for CounterThinker {
        fn think(&mut self) -> bool {
            self.count += 1;
            self.count < self.max
        }
    }

    #[test]
    fn thinker_list_add_and_run() {
        let mut list = ThinkerList::new();
        list.add(Box::new(CounterThinker::new(5)));
        list.add(Box::new(CounterThinker::new(3)));
        assert_eq!(list.count(), 2);

        // Executar 2 ticks
        list.run();
        list.run();
        assert_eq!(list.count(), 2); // ambos ainda ativos

        // No tick 3, o segundo thinker (max=3) retorna false
        list.run();
        assert_eq!(list.count(), 1); // segundo removido

        // Ticks 4 e 5
        list.run();
        list.run();
        assert_eq!(list.count(), 0); // primeiro removido
    }

    #[test]
    fn thinker_list_remove() {
        let mut list = ThinkerList::new();
        list.add(Box::new(CounterThinker::new(10)));
        list.add(Box::new(CounterThinker::new(10)));
        assert_eq!(list.count(), 2);

        list.remove(0);
        assert_eq!(list.count(), 1); // marcado, nao compactado ainda

        list.run();
        assert_eq!(list.count(), 1); // compactado + tick do restante
    }

    #[test]
    fn thinker_list_clear() {
        let mut list = ThinkerList::new();
        list.add(Box::new(CounterThinker::new(10)));
        list.add(Box::new(CounterThinker::new(10)));
        list.clear();
        assert!(list.is_empty());
    }
}

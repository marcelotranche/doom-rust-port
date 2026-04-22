# Análise de Licença — doom-rust

## Restrição Legal Crítica: GPL v2

O DOOM original foi liberado pela id Software em 1997 sob **GPL v2**.

Este repositório:
1. Inclui o código-fonte C original em `references/DOOM-master/` (código GPL v2)
2. Faz referência direta a estruturas, algoritmos e lógica do C original nos comentários
3. É categoricamente um **trabalho derivado** do DOOM — mesmo reescrito em Rust

**Consequência:** A GPL v2 é "copyleft forte". Qualquer obra derivada de código GPL v2 **deve ser distribuída sob GPL v2 (ou versão compatível)**. Licenças permissivas (MIT, Apache-2.0) **não são opção** para a totalidade do port se ele é derivado do DOOM original.

## Recomendação: **GPL v2**

O id Software liberou o DOOM especificamente sob **GPL v2** (sem a cláusula "or later"). Isso significa que obras derivadas devem usar exatamente GPL v2, não podendo "upgradar" unilateralmente para GPL v3.

### Estratégia de licenciamento dual por componente

```
doom-rust/
├── src/          →  GPL v2 (obra derivada do DOOM)
├── docs/         →  CC BY 4.0 (documentação didática original)
├── references/   →  Licenças originais de cada projeto (não alteradas)
└── assets/       →  Freedoom license (BSD-style)
```

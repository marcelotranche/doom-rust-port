//! # Utilitarios e Tipos Fundamentais
//!
//! Tipos base usados por todos os outros modulos do engine:
//! matematica em ponto-fixo, angulos, bounding boxes, e numeros aleatorios.
//!
//! ## Arquivos C originais
//! - `m_fixed.c/h` — Aritmetica fixed-point 16.16
//! - `tables.c/h` — Tabelas de seno/cosseno e tipo Angle
//! - `m_bbox.c/h` — Bounding box para colisao
//! - `m_random.c/h` — Gerador de numeros pseudo-aleatorios

pub mod angle;
pub mod bbox;
pub mod fixed;
pub mod random;
pub mod tables;

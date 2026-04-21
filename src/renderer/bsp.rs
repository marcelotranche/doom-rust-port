//! # Travessia BSP e Clipping de Paredes
//!
//! O DOOM renderiza o mundo percorrendo a arvore BSP (Binary Space Partition)
//! de tras para frente. Para cada subsector visitado, os segs (segmentos de
//! parede) sao clippados contra os solid ranges ja desenhados.
//!
//! ## Algoritmo
//!
//! 1. `R_RenderBSPNode(root)` inicia a travessia recursiva
//! 2. Para cada node, determina em qual lado da partition line a camera esta
//! 3. Visita primeiro o lado da camera (mais proximo)
//! 4. Se o bounding box do outro lado esta visivel, visita tambem
//! 5. Ao chegar em um subsector (folha), processa seus segs
//!
//! ## Clipping com Solid Segments
//!
//! O array `solidsegs` mantem ranges de colunas da tela ja preenchidos
//! por paredes solidas (one-sided). Quando um novo seg e processado:
//! - Se esta inteiramente coberto por solid segs, e ignorado
//! - Se e uma parede solida, e adicionado ao array de solid segs
//! - Se e uma parede com janela (two-sided), e desenhado mas nao
//!   adicionado aos solid segs (paredes atras dele ainda podem ser vistas)
//!
//! ## Arquivo C original: `r_bsp.c`
//!
//! ## Conceitos que o leitor vai aprender
//! - Travessia BSP front-to-back para rendering
//! - Occlusion culling com ranges de colunas
//! - Backface culling via angulos

use crate::map::types::NF_SUBSECTOR;
use crate::map::MapData;
use crate::renderer::state::RenderState;
use crate::utils::angle::{Angle, ANGLETOFINESHIFT};
use crate::utils::fixed::Fixed;
use crate::video::SCREENWIDTH;

/// Segmento de parede visivel coletado durante travessia BSP.
///
/// Armazena as informacoes necessarias para renderizar o segmento
/// na fase de rendering (perspectiva e colunas).
///
/// C original: dados combinados de `curline`, `rw_angle1`, `rw_x`, `rw_stopx`
#[derive(Debug, Clone, Copy)]
pub struct WallSegment {
    /// Indice do seg no array de segs do mapa
    pub seg_index: usize,
    /// Range de colunas X na tela (x1..=x2)
    pub x1: i32,
    pub x2: i32,
    /// Angulo da camera ao primeiro vertice do seg (antes de clipping FOV).
    /// C original: `rw_angle1` em `r_segs.c`
    pub angle1: Angle,
}

/// Numero maximo de solid segments para clipping.
///
/// C original: `#define MAXSEGS 32` em `r_bsp.c`
const MAXSEGS: usize = 32;

/// Range de colunas clipado (primeiro e ultimo pixel visivel).
///
/// C original: `cliprange_t` em `r_bsp.c`
#[derive(Debug, Clone, Copy)]
pub struct ClipRange {
    pub first: i32,
    pub last: i32,
}

/// Estado da travessia BSP — solid segments e drawsegs.
///
/// Encapsula o estado global de clipping que no C original eram
/// globals em `r_bsp.c`.
#[derive(Debug)]
pub struct BspTraversal {
    /// Array de solid segments (ranges de colunas ja preenchidos).
    /// C original: `cliprange_t solidsegs[MAXSEGS]` em `r_bsp.c`
    solid_segs: Vec<ClipRange>,

    /// Segmentos de parede visiveis coletados para rendering.
    /// Cada segmento contem o indice do seg e o range de colunas.
    pub wall_ranges: Vec<WallSegment>,

    /// Indice do seg sendo processado no momento.
    /// C original: `curline` em `r_bsp.c`
    current_seg_index: usize,

    /// Angulo da camera ao v1 do seg atual (antes de FOV clipping).
    /// C original: `rw_angle1` em `r_segs.c`
    current_angle1: Angle,
}

impl BspTraversal {
    /// Cria um novo estado de travessia BSP.
    pub fn new() -> Self {
        BspTraversal {
            solid_segs: Vec::with_capacity(MAXSEGS),
            wall_ranges: Vec::new(),
            current_seg_index: 0,
            current_angle1: Angle::ANG0,
        }
    }

    /// Limpa os solid segments para o inicio de um novo frame.
    ///
    /// Inicializa com dois sentinelas:
    /// - Um range cobrindo tudo a esquerda da tela (x < 0)
    /// - Um range cobrindo tudo a direita da tela (x >= viewwidth)
    ///
    /// C original: `R_ClearClipSegs()` em `r_bsp.c`
    pub fn clear_clip_segs(&mut self, view_width: i32) {
        self.solid_segs.clear();
        self.solid_segs.push(ClipRange {
            first: i32::MIN,
            last: -1,
        });
        self.solid_segs.push(ClipRange {
            first: view_width,
            last: i32::MAX,
        });
        self.wall_ranges.clear();
    }

    /// Renderiza a cena percorrendo a arvore BSP.
    ///
    /// Ponto de entrada do rendering: inicia a travessia recursiva
    /// a partir do node raiz da BSP tree.
    ///
    /// C original: chamado por `R_RenderPlayerView()` em `r_main.c`
    pub fn render_bsp(&mut self, map: &MapData, state: &RenderState) {
        if map.nodes.is_empty() {
            return;
        }
        self.clear_clip_segs(SCREENWIDTH as i32);
        self.render_bsp_node(map, state, (map.nodes.len() - 1) as u16);
    }

    /// Travessia recursiva da BSP tree.
    ///
    /// Para cada node:
    /// 1. Determina em qual lado da partition line a camera esta
    /// 2. Visita primeiro o lado mais proximo (front)
    /// 3. Se o bounding box do lado de tras esta visivel, visita tambem
    ///
    /// C original: `R_RenderBSPNode()` em `r_bsp.c`
    fn render_bsp_node(&mut self, map: &MapData, state: &RenderState, node_id: u16) {
        // Se e um subsector (folha da BSP), processar diretamente
        if node_id & NF_SUBSECTOR != 0 {
            let subsector_id = (node_id & !NF_SUBSECTOR) as usize;
            self.subsector(map, state, subsector_id);
            return;
        }

        let node = &map.nodes[node_id as usize];

        // Determinar em qual lado da partition line a camera esta
        let side = RenderState::point_on_side(state.viewx, state.viewy, node);

        // Visitar primeiro o lado da camera (mais proximo)
        self.render_bsp_node(map, state, node.children[side]);

        // Se o bounding box do outro lado esta visivel, visitar tambem
        let other_side = side ^ 1;
        if self.check_bbox(&node.bbox[other_side], state) {
            self.render_bsp_node(map, state, node.children[other_side]);
        }
    }

    /// Processa um subsector (folha da BSP tree).
    ///
    /// Para cada seg do subsector, tenta adiciona-lo a lista de paredes
    /// visiveis, clippando contra solid segs existentes.
    ///
    /// C original: `R_Subsector()` em `r_bsp.c`
    fn subsector(&mut self, map: &MapData, state: &RenderState, subsector_id: usize) {
        if subsector_id >= map.subsectors.len() {
            return;
        }

        let ss = &map.subsectors[subsector_id];

        for i in 0..ss.num_lines {
            let seg_index = ss.first_line + i;
            if seg_index < map.segs.len() {
                self.add_line(map, state, seg_index);
            }
        }
    }

    /// Clippa e adiciona um seg a lista de paredes visiveis.
    ///
    /// 1. Calcula os angulos dos dois vertices do seg relativos a camera
    /// 2. Faz backface culling (descarta segs virados para longe)
    /// 3. Clippa contra o FOV (field of view)
    /// 4. Converte angulos para colunas X na tela
    /// 5. Clippa contra solid segs existentes
    ///
    /// C original: `R_AddLine()` em `r_bsp.c`
    fn add_line(&mut self, map: &MapData, state: &RenderState, seg_index: usize) {
        let seg = &map.segs[seg_index];
        let v1 = &map.vertexes[seg.v1];
        let v2 = &map.vertexes[seg.v2];

        // Calcular angulos dos vertices
        let angle1 = state.point_to_angle(v1.x, v1.y);
        let angle2 = state.point_to_angle(v2.x, v2.y);

        // Guardar contexto para store_wall_range
        // C original: rw_angle1 = angle1 (antes do clipping FOV)
        self.current_seg_index = seg_index;
        self.current_angle1 = angle1;

        // Backface culling: se o span >= 180 graus, esta virado para longe
        let span = angle1 - angle2;
        if span.0 >= Angle::ANG180.0 {
            return;
        }

        // Clippar contra o FOV
        let mut a1 = angle1 - state.viewangle;
        let mut a2 = angle2 - state.viewangle;

        let clip2 = state.clipangle + state.clipangle; // 2 * clipangle

        let tspan = a1 + state.clipangle;
        if tspan.0 > clip2.0 {
            let excess = tspan - clip2;
            if excess.0 >= span.0 {
                return; // Totalmente fora do lado esquerdo
            }
            a1 = state.clipangle;
        }

        let tspan = state.clipangle - a2;
        if tspan.0 > clip2.0 {
            let excess = tspan - clip2;
            if excess.0 >= span.0 {
                return; // Totalmente fora do lado direito
            }
            a2 = Angle(0u32.wrapping_sub(state.clipangle.0));
        }

        // Converter angulos para colunas X na tela
        let angletox = |angle: Angle| -> i32 {
            let fine = (angle.0 >> 19) as usize;
            if fine < state.viewangletox.len() {
                state.viewangletox[fine]
            } else {
                0
            }
        };

        let x1 = angletox(a1 + Angle::ANG90);
        let x2 = angletox(a2 + Angle::ANG90);

        // "Does not cross a pixel?" — C original: if (x1 == x2) return;
        if x1 == x2 {
            return;
        }

        // Determinar se e parede solida, porta fechada, janela, ou trigger
        // C original: R_AddLine linhas 320-356 em r_bsp.c
        if let Some(back_idx) = seg.back_sector {
            let front = &map.sectors[seg.front_sector];
            let back = &map.sectors[back_idx];

            // Porta fechada: back ceiling <= front floor ou back floor >= front ceiling
            if back.ceiling_height.0 <= front.floor_height.0
                || back.floor_height.0 >= front.ceiling_height.0
            {
                self.clip_solid_wall_segment(x1, x2 - 1);
            }
            // Janela: alturas diferentes
            else if back.ceiling_height != front.ceiling_height
                || back.floor_height != front.floor_height
            {
                self.clip_pass_wall_segment(x1, x2 - 1);
            }
            // Mesmas alturas: rejeitar APENAS se tudo for identico
            // (textures iguais, luz igual, sem mid texture)
            // C original: r_bsp.c linhas 336-347
            else {
                let sidedef = &map.sidedefs[seg.sidedef];
                let has_mid = sidedef.mid_texture[0] != b'-' && sidedef.mid_texture[0] != 0;
                if back.ceiling_pic != front.ceiling_pic
                    || back.floor_pic != front.floor_pic
                    || back.light_level != front.light_level
                    || has_mid
                {
                    self.clip_pass_wall_segment(x1, x2 - 1);
                }
                // Tudo identico e sem mid texture — trigger line, ignorar
            }
        } else {
            // One-sided = parede solida
            self.clip_solid_wall_segment(x1, x2 - 1);
        }
    }

    /// Clippa um segmento de parede solida (one-sided) e adiciona aos solid segs.
    ///
    /// Paredes solidas bloqueiam completamente a visao — apos serem desenhadas,
    /// nenhuma parede atras delas precisa ser processada naquela faixa de colunas.
    ///
    /// C original: `R_ClipSolidWallSegment()` em `r_bsp.c`
    fn clip_solid_wall_segment(&mut self, first: i32, last: i32) {
        // Encontrar o primeiro range que toca este segmento
        let mut start_idx = 0;
        while start_idx < self.solid_segs.len() && self.solid_segs[start_idx].last < first - 1 {
            start_idx += 1;
        }

        if start_idx >= self.solid_segs.len() {
            return;
        }

        if first < self.solid_segs[start_idx].first {
            if last < self.solid_segs[start_idx].first - 1 {
                // Segmento totalmente visivel, inserir novo range
                self.store_wall_range(first, last);
                self.solid_segs.insert(start_idx, ClipRange { first, last });
                return;
            }

            // Fragmento visivel acima do start
            self.store_wall_range(first, self.solid_segs[start_idx].first - 1);
            self.solid_segs[start_idx].first = first;
        }

        if last <= self.solid_segs[start_idx].last {
            return; // Totalmente contido
        }

        // Processar fragmentos entre ranges adjacentes
        let mut next_idx = start_idx;
        while next_idx + 1 < self.solid_segs.len()
            && last >= self.solid_segs[next_idx + 1].first - 1
        {
            self.store_wall_range(
                self.solid_segs[next_idx].last + 1,
                self.solid_segs[next_idx + 1].first - 1,
            );
            next_idx += 1;

            if last <= self.solid_segs[next_idx].last {
                self.solid_segs[start_idx].last = self.solid_segs[next_idx].last;
                // Remover ranges intermediarios
                if next_idx > start_idx {
                    let remove_start = start_idx + 1;
                    let remove_end = next_idx + 1;
                    self.solid_segs.drain(remove_start..remove_end);
                }
                return;
            }
        }

        // Fragmento visivel apos o ultimo range
        self.store_wall_range(self.solid_segs[next_idx].last + 1, last);
        self.solid_segs[start_idx].last = last;

        // Remover ranges intermediarios
        if next_idx > start_idx {
            let remove_start = start_idx + 1;
            let remove_end = next_idx + 1;
            self.solid_segs.drain(remove_start..remove_end);
        }
    }

    /// Clippa um segmento de parede com janela (two-sided).
    ///
    /// Paredes two-sided nao bloqueiam a visao — sao desenhadas mas
    /// nao adicionadas aos solid segs. Paredes atras delas podem
    /// ser visiveis (ex: janelas, pilares, portais).
    ///
    /// C original: `R_ClipPassWallSegment()` em `r_bsp.c`
    fn clip_pass_wall_segment(&mut self, first: i32, last: i32) {
        let mut start_idx = 0;
        while start_idx < self.solid_segs.len() && self.solid_segs[start_idx].last < first - 1 {
            start_idx += 1;
        }

        if start_idx >= self.solid_segs.len() {
            return;
        }

        if first < self.solid_segs[start_idx].first {
            if last < self.solid_segs[start_idx].first - 1 {
                // Totalmente visivel
                self.store_wall_range(first, last);
                return;
            }
            // Fragmento visivel
            self.store_wall_range(first, self.solid_segs[start_idx].first - 1);
        }

        if last <= self.solid_segs[start_idx].last {
            return;
        }

        while start_idx + 1 < self.solid_segs.len()
            && last >= self.solid_segs[start_idx + 1].first - 1
        {
            self.store_wall_range(
                self.solid_segs[start_idx].last + 1,
                self.solid_segs[start_idx + 1].first - 1,
            );
            start_idx += 1;

            if last <= self.solid_segs[start_idx].last {
                return;
            }
        }

        self.store_wall_range(self.solid_segs[start_idx].last + 1, last);
    }

    /// Armazena um range de parede visivel para rendering posterior.
    ///
    /// Coleta o segmento com indice do seg e angulo para que o
    /// rendering possa calcular perspectiva e texturas.
    ///
    /// C original: `R_StoreWallRange()` em `r_segs.c`
    fn store_wall_range(&mut self, start: i32, stop: i32) {
        if start <= stop {
            self.wall_ranges.push(WallSegment {
                seg_index: self.current_seg_index,
                x1: start,
                x2: stop,
                angle1: self.current_angle1,
            });
        }
    }

    /// Verifica se um bounding box e potencialmente visivel.
    ///
    /// Calcula os angulos dos cantos do bbox relativos a camera,
    /// clippa contra o FOV, converte para colunas X, e verifica
    /// contra os solid segs existentes.
    ///
    /// C original: `R_CheckBBox()` em `r_bsp.c`
    fn check_bbox(&self, bbox: &[Fixed; 4], state: &RenderState) -> bool {
        // bbox: [top(0), bottom(1), left(2), right(3)]
        // Tabela de lookup dos cantos: C original `checkcoord[12][4]`
        // Indices: boxx = 0(left), 1(inside), 2(right)
        //          boxy = 0(top), 1(inside), 2(bottom)
        //          boxpos = boxy*4 + boxx
        const CHECKCOORD: [[usize; 4]; 12] = [
            [3, 0, 2, 1], // boxpos=0: right,top -> left,bottom
            [3, 0, 2, 0], // boxpos=1: right,top -> left,top
            [3, 1, 2, 0], // boxpos=2: right,bottom -> left,top
            [0, 0, 0, 0], // boxpos=3: unused
            [2, 0, 2, 1], // boxpos=4: left,top -> left,bottom
            [0, 0, 0, 0], // boxpos=5: inside (handled separately)
            [3, 1, 3, 0], // boxpos=6: right,bottom -> right,top
            [0, 0, 0, 0], // boxpos=7: unused
            [2, 0, 3, 1], // boxpos=8: left,top -> right,bottom
            [2, 1, 3, 1], // boxpos=9: left,bottom -> right,bottom
            [2, 1, 3, 0], // boxpos=10: left,bottom -> right,top
            [0, 0, 0, 0], // boxpos=11: unused
        ];

        let boxx = if state.viewx.0 <= bbox[2].0 {
            0 // left of bbox
        } else if state.viewx.0 < bbox[3].0 {
            1 // inside bbox horizontally
        } else {
            2 // right of bbox
        };

        let boxy = if state.viewy.0 >= bbox[0].0 {
            0 // above bbox
        } else if state.viewy.0 > bbox[1].0 {
            1 // inside bbox vertically
        } else {
            2 // below bbox
        };

        let boxpos = boxy * 4 + boxx;
        if boxpos == 5 {
            return true; // Camera dentro do bbox
        }

        let cc = &CHECKCOORD[boxpos];
        let x1 = bbox[cc[0]];
        let y1 = bbox[cc[1]];
        let x2 = bbox[cc[2]];
        let y2 = bbox[cc[3]];

        let mut angle1 = state.point_to_angle(x1, y1) - state.viewangle;
        let angle2_raw = state.point_to_angle(x2, y2) - state.viewangle;
        let mut angle2 = angle2_raw;

        let span = angle1 - angle2;

        // Sitting on a line?
        if span.0 >= Angle::ANG180.0 {
            return true;
        }

        let clip2 = state.clipangle + state.clipangle;

        // Clip lado esquerdo
        let tspan = angle1 + state.clipangle;
        if tspan.0 > clip2.0 {
            let excess = tspan - clip2;
            if excess.0 >= span.0 {
                return false; // Totalmente fora do FOV esquerdo
            }
            angle1 = state.clipangle;
        }

        // Clip lado direito
        let tspan = state.clipangle - angle2;
        if tspan.0 > clip2.0 {
            let excess = tspan - clip2;
            if excess.0 >= span.0 {
                return false; // Totalmente fora do FOV direito
            }
            angle2 = Angle(0u32.wrapping_sub(state.clipangle.0));
        }

        // Converter angulos para colunas X na tela
        let sx1_idx = ((angle1 + Angle::ANG90).0 >> ANGLETOFINESHIFT) as usize;
        let sx2_idx = ((angle2 + Angle::ANG90).0 >> ANGLETOFINESHIFT) as usize;
        let sx1 = if sx1_idx < state.viewangletox.len() {
            state.viewangletox[sx1_idx]
        } else {
            0
        };
        let sx2 = if sx2_idx < state.viewangletox.len() {
            state.viewangletox[sx2_idx]
        } else {
            0
        };

        // Does not cross a pixel
        if sx1 == sx2 {
            return false;
        }
        let sx2 = sx2 - 1;

        // Verificar contra solid segs
        // Se o range inteiro [sx1, sx2] esta coberto por um unico solid seg,
        // entao esta totalmente ocluido
        for seg in &self.solid_segs {
            if seg.last >= sx2 {
                return !(sx1 >= seg.first && sx2 <= seg.last);
            }
        }

        true
    }
}

impl Default for BspTraversal {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifica que clear_clip_segs inicializa com 2 sentinelas.
    #[test]
    fn clear_clip_segs_init() {
        let mut bsp = BspTraversal::new();
        bsp.clear_clip_segs(320);

        assert_eq!(bsp.solid_segs.len(), 2);
        assert_eq!(bsp.solid_segs[0].first, i32::MIN);
        assert_eq!(bsp.solid_segs[0].last, -1);
        assert_eq!(bsp.solid_segs[1].first, 320);
        assert_eq!(bsp.solid_segs[1].last, i32::MAX);
    }

    /// Verifica clip_solid: segmento totalmente visivel e inserido.
    #[test]
    fn clip_solid_fully_visible() {
        let mut bsp = BspTraversal::new();
        bsp.clear_clip_segs(320);

        bsp.clip_solid_wall_segment(10, 20);

        // Deve ter 3 ranges: sentinela, novo, sentinela
        assert_eq!(bsp.solid_segs.len(), 3);
        assert_eq!(bsp.solid_segs[1].first, 10);
        assert_eq!(bsp.solid_segs[1].last, 20);
        assert_eq!(bsp.wall_ranges.len(), 1);
        assert_eq!(bsp.wall_ranges[0].x1, 10);
        assert_eq!(bsp.wall_ranges[0].x2, 20);
    }

    /// Verifica clip_solid: dois segmentos sem sobreposicao.
    #[test]
    fn clip_solid_two_segments() {
        let mut bsp = BspTraversal::new();
        bsp.clear_clip_segs(320);

        bsp.clip_solid_wall_segment(10, 20);
        bsp.clip_solid_wall_segment(30, 40);

        assert_eq!(bsp.solid_segs.len(), 4);
        assert_eq!(bsp.wall_ranges.len(), 2);
    }

    /// Verifica clip_solid: segmento sobreposto e ignorado.
    #[test]
    fn clip_solid_overlapping() {
        let mut bsp = BspTraversal::new();
        bsp.clear_clip_segs(320);

        bsp.clip_solid_wall_segment(10, 20);
        bsp.clip_solid_wall_segment(12, 18); // dentro do range anterior

        // Segundo segmento nao gera novo wall range (totalmente coberto)
        assert_eq!(bsp.wall_ranges.len(), 1);
    }

    /// Verifica clip_pass: segmento visivel mas nao bloqueia.
    #[test]
    fn clip_pass_visible() {
        let mut bsp = BspTraversal::new();
        bsp.clear_clip_segs(320);

        bsp.clip_pass_wall_segment(10, 20);

        // Pass wall nao adiciona solid seg
        assert_eq!(bsp.solid_segs.len(), 2); // apenas sentinelas
        assert_eq!(bsp.wall_ranges.len(), 1);
    }

    /// Verifica clip_pass: segmento parcialmente coberto.
    #[test]
    fn clip_pass_partially_covered() {
        let mut bsp = BspTraversal::new();
        bsp.clear_clip_segs(320);

        // Primeiro adicionar parede solida
        bsp.clip_solid_wall_segment(10, 20);
        // Depois tentar pass wall que sobrepoe
        bsp.clip_pass_wall_segment(5, 25);

        // Deve gerar dois fragmentos visiveis: 5-9 e 21-25
        assert_eq!(bsp.wall_ranges.len(), 3); // 1 solid + 2 pass fragments
    }
}

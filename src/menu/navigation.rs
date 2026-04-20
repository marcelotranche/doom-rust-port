//! # Sistema de Menus — Navegacao, Input e Rendering
//!
//! O sistema de menus do DOOM e uma hierarquia de paginas navegaveis:
//!
//! ```text
//! MainMenu
//!   +-> New Game  -> EpisodeMenu -> SkillMenu -> iniciar jogo
//!   +-> Options   -> volume, controles, tamanho de tela
//!   +-> Load Game -> 6 slots de save
//!   +-> Save Game -> 6 slots de save (com input de nome)
//!   +-> Quit Game -> confirmacao
//! ```
//!
//! ## Skull cursor
//!
//! O cursor e um craneo animado que alterna entre 2 frames
//! a cada 8 ticks (SKULLANIMCOUNT). O craneo e desenhado
//! a esquerda do item selecionado (offset SKULLXOFF = -32).
//!
//! ## Menu items
//!
//! Cada item tem um `status`:
//! - 0 = nao selecionavel (espacador/titulo)
//! - 1 = selecionavel normalmente (ativa callback ao pressionar Enter)
//! - 2 = slider/esquerda-direita (setas mudam valor)
//!
//! ## Mensagens modais
//!
//! O menu pode exibir mensagens modais sobrepostas ("Are you sure
//! you want to quit?") que capturam input ate o jogador responder.
//!
//! ## Arquivo C original: `m_menu.c`, `m_menu.h`
//!
//! ## Conceitos que o leitor vai aprender
//! - Menu como hierarquia com back-navigation
//! - Skull cursor com animacao por timer
//! - Item status para tipos diferentes de interacao
//! - Mensagens modais com callback

/// Offset X do skull cursor (a esquerda do item).
///
/// C original: `#define SKULLXOFF -32` em `m_menu.c`
pub const SKULLXOFF: i32 = -32;

/// Altura de cada linha de menu.
///
/// C original: `#define LINEHEIGHT 16` em `m_menu.c`
pub const LINEHEIGHT: i32 = 16;

/// Ticks entre frames do skull cursor.
///
/// C original: `8` (hardcoded em `M_Ticker`)
pub const SKULLANIMCOUNT: i32 = 8;

/// Tamanho maximo do nome de save game.
///
/// C original: `#define SAVESTRINGSIZE 24` em `m_menu.c`
pub const SAVESTRINGSIZE: usize = 24;

/// Numero de slots de save game.
///
/// C original: `6` (hardcoded nos arrays)
pub const NUM_SAVE_SLOTS: usize = 6;

// ---------------------------------------------------------------------------
// MenuItem
// ---------------------------------------------------------------------------

/// Status de um item de menu.
///
/// C original: `short status` em `menuitem_t`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItemStatus {
    /// Nao selecionavel (espacador, titulo decorativo)
    Inactive = 0,
    /// Selecionavel — Enter ativa a callback
    Selectable = 1,
    /// Slider — setas esquerda/direita mudam o valor
    Slider = 2,
}

/// Acao executada quando um item de menu e ativado.
///
/// C original: `void (*routine)(int choice)` em `menuitem_t`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    /// Nenhuma acao
    None,
    /// Iniciar novo jogo
    NewGame,
    /// Selecionar episodio
    ChooseEpisode,
    /// Selecionar dificuldade
    ChooseSkill,
    /// Carregar save game
    LoadGame,
    /// Salvar save game
    SaveGame,
    /// Abrir submenu de opcoes
    Options,
    /// Alterar volume de SFX
    SfxVolume,
    /// Alterar volume de musica
    MusicVolume,
    /// Alterar tamanho da tela
    ScreenSize,
    /// Sair do jogo
    QuitGame,
    /// Abrir submenu
    SubMenu,
}

/// Item de menu — uma entrada selecionavel na pagina.
///
/// C original: `menuitem_t` em `m_menu.c`
#[derive(Debug, Clone)]
pub struct MenuItem {
    /// Status do item (inativo, selecionavel, slider)
    pub status: MenuItemStatus,
    /// Nome do lump do WAD para o grafico deste item
    pub name: &'static str,
    /// Acao a executar quando ativado
    pub action: MenuAction,
    /// Tecla de atalho (hotkey)
    pub alpha_key: u8,
}

impl MenuItem {
    /// Cria um item de menu selecionavel.
    pub const fn selectable(name: &'static str, action: MenuAction, key: u8) -> Self {
        MenuItem {
            status: MenuItemStatus::Selectable,
            name,
            action,
            alpha_key: key,
        }
    }

    /// Cria um item de menu slider.
    pub const fn slider(name: &'static str, action: MenuAction, key: u8) -> Self {
        MenuItem {
            status: MenuItemStatus::Slider,
            name,
            action,
            alpha_key: key,
        }
    }

    /// Cria um item de menu inativo (espacador).
    pub const fn inactive(name: &'static str) -> Self {
        MenuItem {
            status: MenuItemStatus::Inactive,
            name,
            action: MenuAction::None,
            alpha_key: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Menu (pagina)
// ---------------------------------------------------------------------------

/// Pagina de menu — contem uma lista de items e metadados de layout.
///
/// Cada menu tem um ponteiro para o menu anterior (para back-navigation
/// com Backspace/Escape) e uma posicao de tela.
///
/// C original: `menu_t` em `m_menu.c`
#[derive(Debug, Clone)]
pub struct Menu {
    /// Items do menu
    pub items: Vec<MenuItem>,
    /// Indice do menu anterior (None = menu raiz)
    pub prev_menu: Option<usize>,
    /// Posicao X de desenho
    pub x: i32,
    /// Posicao Y de desenho
    pub y: i32,
    /// Ultimo item selecionado (restaurado ao voltar)
    pub last_on: usize,
}

impl Menu {
    /// Cria um novo menu.
    pub fn new(items: Vec<MenuItem>, prev_menu: Option<usize>, x: i32, y: i32) -> Self {
        Menu {
            items,
            prev_menu,
            x,
            y,
            last_on: 0,
        }
    }

    /// Retorna o numero de items selecionaveis.
    pub fn selectable_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.status != MenuItemStatus::Inactive)
            .count()
    }
}

// ---------------------------------------------------------------------------
// MenuSystem
// ---------------------------------------------------------------------------

/// Sistema de menus do DOOM — gerencia navegacao e estado.
///
/// C original: globals `currentMenu`, `itemOn`, `menuactive`,
/// `skullAnimCounter`, `whichSkull`, etc. em `m_menu.c`
#[derive(Debug)]
pub struct MenuSystem {
    /// Todos os menus registrados
    pub menus: Vec<Menu>,
    /// Indice do menu ativo
    pub current_menu: usize,
    /// Indice do item selecionado no menu atual
    pub item_on: usize,
    /// Se o menu esta ativo (visivel)
    pub active: bool,
    /// Frame do skull cursor (0 ou 1)
    pub skull_frame: usize,
    /// Contador para animacao do skull
    pub skull_anim_counter: i32,
    /// Se ha mensagem modal para exibir
    pub message_showing: bool,
    /// Texto da mensagem modal
    pub message_text: String,
    /// Se a mensagem precisa de input (y/n)
    pub message_needs_input: bool,
    /// Se estamos no modo de edicao de save string
    pub save_string_enter: bool,
    /// Slot de save sendo editado
    pub save_slot: usize,
    /// Nomes dos save games
    pub save_strings: [String; NUM_SAVE_SLOTS],
    /// Ultima acao pendente (consumida pelo game loop)
    last_action: Option<MenuAction>,
    /// Ultima resposta de mensagem modal
    last_message_response: Option<bool>,
}

impl MenuSystem {
    /// Cria o sistema de menus com a hierarquia padrao do DOOM.
    ///
    /// C original: `M_Init()` em `m_menu.c`
    pub fn new() -> Self {
        let menus = Self::create_default_menus();
        let save_strings = [
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ];

        MenuSystem {
            menus,
            current_menu: 0, // main menu
            item_on: 0,
            active: false,
            skull_frame: 0,
            skull_anim_counter: SKULLANIMCOUNT,
            message_showing: false,
            message_text: String::new(),
            message_needs_input: false,
            save_string_enter: false,
            save_slot: 0,
            save_strings,
            last_action: None,
            last_message_response: None,
        }
    }

    /// Cria a hierarquia de menus padrao do DOOM.
    fn create_default_menus() -> Vec<Menu> {
        // 0: Main Menu
        let main_menu = Menu::new(
            vec![
                MenuItem::selectable("M_NGAME", MenuAction::SubMenu, b'n'),
                MenuItem::selectable("M_OPTION", MenuAction::SubMenu, b'o'),
                MenuItem::selectable("M_LOADG", MenuAction::SubMenu, b'l'),
                MenuItem::selectable("M_SAVEG", MenuAction::SubMenu, b's'),
                MenuItem::selectable("M_QUITG", MenuAction::QuitGame, b'q'),
            ],
            None, // menu raiz
            97,
            64,
        );

        // 1: Episode Menu
        let episode_menu = Menu::new(
            vec![
                MenuItem::selectable("M_EPI1", MenuAction::ChooseEpisode, b'k'),
                MenuItem::selectable("M_EPI2", MenuAction::ChooseEpisode, b't'),
                MenuItem::selectable("M_EPI3", MenuAction::ChooseEpisode, b'i'),
                MenuItem::selectable("M_EPI4", MenuAction::ChooseEpisode, b't'),
            ],
            Some(0), // volta ao main
            48,
            63,
        );

        // 2: Skill Menu
        let skill_menu = Menu::new(
            vec![
                MenuItem::selectable("M_JKILL", MenuAction::ChooseSkill, b'i'),
                MenuItem::selectable("M_ROUGH", MenuAction::ChooseSkill, b'h'),
                MenuItem::selectable("M_HURT", MenuAction::ChooseSkill, b'h'),
                MenuItem::selectable("M_ULTRA", MenuAction::ChooseSkill, b'u'),
                MenuItem::selectable("M_NMARE", MenuAction::ChooseSkill, b'n'),
            ],
            Some(1), // volta ao episode
            48,
            63,
        );

        // 3: Options Menu
        let options_menu = Menu::new(
            vec![
                MenuItem::inactive("M_ENDGAM"),
                MenuItem::selectable("M_MESSG", MenuAction::Options, b'm'),
                MenuItem::slider("M_DETAIL", MenuAction::Options, b'g'),
                MenuItem::slider("M_SCRNSZ", MenuAction::ScreenSize, b's'),
                MenuItem::inactive(""),
                MenuItem::slider("M_SFXVOL", MenuAction::SfxVolume, b's'),
                MenuItem::slider("M_MUSVOL", MenuAction::MusicVolume, b'm'),
            ],
            Some(0), // volta ao main
            60,
            37,
        );

        // 4: Load Game Menu
        let load_menu = Menu::new(
            vec![
                MenuItem::selectable("", MenuAction::LoadGame, b'1'),
                MenuItem::selectable("", MenuAction::LoadGame, b'2'),
                MenuItem::selectable("", MenuAction::LoadGame, b'3'),
                MenuItem::selectable("", MenuAction::LoadGame, b'4'),
                MenuItem::selectable("", MenuAction::LoadGame, b'5'),
                MenuItem::selectable("", MenuAction::LoadGame, b'6'),
            ],
            Some(0), // volta ao main
            80,
            54,
        );

        // 5: Save Game Menu
        let save_menu = Menu::new(
            vec![
                MenuItem::selectable("", MenuAction::SaveGame, b'1'),
                MenuItem::selectable("", MenuAction::SaveGame, b'2'),
                MenuItem::selectable("", MenuAction::SaveGame, b'3'),
                MenuItem::selectable("", MenuAction::SaveGame, b'4'),
                MenuItem::selectable("", MenuAction::SaveGame, b'5'),
                MenuItem::selectable("", MenuAction::SaveGame, b'6'),
            ],
            Some(0), // volta ao main
            80,
            54,
        );

        vec![
            main_menu,     // 0
            episode_menu,  // 1
            skill_menu,    // 2
            options_menu,  // 3
            load_menu,     // 4
            save_menu,     // 5
        ]
    }

    /// Abre o menu principal.
    ///
    /// C original: `M_StartControlPanel()` em `m_menu.c`
    pub fn open(&mut self) {
        self.active = true;
        self.current_menu = 0;
        self.item_on = self.menus[0].last_on;
    }

    /// Fecha o menu.
    pub fn close(&mut self) {
        self.active = false;
        self.message_showing = false;
        self.save_string_enter = false;
    }

    /// Atualiza o menu a cada tick.
    ///
    /// Anima o skull cursor alternando entre frames.
    ///
    /// C original: `M_Ticker()` em `m_menu.c`
    pub fn ticker(&mut self) {
        self.skull_anim_counter -= 1;
        if self.skull_anim_counter <= 0 {
            self.skull_frame = 1 - self.skull_frame; // alterna 0/1
            self.skull_anim_counter = SKULLANIMCOUNT;
        }
    }

    /// Processa um evento de input no menu.
    ///
    /// Retorna `true` se o evento foi consumido pelo menu.
    ///
    /// C original: `M_Responder()` em `m_menu.c`
    pub fn responder(&mut self, key: u8, key_down: bool) -> bool {
        if !key_down {
            return false;
        }

        // Mensagem modal captura tudo
        if self.message_showing {
            return self.handle_message_input(key);
        }

        // Edicao de nome de save
        if self.save_string_enter {
            return self.handle_save_input(key);
        }

        if !self.active {
            // F-keys funcionam mesmo com menu fechado
            return self.handle_fkeys(key);
        }

        self.handle_menu_input(key)
    }

    /// Processa input de navegacao normal do menu.
    fn handle_menu_input(&mut self, key: u8) -> bool {
        let menu = &self.menus[self.current_menu];
        let num_items = menu.items.len();

        match key {
            // Seta para baixo
            0xad => {
                // Proximo item selecionavel
                loop {
                    self.item_on = (self.item_on + 1) % num_items;
                    if self.menus[self.current_menu].items[self.item_on].status
                        != MenuItemStatus::Inactive
                    {
                        break;
                    }
                }
                true
            }

            // Seta para cima
            0xae => {
                // Item anterior selecionavel
                loop {
                    self.item_on = if self.item_on == 0 {
                        num_items - 1
                    } else {
                        self.item_on - 1
                    };
                    if self.menus[self.current_menu].items[self.item_on].status
                        != MenuItemStatus::Inactive
                    {
                        break;
                    }
                }
                true
            }

            // Enter — ativar item
            13 => {
                let item = &self.menus[self.current_menu].items[self.item_on];
                if item.status != MenuItemStatus::Inactive {
                    self.last_action = Some(item.action);
                }
                true
            }

            // Escape — voltar/fechar
            27 => {
                self.menus[self.current_menu].last_on = self.item_on;
                if let Some(prev) = self.menus[self.current_menu].prev_menu {
                    self.current_menu = prev;
                    self.item_on = self.menus[prev].last_on;
                } else {
                    self.close();
                }
                true
            }

            // Backspace — voltar ao menu anterior
            127 => {
                self.menus[self.current_menu].last_on = self.item_on;
                if let Some(prev) = self.menus[self.current_menu].prev_menu {
                    self.current_menu = prev;
                    self.item_on = self.menus[prev].last_on;
                }
                true
            }

            // Hotkey
            _ => {
                let key_lower = key.to_ascii_lowercase();
                for (i, item) in self.menus[self.current_menu].items.iter().enumerate() {
                    if item.alpha_key == key_lower && item.status != MenuItemStatus::Inactive {
                        self.item_on = i;
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Processa input durante mensagem modal.
    fn handle_message_input(&mut self, key: u8) -> bool {
        if self.message_needs_input {
            // Precisa de y/n
            if key == b'y' || key == b'Y' {
                self.message_showing = false;
                self.last_message_response = Some(true);
            } else if key == b'n' || key == b'N' || key == 27 {
                self.message_showing = false;
                self.last_message_response = Some(false);
            }
        } else {
            // Qualquer tecla fecha a mensagem
            self.message_showing = false;
        }
        true
    }

    /// Processa input durante edicao de nome de save.
    fn handle_save_input(&mut self, key: u8) -> bool {
        match key {
            // Enter — confirmar
            13 => {
                self.save_string_enter = false;
                self.last_action = Some(MenuAction::SaveGame);
                true
            }

            // Escape — cancelar
            27 => {
                self.save_string_enter = false;
                true
            }

            // Backspace — deletar caractere
            127 | 8 => {
                if let Some(s) = self.save_strings.get_mut(self.save_slot) {
                    s.pop();
                }
                true
            }

            // Caractere imprimivel
            32..=126 => {
                if let Some(s) = self.save_strings.get_mut(self.save_slot) {
                    if s.len() < SAVESTRINGSIZE {
                        s.push(key as char);
                    }
                }
                true
            }

            _ => true,
        }
    }

    /// Processa F-keys (funcionam sem menu aberto).
    fn handle_fkeys(&mut self, _key: u8) -> bool {
        // TODO: F1=help, F2=save, F3=load, F4=volume,
        // F5=detail, F6=quicksave, F7=endgame, F8=messages,
        // F9=quickload, F10=quit, F11=gamma
        false
    }

    /// Navega para um submenu.
    ///
    /// C original: `M_SetupNextMenu()` em `m_menu.c`
    pub fn goto_menu(&mut self, menu_index: usize) {
        if menu_index < self.menus.len() {
            self.menus[self.current_menu].last_on = self.item_on;
            self.current_menu = menu_index;
            self.item_on = self.menus[menu_index].last_on;
        }
    }

    /// Exibe uma mensagem modal.
    ///
    /// C original: `M_StartMessage()` em `m_menu.c`
    pub fn show_message(&mut self, text: &str, needs_input: bool) {
        self.message_showing = true;
        self.message_text = text.to_string();
        self.message_needs_input = needs_input;
        self.last_message_response = None;
    }

    /// Retorna a posicao Y do skull cursor.
    ///
    /// C original: `currentMenu->y - 5 + itemOn * LINEHEIGHT`
    pub fn skull_y(&self) -> i32 {
        let menu = &self.menus[self.current_menu];
        menu.y - 5 + self.item_on as i32 * LINEHEIGHT
    }

    /// Retorna a posicao X do skull cursor.
    pub fn skull_x(&self) -> i32 {
        let menu = &self.menus[self.current_menu];
        menu.x + SKULLXOFF
    }

    /// Consome a ultima acao pendente.
    pub fn take_action(&mut self) -> Option<MenuAction> {
        self.last_action.take()
    }

    /// Consome a ultima resposta de mensagem modal.
    pub fn take_message_response(&mut self) -> Option<bool> {
        self.last_message_response.take()
    }
}

// Precisamos adicionar os campos que foram referenciados nos metodos
// Vamos usar uma abordagem com campos adicionais no struct

impl Default for MenuSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_system_init() {
        let ms = MenuSystem::new();
        assert!(!ms.active);
        assert_eq!(ms.current_menu, 0);
        assert_eq!(ms.skull_frame, 0);
        assert_eq!(ms.menus.len(), 6);
    }

    #[test]
    fn menu_open_close() {
        let mut ms = MenuSystem::new();
        ms.open();
        assert!(ms.active);
        assert_eq!(ms.current_menu, 0); // main menu

        ms.close();
        assert!(!ms.active);
    }

    #[test]
    fn skull_animation() {
        let mut ms = MenuSystem::new();
        assert_eq!(ms.skull_frame, 0);

        // Ticker 8 vezes para mudar frame
        for _ in 0..SKULLANIMCOUNT {
            ms.ticker();
        }
        assert_eq!(ms.skull_frame, 1);

        // Mais 8 vezes para voltar
        for _ in 0..SKULLANIMCOUNT {
            ms.ticker();
        }
        assert_eq!(ms.skull_frame, 0);
    }

    #[test]
    fn menu_navigate_down() {
        let mut ms = MenuSystem::new();
        ms.open();
        assert_eq!(ms.item_on, 0);

        // Seta para baixo (KEY_DOWNARROW = 0xad)
        ms.responder(0xad, true);
        assert_eq!(ms.item_on, 1);

        // Navegar alem do ultimo volta ao primeiro
        for _ in 0..4 {
            ms.responder(0xad, true);
        }
        assert_eq!(ms.item_on, 0); // wraparound
    }

    #[test]
    fn menu_navigate_up() {
        let mut ms = MenuSystem::new();
        ms.open();

        // Seta para cima (KEY_UPARROW = 0xae)
        ms.responder(0xae, true);
        // Deve ir para o ultimo item (4)
        assert_eq!(ms.item_on, 4);
    }

    #[test]
    fn menu_escape_closes() {
        let mut ms = MenuSystem::new();
        ms.open();
        assert!(ms.active);

        ms.responder(27, true); // Escape
        assert!(!ms.active);
    }

    #[test]
    fn menu_goto_submenu() {
        let mut ms = MenuSystem::new();
        ms.open();

        ms.goto_menu(1); // episode menu
        assert_eq!(ms.current_menu, 1);

        // Escape volta ao main
        ms.responder(27, true);
        assert_eq!(ms.current_menu, 0);
    }

    #[test]
    fn menu_message_modal() {
        let mut ms = MenuSystem::new();
        ms.open();
        ms.show_message("Quit game?", true);
        assert!(ms.message_showing);

        // 'n' fecha sem aceitar
        ms.responder(b'n', true);
        assert!(!ms.message_showing);
        assert_eq!(ms.take_message_response(), Some(false));
    }

    #[test]
    fn menu_message_yes() {
        let mut ms = MenuSystem::new();
        ms.show_message("Sure?", true);

        ms.responder(b'y', true);
        assert!(!ms.message_showing);
        assert_eq!(ms.take_message_response(), Some(true));
    }

    #[test]
    fn menu_hotkey() {
        let mut ms = MenuSystem::new();
        ms.open();

        // 'q' deve selecionar Quit Game (index 4)
        ms.responder(b'q', true);
        assert_eq!(ms.item_on, 4);
    }

    #[test]
    fn menu_skip_inactive() {
        let mut ms = MenuSystem::new();
        ms.goto_menu(3); // options menu (tem items inativos)
        ms.active = true;
        ms.item_on = 1; // messages (selecionavel)

        // Seta para cima deve pular o item inativo (0)
        ms.responder(0xae, true);
        // Deve pular item 0 (inativo) e ir para o ultimo selecionavel
        assert_ne!(ms.item_on, 0);
    }

    #[test]
    fn menu_constants() {
        assert_eq!(SKULLXOFF, -32);
        assert_eq!(LINEHEIGHT, 16);
        assert_eq!(SAVESTRINGSIZE, 24);
        assert_eq!(NUM_SAVE_SLOTS, 6);
    }
}

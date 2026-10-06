//! `ContextMenu` — o **menu de clique direito** do `empire-ui`, com o visual do design system
//! [coss][1].
//!
//! [1]: https://github.com/cosscom/coss — `apps/ui/registry/default/ui/context-menu.tsx`
//!
//! # Este módulo NÃO desenha o menu
//!
//! As strings de classe do `context-menu.tsx` são **byte-a-byte** as do `menu.tsx`: popup, item,
//! separador, checkbox, radio, rótulo de grupo, atalho e sub-gatilho, utilitário por utilitário. O
//! que muda entre os dois componentes não é um pixel — é **como abrem**:
//!
//! | | [`crate::menu::Menu`] | `ContextMenu` |
//! |---|---|---|
//! | abre por | clique num gatilho (um botão) | **clique direito** numa região |
//! | ancora em | os `bounds` do gatilho | a **posição do ponteiro** |
//! | não cabe na janela | **desliza** pra dentro (com 8px de margem) | **vira** pro outro lado |
//! | gatilho na árvore | sim, é ele que se vê | nenhum — a região é de quem chama |
//!
//! Então este arquivo não tem paleta, nem constante de geometria, nem render de item: **tudo isso é
//! do [`crate::menu`]**, e o que existe aqui é a região que responde ao clique direito. Duplicar o
//! desenho criaria duas verdades pro mesmo pixel — e a altura do item (28,25px medidos na janela) já
//! está travada por teste lá.
//!
//! # Anatomia
//!
//! ```text
//! ┌───────────────────────────────┐
//! │ a REGIÃO (`ContextMenu`):     │   ← embrulha os filhos de quem chama; é o
//! │ os seus filhos, como eles     │     `ContextMenuTrigger` da referência
//! │ já eram                       │
//! │        ▛ (clique direito)     │
//! │        ┌────────────────────┐ │
//! │        │ ⧉  Duplicar   ⌘D   │ │  ← o popup, os itens, o teclado e o fechamento:
//! │        │ ────────────────── │ │    TUDO do `crate::menu`
//! │        │ ✓  Mostrar grade   │ │
//! │        │    Apagar          │ │
//! │        └────────────────────┘ │
//! └───────────────────────────────┘
//! ```
//!
//! # Como se usa
//!
//! São duas peças: o **estado** (um [`crate::menu::Menu`] sem gatilho, numa entidade) e a **região**
//! (um elemento que embrulha os filhos). O estado é um `Menu` de verdade, então todos os builders
//! dele continuam valendo — [`crate::menu::Menu::width`], [`crate::menu::Menu::side`], … — e é nele
//! que se assina o evento.
//!
//! ```ignore
//! use empire_ui::context_menu::{ContextMenu, ContextMenuEvent};
//! use empire_ui::menu::MenuItem;
//!
//! // 1. o estado
//! let menu = cx.new(|cx| {
//!     ContextMenu::menu(
//!         vec![
//!             MenuItem::new("Duplicar").icon("icons/copy.svg").shortcut("⌘D"),
//!             MenuItem::separator(),
//!             MenuItem::checkbox("Mostrar grade", true),
//!             MenuItem::new("Apagar").destructive(),
//!         ],
//!         cx,
//!     )
//! });
//! cx.subscribe(&menu, |_this, _m, ev: &ContextMenuEvent, _cx| println!("{ev:?}")).detach();
//!
//! // 2. a região, no render — no lugar do `div()` que já embrulhava o conteúdo
//! ContextMenu::new(&self.menu)
//!     .flex()
//!     .flex_col()
//!     .gap_2()
//!     .child("clique com o botão direito aqui")
//! ```
//!
//! A região **é** o container de quem chama: ela expõe [`gpui::Styled`] e
//! [`gpui::InteractiveElement`], então troque o seu `div()` por ela e mantenha as classes. As duas
//! peças que ela acrescenta (o popup e o `canvas` que mede a região) são **absolutas**, fora do
//! fluxo: o layout dos seus filhos não muda em nada.
//!
//! # Como fecha
//!
//! - **escolher um item** — ação fecha, checkbox/radio não (o default do [`crate::menu`]);
//! - **`Escape`** — a região dá foco ao menu no clique direito, então o teclado do `Menu` (setas,
//!   `Home`/`End`, `Enter`, `Escape`) funciona sem uma linha nova aqui;
//! - **clique fora** — de qualquer botão, em qualquer lugar que não seja o popup nem a região;
//! - **clique esquerdo na região** — dispensa o menu (é o "clicar no documento fecha");
//! - **outro clique direito na região** — não fecha: **move** o popup pro novo ponto.
//!
//! # O que NÃO está aqui (declarado, não esquecido)
//!
//! - **Submenu** (`ContextMenuSub`/`SubTrigger`/`SubPopup`): ausente, como no [`crate::menu`] — e
//!   pelo mesmo motivo (um segundo nível de ancoragem e atraso de hover). As classes do
//!   `ContextMenuSubTrigger` são idênticas às do `MenuSubTrigger`, então quando um for feito o outro
//!   sai junto.
//! - **`variant="switch"` do checkbox item** e **`ContextMenuLinkItem`**: as duas ausências do
//!   [`crate::menu`], herdadas — o link tem exatamente as classes do item comum.
//! - **Abrir por teclado** (o `Shift+F10` do Base UI): um menu de contexto sem gatilho não tem onde
//!   se ancorar quando quem pede é o teclado. Ver a guarda no `Menu::on_key`.
//! - **Devolver o foco ao fechar.** A região **rouba** o foco ao abrir (é o que faz o `Escape` e as
//!   setas chegarem), e não o devolve: `Menu::set_open` é público e não recebe `&mut Window`, então
//!   não há por onde focar de volta sem mudar a API do outro módulo. Consequência real: depois de
//!   dispensar o menu, o campo de texto que estava focado antes não volta a estar. Declarado.
//! - **Abrir por pressão longa** (o toque do Base UI): não há gesto de toque aqui.
//! - **`onOpenChangeComplete`** e as animações: a referência não especifica nenhuma (ver a mesma nota
//!   no [`crate::menu`]).

use gpui::{Bounds,Pixels,prelude::FluentBuilder};
use crate::menu::{Menu, MenuAlign, MenuEvent, MenuItem, MenuSide};
use gpui::{
    canvas, div, px, AnyElement, App, Context, Div, Entity, Focusable, InteractiveElement,
    Interactivity, IntoElement, MouseButton, MouseDownEvent, ParentElement, RenderOnce,
    StyleRefinement, Styled, Window,
};

/// O evento de um menu de contexto — **literalmente** o [`MenuEvent`].
///
/// Não é um enum novo de propósito: o menu de contexto tem os mesmos itens do menu suspenso e
/// portanto as mesmas escolhas a comunicar (`Select`, `CheckedChange`, `RadioChange`, `OpenChange`).
/// Um segundo tipo idêntico só obrigaria quem trata os dois a escrever o mesmo `match` duas vezes.
pub type ContextMenuEvent = MenuEvent;

// =================================================================================================
// Posicionamento
// =================================================================================================

/// De que lado do ponteiro o popup cresce.
///
/// ⚠️ **Desvio consciente do `.tsx`**, e é o ponto do módulo que mais merece ser medido na tela. O
/// `ContextMenuPopup` da referência declara `side = "bottom"`, `align = "center"` e
/// `sideOffset = 4` — exatamente os mesmos defaults do `MenuPopup`, porque os dois arquivos são o
/// mesmo wrapper copiado. Só que ali o "gatilho" é um elemento **virtual de tamanho zero** no ponto
/// do clique, e aplicar `align: center` + `sideOffset: 4` a ele centralizaria o popup NO cursor e o
/// jogaria 4px abaixo: o cursor ficaria de fora do menu, na borda de cima, e não sobre o primeiro
/// item.
///
/// Nenhum menu de contexto se comporta assim. Aqui o canto de cima-esquerda do popup vai
/// **exatamente** no ponteiro ([`CTX_SIDE`] + [`CTX_ALIGN`] + [`CTX_SIDE_OFFSET`] = `bottom`,
/// `start`, `0`), e é o [`gpui::anchored`] que vira o canto perto das bordas.
///
/// Quem quiser o literal da referência tem o caminho aberto: o estado é um [`Menu`], então
/// `.align(MenuAlign::Center).side_offset(4.0)` depois do [`ContextMenu::menu`] entrega aquilo — com
/// a ressalva de que `align: center` num anchor de largura zero cai no container de centralização do
/// `Menu`, que com largura 0 é 1px (ver o doc do outro módulo).
const CTX_SIDE: MenuSide = MenuSide::Bottom;

/// Ver [`CTX_SIDE`]: `start` põe a borda ESQUERDA do popup no ponteiro.
const CTX_ALIGN: MenuAlign = MenuAlign::Start;

/// Ver [`CTX_SIDE`]: **zero**, e não os 4px da referência — o popup encosta no ponteiro.
const CTX_SIDE_OFFSET: f32 = 0.0;

// =================================================================================================
// A região
// =================================================================================================

/// A **região que abre o menu no clique direito** — o `ContextMenuTrigger` da referência.
///
/// Embrulha os filhos de quem chama e não desenha nada por si: ela é o `div()` que já estava ali,
/// com um clique direito a mais. Ver o doc do módulo pra montagem completa.
#[derive(IntoElement)]
pub struct ContextMenu {
    /// A raiz. É ela que recebe o estilo e os handlers de quem chama (por [`Styled`] e
    /// [`InteractiveElement`]), pra a região poder SUBSTITUIR o container do call site em vez de
    /// virar mais um nível de layout.
    base: Div,
    /// O estado do menu — o popup, os itens e o teclado moram nele.
    menu: Entity<Menu>,
    mount_menu: bool,
}

impl ContextMenu {
    /// O **estado**: um [`Menu`] sem gatilho, ancorado no ponteiro. Ponha numa entidade
    /// (`cx.new(|cx| ContextMenu::menu(itens, cx))`) e assine o [`ContextMenuEvent`] nela.
    ///
    /// O que volta é um `Menu` de verdade: todos os builders dele continuam disponíveis
    /// ([`Menu::width`], [`Menu::side`], [`Menu::align`], [`Menu::align_offset`]), e os `set_*`
    /// (`set_items`, `set_checked`, `select_radio`, `set_open`) também. O que **não** faz sentido
    /// aqui é `trigger_button`/`trigger_icon_button`/`trigger`: um menu de contexto não tem gatilho
    /// visível, e o gatilho não é renderizado enquanto a ancoragem for a do ponteiro.
    pub fn menu(items: Vec<MenuItem>, cx: &mut Context<Menu>) -> Menu {
        Menu::new(items, cx)
            .side(CTX_SIDE)
            .align(CTX_ALIGN)
            .side_offset(CTX_SIDE_OFFSET)
            .pointer_anchored()
    }

    /// A **região**, no render. Estilize como estilizaria o seu `div()`.
    /// Share one popup host between virtualized regions. Mount the menu entity once in the parent.
    pub fn detached(mut self)->Self {self.mount_menu=false;self}

    pub fn new(menu: &Entity<Menu>) -> Self {
        Self {
            // `relative` porque as duas peças que a região acrescenta são absolutas: o `canvas` que
            // a mede e a caixa 0×0 que hospeda o popup.
            base: div().relative(),
            menu: menu.clone(),
            mount_menu:true,
        }
    }
}

impl Styled for ContextMenu {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for ContextMenu {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl InteractiveElement for ContextMenu {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl RenderOnce for ContextMenu {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let Self { base, menu, mount_menu } = self;
        let region=std::rc::Rc::new(std::cell::Cell::new(Bounds::<Pixels>::default()));

        base
            // --- O clique DIREITO: abre no ponteiro, ou move o popup que já está aberto ----------
            //
            // `on_mouse_down` e não `on_click`: o menu tem que aparecer no botão apertado, como o de
            // qualquer app nativo — e `on_click` exigiria um `id` na região, que é de quem chama.
            //
            // Ele roda na fase de BOLHA, depois do `on_mouse_down_out` do popup (que é de captura).
            // Não há corrida: o popup só fecha num clique fora da REGIÃO, e a região é justamente o
            // retângulo que o `canvas` abaixo mede.
            .on_mouse_down(MouseButton::Right, {
                let menu = menu.clone();
                let region=region.clone();
                move |event: &MouseDownEvent, window, cx| {
                    // Abrir com o ponteiro não pode acender anel de foco (ver `crate::focus_ring`).
                    crate::focus_ring::pointer_used(window);
                    // Sem foco no menu, `Escape` e as setas não chegam nele: o teclado do `Menu` é
                    // ouvido pela raiz dele, e o GPUI entrega tecla pelo caminho do FOCO. Roubar o
                    // foco é o que todo menu faz; devolvê-lo ao fechar está declarado como ausente
                    // no doc do módulo.
                    menu.read(cx).focus_handle(cx).focus(window);
                    menu.update(cx, |menu, cx| {menu.set_anchor_region(region.get());menu.open_at(event.position, cx);});
                    // Consome o clique: numa região dentro de outra região, quem ganha é a de
                    // dentro (a bolha vai do topo pra trás), e sem isto as duas abririam de uma vez.
                    cx.stop_propagation();
                }
            })
            // --- O clique ESQUERDO na região: dispensa -------------------------------------------
            //
            // Do ponto de vista do popup este clique é "fora", mas ele NÃO fecha por lá: a região é
            // o "trigger rect" do menu, e cliques nela são tratados aqui (é a mesma divisão de
            // trabalho do gatilho do `Menu`). Um clique no próprio popup não chega até aqui, porque
            // o popup é `occlude`.
            .on_mouse_down(MouseButton::Left, {
                let menu = menu.clone();
                move |_event, _window, cx| {
                    menu.update(cx, |menu, cx| menu.set_open(false, cx));
                }
            })
            // --- A medida da região --------------------------------------------------------------
            //
            // É o retângulo em que um mouse-down não conta como clique fora. Absoluto e `size_full`
            // pra coincidir com a caixa da região; se quem chama puser `padding` na região, o que se
            // mede é a padding box — a diferença aparece só num clique direito dado DENTRO do
            // respiro, que aí fecha e reabre o popup em vez de só movê-lo.
            .child(
                canvas(
                    {
                        let menu = menu.clone();
                        let region=region.clone();
                        move |bounds, _window, cx| {
                            region.set(bounds);
                            if mount_menu {menu.update(cx, |menu, _| menu.set_anchor_region(bounds));}
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            // --- O popup ------------------------------------------------------------------------
            //
            // Numa caixa ABSOLUTA de 0×0: assim a raiz do `Menu` (que sem gatilho não tem conteúdo
            // nenhum) não entra no fluxo e não acrescenta uma linha nem um `gap` ao layout de quem
            // chama. O popup em si é `deferred(anchored(..))` em coordenadas de JANELA, então não
            // liga pra onde este ponto está.
            .when(mount_menu,|base| base.child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .w(px(0.0))
                    .h(px(0.0))
                    .child(menu),
            ))
    }
}

#[cfg(test)]
// Travar o valor que veio da referência (ou o desvio dela) É o propósito destes testes — mesma
// decisão do `crate::menu` e dos outros módulos portados do coss.
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    /// **O desvio de posicionamento, travado.**
    ///
    /// Ver o doc de [`CTX_SIDE`]: o `.tsx` declara `align: center` e `sideOffset: 4`, e aqui é
    /// `start` com offset 0, pra o canto do popup cair EXATAMENTE no ponteiro. Se alguém "corrigir"
    /// isto pro literal da referência, este teste falha e aponta pro porquê.
    #[test]
    fn o_canto_do_popup_vai_no_ponteiro() {
        assert_eq!(CTX_SIDE_OFFSET, 0.0, "encosta no ponteiro (a referência diz 4)");
        assert_eq!(CTX_ALIGN, MenuAlign::Start, "a referência diz center");
        assert_eq!(CTX_SIDE, MenuSide::Bottom, "cresce pra baixo, como a referência");
        assert_ne!(
            CTX_ALIGN,
            MenuAlign::default(),
            "o default do Menu é center: o menu de contexto o SOBREPÕE de propósito"
        );
    }
}

#[cfg(test)]
mod tests_de_janela {
    //! Os testes que precisam de uma JANELA: é aqui que se confere **onde o popup foi pintado** —
    //! contra o ponto do clique, e clicando nos itens pra provar que a hitbox está no mesmo lugar
    //! que a pintura. Nenhum teste puro pega um popup 100px fora do cursor.

    use super::*;
    use crate::theme;
    use gpui::{
        point, AppContext as _, Bounds, Modifiers, Pixels, Point, TestAppContext, VisualTestContext,
    };
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    /// A região do harness: `(x, y, largura, altura)`. Longe da origem e longe das bordas de
    /// propósito — um posicionamento que ignorasse o ponto do clique passaria num harness em (0,0),
    /// e um popup que grudasse na janela esconderia o erro.
    const REGIAO: (f32, f32, f32, f32) = (100.0, 80.0, 420.0, 320.0);

    // --- A geometria que o `crate::menu` desenha ---------------------------------------------------
    //
    // Estes números NÃO são declarados aqui: eles são as constantes do `crate::menu` (privadas dele),
    // repetidas como literal de teste pra este módulo poder mirar uma linha do popup. A soma
    // ([`ALTURA_DO_POPUP`]) é o mesmo 131 que o teste de janela do `menu.rs` trava — se alguém mudar
    // o desenho lá, é este número que quebra aqui, e é exatamente o alarme que se quer: prova que o
    // menu de contexto usa o desenho do outro módulo, não uma cópia.

    /// A borda do popup — `border`.
    const BORDA: f32 = 1.0;
    /// O respiro da lista — `p-1`.
    const RESPIRO: f32 = 4.0;
    /// A altura de uma linha clicável — `sm:min-h-7`.
    const LINHA: f32 = 28.0;
    /// A altura do separador — `h-px` mais o `my-1` dos dois lados.
    const SEPARADOR: f32 = 9.0;
    /// A altura do popup da lista de teste: 2×(borda + respiro) + 4 linhas + 1 separador.
    const ALTURA_DO_POPUP: f32 = 2.0 * (BORDA + RESPIRO) + 4.0 * LINHA + SEPARADOR;

    /// Um container que planta a região num ponto conhecido da janela.
    struct Harness {
        menu: Entity<Menu>,
        /// Se a região ocupa a janela inteira (o caso do teste de borda).
        cheia: bool,
    }

    impl gpui::Render for Harness {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let regiao = ContextMenu::new(&self.menu).child(div().child("clique com o direito"));
            let regiao = if self.cheia {
                regiao.size_full()
            } else {
                regiao.w(px(REGIAO.2)).h(px(REGIAO.3))
            };
            let (pt, pl) = if self.cheia {
                (0.0, 0.0)
            } else {
                (REGIAO.1, REGIAO.0)
            };
            div().size_full().pt(px(pt)).pl(px(pl)).child(regiao)
        }
    }

    /// A lista de teste — a MESMA do teste de janela do `crate::menu`, pra a altura do popup poder
    /// ser comparada: `0` ação (ícone + atalho) · `1` ação DESABILITADA · `2` separador ·
    /// `3` checkbox · `4` ação destrutiva.
    fn itens() -> Vec<MenuItem> {
        vec![
            MenuItem::new("Duplicar").icon("icons/copy.svg").shortcut("⌘D"),
            MenuItem::new("Inerte").disabled(true),
            MenuItem::separator(),
            MenuItem::checkbox("Grade", false),
            MenuItem::new("Apagar").destructive(),
        ]
    }

    /// Abre a janela e devolve o menu, os eventos capturados e o contexto visual.
    #[allow(clippy::type_complexity)]
    fn montar(
        cx: &mut TestAppContext,
        cheia: bool,
    ) -> (
        Entity<Menu>,
        Rc<RefCell<Vec<MenuEvent>>>,
        VisualTestContext,
    ) {
        theme::set_theme(theme::ThemeMode::Dark);
        let eventos: Rc<RefCell<Vec<MenuEvent>>> = Rc::new(RefCell::new(Vec::new()));
        let capturados = eventos.clone();

        let window = cx.add_window(move |_window, cx| {
            let menu = cx.new(|cx| ContextMenu::menu(itens(), cx));
            cx.subscribe(&menu, move |_this, _m, ev: &MenuEvent, _cx| {
                capturados.borrow_mut().push(ev.clone());
            })
            .detach();
            Harness { menu, cheia }
        });

        let harness = window.root(cx).expect("view raiz");
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let menu = vcx.read(|cx| harness.read(cx).menu.clone());
        (menu, eventos, vcx)
    }

    /// Um clique direito num ponto da janela.
    fn clique_direito(vcx: &mut VisualTestContext, p: Point<Pixels>) {
        vcx.simulate_mouse_down(p, MouseButton::Right, Modifiers::default());
        vcx.simulate_mouse_up(p, MouseButton::Right, Modifiers::default());
        vcx.run_until_parked();
    }

    /// A caixa do popup, medida no prepaint (ou seja: onde ele REALMENTE está).
    fn caixa(menu: &Entity<Menu>, vcx: &mut VisualTestContext) -> (f32, f32, f32, f32) {
        vcx.read(|cx| {
            let b = menu.read(cx).popup_bounds().expect("o popup se mede ao abrir");
            (
                f32::from(b.origin.x),
                f32::from(b.origin.y),
                f32::from(b.size.width),
                f32::from(b.size.height),
            )
        })
    }

    /// O topo da linha `i` a partir da borda de cima do popup — só a soma do que vem antes dela.
    fn topo_da_linha(i: usize) -> f32 {
        let antes: f32 = (0..i)
            .map(|k| if k == 2 { SEPARADOR } else { LINHA })
            .sum();
        antes + BORDA + RESPIRO
    }

    /// O centro da linha `i`, em coordenadas da JANELA, derivado do PONTO DO CLIQUE e da geometria
    /// do `crate::menu` — nenhuma medida do popup entra na conta. Se o popup for pintado noutro
    /// lugar, o clique erra o item e o teste falha.
    fn centro_da_linha(clique: Point<Pixels>, i: usize) -> Point<Pixels> {
        let altura = if i == 2 { SEPARADOR } else { LINHA };
        point(
            clique.x + px(20.0),
            clique.y + px(topo_da_linha(i) + altura / 2.0),
        )
    }

    /// **O popup abre com o canto no ponteiro, e com o desenho do [`crate::menu`].**
    ///
    /// Os dois fatos que mais importam nesta tarefa, num teste só: a origem da caixa medida é
    /// EXATAMENTE o ponto do clique direito (é o `sideOffset: 0` + `align: start` do desvio
    /// declarado em [`CTX_SIDE`]), e a altura é o mesmo 131 que o `menu.rs` trava — ou seja, o
    /// desenho é o de lá, não uma cópia.
    #[gpui::test]
    fn abre_com_o_canto_no_ponteiro_e_com_o_desenho_do_menu(cx: &mut TestAppContext) {
        let (menu, eventos, mut vcx) = montar(cx, false);
        assert!(!vcx.read(|cx| menu.read(cx).is_open()), "nasce fechado");

        let p = point(px(200.0), px(150.0));
        clique_direito(&mut vcx, p);
        assert!(vcx.read(|cx| menu.read(cx).is_open()), "o clique direito abre");
        assert_eq!(&*eventos.borrow(), &[MenuEvent::OpenChange(true)]);

        let (x, y, w, h) = caixa(&menu, &mut vcx);
        assert_eq!(
            (x, y),
            (f32::from(p.x), f32::from(p.y)),
            "o canto de cima-esquerda do popup vai EXATAMENTE no ponteiro"
        );
        assert_eq!(
            h, ALTURA_DO_POPUP,
            "a altura é a do popup do `crate::menu`: borda + p-1 + 4 linhas de 28 + separador de 9"
        );
        assert_eq!(h, 131.0, "o mesmo número que o teste de janela do menu.rs trava");
        assert!(w >= 128.0, "o `min-w-32` do popup é um PISO, veio {w}");

        // Abrir não pode acender anel de foco: é ponteiro, não teclado (o `focus-visible` da casa).
        assert!(!crate::focus_ring::visible(), "o clique não acende o anel");
    }

    /// **O clique cai no item certo, e só a AÇÃO fecha.**
    ///
    /// O ponto clicado é derivado do PONTO DO CLIQUE DIREITO e das constantes de geometria — se o
    /// popup fosse pintado num lugar e tivesse hitbox noutro (o defeito que o `menu.rs` descreve na
    /// armadilha da medida), o clique cairia fora e nada disto conferiria.
    #[gpui::test]
    fn o_clique_cai_no_item_certo_e_so_a_acao_fecha(cx: &mut TestAppContext) {
        let (menu, eventos, mut vcx) = montar(cx, false);
        let p = point(px(200.0), px(150.0));
        clique_direito(&mut vcx, p);

        // Linha 3: o checkbox. Marca, emite, e o menu FICA ABERTO.
        vcx.simulate_click(centro_da_linha(p, 3), Modifiers::default());
        vcx.run_until_parked();
        assert_eq!(
            &*eventos.borrow(),
            &[
                MenuEvent::OpenChange(true),
                MenuEvent::CheckedChange { index: 3, checked: true }
            ]
        );
        assert!(
            vcx.read(|cx| menu.read(cx).is_open()),
            "marcar um checkbox não fecha o menu de contexto"
        );

        // Linha 4: a ação destrutiva. Emite Select(4) e FECHA.
        vcx.simulate_click(centro_da_linha(p, 4), Modifiers::default());
        vcx.run_until_parked();
        assert_eq!(
            &eventos.borrow()[2..],
            &[MenuEvent::Select(4), MenuEvent::OpenChange(false)]
        );
        assert!(!vcx.read(|cx| menu.read(cx).is_open()), "a ação fecha");

        // E a linha 1 (desabilitada) e a 2 (separador) não têm hitbox: reabrindo e clicando nelas,
        // nada acontece.
        clique_direito(&mut vcx, p);
        for linha in [1, 2] {
            vcx.simulate_click(centro_da_linha(p, linha), Modifiers::default());
            vcx.run_until_parked();
            assert!(
                vcx.read(|cx| menu.read(cx).is_open()),
                "a linha {linha} não pode fechar o menu"
            );
        }
    }

    /// **Um segundo clique direito MOVE o popup — não fecha e reabre.**
    ///
    /// Este é o teste que prova que a região foi medida: se ela não fosse o "trigger rect" do menu,
    /// o `on_mouse_down_out` do popup trataria este clique como clique fora, fecharia, e a região
    /// abriria de novo — e o log de quem assina teria um par `OpenChange(false)`/`OpenChange(true)`
    /// que não corresponde a nada que o usuário fez.
    #[gpui::test]
    fn o_segundo_clique_direito_move_o_popup(cx: &mut TestAppContext) {
        let (menu, eventos, mut vcx) = montar(cx, false);

        clique_direito(&mut vcx, point(px(200.0), px(150.0)));
        assert_eq!(caixa(&menu, &mut vcx).0, 200.0);

        let outro = point(px(340.0), px(260.0));
        clique_direito(&mut vcx, outro);
        let (x, y, _, _) = caixa(&menu, &mut vcx);
        assert_eq!((x, y), (340.0, 260.0), "o popup se mudou pro novo ponto");
        assert!(vcx.read(|cx| menu.read(cx).is_open()));
        assert_eq!(
            &*eventos.borrow(),
            &[MenuEvent::OpenChange(true)],
            "abriu UMA vez: mover não é fechar e abrir"
        );
    }

    /// **Fecha no `Escape`, no clique esquerdo na região e no clique fora dela.**
    #[gpui::test]
    fn fecha_no_escape_no_clique_esquerdo_e_no_clique_fora(cx: &mut TestAppContext) {
        let (menu, _ev, mut vcx) = montar(cx, false);
        let p = point(px(200.0), px(150.0));

        // `Escape` — só chega porque a região deu foco ao menu no clique direito.
        clique_direito(&mut vcx, p);
        vcx.simulate_keystrokes("escape");
        vcx.run_until_parked();
        assert!(!vcx.read(|cx| menu.read(cx).is_open()), "escape fecha");

        // Clique esquerdo na REGIÃO, fora do popup: dispensa.
        clique_direito(&mut vcx, p);
        vcx.simulate_click(point(px(480.0), px(370.0)), Modifiers::default());
        vcx.run_until_parked();
        assert!(
            !vcx.read(|cx| menu.read(cx).is_open()),
            "clique esquerdo na região dispensa o menu"
        );

        // Clique bem longe, fora da região: fecha pelo `on_mouse_down_out` do popup.
        clique_direito(&mut vcx, p);
        vcx.simulate_click(point(px(5.0), px(5.0)), Modifiers::default());
        vcx.run_until_parked();
        assert!(!vcx.read(|cx| menu.read(cx).is_open()), "clique fora fecha");
    }

    /// **Perto da borda o popup VIRA pro outro lado, e cabe na janela.**
    ///
    /// É o que o `snap_to_window_with_margin` do [`crate::menu`] **não** faz: ele desliza, e um
    /// popup deslizado seria pintado por cima do cursor. Aqui vale o `SwitchAnchor` do
    /// [`gpui::anchored`], que troca o canto de ancoragem: a borda DIREITA do popup passa a encostar
    /// no ponteiro, e a de BAIXO também.
    #[gpui::test]
    fn perto_da_borda_o_popup_vira_pro_outro_lado(cx: &mut TestAppContext) {
        let (menu, _ev, mut vcx) = montar(cx, true);
        let viewport = vcx.update(|window, _cx| window.viewport_size());
        let (vw, vh) = (
            f32::from(viewport.width).floor(),
            f32::from(viewport.height).floor(),
        );

        // Um clique a 20px da quina de baixo-à-direita: nem a largura nem a altura do popup cabem
        // pra frente, então ele tem que crescer pra trás nos dois eixos.
        let p = point(px(vw - 20.0), px(vh - 20.0));
        clique_direito(&mut vcx, p);
        let (x, y, w, h) = caixa(&menu, &mut vcx);

        assert_eq!(
            x + w,
            vw - 20.0,
            "virou: a borda DIREITA do popup é que encosta no ponteiro"
        );
        assert_eq!(y + h, vh - 20.0, "e a de BAIXO também");
        assert!(x >= 0.0 && y >= 0.0, "e ele continua dentro da janela ({x}, {y})");

        // Já longe das bordas, não vira: o canto de cima-esquerda volta pro ponteiro.
        let meio = point(px(200.0), px(150.0));
        clique_direito(&mut vcx, meio);
        let (x, y, _, _) = caixa(&menu, &mut vcx);
        assert_eq!((x, y), (200.0, 150.0), "com espaço, o canto vai no ponteiro");
    }

    /// Um harness que **mede a caixa da região** — o que prova que ela não engorda o layout.
    struct HarnessLayout {
        menu: Entity<Menu>,
        medida: Rc<Cell<Bounds<Pixels>>>,
    }

    /// A altura de cada filho do teste de layout.
    const FILHO: f32 = 40.0;
    /// O `gap` da região no teste de layout — o valor que uma peça a mais em fluxo duplicaria.
    const VAO: f32 = 8.0;

    impl gpui::Render for HarnessLayout {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let medida = self.medida.clone();
            // A raiz é uma LINHA com `items_start`: assim a região encolhe em volta do conteúdo nos
            // dois eixos (num pai de bloco ela ocuparia a largura da janela e a medida não diria
            // nada). A sonda é uma peça absoluta DENTRO da região, igual às que a região monta —
            // então o que ela mede é a caixa da região.
            div().size_full().flex().items_start().child(
                ContextMenu::new(&self.menu)
                    .flex()
                    .flex_col()
                    .gap(px(VAO))
                    .child(div().w(px(120.0)).h(px(FILHO)))
                    .child(div().w(px(120.0)).h(px(FILHO)))
                    .child(
                        canvas(
                            move |bounds, _window, _cx| medida.set(bounds),
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full(),
                    ),
            )
        }
    }

    /// **A região não acrescenta NADA ao layout de quem chama.**
    ///
    /// É a razão de as duas peças que ela monta (o `canvas` da medida e a caixa que hospeda o popup)
    /// serem absolutas. Num container com `gap` — o caso realista — uma peça a mais **em fluxo**
    /// somaria um vão inteiro à altura, e o defeito seria um respiro de 8px que ninguém pediu, só
    /// nos containers que ganharam menu de contexto. A conta aqui é exata: dois filhos de 40 e um
    /// vão de 8.
    #[gpui::test]
    fn a_regiao_nao_perturba_o_layout(cx: &mut TestAppContext) {
        theme::set_theme(theme::ThemeMode::Dark);
        let medida: Rc<Cell<Bounds<Pixels>>> = Rc::new(Cell::new(Bounds::default()));
        let sonda = medida.clone();
        let window = cx.add_window(move |_window, cx| {
            let menu = cx.new(|cx| ContextMenu::menu(itens(), cx));
            HarnessLayout {
                menu,
                medida: sonda,
            }
        });
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();

        let b = medida.get();
        assert_eq!(
            f32::from(b.size.height),
            2.0 * FILHO + VAO,
            "a altura é só a dos filhos de quem chama, com UM vão entre eles"
        );
        assert_eq!(
            f32::from(b.size.width),
            120.0,
            "e a largura também é só a deles"
        );
    }

    /// **O teclado do [`crate::menu`] vem de graça** — e um menu de contexto FECHADO não abre por
    /// tecla.
    ///
    /// A segunda metade é a guarda do `Menu::on_key`: a região rouba o foco ao abrir e não o
    /// devolve ao fechar (declarado no doc do módulo), então sem a guarda uma seta depois do
    /// `Escape` ressuscitaria o menu no lugar que o usuário acabou de dispensar.
    #[gpui::test]
    fn o_teclado_anda_e_escolhe_mas_nao_abre(cx: &mut TestAppContext) {
        let (menu, eventos, mut vcx) = montar(cx, false);
        clique_direito(&mut vcx, point(px(200.0), px(150.0)));

        // Abrir com o ponteiro não destaca nada; a primeira seta entra pelo primeiro alcançável.
        assert_eq!(vcx.read(|cx| menu.read(cx).highlighted()), None);
        vcx.simulate_keystrokes("down");
        vcx.run_until_parked();
        assert_eq!(vcx.read(|cx| menu.read(cx).highlighted()), Some(0));
        assert!(crate::focus_ring::visible(), "o teclado acende o anel");

        // `↓` pula a linha 1 (desabilitada) e a 2 (separador).
        vcx.simulate_keystrokes("down");
        vcx.run_until_parked();
        assert_eq!(vcx.read(|cx| menu.read(cx).highlighted()), Some(3));

        // `Enter` escolhe: emite e fecha.
        vcx.simulate_keystrokes("enter");
        vcx.run_until_parked();
        assert_eq!(
            &eventos.borrow()[1..],
            &[MenuEvent::CheckedChange { index: 3, checked: true }],
            "o checkbox marca e NÃO fecha, mesmo pelo Enter"
        );
        vcx.simulate_keystrokes("escape");
        vcx.run_until_parked();
        assert!(!vcx.read(|cx| menu.read(cx).is_open()));

        // Fechado, o foco continua no menu — e nenhuma tecla o reabre.
        for tecla in ["down", "up", "enter", "space"] {
            vcx.simulate_keystrokes(tecla);
            vcx.run_until_parked();
            assert!(
                !vcx.read(|cx| menu.read(cx).is_open()),
                "`{tecla}` não pode abrir um menu de contexto: ele não tem gatilho nem âncora"
            );
        }
    }
}

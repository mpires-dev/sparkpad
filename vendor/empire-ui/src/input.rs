//! `Input` — o **campo de texto** do `empire-ui`: a moldura completa de um campo de formulário
//! (label, hint, erro, contador, prefixo/sufixo, tamanhos) em volta de um núcleo de edição de
//! texto maduro.
//!
//! # Divisão de responsabilidade
//!
//! A **edição de texto** (IME, clusters de grafema, seleção, undo/redo, movimentação por palavra,
//! soft wrap) fica com o [`InputState`] do `gpui-component`, que já é usado em produção e é
//! mantido upstream. Reimplementar isso começaria pior: IME e grafemas são exatamente onde uma
//! implementação nova erra.
//!
//! A **apresentação** é nossa, e é o que faltava: o [`Input`] deles renderiza um campo nu — sem
//! label, sem texto de ajuda, sem estado de erro, sem contador, sem tamanhos. Hoje cada ponto da
//! app remonta essa moldura à mão (é o que o [`crate::scrub_input::ScrubInput`] e o
//! [`crate::color_picker::ColorPicker`] fazem). Este módulo centraliza isso.
//!
//! # Anatomia
//!
//! ```text
//! ┌─ label ──────────────────────────── *      ← .label("Nome") + .required()
//! │ ┌──────────────────────────────────────┐
//! │ │ ⌕  texto digitado             ✕  un │  ← .prefix(...) · núcleo · .suffix(...)
//! │ └──────────────────────────────────────┘
//! │ texto de ajuda / mensagem de erro   12/40  ← .hint(...) | .error(...) · .max_len(40)
//! ```
//!
//! # Uso
//!
//! O estado vive numa entidade (uma por campo), criada uma vez; o [`Input`] é um **elemento de
//! render**, remontado a cada frame — o mesmo bom padrão do `gpui-component`:
//!
//! ```ignore
//! // uma vez, no `new` da sua view:
//! let email = cx.new(|cx| empire_ui::input::single_line(window, cx).placeholder("voce@exemplo.com"));
//!
//! // a cada render:
//! Input::new(&self.email)
//!     .label("E-mail")
//!     .required()
//!     .prefix(icon("iconoir/regular/mail.svg"))
//!     .hint("Usamos só para o recibo.")
//!     .max_len(120)
//! ```
//!
//! # Validação
//!
//! O estado de validade é um [`Validity`] — um enum, não um par de booleanos, justamente pra ser
//! impossível representar "erro e sucesso ao mesmo tempo". A mensagem de erro **substitui** o
//! hint na linha de baixo (em vez de empilhar as duas), porque quando há erro é ele que o usuário
//! precisa ler; o hint volta sozinho quando o erro sai.
//!
//! Quem decide o que é válido é quem usa: assine [`gpui_component::input::InputEvent::Change`] e
//! chame `.error(...)` no render seguinte. O núcleo também aceita `pattern`/`validate` (regex e
//! predicado) se você preferir barrar a digitação em vez de sinalizar depois.
//!
//! # Textarea
//!
//! O **multi-linha** é o mesmo componente: [`Input::textarea`] (ou [`Input::rows`]) troca a
//! geometria vertical, e o estado vem de [`multi_line`] ou [`growing`].
//!
//! A **superfície não é reescrita**, porque no original ela é a mesma: o `textarea.tsx` do coss
//! repete, classe por classe, a lista do `input.tsx` — `rounded-lg border border-input bg-background
//! shadow-xs/5 ring-ring/24 transition-shadow`, o bisel do `before:` e os ramos de
//! `has-focus-visible` / `has-aria-invalid` / `has-disabled`. Uma segunda cópia daquilo aqui seria
//! uma segunda fonte de verdade pra dez tokens.
//!
//! O que é do textarea é a **geometria**, e ela é por tamanho:
//!
//! | tamanho | piso do miolo | respiro vertical | respiro horizontal |
//! |---------|---------------|------------------|--------------------|
//! | `Sm`    | 66            | 3                | 9                  |
//! | `Md`    | 70            | 5                | 11                 |
//! | `Lg`    | 74            | 7                | 11                 |
//!
//! Os três pisos são **a mesma frase dita três vezes**: `3 linhas × 20 + 2 × respiro`. A entrelinha
//! de 20px é o par do Tailwind pro `text-sm` do wrapper (14/20, ver [`TEXTAREA_LINE_HEIGHT`]), e é
//! ela que faz 70px caberem exatamente três linhas. Por isso os pisos não são três números soltos —
//! são derivados, e é o que o teste `piso_da_textarea_e_tres_linhas_mais_respiro` trava.
//!
//! ## `field-sizing-content`: piso, não altura
//!
//! No coss o `min-h-*` é um **piso** e a caixa CRESCE com o conteúdo (`field-sizing-content`). Aqui
//! isso é reproduzível, e **quem decide é o estado**:
//!
//! - [`multi_line`] → caixa **parada** no piso, com rolagem interna. É o `<textarea>` clássico.
//! - [`growing`] → caixa que **cresce** do piso até um teto de linhas. É o `field-sizing-content`.
//!
//! A moldura declara `min_h` (nunca `h`) nos dois casos; o crescimento vem do elemento do núcleo,
//! que em modo `auto_grow` pede `linhas_quebradas × entrelinha` de altura mínima. O teste
//! `textarea_que_cresce_sobe_de_20_em_20_ate_o_teto` mede isso na janela, com pixel de verdade.
//!
//! # Diferenças em relação ao original — todas declaradas
//!
//! Esta seção cobre o **textarea**; o campo de uma linha está documentado ao longo do arquivo, item
//! por item, onde cada número é escrito.
//!
//! **Não reproduzível**
//!
//! - **`field-sizing-content` sem teto.** No CSS a caixa cresce indefinidamente. Aqui o crescimento
//!   existe ([`growing`]) mas **exige um teto em linhas**: o elemento do núcleo deriva a altura
//!   mínima de `rows.min(max_rows) × entrelinha`, e sem `max_rows` não há o que clampar. Um teto é
//!   de todo modo o que se quer numa janela nativa — sem ele um textarea empurraria o formulário
//!   inteiro pra fora da tela.
//! - **`rounded-[inherit]` no miolo.** Não há herança de estilo no GPUI. O efeito prático é o mesmo:
//!   o miolo não desenha raio NENHUM (quem arredonda é a moldura) e nada escapa da curva, porque a
//!   moldura tem `overflow_hidden`.
//!
//! **Resolvido em número**
//!
//! - **`min-h-17.5` / `min-h-16.5` / `min-h-18.5`** → 70 / 66 / 74 (`--spacing` = 4px). Ver
//!   [`InputSize::textarea_min_content_height`].
//! - **`py-[calc(--spacing(1.5)-1px)]` / `(1)` / `(2)`** → 5 / 3 / 7. O `-1px` desconta a borda, do
//!   mesmo jeito que no `px` do campo de uma linha. Ver [`InputSize::textarea_pad_y`].
//! - **`px-[calc(--spacing(3)-1px)]` / `(2.5)`** → 11 / 9: é o MESMO [`InputSize::pad_x`] do campo de
//!   uma linha, não um respiro próprio do textarea (conferido classe por classe no original).
//! - **`sm:text-sm`** → o PAR do Tailwind, `(14, 20)`. Fixar a entrelinha é obrigatório e é onde um
//!   textarea mente por inteiro: o default do GPUI é `relative(1.618_034)`, que sobre 14px dá
//!   **22,65** — cada linha 2,65px mais alta, três linhas medindo 67,96 em vez de 60 e o piso de 70
//!   virando 78. Travado em `entrelinha_da_textarea_e_o_par_do_tailwind`.
//! - **`placeholder:text-muted-foreground/72`** → o `placeholder` da [`FieldPalette`], o mesmo do
//!   campo de uma linha (o `/72` já está resolvido no alfa `b8`).
//! - **Os `max-sm:`** (`min-h-20.5/19.5/21.5` = 82/78/86) são o ramo de tela **estreita**, onde o
//!   corpo é `text-base` (16/24) — e ali os pisos também são três linhas: `3 × 24 + 2 × respiro`.
//!   Uma janela de desktop está sempre acima do breakpoint de 640px, então valem os do `sm:`. Mesma
//!   decisão do resto da lib.
//!
//! **Desvio consciente**
//!
//! - **[`Input::rows`] mudou de significado.** Era altura FIXA de `n × 30 + 12` (a entrelinha de um
//!   campo de uma linha, mais um respiro de 6px que não vinha de lugar nenhum). Agora é **piso** de
//!   `n × 20 + 2 × respiro do tamanho`. No `Md`, `rows(3)` = 72 de altura externa — exatamente o
//!   `min-h-17.5` da referência, o que é a prova de que os dois modelos convergiram e não brigam.
//! - **[`Input::height`] numa textarea fixa a caixa** e desliga o crescimento. É o escape hatch que
//!   já existia (ver o doc dele), e é o único jeito de pedir uma caixa multi-linha fora da escala de
//!   linhas. Não é um quinto override: é o mesmo, com efeito declarado no caso multi-linha.
//!
//! **Superset consciente**
//!
//! - **[`Input::rows`]**: o `size` do textarea do coss aceita `"sm" | "default" | "lg" | number`, mas
//!   o `number` lá só cai no `data-size` — a geometria dele é a do `default`, e o piso é sempre de
//!   três linhas. Um piso em LINHAS é nosso.
//! - **[`growing`]**: o modo de crescimento é do estado do núcleo, e expor os dois limites em linhas
//!   (em vez de um `field-sizing` booleano) é a forma que o GPUI permite.
//! - **[`Input::unstyled`]**: mesmo nome e mesmo papel do `unstyled` da referência, com um item a
//!   mais — lá o `<span>` sem superfície ainda é o dono do foco; aqui o desfoque por clique fora
//!   também passa a ser de quem hospeda (ver o doc do método).
//! - **A alça de redimensionar.** No DOM ela é do NAVEGADOR, não do componente: o `<textarea>` nasce
//!   `resize: both` e o coss nem a desliga aqui — só dentro do `input-group`, com `resize-none`. No
//!   GPUI nada disso vem de graça, então ou a alça é **ausente** (era o que esta seção declarava até
//!   aqui) ou ela é **nossa**, desenhada com cada decisão escrita. É a segunda: um controle no canto
//!   inferior direito da moldura, que arrasta a altura da caixa. As quatro decisões, todas visíveis
//!   em `alca_de_redimensionar` e vizinhas:
//!
//!   1. **Só o eixo VERTICAL** (o navegador dá `both`). A largura de um campo aqui é do layout do
//!      formulário — o default do [`Input`] é `w_full`, e um campo de largura arrastável brigaria com
//!      o container em vez de obedecê-lo. A altura é o eixo que o usuário de fato quer mexer numa
//!      textarea, e é o único que não tem outro dono. O cursor é `ResizeUpDown`, que diz isso antes
//!      do primeiro arraste.
//!   2. **O piso é o piso do coss.** Arrastar pra cima para no `min-h-*` do tamanho (ou no
//!      [`Input::rows`], quando é ele que declara o piso): a alça é um superset, não uma licença pra
//!      furar a geometria da referência.
//!   3. **O teto é em LINHAS** — `ARRASTE_MAX_LINHAS`, pelo mesmo motivo que [`growing`] exige um
//!      teto, e na mesma unidade do resto da geometria vertical.
//!   4. **O arraste vence o crescimento automático.** Com [`growing`] a caixa acompanha o texto; no
//!      instante em que o usuário arrasta, ela passa a obedecer a ele. Intenção explícita ganha de
//!      automática, e o caminho é o mesmo que [`Input::height`] já usava (`h` em vez de `min_h`).
//!
//!   Dentro do [`crate::input_group::InputGroup`] a alça **não aparece** — é o `resize-none` da
//!   referência, e sai de graça: no grupo o campo é [`Input::unstyled`], e a alça é peça da moldura.
//!   [`Input::resize_none`] desliga a alça em qualquer outro lugar, com o nome da classe do coss.
//!
//! # Sem cobertura de teste — declarado
//!
//! - **O visual da alça e o que o mouse faz com ele**: o glifo desenhado, o realce de hover e o
//!   `CursorStyle::ResizeUpDown`. Nenhum dos três é observável de um teste de janela — o `debug_bounds`
//!   dá caixa, não tinta nem cursor —, e como todo pixel desta lib eles são revistos em janela. O que
//!   É testável está travado: o alvo caber na moldura, os três lados (19/21/23), a fração de tinta e o
//!   caminho do ícone existir no bundle.
//!
//!   O **gesto**, ao contrário do visual, é testado de verdade: `tests/textarea_resize_handle.rs`
//!   arrasta a alça numa janela e mede a caixa — inclusive o conflito com o [`growing`], que é a
//!   pergunta de projeto mais delicada da peça.
//! - **A altura sobreviver a um `InputState` novo.** Ela mora numa tabela chaveada pelo
//!   [`gpui::EntityId`] do estado (ver `AlturasDeAlca`), então um estado recriado nasce sem altura
//!   arrastada. É consequência declarada de não pôr um campo nosso dentro do núcleo vendorizado, e
//!   não há teste porque não há comportamento a garantir — há um limite a lembrar.
//! - **A entrada da tabela ser recolhida quando o campo morre.** Não há hook de `drop` de entidade
//!   alcançável de um elemento `RenderOnce`; a tabela só cresce, e cresce um item por arraste manual.
//!   Declarado no doc de `AlturasDeAlca`.

use gpui::prelude::FluentBuilder;
use gpui::AnimationExt as _;
use gpui::{
    div, px, rgb, AnyElement, App, AppContext as _, Context, CursorStyle, Div, DragMoveEvent, Empty,
    Entity, EntityId, Focusable, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, Pixels, Render, RenderOnce, SharedString, StatefulInteractiveElement, Styled,
    Window,
};
use gpui_component::input::{Input as CoreInput, InputState};

use crate::color::{opaque, Rgba8};
use crate::theme;


// =================================================================================================
// Construtores de estado (evitam o footgun de configurar o núcleo em dois lugares)
// =================================================================================================

/// Estado de um campo de **uma linha**. Use com `cx.new(|cx| single_line(window, cx))`.
///
/// Aceita os builders do núcleo por cima: `.placeholder(..)`, `.default_value(..)`,
/// `.masked(true)`, `.pattern(..)`, `.validate(..)`.
pub fn single_line(window: &mut Window, cx: &mut Context<InputState>) -> InputState {
    InputState::new(window, cx)
}

/// Estado de uma **textarea de caixa parada**, com `rows` linhas visíveis e quebra de linha ligada.
/// O texto que passar de `rows` linhas ROLA dentro da caixa — é o `<textarea>` clássico.
///
/// Existe pra evitar um erro fácil: a altura da moldura e o modo multi-linha do núcleo são duas
/// configurações diferentes, e ligar só uma dá um campo de uma linha com cara de textarea (ou o
/// contrário). Aqui o `rows` vale pros dois — passe o MESMO valor pro [`Input::rows`].
///
/// Pra reproduzir o `field-sizing-content` da referência (caixa que cresce com o conteúdo), use
/// [`growing`].
///
/// # Por que isto é um `auto_grow` de piso e teto IGUAIS, e não o modo "texto puro" do núcleo
///
/// Mesmo piso e mesmo teto é exatamente "caixa parada": o miolo pede SEMPRE `rows` linhas, e o que
/// passar disso rola por dentro. Foi preciso porque a moldura de uma textarea declara `min_h` (é o
/// piso do `field-sizing-content`, ver o doc do módulo) — e no modo `multi_line` do núcleo o elemento
/// de texto pede **uma** linha de altura mínima e conta com o pai pra lhe dar o resto, por
/// porcentagem. Porcentagem de filho contra pai dimensionado por `min_h` não resolve no layout, e o
/// resultado medido era uma caixa de 92px em que só os 20 primeiros aceitavam clique: o texto aparecia
/// inteiro e as linhas de baixo eram inclicáveis. Com `auto_grow` o miolo pede a altura inteira e a
/// moldura só confirma.
///
/// O teto é `max(2)` porque um teto de 1 desligaria o multi-linha do núcleo (lá `is_multi_line` é
/// `max_rows > 1`) e o campo passaria a recusar Enter. Uma textarea de uma linha é um
/// [`single_line`] — não use `rows = 1` aqui.
pub fn multi_line(rows: usize, window: &mut Window, cx: &mut Context<InputState>) -> InputState {
    InputState::new(window, cx)
        .auto_grow(rows, rows.max(2))
        .soft_wrap(true)
}

/// Estado de uma **textarea que cresce com o conteúdo**, de `min_rows` até `max_rows` linhas — é o
/// `field-sizing-content` da referência.
///
/// Passe o MESMO `min_rows` pro [`Input::rows`] (ou use [`TEXTAREA_ROWS`] nos dois, que é o piso da
/// referência): o piso da moldura e o piso do miolo são duas configurações diferentes, e discordar
/// dá um campo que já nasce com um vão embaixo do texto.
///
/// # Por que o teto é obrigatório
///
/// O elemento do núcleo pede `linhas_quebradas.min(max_rows) × entrelinha` de altura mínima, e é daí
/// que o crescimento vem. Sem `max_rows` não há o que clampar — e um textarea sem teto empurraria o
/// formulário inteiro pra fora da janela no primeiro parágrafo colado. Passado o teto, o texto rola
/// dentro da caixa, como no [`multi_line`].
///
/// ```ignore
/// // uma vez, no `new` da view: cresce de 3 (o piso do coss) até 10 linhas.
/// let notas = cx.new(|cx| empire_ui::input::growing(input::TEXTAREA_ROWS, 10, window, cx));
///
/// // a cada render:
/// Input::new(&self.notas).rows(empire_ui::input::TEXTAREA_ROWS).label("Notas")
/// ```
pub fn growing(
    min_rows: usize,
    max_rows: usize,
    window: &mut Window,
    cx: &mut Context<InputState>,
) -> InputState {
    InputState::new(window, cx)
        // `auto_grow` já implica multi-linha (`is_multi_line` = `max_rows > 1`), mas a quebra de
        // linha é decisão separada e é o que faz o crescimento acompanhar o texto ENVOLVIDO, e não
        // só os `\n`.
        .auto_grow(min_rows, max_rows)
        .soft_wrap(true)
}

// =================================================================================================
// Atalhos que faltam pra paridade com o navegador
// =================================================================================================

/// O handler de "clicar fora tira o foco", compartilhado pelo [`Input`] e pelo [`crate::select`].
///
/// # Por que existe
///
/// O GPUI não desfoca nada sozinho: um campo focado continua focado — e com o anel aceso — mesmo
/// depois de um clique no vazio. No navegador não é assim, e a diferença incomoda: o campo fica
/// parecendo ativo quando não está.
///
/// # Por que dá pra desfocar SÍNCRONO, e por que isso não é óbvio
///
/// O risco aparente é o caso mais comum de todos: clicar do campo A **direto no campo B**. Ali o
/// mesmo evento de mouse tira o foco do A e dá foco ao B — na ordem errada, o A apagaria o foco que o
/// B acabou de ganhar e o clique não focaria nada.
///
/// Não acontece, e a razão está na fase de despacho:
///
/// - `on_mouse_down_out` roda em **`DispatchPhase::Capture`** (declarado no doc do GPUI);
/// - o foco de um `track_focus` roda em **`DispatchPhase::Bubble`** (`gpui/src/elements/div.rs`, no
///   `on_mouse_event` do `Interactivity::paint`).
///
/// Captura vem antes de bolha, então este desfoque **sempre** precede o foco do vizinho e o resultado
/// final é o vizinho focado. O teste `do_campo_a_pro_campo_b_o_b_fica_focado` em
/// `tests/blur_on_outside_click.rs` é o que garante isso: se a ordem mudar no GPUI, ele cai.
///
/// Uma primeira versão adiava a checagem um frame ("se ainda sou eu que estou focado, ninguém tomou
/// meu lugar"), o que seria correto sob qualquer ordem — mas `Window::on_next_frame` só empilha o
/// callback, e ele é drenado por um callback de frame da plataforma que o executor de teste não
/// dirige: o desfoque simplesmente nunca acontecia. Com a ordem das fases provada, o síncrono é mais
/// simples e verificável.
pub fn blur_on_outside_click(
    handle: gpui::FocusHandle,
) -> impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static {
    move |_event, window, _cx| {
        if handle.is_focused(window) {
            // `Window::blur` já pede o frame novo.
            window.blur();
        }
    }
}

/// Registra os atalhos de teclado que o núcleo do `gpui-component` **não** vincula, pra fechar a
/// paridade com o navegador. Chame **uma vez** no bootstrap, depois do `gpui_component::init(cx)`:
///
/// ```ignore
/// gpui_component::init(cx);
/// empire_ui::input::init(cx);
/// ```
///
/// É explícito (e não um efeito colateral de usar o componente) porque `bind_keys` é global da
/// app: uma lib não deve mexer no mapa de teclas de quem a usa sem que a app peça.
///
/// O núcleo já cobre o essencial — `cmd/ctrl A·C·V·X·Z`, `ctrl-y`, setas com `alt`/`cmd`,
/// `home`/`end`, `shift+` pra estender, `pageup`/`pagedown`. O que se adiciona aqui:
///
/// - **`ctrl-shift-z` → Redo**: no navegador em Windows/Linux é o refazer mais usado; o núcleo só
///   vincula `ctrl-y`.
/// - **`ctrl-home`/`ctrl-end` → começo/fim do conteúdo** e as versões com `shift` pra selecionar:
///   é o padrão web fora do macOS (no macOS o equivalente é `cmd-up`/`cmd-down`, que o núcleo já
///   tem). Registrados só fora do macOS pra não criar um atalho que ninguém espera no Mac.
pub fn init(cx: &mut gpui::App) {
    use gpui::KeyBinding;
    use gpui_component::input::Redo;

    // O contexto de teclas do núcleo. É a string que o `Input` empilha no dispatch, então um
    // bind aqui só vale com um campo de texto focado (não rouba a tecla do resto da app).
    const INPUT_CONTEXT: &str = "Input";

    cx.bind_keys([KeyBinding::new("ctrl-shift-z", Redo, Some(INPUT_CONTEXT))]);

    // Arma o rastreio de modalidade que decide se o anel de foco aparece. Fica aqui, e não numa
    // `init` própria, pra que todo host que já inicializava o crate ganhe isso sem mudar nada — sem
    // ele, navegar por `Tab` não acenderia anel em lugar nenhum. Ver `crate::focus_ring`.
    crate::focus_ring::init(cx);

    // As teclas de passo do `crate::number_field` (`↑`/`↓`, com `alt`/`shift`, e `Home`/`End`). Fica
    // aqui, e não numa `init` própria, pelo mesmo motivo do `focus_ring`: todo host que já
    // inicializava o crate ganha isso sem mudar nada.
    //
    // ⚠️ **A ordem importa.** Os bindings de lá sobrescrevem o `up`→`MoveUp` do núcleo por ordem de
    // registro (ver `crate::number_field::PREDICADO`), então esta linha tem que rodar DEPOIS do
    // `gpui_component::init` — que é o que o doc desta função já manda.
    crate::number_field::init(cx);

    // Alinha as cores de texto/placeholder do núcleo já no arranque — sem isto o primeiro frame
    // sai com o texto no tema do `gpui-component` e a moldura no nosso. Nas trocas de tema quem
    // cuida disso é `crate::theme::sync_core_theme`.
    apply_core_text_colors(cx);

    // Fora do macOS: começo/fim do conteúdo com `ctrl`. No macOS o equivalente é `cmd-up`/
    // `cmd-down`, que o núcleo já vincula — registrar `ctrl-home` lá criaria um atalho que
    // ninguém espera no Mac.
    #[cfg(not(target_os = "macos"))]
    {
        use gpui_component::input::{MoveToEnd, MoveToStart, SelectToEnd, SelectToStart};
        cx.bind_keys([
            KeyBinding::new("ctrl-home", MoveToStart, Some(INPUT_CONTEXT)),
            KeyBinding::new("ctrl-end", MoveToEnd, Some(INPUT_CONTEXT)),
            KeyBinding::new("ctrl-shift-home", SelectToStart, Some(INPUT_CONTEXT)),
            KeyBinding::new("ctrl-shift-end", SelectToEnd, Some(INPUT_CONTEXT)),
        ]);
    }
}

// =================================================================================================
// Paleta do campo (visual portado do `coss`)
// =================================================================================================
//
// O visual do campo segue o `Input` do design system [coss][1]. Os valores abaixo são os utilitários
// Tailwind daquele componente **resolvidos** para números concretos, para um app nativo:
//
// - O breakpoint `sm:` do Tailwind é ≥640px, e uma janela de desktop está sempre acima disso — então
//   as alturas efetivas são as variantes `sm:` (30px no padrão, não 34px).
// - As cores vêm das custom properties do tema do coss, com a paleta `neutral`/`red` do Tailwind
//   resolvida: `neutral-400 #a3a3a3`, `neutral-500 #737373`, `neutral-800 #262626`,
//   `neutral-100 #f5f5f5`, `red-500 #ef4444`. Onde o coss usa `color-mix`, o resultado já está
//   calculado aqui.
//
// Fica num struct próprio (e não na [`crate::theme::Palette`]) porque são tokens de UM componente —
// é o mesmo padrão que o `pve-ui` usa pros tokens de timeline.
//
// [1]: https://github.com/cosscom/coss — `apps/ui/registry/default/ui/input.tsx`

/// Tokens visuais do campo, por tema.
///
/// ⚠️ **TODAS as cores aqui são `0xRRGGBBAA`** — com o byte de alfa, sempre, mesmo quando opacas
/// (`…ff`). É o formato que o [`gpui::rgba`] espera, e o único consumido neste módulo.
///
/// A convenção é uniforme de propósito. Uma versão anterior misturava valores de 6 dígitos
/// (opacos, pra `rgb`) com os de 8 na mesma struct e mandava todos pro `rgba`: `rgba(0xffffff)` é
/// lido como `0x00FFFFFF`, ou seja **ciano**, e foi exatamente o que apareceu na tela (campos
/// ciano no tema claro, borda de foco esverdeada no escuro). Com uma convenção só, esse erro não
/// tem como voltar.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FieldPalette {
    /// Fundo do campo. No escuro é translúcido (`bg-input/32`), então "levanta" sobre o painel.
    pub(crate) bg: Rgba8,
    /// Cor do texto digitado (`--foreground`).
    ///
    /// `pub(crate)` porque quem desenha peças DENTRO de um campo precisa da mesma tinta do texto que
    /// está ao lado — é o que o [`crate::scrub_input`] usa no diamante de keyframe aceso. Copiar o
    /// número lá seria uma segunda fonte de verdade pro `--foreground` do campo.
    pub(crate) text: Rgba8,
    /// Cor do placeholder (`--muted-foreground` a 72%).
    pub(crate) placeholder: Rgba8,
    /// Borda em repouso (`--input`).
    border: Rgba8,
    /// Borda quando focado (`--ring`, sólida).
    ring: Rgba8,
    /// Halo de foco de 3px (`ring-ring/24`).
    ring_glow: Rgba8,
    /// Borda quando inválido e SEM foco (`--destructive` a 36%).
    danger_border: Rgba8,
    /// Borda quando inválido e COM foco (`--destructive` a 64%).
    danger_border_focus: Rgba8,
    /// Halo de foco quando inválido (16% no claro, 24% no escuro).
    danger_glow: Rgba8,
    /// Cor de mensagem/asterisco de erro (`--destructive` sólido).
    danger: Rgba8,
    /// Sombra externa em repouso (`shadow-xs/5`).
    shadow: Rgba8,
    /// Fio de bisel de 1px em repouso. No claro é escuro e desce 1px; no escuro é claro e sobe 1px
    /// — é o `before:shadow-[0_1px_black/4%]` / `[0_-1px_white/6%]` do coss.
    bevel: Rgba8,
    /// Sentido do bisel: `+1` desce (claro), `-1` sobe (escuro).
    bevel_dir: f32,
}

/// Tema **claro** do campo.
const FIELD_LIGHT: FieldPalette = FieldPalette {
    bg: Rgba8(0xffffffff),
    text: Rgba8(0x262626ff),
    // muted-foreground = mix(neutral-500 90%, black) = #686868, a 72% de alfa.
    placeholder: Rgba8(0x686868b8),
    border: Rgba8(0x0000001a), // black 10%
    ring: Rgba8(0xa3a3a3ff), // neutral-400
    ring_glow: Rgba8(0xa3a3a33d),
    danger_border: Rgba8(0xef44445c),
    danger_border_focus: Rgba8(0xef4444a3),
    danger_glow: Rgba8(0xef444429), // 16%
    danger: Rgba8(0xef4444ff),
    shadow: Rgba8(0x0000000d), // black 5%
    bevel: Rgba8(0x0000000a),  // black 4%
    bevel_dir: 1.0,
};

/// Tema **escuro** do campo.
const FIELD_DARK: FieldPalette = FieldPalette {
    // `dark:bg-input/32`: --input escuro é branco a 8%, e o /32 do Tailwind o multiplica → ~2.5%.
    bg: Rgba8(0xffffff07),
    text: Rgba8(0xf5f5f5ff),
    // muted-foreground = mix(neutral-500 90%, white) = #818181, a 72%.
    placeholder: Rgba8(0x818181b8),
    border: Rgba8(0xffffff14), // white 8%
    ring: Rgba8(0x737373ff), // neutral-500
    ring_glow: Rgba8(0x7373733d),
    // destructive escuro = mix(red-500 90%, white) = #f15757.
    danger_border: Rgba8(0xf157575c),
    danger_border_focus: Rgba8(0xf15757a3),
    danger_glow: Rgba8(0xf157573d), // 24% no escuro
    danger: Rgba8(0xf15757ff),
    shadow: Rgba8(0x0000000d),
    // ⚠️ DESVIO CONSCIENTE do coss: o original usa branco a **6%** (alfa 15). Aqui é o DOBRO —
    // alfa 30 ≈ 11,8% — por decisão de design: a 6% o filete é imperceptível no nosso fundo escuro.
    // NÃO "corrija" isto pra 0x0f achando que é erro de porte; se a intenção mudar, mude junto o
    // teste `bisel_escuro_e_o_dobro_da_referencia`.
    bevel: Rgba8(0xffffff1e),
    bevel_dir: -1.0,
};

/// A paleta do campo no tema corrente.
pub(crate) fn field() -> &'static FieldPalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &FIELD_DARK,
        theme::ThemeMode::Light => &FIELD_LIGHT,
    }
}

/// Raio da moldura — `rounded-lg` = `--radius` = `0.625rem` = **10px**.
pub(crate) const FIELD_RADIUS: f32 = 10.0;

/// Espessura do halo de foco — `has-focus-visible:ring-[3px]`.
const FOCUS_RING_WIDTH: f32 = 3.0;

/// Duração da transição do halo de foco — o `transition-shadow` do coss.
///
/// 150ms é o default do Tailwind (`transition-*` sem `duration-*`).
pub(crate) const FOCUS_TRANSITION: std::time::Duration = std::time::Duration::from_millis(150);

/// Teto da tabela de transições. Estourar só faz as próximas transições daqueles campos saírem
/// instantâneas — degrada, não quebra.
const FOCUS_TABLE_CAP: usize = 512;

thread_local! {
    /// Último estado de foco visto por campo, com o instante em que mudou.
    ///
    /// Existe porque o [`Input`] é um elemento `RenderOnce` — sem estado próprio, ele não saberia
    /// que o campo ACABOU de perder o foco, e o halo sumiria de um frame pro outro em vez de
    /// desvanecer. É a mesma natureza do `thread_local` do tema: a UI roda numa thread só.
    static FOCUS_TRANSITIONS: std::cell::RefCell<
        std::collections::HashMap<gpui::EntityId, (bool, std::time::Instant)>,
    > = std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Se o campo está no meio de uma transição de foco, devolve `Some(entrando)`; `None` se está
/// estável (e portanto não precisa animar).
///
/// O PRIMEIRO render de um campo nunca anima: sem isto, todo campo da tela piscaria um halo ao
/// abrir a janela.
pub(crate) fn focus_transition(id: gpui::EntityId, focused: bool) -> Option<bool> {
    FOCUS_TRANSITIONS.with(|table| {
        let mut table = table.borrow_mut();
        if table.len() > FOCUS_TABLE_CAP {
            table.clear();
        }
        match table.get(&id).copied() {
            // Primeiro render: registra já "vencido", pra renderizar o estado final direto.
            None => {
                let past = std::time::Instant::now() - FOCUS_TRANSITION;
                table.insert(id, (focused, past));
                None
            }
            Some((prev, since)) => {
                if prev != focused {
                    table.insert(id, (focused, std::time::Instant::now()));
                    Some(focused)
                } else if since.elapsed() < FOCUS_TRANSITION {
                    // Ainda dentro da janela: segue animando (o id do `with_animation` é estável
                    // aqui, então o GPUI continua a animação de onde estava).
                    Some(focused)
                } else {
                    None
                }
            }
        }
    })
}



/// **Entrelinha do miolo de uma textarea** — o par do Tailwind pro `sm:text-sm` do wrapper: corpo
/// 14, entrelinha **20**.
///
/// ⚠️ **É o número mais perigoso deste arquivo.** Num campo de uma linha a entrelinha é a altura do
/// miolo (é o truque que centra o glifo, ver [`InputSize::content_height`]) e um erro ali só desloca
/// o texto; numa textarea ela multiplica: decide a altura de CADA linha, então erra o piso, erra o
/// crescimento e erra a conta de "quantas linhas cabem". Sem fixá-la, o default do GPUI é
/// `relative(1.618_034)` → **22,65px** sobre 14px, e o piso de 70 do coss passaria a caber 3,09
/// linhas em vez de 3.
///
/// Travada em `entrelinha_da_textarea_e_o_par_do_tailwind`, com o valor que sairia sem fixar.
pub const TEXTAREA_LINE_HEIGHT: f32 = 20.0;

/// **Piso da textarea em linhas** — as três variantes do coss (`min-h-17.5/16.5/18.5`) são, todas,
/// espaço pra exatamente **três** linhas mais o respiro vertical do tamanho.
///
/// Existe pra quem cria o estado com [`growing`] poder pedir o piso da referência pelo nome, em vez
/// de repetir um `3` que ninguém sabe de onde veio.
pub const TEXTAREA_ROWS: usize = 3;

/// Alinha as cores de texto do NÚCLEO com esta paleta.
///
/// Necessário porque o elemento de texto do `gpui-component` pinta o conteúdo e o placeholder com
/// `cx.theme().foreground` / `.muted_foreground` — cores do tema **dele**, que um
/// `.text_color()` do nosso lado não alcança.
///
/// Chame junto de toda troca de tema (é o que [`crate::theme::sync_core_theme`] faz).
pub(crate) fn apply_core_text_colors(cx: &mut App) {
    let f = field();
    let theme = cx.global_mut::<gpui_component::Theme>();
    theme.foreground = f.text.hsla();
    theme.muted_foreground = f.placeholder.hsla();
}

// =================================================================================================
// Tamanho
// =================================================================================================

/// Tamanho do campo. Controla altura, corpo do texto, respiro horizontal e raio de uma vez —
/// pra não existir campo "quase md" com números escolhidos a olho no call site.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum InputSize {
    /// Compacto — inspector, barras de ferramentas, tabelas densas.
    Sm,
    /// Padrão.
    #[default]
    Md,
    /// Confortável — formulários de destaque, telas de onboarding.
    Lg,
}

impl InputSize {
    /// Altura do MIOLO — o `sm:h-6.5 / sm:h-7.5 / sm:h-8.5` do coss, que no original está no
    /// `<input>` (e não no wrapper).
    ///
    /// É também o `leading-*`: no coss a altura de linha é IGUAL à altura, e é exatamente assim que
    /// o texto fica centralizado. Ver [`Self::height`].
    pub fn content_height(self) -> f32 {
        match self {
            InputSize::Sm => 26.0,
            InputSize::Md => 30.0,
            InputSize::Lg => 34.0,
        }
    }

    /// Altura EXTERNA da moldura = miolo + as duas bordas de 1px.
    ///
    /// No coss o `h-*` está no `<input>` e a borda no wrapper, que não tem altura própria — então a
    /// caixa visível mede `h + 2px`. É por isso que o padding horizontal lá é
    /// `calc(--spacing(3) - 1px)`: o `-1px` desconta a borda pra o recuo TOTAL a partir da borda
    /// externa fechar em 12px. Uma versão anterior daqui usava o `h-*` como altura externa, e os
    /// três tamanhos saíam 2px curtos.
    pub fn height(self) -> f32 {
        self.content_height() + 2.0
    }

    /// Corpo do texto digitado.
    ///
    /// **14px nos três tamanhos** — no coss o corpo vem do wrapper (`sm:text-sm`) e as variantes de
    /// tamanho mudam só altura, respiro e line-height. Antes daqui variávamos o corpo por tamanho;
    /// isso saiu pra bater com o original.
    pub fn text_size(self) -> f32 {
        let _ = self;
        14.0
    }

    /// Respiro horizontal interno — `px-[calc(--spacing(2.5)-1px)]` no Sm e
    /// `px-[calc(--spacing(3)-1px)]` nos outros. O `-1px` desconta a borda.
    pub fn pad_x(self) -> f32 {
        match self {
            InputSize::Sm => 9.0,
            InputSize::Md | InputSize::Lg => 11.0,
        }
    }

    /// Raio da moldura — `rounded-lg`, igual nos três tamanhos.
    pub fn radius(self) -> f32 {
        let _ = self;
        FIELD_RADIUS
    }

    /// Entrelinha do miolo de uma **textarea** — [`TEXTAREA_LINE_HEIGHT`], igual nos três tamanhos.
    ///
    /// Não varia porque o corpo do texto também não varia (ver [`Self::text_size`]): no coss os dois
    /// vêm do `sm:text-sm` do wrapper, e as variantes de tamanho mudam só piso e respiro. Existe como
    /// método — em vez de a moldura ler a constante — pra que TODA métrica do campo seja lida do mesmo
    /// lugar, e um tamanho futuro que precise mudar a entrelinha tenha um só lugar pra fazê-lo.
    pub fn textarea_line_height(self) -> f32 {
        let _ = self;
        TEXTAREA_LINE_HEIGHT
    }

    /// Respiro VERTICAL do miolo de uma textarea — `py-[calc(--spacing(1.5)-1px)]` no `Md`,
    /// `(1)` no `Sm` e `(2)` no `Lg`. O `-1px` desconta a borda, igual ao [`Self::pad_x`].
    ///
    /// Só existe no multi-linha: num campo de uma linha o respiro vertical é **zero**, e o que
    /// centra o texto é a entrelinha igual à altura do miolo (ver [`Self::content_height`]).
    pub fn textarea_pad_y(self) -> f32 {
        match self {
            InputSize::Sm => 3.0,
            InputSize::Md => 5.0,
            InputSize::Lg => 7.0,
        }
    }

    /// **Piso** da altura do MIOLO de uma textarea — `min-h-16.5 / 17.5 / 18.5` do coss, ou seja
    /// 66 / 70 / 74.
    ///
    /// É calculado, não tabelado, porque os três valores da referência são a mesma frase: espaço pra
    /// [`TEXTAREA_ROWS`] linhas de [`TEXTAREA_LINE_HEIGHT`] mais o respiro vertical do tamanho. Ter
    /// os três números escritos à mão aqui esconderia isso e deixaria três oportunidades de errar um.
    ///
    /// É PISO, não altura: com [`growing`] a caixa cresce daqui pra cima (ver o doc do módulo).
    pub fn textarea_min_content_height(self) -> f32 {
        TEXTAREA_ROWS as f32 * TEXTAREA_LINE_HEIGHT + 2.0 * self.textarea_pad_y()
    }

    /// Piso da altura EXTERNA de uma textarea = piso do miolo + as duas bordas de 1px — 68 / 72 / 76.
    ///
    /// Mesma relação (e mesmo porquê) de [`Self::content_height`] pra [`Self::height`]: no coss o
    /// `min-h-*` está no `<textarea>` e a borda no wrapper, que não tem altura própria.
    pub fn textarea_min_height(self) -> f32 {
        self.textarea_min_content_height() + 2.0
    }

    /// Lado dos ícones de prefixo/sufixo.
    pub fn icon(self) -> f32 {
        match self {
            InputSize::Sm => 13.0,
            InputSize::Md => 15.0,
            InputSize::Lg => 17.0,
        }
    }

    /// Corpo do texto da label e da linha de apoio (hint / erro / contador).
    pub fn aux_text_size(self) -> f32 {
        match self {
            InputSize::Sm => 10.0,
            InputSize::Md => 11.0,
            InputSize::Lg => 12.0,
        }
    }
}

// =================================================================================================
// Validade
// =================================================================================================

/// Estado de validade do campo. Enum (e não `is_error: bool` + `is_success: bool`) pra tornar
/// "erro e sucesso ao mesmo tempo" inexpressável.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Validity {
    /// Sem julgamento — a moldura fica neutra e o hint aparece normalmente.
    #[default]
    Neutral,
    /// Inválido, com a mensagem que **substitui** o hint.
    Error(SharedString),
    /// Válido, com a mensagem de confirmação.
    Success(SharedString),
}

impl Validity {
    /// Cor da borda/mensagem deste estado, ou `None` quando neutro.
    fn color(&self) -> Option<Rgba8> {
        match self {
            Validity::Neutral => None,
            Validity::Error(_) => Some(field().danger),
            Validity::Success(_) => Some(opaque(theme::SUCCESS())),
        }
    }

    /// A mensagem, se houver.
    fn message(&self) -> Option<&SharedString> {
        match self {
            Validity::Neutral => None,
            Validity::Error(m) | Validity::Success(m) => Some(m),
        }
    }
}

// =================================================================================================
// O componente
// =================================================================================================

/// Largura do campo.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
enum Width {
    /// Ocupa a largura disponível do container.
    #[default]
    Full,
    /// Largura fixa.
    Fixed(Pixels),
}

/// De onde vem o **piso** vertical de uma textarea.
///
/// É um enum (e não um `f32` já resolvido no builder) porque os dois caminhos são pedidos
/// diferentes: "o piso da referência pro meu tamanho" acompanha o tamanho se ele mudar depois, e "um
/// piso de N linhas" não. Guardar só o número apagaria essa diferença.
#[derive(Clone, Copy, PartialEq, Debug)]
enum TextareaFloor {
    /// O `min-h-*` do tamanho — o piso da referência ([`TEXTAREA_ROWS`] linhas).
    Size,
    /// Piso de `n` linhas.
    Rows(usize),
}

impl TextareaFloor {
    /// O piso da altura do MIOLO, em px, no tamanho dado.
    ///
    /// `Rows(3)` e [`Self::Size`] dão o MESMO número — é a conta da referência, e é a prova de que os
    /// dois modelos (linhas × `min-h`) convergiram. Travado em
    /// `piso_por_linhas_bate_com_o_piso_da_referencia`.
    fn min_content_height(self, size: InputSize) -> f32 {
        match self {
            TextareaFloor::Size => size.textarea_min_content_height(),
            // `max(1)`: `rows(0)` é erro de quem chama, e uma caixa de altura zero fica inclicável —
            // uma linha é o mínimo que ainda é um campo.
            TextareaFloor::Rows(n) => {
                n.max(1) as f32 * TEXTAREA_LINE_HEIGHT + 2.0 * size.textarea_pad_y()
            }
        }
    }

    /// O piso da altura EXTERNA — miolo + as duas bordas de 1px da moldura.
    ///
    /// O ramo [`Self::Size`] delega em [`InputSize::textarea_min_height`] em vez de somar 2 aqui, pra
    /// que o piso da referência tenha **um** lugar só: quem for ler o número no tamanho e quem for
    /// desenhá-lo leem a mesma função.
    fn min_height(self, size: InputSize) -> f32 {
        match self {
            TextareaFloor::Size => size.textarea_min_height(),
            TextareaFloor::Rows(_) => self.min_content_height(size) + 2.0,
        }
    }
}

/// Campo de texto com a moldura completa de formulário.
///
/// É um **elemento de render** (não uma entidade): construa a cada frame, apontando pro
/// [`InputState`] que guarda o texto. Ver o doc do módulo pro conjunto todo.
#[derive(IntoElement)]
pub struct Input {
    /// Respiro horizontal do miolo, quando o call site sobrescreve o do tamanho. Ver [`Input::pad_x`].
    pad_x: Option<f32>,
    /// Altura do MIOLO, quando sobrescrita. Ver [`Input::height`].
    content_height: Option<f32>,
    /// Corpo do texto digitado, quando sobrescrito. Ver [`Input::text_size`].
    text_size: Option<f32>,
    /// Raio da moldura, quando sobrescrito. Ver [`Input::radius`].
    radius: Option<f32>,
    pub(crate) state: Entity<InputState>,
    label: Option<SharedString>,
    hint: Option<SharedString>,
    validity: Validity,
    required: bool,
    size: InputSize,
    disabled: bool,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    addon_before: Option<SharedString>,
    addon_after: Option<SharedString>,
    /// Como este campo encosta nos vizinhos, quando está num [`crate::group::Group`]. Quem preenche
    /// é o grupo, pela posição — ver `crate::group::GroupChild`.
    pub(crate) join: crate::group::Join,
    max_len: Option<usize>,
    counter: bool,
    clearable: bool,
    mask_toggle: bool,
    width: Width,
    /// Piso vertical quando o campo é **multi-linha**; `None` = campo de uma linha. Ver
    /// [`Input::textarea`].
    textarea: Option<TextareaFloor>,
    /// Sem a moldura própria — a superfície é de quem hospeda. Ver [`Input::unstyled`].
    unstyled: bool,
    /// Sem a alça de redimensionar. Ver [`Input::resize_none`].
    resize_none: bool,
}

impl Input {
    /// Cria o campo apontando pro estado dado.
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            pad_x: None,
            content_height: None,
            text_size: None,
            radius: None,
            state: state.clone(),
            label: None,
            hint: None,
            validity: Validity::Neutral,
            required: false,
            size: InputSize::default(),
            disabled: false,
            prefix: None,
            suffix: None,
            addon_before: None,
            addon_after: None,
            join: crate::group::Join::NONE,
            max_len: None,
            counter: false,
            clearable: false,
            mask_toggle: false,
            width: Width::default(),
            textarea: None,
            unstyled: false,
            resize_none: false,
        }
    }

    /// Label acima do campo.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Marca como obrigatório (asterisco ao lado da label). **Só sinaliza** — não valida nada;
    /// quem valida é quem usa (ver a seção de validação no doc do módulo).
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Texto de ajuda abaixo do campo. Fica escondido enquanto houver mensagem de erro/sucesso.
    pub fn hint(mut self, hint: impl Into<SharedString>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Marca o campo como **inválido**, com a mensagem.
    pub fn error(mut self, message: impl Into<SharedString>) -> Self {
        self.validity = Validity::Error(message.into());
        self
    }

    /// Marca o campo como **válido**, com a mensagem.
    pub fn success(mut self, message: impl Into<SharedString>) -> Self {
        self.validity = Validity::Success(message.into());
        self
    }

    /// Define a validade direto (útil quando ela vem de um `match` na sua lógica).
    pub fn validity(mut self, validity: Validity) -> Self {
        self.validity = validity;
        self
    }

    /// Tamanho do campo.
    pub fn size(mut self, size: InputSize) -> Self {
        self.size = size;
        self
    }

    /// Desabilita: o campo não recebe foco nem edição, e a moldura esmaece.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Elemento colado à **esquerda** dentro da moldura (ícone, normalmente).
    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }

    /// Elemento colado à **direita** dentro da moldura (unidade, botão, ícone).
    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    /// **Botão de ícone pra pôr no [`Self::suffix`]** — a mesma peça que o olho do campo de senha.
    ///
    /// Existe porque um [`crate::Button`] aqui **não serve**: ele traz respiro próprio, e isso abre um
    /// vão entre o ícone e a borda direita do campo (foi por isso que o olho nunca foi um `Button`).
    /// Este alvo é quadrado de `icone + 6`, raio 4, e o realce de hover é o do fundo encaixado.
    ///
    /// Recebe o **tamanho do campo** porque o ícone acompanha a escala — passar o do campo errado
    /// deixa o ícone fora de proporção, e não há como a função adivinhar. E recebe o `disabled` do
    /// campo porque desabilitado ele perde hover, cursor e o clique, e esmaece: sem isso, o olho do
    /// campo de senha teria que manter uma cópia própria deste desenho, e as duas divergiriam.
    ///
    /// ```ignore
    /// Input::new(&state).size(InputSize::Sm).suffix(Input::icon_button(
    ///     ("copy", state.entity_id()),
    ///     "icons/copy.svg",
    ///     InputSize::Sm,
    ///     false,
    ///     cx.listener(|this, _, _, cx| this.copiar(cx)),
    /// ))
    /// ```
    pub fn icon_button(
        id: impl Into<gpui::ElementId>,
        icon: impl Into<SharedString>,
        size: InputSize,
        disabled: bool,
        on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    ) -> impl IntoElement {
        div()
            .id(id.into())
            .flex()
            .items_center()
            .justify_center()
            .flex_none()
            .size(px(size.icon() + 6.0))
            .rounded(px(4.0))
            .when(!disabled, |d| {
                d.cursor(gpui::CursorStyle::PointingHand)
                    .hover(|h| h.bg(rgb(theme::BG_INSET_HOVER())))
                    .on_click(on_click)
            })
            .child(
                gpui::svg()
                    .path(icon.into())
                    .size(px(size.icon()))
                    .flex_none()
                    .text_color(rgb(if disabled {
                        theme::TEXT_FAINT()
                    } else {
                        theme::ICON()
                    })),
            )
    }

    /// Segmento fixo **antes** do campo, com fundo próprio e divisória (`https://`, `R$`).
    /// Diferente de [`Self::prefix`]: o addon é um bloco, não um ícone solto.
    pub fn addon_before(mut self, text: impl Into<SharedString>) -> Self {
        self.addon_before = Some(text.into());
        self
    }

    /// Segmento fixo **depois** do campo (`.com`, `kg`, `%`).
    pub fn addon_after(mut self, text: impl Into<SharedString>) -> Self {
        self.addon_after = Some(text.into());
        self
    }

    /// Liga o **contador de caracteres** com limite. Passa dos `max` → o contador fica em
    /// [`theme::DANGER`]; chegando perto (≥90%), em [`theme::WARNING`].
    ///
    /// ⚠️ O limite é **indicativo**: não corta a digitação. Barrar de verdade é decisão de quem
    /// usa (o núcleo tem `pattern`/`validate` pra isso) — um campo que para de aceitar tecla sem
    /// avisar é pior que um que mostra o excesso em vermelho.
    pub fn max_len(mut self, max: usize) -> Self {
        self.max_len = Some(max);
        self.counter = true;
        self
    }

    /// Liga o contador **sem** limite (só `N caracteres`).
    pub fn counter(mut self) -> Self {
        self.counter = true;
        self
    }

    /// Botão de **limpar** (✕) no canto direito quando há texto. Delegado ao núcleo.
    pub fn clearable(mut self) -> Self {
        self.clearable = true;
        self
    }

    /// Botão de **mostrar/esconder** o texto — para campo de senha (use com `.masked(true)` no
    /// estado).
    ///
    /// É um **toggle**: um clique revela, outro esconde. O botão do núcleo é press-and-hold (revela
    /// no mouse-down, esconde no mouse-up) e vem com o padding do `Button` dele, que abria um vão
    /// no fim do campo — por isso a apresentação é nossa.
    pub fn mask_toggle(mut self) -> Self {
        self.mask_toggle = true;
        self
    }

    /// Largura fixa (o default é ocupar o container).
    /// **Sobrescreve o respiro horizontal do miolo**, em px.
    ///
    /// O default vem do [`InputSize`] e é o valor do coss (9 no `Sm`, 11 nos outros) — use este
    /// escape hatch só quando o contexto for mais denso do que um formulário, e **declare** no
    /// componente que chama.
    ///
    /// Existe por um caso concreto: o [`crate::ScrubInput`] é um chip de inspector de 150px com
    /// ícone-alça e 4 caracteres de número, e os 9px do coss (pensados pra campo de formulário)
    /// empurravam o glifo 13px pra dentro. Sem este hook, a alternativa era o `ScrubInput` voltar a
    /// desenhar a própria superfície — que é justamente o que a refatoração dele acabou de remover.
    pub fn pad_x(mut self, pad: f32) -> Self {
        self.pad_x = Some(pad);
        self
    }

    /// **Sobrescreve a altura EXTERNA do campo**, em px (com as duas bordas de 1px).
    ///
    /// Mesmo escape hatch do [`Self::pad_x`], e a mesma regra: o default vem do [`InputSize`] (28 no
    /// `Sm`) e sobrescrever é desvio a ser declarado por quem chama. O valor guardado é o do MIOLO
    /// (`externa − 2`), que é o que o layout usa; o parâmetro é a externa porque é ela que se mede na
    /// tela e é nela que uma especificação de densidade é escrita.
    pub fn height(mut self, outer: f32) -> Self {
        self.content_height = Some(outer - 2.0);
        self
    }

    /// **Sobrescreve o corpo do texto digitado**, em px.
    ///
    /// Mesmo escape hatch do [`Self::pad_x`]. O default é **14 nos três tamanhos** — no coss o corpo
    /// vem do wrapper e as variantes de tamanho mudam só altura e respiro, então isto não é "o corpo
    /// do tamanho X", é uma exceção.
    ///
    /// ⚠️ A entrelinha do miolo continua sendo a **altura do miolo** (é o truque que centra o dígito
    /// sem `items_center` — ver o [`crate::otp_field`]), então ela acompanha [`Self::height`] e não
    /// este valor.
    pub fn text_size(mut self, size: f32) -> Self {
        self.text_size = Some(size);
        self
    }

    /// **Sobrescreve o raio da moldura**, em px.
    ///
    /// Mesmo escape hatch do [`Self::pad_x`] (o porquê está lá). O default é [`FIELD_RADIUS`] = 10 nos
    /// três tamanhos, porque no coss o raio do campo é `rounded-lg` e não muda com o tamanho — num
    /// campo BAIXO isso lê como pílula, e é aí que sobrescrever se justifica.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }


    pub fn width(mut self, width: Pixels) -> Self {
        self.width = Width::Fixed(width);
        self
    }

    /// **Torna o campo multi-linha**, com o piso vertical da referência: o `min-h-*` do tamanho
    /// (68 / 72 / 76 de altura externa), que é espaço pra [`TEXTAREA_ROWS`] linhas.
    ///
    /// O estado tem que concordar — crie-o com [`multi_line`] (caixa parada) ou [`growing`] (caixa que
    /// cresce). Só chamar isto deixaria um campo de UMA linha dentro de uma caixa alta.
    pub fn textarea(mut self) -> Self {
        self.textarea = Some(TextareaFloor::Size);
        self
    }

    /// Multi-linha com piso de `rows` linhas — use com o MESMO `rows` do [`multi_line`] (ou o mesmo
    /// `min_rows` do [`growing`]) usado no estado.
    ///
    /// ⚠️ **O significado mudou** e o número na tela mudou com ele. Antes era altura FIXA de
    /// `rows × 30 + 12`, com a entrelinha de um campo de uma linha e um respiro de 6px que não vinha
    /// da referência. Agora é **piso** de `rows × 20 + 2 × respiro do tamanho` — com a entrelinha
    /// certa (ver [`TEXTAREA_LINE_HEIGHT`]) e o respiro do coss.
    ///
    /// O reencontro dos dois modelos: no `Md`, `rows(3)` dá 72 de altura externa, que é exatamente o
    /// `min-h-17.5` da referência. Ou seja, `rows(TEXTAREA_ROWS)` ≡ [`Self::textarea`].
    pub fn rows(mut self, rows: usize) -> Self {
        self.textarea = Some(TextareaFloor::Rows(rows));
        self
    }

    /// Renderiza o campo **sem a moldura própria**: sem borda, fundo, sombra, bisel, anel de foco nem
    /// raio **e o esmaecimento de desabilitado** — só o miolo. Serve pra hospedar o campo dentro de
    /// outra superfície que já desenha tudo isso
    /// (é o `unstyled` da referência, e é o que o [`crate::input_group::InputGroup`] usa).
    ///
    /// # O que muda na geometria
    ///
    /// Some também o **par de bordas de 1px**, e com ele os 2px que a moldura acrescentava: o campo
    /// passa a medir o MIOLO (30 em vez de 32 no `Md`; piso de 70 em vez de 72 numa textarea). É o que
    /// a referência faz — lá o `h-7.5` do `<input>` sempre foi a altura do miolo, e as bordas eram do
    /// wrapper que agora é de quem hospeda. O respiro horizontal **não** muda: o `-1px` do
    /// `px-[calc(--spacing(3)-1px)]` continua descontando uma borda, que agora é a do hospedeiro.
    ///
    /// # O desfoque por clique fora passa a ser de quem hospeda
    ///
    /// Sem moldura, "fora do campo" deixa de querer dizer "fora do controle": num grupo, o clique cai
    /// no respiro do wrapper ou num addon, que são parte do MESMO controle — desfocar ali seria um
    /// bug. Então o handler sai daqui, e quem hospeda o põe na própria superfície:
    ///
    /// ```ignore
    /// div().on_mouse_down_out(input::blur_on_outside_click(state.read(cx).focus_handle(cx)))
    /// ```
    ///
    /// ([`blur_on_outside_click`] é público exatamente pra isso — o [`crate::select`] já o usa assim.)
    pub fn unstyled(mut self) -> Self {
        self.unstyled = true;
        self
    }

    /// **Desliga a alça de redimensionar** de uma textarea — o `resize-none` da referência, com o nome
    /// dela.
    ///
    /// A alça vem **ligada** em toda textarea, porque é o que o navegador faz: o `<textarea>` nasce
    /// redimensionável e o coss não o desliga no componente. Ela sai sozinha nos dois lugares em que a
    /// referência também a tira — dentro do [`crate::input_group::InputGroup`] (onde o campo é
    /// [`Input::unstyled`], que é exatamente onde o coss escreve `resize-none`) e num campo
    /// desabilitado. Este método é pro terceiro caso: a tela que quer uma caixa multi-linha de altura
    /// imutável.
    ///
    /// Num campo de UMA linha não faz nada — não há altura pra arrastar.
    pub fn resize_none(mut self) -> Self {
        self.resize_none = true;
        self
    }

    /// Cor da borda conforme a precedência de estados, seguindo o coss.
    ///
    /// A ordem é intencional e é a mesma que os seletores `has-*` dele impõem: inválido+foco é o
    /// estado mais específico, depois inválido, depois foco, depois repouso.
    fn border_color(&self, focused: bool) -> Rgba8 {
        border_color_for(matches!(self.validity, Validity::Error(_)), focused)
    }

    /// A pilha de sombras da moldura, conforme o estado.
    ///
    /// Em repouso o coss desenha duas coisas: a sombra externa (`shadow-xs/5`) e um fio de bisel de
    /// 1px (o pseudo-elemento `before`). Ambas desaparecem quando o campo está **focado,
    /// inválido ou desabilitado** (`has-[:disabled,:focus-visible,[aria-invalid]]:shadow-none`), e
    /// o foco põe no lugar delas um halo de 3px.
    ///
    /// O halo é um `BoxShadow` de blur 0 e spread 3 — é assim que se desenha um `ring` do Tailwind
    /// sem ter `ring` no GPUI.
    fn shadow_stack(&self, ring_k: f32, rest_k: f32) -> Vec<gpui::BoxShadow> {
        shadow_stack_for(
            self.disabled,
            matches!(self.validity, Validity::Error(_)),
            ring_k,
            rest_k,
        )
    }

    /// O **fio de bisel** — ver [`bevel_for`] pra regra de quando ele aparece.
    fn bevel(&self, focused: bool) -> Option<Div> {
        if self.unstyled {
            return None;
        }
        bevel_for(
            self.disabled,
            focused,
            matches!(self.validity, Validity::Error(_)),
            self.join,
        )
    }
}

/// Pilha de sombras da moldura, com o alfa do halo e o da sombra de repouso escalados
/// independentemente — é o que permite o cross-fade do `transition-shadow`.
pub(crate) fn shadow_stack_for(disabled: bool, invalid: bool, ring_k: f32, rest_k: f32) -> Vec<gpui::BoxShadow> {
    let f = field();
    if disabled {
        return Vec::new();
    }

    {
        let mut out = Vec::with_capacity(1);
        let _ = ring_k;
        // No coss a sombra de repouso desaparece quando o campo está inválido
        // (`has-aria-invalid:shadow-none`), mesmo sem foco.
        if rest_k > 0.0 && !invalid {
            // `shadow-xs/5` = 0 1px 2px rgba(0,0,0,.05)
            out.push(gpui::BoxShadow {
                color: f.shadow.scaled(rest_k),
                offset: gpui::point(px(0.0), px(1.0)),
                blur_radius: px(2.0),
                spread_radius: px(0.0),
            });
        }
        out
    }
}

/// Quando o anel de foco está na árvore.
///
/// Aparece ao focar — e CONTINUA na árvore durante o desvanecer ao perder o foco, senão o
/// `transition-shadow` não teria o que animar na saída. Nunca aparece desabilitado.
///
/// # O halo
///
/// É o `has-focus-visible:ring-[3px]` do coss.
///
/// ⚠️ Não é uma sombra. Era, e estava errado por dois motivos, os dois vindos do
/// `Window::paint_shadows` do GPUI (a mesma armadilha do bisel, ver [`bevel_for`]):
///
/// 1. A sombra é pintada como um retângulo arredondado CHEIO atrás do elemento. Como o fundo do
///    campo é translúcido no tema escuro (`bg-input/32` ≈ 2,5% de branco), o halo inteiro
///    atravessava e **tingia o fundo do campo** ao focar — algo que não existe no original.
/// 2. O `spread_radius` do GPUI dilata os limites mas **mantém o raio**, então a curvatura do halo
///    saía errada nas quinas.
///
/// Um `ring` do Tailwind é geometricamente um anel: a forma do elemento dilatada em 3px, com o raio
/// externo crescendo junto. Então é isso que se desenha — um overlay 3px maior em cada lado, com
/// borda de 3px e raio `10 + 3`. Fica **fora** da moldura (irmão, não filho), porque a moldura tem
/// Se o **esmaecimento de desabilitado** se aplica — a sexta peça da superfície.
///
/// Função (e não um `&&` solto no render) pelo mesmo motivo do [`ring_visible`]: é uma DECISÃO, e
/// decisão desta base se testa sem GPU. Aqui isso não é preciosismo — a regra nasceu de um bug de
/// integração (esmaecimento em dobro, 0,64² ≈ 0,41, quando o hospedeiro também esmaece) e a primeira
/// versão da correção passou num `when` inline, onde nenhuma mutação a matava.
pub(crate) fn dim_visible(disabled: bool, unstyled: bool) -> bool {
    disabled && !unstyled
}

/// `overflow_hidden` e recortaria o anel.
pub(crate) fn ring_visible(disabled: bool, focused: bool, fading_out: bool) -> bool {
    !disabled && (focused || fading_out)
}

/// O overlay do anel em si — ver [`ring_visible`] pra regra de quando ele entra na árvore.
pub(crate) fn ring_overlay(invalid: bool, join: crate::group::Join) -> Div {
    let f = field();
    let cor = if invalid { f.danger_glow } else { f.ring_glow };
    // Num grupo o anel PARA na emenda: sem `z-index` no GPUI ele não apareceria por cima do vizinho,
    // e quem fecha o contorno naquele lado é o separador, que vira a cor do anel. Ver o doc de
    // `crate::group`.
    join.ring_overlay(-FOCUS_RING_WIDTH, FIELD_RADIUS + FOCUS_RING_WIDTH)
        .border_3()
        .border_color(cor.hsla())
}

/// O **fio de bisel** de 1px sobreposto ao campo.
///
/// ⚠️ **A técnica do coss não traduz pro GPUI.** Lá é um pseudo-elemento transparente cobrindo a
/// padding box, com `box-shadow: 0 ±1px <cor>`: só o filete que ESCAPA fica visível, porque o CSS
/// nunca pinta a sombra embaixo da border-box de quem a projeta. O `Window::paint_shadows` do GPUI
/// insere a sombra como um retângulo arredondado **completo**, sem recortar a área do próprio
/// elemento — num overlay transparente isso vira uma lavagem da cor sobre o campo INTEIRO (4% de
/// preto no claro, 6% de branco no escuro). Era o que esta função fazia, e ficava visivelmente feio.
///
/// Então o filete é desenhado como o que ele é: uma **borda de 1px num único lado** do overlay.
///
/// Documentação histórica do que se tentou antes — a geometria pretendida é a do
    /// `before:absolute inset-0 rounded-[calc(var(--radius-lg)-1px)]` do coss.
    ///
    /// Antes isto era um 2º `BoxShadow` na própria moldura, o que dava um fio de raio 10px em vez
    /// de 9px e nascendo da borda externa em vez da interna. Como filho absoluto recuado em 1px e
    /// com raio 9px, o traço nasce onde nasce no original.
    ///
    /// Some quando o campo está focado, inválido ou desabilitado — é o
    /// `not-has-disabled:not-has-focus-visible:not-has-aria-invalid:before:shadow-*`.
pub(crate) fn bevel_for(
    disabled: bool,
    focused: bool,
    invalid: bool,
    join: crate::group::Join,
) -> Option<Div> {
    let f = field();
    if disabled || focused || invalid {
        return None;
    }
    // O overlay cobre a BORDER box da moldura e é IRMÃO dela, não filho.
    //
    // Duas coisas descobertas medindo com o filete pintado de magenta:
    //
    // 1. **O efeito do coss cai SOBRE a borda.** Lá o pseudo-elemento fica na padding box e a
    //    sombra sai 1px pra fora, então ela clareia a própria borda. Um filete 1px pra DENTRO
    //    (o que esta função fazia) cria uma segunda linha ao lado da borda — e a 6% de alfa isso
    //    é praticamente invisível, que foi o sintoma relatado.
    // 2. **Como filho, ele é recortado.** A moldura tem `overflow_hidden`, então um overlay que
    //    passe da padding box some — sobrava só um arco em cada quina. Por isso ele é irmão: o
    //    container relativo do campo não recorta.
    //
    // Sendo irmão em `inset: 0`, o overlay coincide com a border box da moldura, e a borda de 1px
    // dele cai exatamente sobre a borda dela.
    //
    // Num grupo, o lado da emenda avança meio pixel pra dentro do vizinho, senão o filete dele e o
    // do vizinho podem não se encontrar (ver `crate::group::Join::bevel_overlay`).
    let overlay = join
        .bevel_overlay(0.0, FIELD_RADIUS)
        .border_color(f.bevel.hsla());
    // Claro: filete escuro embaixo. Escuro: filete claro em cima.
    Some(if f.bevel_dir > 0.0 {
        overlay.border_b_1()
    } else {
        overlay.border_t_1()
    })
}

// =================================================================================================
// A alça de redimensionar da textarea
// =================================================================================================
//
// **Superset consciente**, e o único deste módulo que acrescenta uma peça interativa em vez de um
// número: no DOM a alça é do NAVEGADOR (o `<textarea>` nasce `resize: both` e o coss só a desliga
// dentro do `input-group`), e no GPUI não existe nada equivalente. Ou ela é ausente, ou ela é nossa.
// É nossa — com o eixo, os limites, a precedência e o visual escritos aqui. O porquê de cada decisão
// está na seção "Superset consciente" do doc do módulo; o que está aqui é a mecânica.

/// **Teto do arraste, em linhas.**
///
/// Um teto é obrigatório pelo MESMO motivo que [`growing`] exige um: sem ele, um arraste longo
/// empurra o formulário inteiro pra fora da janela — e a alça, que vive no canto de BAIXO, sai da
/// tela junto, deixando o usuário sem como voltar. O número é em linhas porque toda a geometria
/// vertical desta textarea é em linhas ([`TEXTAREA_ROWS`], o `max_rows` do [`growing`]): um teto em px
/// seria a única medida vertical deste componente que não se lê em linhas.
///
/// 20 linhas = 400px de conteúdo — o dobro do teto que o exemplo do [`growing`] usa (10) e mais da
/// metade da altura útil de uma janela de 720px. Passado isso, o que se quer é a rolagem interna que
/// a caixa já tem, não uma caixa mais alta.
const ARRASTE_MAX_LINHAS: usize = 20;

/// **Fração da tinta de ícone do tema** com que a alça é pintada em repouso; no hover ela vai a
/// cheio.
///
/// Mais apagada que a do ícone-alça do [`crate::scrub_input`] (que é metade) por uma diferença real
/// de contexto: lá a alça mora num slot de prefixo que é só dela, aqui ela mora POR CIMA da área de
/// texto e pode cair sobre uma linha escrita. Uma afordância secundária que atravessa o conteúdo
/// precisa ser mais discreta que uma que tem lugar próprio.
const ALCA_TINTA_REPOUSO: f32 = 0.4;

/// **Folga da alça sobre o lado de um ícone de campo** — o que transforma o desenho num alvo de
/// arraste.
///
/// É o mesmo `+6` de [`Input::icon_button`], que é o alvo de qualquer botãozinho dentro de um campo:
/// um alvo do tamanho exato do desenho obriga o usuário a acertar o traço.
///
/// ⚠️ Diferente do `icon_button`, aqui a folga **entra no glifo** em vez de virar respiro em volta
/// dele: o lado da alça é `icon() + ALCA_FOLGA` e o SVG é desenhado nesse tamanho inteiro. Dois
/// motivos, os dois medidos:
///
/// 1. o `resize_corner.svg` já carrega ~18% de margem transparente dentro do próprio `viewBox` (o
///    vértice da quina fica em 19,6 de 24), e é ela que mantém o traço fora da borda — uma segunda
///    folga por layout empurraria a quina pra longe do canto, que é justamente onde o usuário procura
///    por ela. ⚠️ A tinta ocupa só o quadrante de baixo à direita do `viewBox` (de 11,4 a 19,6): o
///    glifo é PEQUENO de propósito, e o resto da caixa é margem transparente. Encolher o glifo pelo
///    `viewBox` em vez de pelo lado do elemento é o que permite a alça ser discreta **sem** encolher
///    o alvo de arraste — ver o motivo 2;
/// 2. como o `gpui::svg()` só reage ao hover dentro das PRÓPRIAS bounds (ver
///    [`alca_de_redimensionar`]), um glifo menor que a caixa deixaria uma coroa de área que arrasta mas
///    não acende. Glifo e alvo no mesmo número é a mesma decisão do diamante de keyframe do
///    [`crate::scrub_input`], e pelo mesmo motivo.
const ALCA_FOLGA: f32 = 6.0;

/// Caminho do glifo da alça na [`crate::assets::Assets`].
///
/// É uma constante, e não a string escrita dentro de [`alca_de_redimensionar`], **pra o teste poder
/// apontar pra ela**: com o caminho literal no render, um teste que verificasse
/// `"icons/resize_corner.svg"` na tabela de assets estaria verificando a si mesmo — o render poderia
/// passar a pedir outro arquivo e nada quebraria. Ver `o_glifo_da_alca_esta_no_bundle`.
///
/// ⚠️ **O traço deste arquivo é `1.14`, e não o `1.5` do resto dos glifos finos da casa.** A regra da
/// casa não é o número, é o RESULTADO: 1,0px de tinta. O `1.5` produz isso num `viewBox` de 24
/// desenhado a 16px, e a alça é desenhada a `icon() + ALCA_FOLGA` — 19 · 21 · 23. Com `1.5` o traço
/// sairia 1,31px no tamanho padrão, mais grosso que os ícones do próprio campo ao lado. `24/21`
/// devolve 1,0px exato no `Md` (0,90 no `Sm`, 1,09 no `Lg`).
const ALCA_GLIFO: &str = "icons/resize_corner.svg";

/// **Teto do arraste**, em altura EXTERNA de px, no tamanho dado — [`ARRASTE_MAX_LINHAS`] linhas mais
/// o respiro vertical do tamanho e as duas bordas de 1px.
///
/// A conta é a MESMA de [`TextareaFloor::min_height`] com outro número de linhas, e não uma segunda
/// fórmula: piso e teto do mesmo eixo têm que se medir do mesmo jeito, senão um dia um deles conta as
/// bordas e o outro não.
fn teto_do_arraste(size: InputSize) -> f32 {
    ARRASTE_MAX_LINHAS as f32 * TEXTAREA_LINE_HEIGHT + 2.0 * size.textarea_pad_y() + 2.0
}

/// **A matemática do arraste**: altura EXTERNA nova, a partir da altura no início do gesto e do
/// deslocamento vertical do mouse desde a âncora.
///
/// `dy > 0` (mouse pra baixo) cresce a caixa — a alça está na borda de BAIXO, e a caixa segue o dedo.
///
/// O delta é medido contra a **âncora** do gesto e nunca acumulado move a move: é a mesma lição do
/// [`crate::scrub_input`] — acumulando, cada arredondamento vira erro permanente e voltar o mouse ao
/// ponto de partida não devolve a altura de partida.
///
/// É função pura, e não um `clamp` dentro do handler de arraste, porque uma decisão embutida num
/// handler de render é inalcançável por teste.
///
/// O `piso.max(teto)` existe porque `f32::clamp` **entra em pânico** com `min > max`: se um dia o teto
/// de um tamanho ficar abaixo do piso dele, o campo para no piso em vez de derrubar a app.
fn altura_apos_arraste(altura_inicial: f32, dy: f32, piso: f32, teto: f32) -> f32 {
    (altura_inicial + dy).clamp(piso, piso.max(teto))
}

/// **A precedência da altura de uma textarea**: `Some(altura externa fixa)` quando alguém a fixou,
/// `None` quando a caixa fica em `min_h` (o piso mais o crescimento do núcleo).
///
/// A ordem é a decisão, e ela é:
///
/// 1. o que o usuário **arrastou** — intenção explícita e direta;
/// 2. o que o call site declarou em [`Input::height`] (que guarda o MIOLO, daí o `+ 2.0`) — uma
///    decisão de código;
/// 3. nada, e aí o piso manda.
///
/// O arraste vencer o [`growing`] sai daqui de graça: um `Some` vira `h` na moldura, e o miolo em
/// `auto_grow` só pede altura MÍNIMA — então a caixa para de acompanhar o texto no instante em que o
/// gesto começa. É o mesmo mecanismo que [`Input::height`] já usava.
///
/// O arraste vencer o `height()` é decisão, não consequência: um `height()` é o default que o código
/// escolheu, e quem quer uma caixa multi-linha realmente imutável pede [`Input::resize_none`] — senão a
/// alça ficaria na tela sem fazer nada.
///
/// É função, e não um `or_else` no meio do render, porque é ela que o teste
/// `o_arraste_vence_o_height_do_call_site` guarda: um `or_else` inline seria copiado pro teste, e o
/// teste concordaria consigo mesmo com a ordem invertida no render.
fn altura_fixada_da_textarea(arrastada: Option<f32>, content_height: Option<f32>) -> Option<f32> {
    arrastada.or_else(|| content_height.map(|miolo| miolo + 2.0))
}

/// Se a **alça** entra na árvore.
///
/// Função (e não um `&&` no meio do render) pelo mesmo motivo de [`ring_visible`] e [`dim_visible`]:
/// é uma DECISÃO, e decisão desta base se testa sem GPU.
///
/// - `multi_linha`: um campo de uma linha não tem altura pra arrastar.
/// - `unstyled`: a alça é peça da MOLDURA, e sem moldura ela sai com o resto. É exatamente onde o coss
///   põe o `resize-none` — dentro do `input-group`, que é o único lugar que hospeda o campo sem
///   moldura. O `resize-none` da referência sai, portanto, de graça.
/// - `disabled`: um campo que não aceita edição também não aceita redimensionamento.
/// - `resize_none`: o desligamento explícito ([`Input::resize_none`]).
fn alca_visivel(multi_linha: bool, unstyled: bool, disabled: bool, resize_none: bool) -> bool {
    multi_linha && !unstyled && !disabled && !resize_none
}

/// Payload de arraste da alça.
///
/// ⚠️ **Tipo PRÓPRIO, e é o ponto todo dele.** O GPUI casa carga de arraste por **tipo**
/// (`on_drag_move::<T>` só dispara quando o arraste ativo é um `T`), então reaproveitar o payload do
/// [`crate::scrub_input`] faria o arraste de um `ScrubInput` redimensionar toda textarea na tela e
/// vice-versa. O campo `dono` faz a segunda separação, entre duas textareas irmãs.
#[derive(Clone)]
struct ArrasteDeAlca {
    /// Id do [`InputState`] cuja alça iniciou o arraste.
    dono: EntityId,
}

impl Render for ArrasteDeAlca {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Arraste "fantasma": quem se move é a borda da caixa, não um chrome atrás do cursor. Mesma
        // decisão do `ScrubDrag` do `scrub_input`.
        Empty
    }
}

/// O que uma textarea redimensionável guarda **entre quadros**.
#[derive(Clone, Copy, Debug, Default)]
struct EstadoDaAlca {
    /// Altura EXTERNA que o usuário fixou arrastando, em px. `None` = ninguém arrastou este campo
    /// ainda, e a altura é a que o piso e o [`growing`] decidirem.
    altura: Option<f32>,
    /// `y` do mouse no `mouse_down` na alça. É a âncora do gesto (ver [`altura_apos_arraste`]).
    ancora_y: Option<f32>,
    /// Altura EXTERNA no instante em que o gesto começou.
    ///
    /// Só é conhecida no **primeiro** `DragMoveEvent`, e não no `mouse_down`: com [`growing`] a caixa
    /// pode estar bem acima do piso, e a altura real de um quadro pintado não está em lugar nenhum do
    /// `render` — ela está no `bounds` que o evento de arraste carrega. Ancorar no piso daria um salto
    /// visível de 132px pra 72px no primeiro pixel de arraste.
    altura_no_inicio: Option<f32>,
}

/// Altura arrastada de cada textarea da app, por [`EntityId`] do [`InputState`] — uma **global do
/// [`App`]**.
///
/// # Por que a altura mora AQUI
///
/// Ela precisa sobreviver entre quadros, e nenhum dos lugares óbvios serve:
///
/// - **no [`Input`]**, não: ele é `RenderOnce`, remontado a cada frame. Um campo nele seria
///   reinicializado toda vez que o mouse se movesse — é a mesma razão pela qual
///   [`FOCUS_TRANSITIONS`] existe neste arquivo.
/// - **no [`InputState`]**, não: ele é do `gpui-component` vendorizado, e a altura EXTERNA da moldura é
///   um conceito que o núcleo não tem (a moldura é nossa, ver `Input::new`). Patchar o fork pra
///   guardar uma métrica de apresentação é dívida no lugar errado.
/// - **numa entidade NOSSA passada pelo call site**, não: `Input::new(&state)` recebe **uma** entidade,
///   e exigir uma segunda mudaria a assinatura de todo call site de textarea — o mesmo "configurar em
///   dois lugares" que os construtores de estado deste módulo existem pra evitar (ver [`multi_line`]).
///   O piso e o modo de crescimento já são duas configurações que precisam concordar; uma terceira
///   seria uma terceira chance de discordar.
///
/// Sobra o estado de app, chaveado pelo id do estado — a mesma chave de [`FOCUS_TRANSITIONS`].
///
/// # Global do `App`, e não um `thread_local`
///
/// [`FOCUS_TRANSITIONS`] é um `thread_local`, e a tentação era copiar aquilo. Mas `thread_local` é por
/// **thread**, e a altura é por **app**: um processo pode hospedar mais de um [`App`] na mesma thread —
/// é literalmente o que o harness de teste do GPUI faz — e ali os [`EntityId`] recomeçam do zero, então
/// duas apps compartilhariam altura pelo id repetido. Uma global do `App` tem exatamente o escopo do
/// dado. (Lá o mesmo defeito é inócuo: o que vaza é "este campo estava focado no frame anterior", que
/// o frame seguinte corrige sozinho.)
///
/// # Sem teto de tamanho, de propósito
///
/// [`FOCUS_TRANSITIONS`] limpa a tabela inteira quando passa de [`FOCUS_TABLE_CAP`], e ali isso degrada
/// de leve (a próxima transição sai instantânea). Aqui limpar **desfaria o arraste do usuário**, que é
/// um bug visível. E uma entrada só existe depois de um arraste MANUAL: a tabela tem tantas entradas
/// quantas textareas a pessoa redimensionou à mão na sessão — um número que se conta nos dedos.
#[derive(Default)]
struct AlturasDeAlca(std::collections::HashMap<EntityId, EstadoDaAlca>);

impl gpui::Global for AlturasDeAlca {}

/// Ancora um gesto de arraste: guarda o `y` do `mouse_down` e esquece a altura inicial do gesto
/// anterior (ela é relida do primeiro `DragMoveEvent` — ver [`EstadoDaAlca::altura_no_inicio`]).
fn ancorar_alca(cx: &mut App, id: EntityId, y: f32) {
    let entrada = cx.default_global::<AlturasDeAlca>().0.entry(id).or_default();
    entrada.ancora_y = Some(y);
    entrada.altura_no_inicio = None;
}

/// Aplica um passo do arraste. Devolve `true` quando a altura mudou — e portanto quando vale pedir um
/// quadro novo.
///
/// `altura_pintada` é a altura EXTERNA que a moldura tinha no quadro em que o gesto começou, lida do
/// `bounds` do evento; ela só é consumida no primeiro passo.
///
/// Um arraste que **não** começou na alça deste campo não tem âncora aqui, e sai sem escrever nada:
/// é o que impede um `mouse_down` perdido de virar redimensionamento.
fn arrastar_alca(
    cx: &mut App,
    id: EntityId,
    y: f32,
    altura_pintada: f32,
    piso: f32,
    teto: f32,
) -> bool {
    let Some(entrada) = cx.default_global::<AlturasDeAlca>().0.get_mut(&id) else {
        return false;
    };
    let Some(ancora_y) = entrada.ancora_y else {
        return false;
    };
    let inicial = *entrada.altura_no_inicio.get_or_insert(altura_pintada);
    let nova = altura_apos_arraste(inicial, y - ancora_y, piso, teto);
    if entrada.altura == Some(nova) {
        return false;
    }
    entrada.altura = Some(nova);
    true
}

/// A altura EXTERNA que o usuário fixou arrastando, se fixou.
///
/// `try_global` (e não `default_global`) porque ler no render não deve criar a tabela: numa app sem
/// nenhuma textarea arrastada ela nunca chega a existir.
fn altura_da_alca(cx: &App, id: EntityId) -> Option<f32> {
    cx.try_global::<AlturasDeAlca>()?.0.get(&id)?.altura
}

/// A **alça**: um alvo de arraste no canto inferior direito, DENTRO da moldura.
///
/// # O desenho
///
/// O glifo é `icons/resize_corner.svg`: a própria quina, dois traços em ângulo reto que repetem o canto
/// da moldura onde ela mora. Diz "esta borda se move" em vez do "pegue aqui" genérico de um punho de
/// pontos, que é o mesmo desenho usado por listas reordenáveis e não distingue arrastar de reordenar.
/// ⚠️ Um caminho errado aqui **não dá
/// erro**: a `AssetSource` devolve `None`, o `svg` não desenha nada e ninguém é avisado. É o que o
/// teste `o_glifo_da_alca_esta_no_bundle` cobre.
///
/// O lado sai do [`InputSize::icon`] do campo — o mesmo de qualquer ícone dentro desta moldura, e não
/// um sétimo número de ícone — mais [`ALCA_FOLGA`], que é o que o torna um ALVO. Glifo e alvo têm o
/// mesmo lado, e o recuo da tinta até a quina vem de dentro do próprio SVG; o porquê está no doc de
/// [`ALCA_FOLGA`].
///
/// # O arraste
///
/// Mesmo par `on_drag` + `DragMoveEvent` do [`crate::scrub_input`] e das divisórias de painel: a alça
/// é o `drag source`, e quem ouve o movimento é a moldura (ver `RenderOnce for Input`). A âncora é
/// capturada no `mouse_down`, ANTES do primeiro move — o GPUI só declara o arraste depois de 2px de
/// deslocamento, e ancorar no primeiro move perderia esses 2px.
///
/// O `mouse_down` para a propagação: sem isso, apertar a alça também posicionaria o cursor de texto
/// embaixo dela.
///
/// Os LIMITES não estão aqui de propósito: quem clampa é o ouvinte do movimento, na moldura, porque é
/// lá que a altura pintada é conhecida. A alça só sabe abrir o gesto.
fn alca_de_redimensionar(id: EntityId, size: InputSize) -> impl IntoElement {
    // Glifo e alvo no MESMO número — ver [`ALCA_FOLGA`] pelos dois motivos.
    let lado = size.icon() + ALCA_FOLGA;
    let tinta = opaque(theme::ICON());
    div()
        .id(("textarea-resize", id))
        // A alça é o alvo de cima: sem `occlude`, o miolo de texto embaixo dela também responderia.
        .occlude()
        .absolute()
        .bottom_0()
        .right_0()
        .flex()
        .size(px(lado))
        // Só o eixo vertical — o cursor diz isso antes do primeiro arraste.
        .cursor(CursorStyle::ResizeUpDown)
        .child(
            // O `gpui::svg()` pinta com a `text_color` do PRÓPRIO elemento `svg` e NÃO herda a do
            // pai (se for `None`, não desenha nada e não loga erro) — por isso a cor e o realce de
            // hover vão direto nele, com `.flex_none()` + `.size()` pra não colapsar pra 0. Mesma
            // armadilha documentada no `scrub_input`.
            gpui::svg()
                .path(ALCA_GLIFO)
                .size(px(lado))
                .flex_none()
                .text_color(tinta.scaled(ALCA_TINTA_REPOUSO))
                .hover(|s| s.text_color(tinta.hsla())),
        )
        // A âncora do gesto, capturada antes do 1º move.
        .on_mouse_down(
            MouseButton::Left,
            move |e: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
                ancorar_alca(cx, id, f32::from(e.position.y));
            },
        )
        .on_drag(ArrasteDeAlca { dono: id }, move |arraste, _pos, _window, cx| {
            cx.stop_propagation();
            cx.new(|_| arraste.clone())
        })
}

impl Input {
    /// Um segmento de addon (bloco com fundo próprio, colado na moldura).
    fn addon(&self, text: &SharedString, before: bool) -> Div {
        let s = self.size;
        div()
            .flex()
            .items_center()
            .flex_none()
            .h_full()
            .px(px(self.pad_x.unwrap_or_else(|| s.pad_x())))
            .bg(rgb(theme::BG_INSET()))
            .text_size(px(self.text_size.unwrap_or_else(|| s.text_size())))
            .text_color(rgb(theme::TEXT_MUTED()))
            .map(|d| {
                // A divisória fica do lado que encosta no campo.
                if before {
                    d.border_r_1().border_color(theme::border_divider())
                } else {
                    d.border_l_1().border_color(theme::border_divider())
                }
            })
            .child(text.clone())
    }
}

/// Cor da borda por estado (livre, pra ser testável sem construir um `Input`).
///
/// `disabled` NÃO entra: no coss o campo desabilitado mantém as cores e o conjunto todo recebe
/// `opacity-64`. Ter um par "apagado" de cada token além disso só daria duas fontes de verdade.
pub(crate) fn border_color_for(invalid: bool, focused: bool) -> Rgba8 {
    let f = field();
    match (invalid, focused) {
        (true, true) => f.danger_border_focus,
        (true, false) => f.danger_border,
        (false, true) => f.ring,
        (false, false) => f.border,
    }
}

impl RenderOnce for Input {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let s = self.size;

        // Uma leitura só do estado: valor corrente (pro contador) + handle de foco (pro anel).
        // O `focus_handle` devolve um clone, então o empréstimo de `cx` morre no fim do bloco.
        let (value, focus_handle) = {
            let st = self.state.read(cx);
            (st.value(), st.focus_handle(cx))
        };
        let focused = focus_handle.is_focused(window);
        // Conta em CARACTERES (não bytes): `len()` daria 2 num "ç" e 4 num emoji, e o contador
        // mentiria pro usuário em qualquer texto acentuado — que é o caso comum aqui.
        let char_count = value.chars().count();

        // --- Linha da label -------------------------------------------------------------------
        let label_row = self.label.as_ref().map(|text| {
            div()
                .flex()
                .items_center()
                .gap(px(3.0))
                .child(
                    div()
                        .text_size(px(s.aux_text_size()))
                        .text_color(rgb(if self.disabled {
                            theme::TEXT_FAINT()
                        } else {
                            theme::TEXT_LABEL()
                        }))
                        .child(text.clone()),
                )
                .when(self.required, |d| {
                    d.child(
                        div()
                            .text_size(px(s.aux_text_size()))
                            .text_color(field().danger.hsla())
                            .child("*"),
                    )
                })
        });

        // --- Moldura + núcleo -----------------------------------------------------------------
        let core = CoreInput::new(&self.state)
            // A moldura é NOSSA: desligamos a aparência do núcleo pra não ter duas bordas e dois
            // fundos disputando (é o que já se fazia à mão em todo call site do editor).
            .appearance(false)
            .cleanable(self.clearable)
            .disabled(self.disabled)
            // O `mask_toggle` do núcleo NÃO é usado: ele é press-and-hold e traz o padding do
            // `Button` dele. O nosso é montado como sufixo, abaixo.
            //
            // O tamanho do núcleo tem que ACOMPANHAR o nosso (line-height e paddings internos dele
            // derivam daí). Sem isto ele fica no default `Medium` — altura fixa de 32px — e o texto
            // sai do centro no `Sm` e no `Lg`.
            // `bare_metrics` (fork) desliga altura/padding/corpo próprios do núcleo — a moldura é
            // nossa e passa a ser a única fonte de geometria. Sem isto o `input_h` dele vence: num
            // campo de uma linha o `h_full()` era ignorado em silêncio, e no `Lg` o miolo ficava
            // com 44px dentro de uma moldura de 36px.
            .bare_metrics()
            // A ENTRELINHA, que é onde as duas geometrias se separam:
            //
            // - campo de uma linha: `leading` = altura do miolo, que é como o coss centraliza o texto
            //   verticalmente (`leading-7.5` casa com `h-7.5`). Com a caixa de linha do tamanho exato
            //   do miolo, o glifo cai no centro sem precisar de padding.
            // - textarea: `leading` é o par do `text-sm` (20px). Aqui a entrelinha MULTIPLICA — ela é
            //   a altura de cada linha — então usar a altura do miolo daria linhas de 70px. Ver
            //   `TEXTAREA_LINE_HEIGHT`.
            .line_height(px(match self.textarea {
                Some(_) => s.textarea_line_height(),
                None => self.content_height.unwrap_or_else(|| s.content_height()),
            }))
            .pl(px(0.0))
            .pr(px(0.0))
            // O respiro vertical é da moldura (dela, ou do `content` no caso da textarea), então o do
            // núcleo é zero — dois respiros somariam.
            //
            // E o miolo PREENCHE a altura disponível: num campo de uma linha é assim que o
            // `items_center` interno dele centraliza o texto na altura EXATA da nossa moldura, em
            // qualquer tamanho. Sem isto a área de texto colapsa e o campo fica inclicável (o teste
            // `tres_cliques_em_textarea_pegam_so_a_linha` pegou justamente isso — nem o foco chegava).
            //
            // ⚠️ Numa textarea o `h_full` NÃO é o que dá altura ao miolo: a moldura ali é `min_h`, e
            // porcentagem de filho contra pai dimensionado por `min_h` não resolve. Quem pede a altura
            // é o próprio elemento do núcleo, em LINHAS — ver o doc de [`multi_line`], que é onde essa
            // armadilha está contada por inteiro.
            .py(px(0.0))
            .h_full()
            .text_size(px(self.text_size.unwrap_or_else(|| s.text_size())));
        // NOTA: o texto e o placeholder são pintados pelo elemento do núcleo com
        // `cx.theme().foreground` / `.muted_foreground` — cores do tema do `gpui-component`, que um
        // `.text_color()` daqui NÃO alcança. Elas são alinhadas com a nossa paleta por
        // `apply_core_text_colors`, chamada em `theme::sync_core_theme`.

        // `unstyled`: a moldura é de quem hospeda. Uma variável só governa as **SEIS** peças da
        // superfície (fundo+borda, sombra, bisel, anel, raio e o esmaecimento de desabilitado) pra não
        // sobrar uma pintada sozinha.
        //
        // A sexta entrou depois, num merge: ela tinha ficado fora e o resultado era esmaecimento em
        // dobro quando o hospedeiro também esmaece (0,64² ≈ 0,41). A correção passou por um gate
        // próprio (`self.disabled && !self.unstyled`) e isso deixou o comentário mentindo — agora ela
        // passa por AQUI, que é o ponto do "uma peça, um lugar".
        //
        // A **SÉTIMA** é a alça de redimensionar, e ela é a única que NÃO passa por esta variável: ela
        // tem gate próprio ([`alca_visivel`]) porque depende de mais coisas que a moldura (ser
        // multi-linha, o `resize_none`), e um `bool` que decide seis peças por um motivo e uma sétima
        // por quatro deixaria de ser legível. O que ela compartilha com as seis está travado no teste
        // `a_alca_sai_com_a_moldura`: sem moldura, sem alça.
        let superficie = !self.unstyled;
        let fp = field();
        let mut frame = div()
            .flex()
            .items_center()
            // `relative` porque o fio de bisel é um filho ABSOLUTO recuado em 1px.
            .relative()
            .overflow_hidden();
        if superficie {
            frame = frame
                // O fundo do campo no escuro é translúcido (`bg-input/32`), então usa `rgba`.
                .bg(fp.bg.hsla())
                .border_color(self.border_color(focused).hsla());
            // Raio e bordas vêm da COSTURA: solto são os quatro cantos e os quatro lados; num grupo, o
            // lado da emenda perde os dois (ver `crate::group`).
            frame = self.join.rounded(frame, self.radius.unwrap_or_else(|| s.radius()));
            frame = self.join.borders(frame);
        }

        // As alturas do [`InputSize`] são EXTERNAS: contam o par de bordas de 1px. Sem moldura essas
        // bordas não existem, e o campo mede o miolo — é o que a referência faz no `unstyled`, onde o
        // par de bordas é do hospedeiro (ver [`Input::unstyled`]).
        let sem_bordas = if superficie { 0.0 } else { 2.0 };

        // Altura. São dois modelos diferentes, de propósito:
        //
        // - campo de uma linha: altura FIXA. A moldura é a fonte dela (o miolo a preenche com
        //   `h_full`), então é aqui que o override de [`Input::height`] entra — no miolo ele só
        //   ajustaria a entrelinha.
        // - textarea: `min_h`, nunca `h` — é o `field-sizing-content` da referência. O piso é daqui; o
        //   crescimento acima dele vem do elemento do núcleo em modo `auto_grow` (ver [`growing`]).
        //   Com um estado [`multi_line`] o núcleo não cresce e a caixa fica parada no piso.
        frame = match self.textarea {
            Some(piso) => {
                // Numa textarea o texto começa no TOPO — sem isto o `items_center` da moldura
                // centraria o bloco de linhas e o respiro de cima deixaria de ser o do coss.
                let frame = frame.items_start();
                // Quem decide a altura é `altura_fixada_da_textarea` — a precedência inteira, e o
                // porquê dela, estão no doc daquela função.
                match altura_fixada_da_textarea(
                    altura_da_alca(cx, self.state.entity_id()),
                    self.content_height,
                ) {
                    Some(externa) => frame.h(px(externa - sem_bordas)),
                    None => frame.min_h(px(piso.min_height(s) - sem_bordas)),
                }
            }
            None => frame.h(px(self
                .content_height
                .map_or_else(|| s.height(), |miolo| miolo + 2.0)
                - sem_bordas)),
        };
        frame = match self.width {
            Width::Full => frame.w_full(),
            Width::Fixed(w) => frame.w(w).flex_none(),
        };

        // Os addons são construídos ANTES de consumir `prefix`/`suffix`: `self.addon(..)` empresta
        // `self`, e mover os campos de elemento primeiro tornaria esse empréstimo inválido.
        let addon_before = self.addon_before.as_ref().map(|a| self.addon(a, true));
        let addon_after = self.addon_after.as_ref().map(|a| self.addon(a, false));
        // Idem pro bisel: montado ANTES de `prefix`/`suffix` serem movidos. Sem superfície não há
        // bisel — o fio pertence à borda que agora é de quem hospeda.
        let bevel = superficie.then(|| self.bevel(focused)).flatten();

        // --- `transition-shadow`: cross-fade entre halo de foco e sombra de repouso ------------
        //
        // O GPUI não tem transição declarativa de estilo, mas tem animação por frame. Então o
        // cross-fade é feito à mão: a pilha de sombras é reconstruída a cada frame, com o alfa do
        // halo e o da sombra de repouso escalados em sentidos opostos.
        //
        // Calculado AQUI (e não junto da moldura) porque `shadow_stack` empresta `self`, e mais
        // abaixo `prefix`/`suffix` já foram movidos pra dentro do conteúdo.
        let transicao = focus_transition(self.state.entity_id(), focused);
        // A sombra de repouso: presente quando o campo NÃO está focado. (Ela não faz cross-fade —
        // são 5% de preto, imperceptível em 150ms; o que se anima é o anel.)
        let stack_estavel = if superficie {
            self.shadow_stack(0.0, if focused { 0.0 } else { 1.0 })
        } else {
            Vec::new()
        };
        // O anel aparece quando focado — e também durante o desvanecer ao PERDER o foco. Sem
        // superfície não há anel: quem hospeda desenha o dele em volta do controle inteiro.
        let mostra_anel =
            superficie && ring_visible(self.disabled, focused, transicao == Some(false));

        if let Some(a) = addon_before {
            frame = frame.child(a);
        }

        // Conteúdo (prefixo · núcleo · sufixo) num container com o respiro horizontal. Fica
        // separado dos addons de propósito: os addons encostam na borda, o conteúdo respira.
        let mut content = div()
            .flex()
            .items_center()
            .gap(px(7.0))
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .px(px(self.pad_x.unwrap_or_else(|| s.pad_x())))
            // Textarea ganha respiro VERTICAL — o `py-[calc(--spacing(1.5)-1px)]` do coss. Num campo
            // de uma linha ele é zero, e quem centra o texto é a entrelinha.
            .when(self.textarea.is_some(), |d| {
                d.py(px(s.textarea_pad_y()))
            });

        // A seleção por múltiplos cliques (2 = palavra, 3+ = parágrafo, com a granularidade
        // preservada pro arraste e o shift+clique seguintes) é resolvida DENTRO do núcleo, pelo
        // fork em `vendor/gpui-component` — ver `PATCHES.md` de lá e
        // `crates/empire-ui/tests/selection_parity.rs`. Antes havia aqui um handler que despachava
        // ações no `defer`; saiu porque duplicava o trabalho e não conseguia arredondar o arraste.

        if let Some(p) = self.prefix {
            content = content.child(div().flex().items_center().flex_none().child(p));
        }
        // O wrapper do núcleo também estica na vertical — a altura tem que descer a cadeia toda
        // (frame → content → wrapper → núcleo), senão a textarea colapsa.
        content = content.child(div().flex_1().min_w(px(0.0)).h_full().child(core));
        if let Some(sfx) = self.suffix {
            content = content.child(div().flex().items_center().flex_none().child(sfx));
        }

        // --- Toggle de mostrar/esconder senha -------------------------------------------------
        //
        // Nosso, e não o `mask_toggle()` do núcleo, por dois motivos que se viam na tela: o dele é
        // press-and-hold (revela no mouse-down, esconde no mouse-up) em vez de alternar, e é um
        // `Button` com padding próprio, que abria um vão entre o ícone e a borda direita.
        //
        // Fica como último item do `content`, então respeita o mesmo respiro horizontal do prefixo
        // — alinhado com a borda, sem vão.
        //
        // O desenho é o de [`Self::icon_button`], que é a MESMA peça que qualquer outro botãozinho
        // dentro de um campo usa (ver o copiar do `color_picker`). Aqui só muda o ícone e o que o
        // clique faz.
        if self.mask_toggle {
            let state = self.state.clone();
            let masked = state.read(cx).is_masked();
            // Escondido → olho aberto ("clique pra revelar"); revelado → olho fechado
            // ("clique pra esconder").
            let icon = if masked {
                "iconoir/regular/eye.svg"
            } else {
                "iconoir/regular/eye-closed.svg"
            };
            content = content.child(Self::icon_button(
                ("mask-toggle", self.state.entity_id()),
                icon,
                s,
                self.disabled,
                move |_, window, cx| {
                    state.update(cx, |st, cx| st.set_masked(!masked, window, cx));
                },
            ));
        }

        frame = frame.child(content);

        if let Some(a) = addon_after {
            frame = frame.child(a);
        }

        // --- A alça de redimensionar ----------------------------------------------------------
        //
        // Filha ABSOLUTA da moldura, então ela não entra no fluxo e não muda a altura de nada — as
        // medidas de `tests/textarea_geometry.rs` continuam valendo com ela na tela.
        //
        // O ouvinte do movimento fica na MOLDURA, e não na alça: durante um arraste o mouse sai da
        // alça em duas linhas de deslocamento, e é o `bounds` da moldura que carrega a altura EXTERNA
        // pintada — o único lugar onde ela existe (ver `EstadoDaAlca::altura_no_inicio`). O
        // `on_drag_move` do GPUI dispara em qualquer posição do mouse enquanto o arraste do TIPO certo
        // estiver ativo, então tirar o mouse da moldura não interrompe o gesto.
        let mostra_alca = alca_visivel(
            self.textarea.is_some(),
            self.unstyled,
            self.disabled,
            self.resize_none,
        );
        if let Some(piso) = self.textarea.filter(|_| mostra_alca) {
            let id = self.state.entity_id();
            let piso_externo = piso.min_height(s);
            let teto = teto_do_arraste(s);
            frame = frame
                .child(alca_de_redimensionar(id, s))
                .on_drag_move(move |e: &DragMoveEvent<ArrasteDeAlca>, window, cx| {
                    // Arraste da alça de OUTRA textarea: o tipo casa, o dono não.
                    if e.drag(cx).dono != id {
                        return;
                    }
                    if arrastar_alca(
                        cx,
                        id,
                        f32::from(e.event.position.y),
                        f32::from(e.bounds.size.height),
                        piso_externo,
                        teto,
                    ) {
                        // O `Input` é `RenderOnce` e não tem entidade pra notificar; quem pede o
                        // quadro novo é a janela. Só quando a altura de fato mudou — dentro do
                        // clamp, arrastar mais não repinta.
                        window.refresh();
                    }
                });
        }

        let frame = if self.unstyled {
            frame
        } else {
            frame.shadow(stack_estavel)
        };

        // O anel é IRMÃO da moldura (não filho): a moldura tem `overflow_hidden`, que recortaria um
        // filho que se estende 3px pra fora. O container só existe pra ancorar o absoluto.
        let mut wrap = div()
            .relative()
            .map(|d| match self.width {
                Width::Full => d.w_full(),
                Width::Fixed(w) => d.w(w).flex_none(),
            })
            // Clicar fora tira o foco, como no navegador. Ver `blur_on_outside_click`.
            //
            // Só quando a moldura é NOSSA: sem ela, "fora do campo" deixa de querer dizer "fora do
            // controle" — num grupo o clique cai no respiro do wrapper ou num addon, que são parte do
            // MESMO controle, e desfocar ali seria um bug. Quem hospeda põe o handler na própria
            // superfície (ver [`Input::unstyled`]).
            .when(superficie, |d| {
                d.on_mouse_down_out(blur_on_outside_click(focus_handle.clone()))
            });
        wrap = wrap.child(frame);
        // O bisel entra DEPOIS da moldura (pinta em cima dela) e como irmão, pra não ser recortado
        // pelo `overflow_hidden` dela.
        if let Some(b) = bevel {
            wrap = wrap.child(b);
        }

        if mostra_anel {
            let anel = ring_overlay(matches!(self.validity, Validity::Error(_)), self.join);
            wrap = match transicao {
                // Estável: anel cheio, sem pedir frames ao compositor.
                None => wrap.child(anel),
                // `transition-shadow`: entra e sai desvanecendo. O id inclui o SENTIDO, pra entrar
                // e sair reiniciarem a animação em vez de uma continuar da outra.
                Some(entrando) => wrap.child(
                    anel.with_animation(
                        gpui::ElementId::NamedInteger(
                            if entrando { "ring-in".into() } else { "ring-out".into() },
                            self.state.entity_id().as_u64(),
                        ),
                        gpui::Animation::new(FOCUS_TRANSITION).with_easing(gpui::ease_in_out),
                        move |el, delta| {
                            el.opacity(if entrando { delta } else { 1.0 - delta })
                        },
                    ),
                ),
            };
        }
        let frame_el = wrap.into_any_element();

        // --- Linha de apoio: mensagem/hint à esquerda, contador à direita ---------------------
        let message = self.validity.message().cloned();
        let msg_color = self
            .validity
            .color()
            .unwrap_or_else(|| opaque(theme::TEXT_FAINT()));
        // O erro SUBSTITUI o hint (ver doc do módulo).
        let support_text = message.clone().or_else(|| self.hint.clone());

        let counter_text = self.counter.then(|| match self.max_len {
            Some(max) => format!("{char_count}/{max}"),
            None => format!("{char_count}"),
        });
        // Elevados a `0xRRGGBBAA` porque este módulo consome tudo via `rgba` (ver [`FieldPalette`]).
        let counter_color = opaque(match self.max_len {
            Some(max) if char_count > max => theme::DANGER(),
            // ≥90% do limite: avisa antes de estourar, não depois.
            Some(max) if max > 0 && char_count * 10 >= max * 9 => theme::WARNING(),
            _ => theme::TEXT_FAINT(),
        });

        let support_row = (support_text.is_some() || counter_text.is_some()).then(|| {
            div()
                .flex()
                .items_start()
                .justify_between()
                .gap(px(8.0))
                .w_full()
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .text_size(px(s.aux_text_size()))
                        .text_color(
                            if message.is_some() {
                                msg_color
                            } else {
                                opaque(theme::TEXT_FAINT())
                            }
                            .hsla(),
                        )
                        .children(support_text),
                )
                .children(counter_text.map(|t| {
                    div()
                        .flex_none()
                        .text_size(px(s.aux_text_size()))
                        .text_color(counter_color.hsla())
                        .child(t)
                }))
        });

        div()
            .flex()
            .flex_col()
            .gap(px(5.0))
            .map(|d| match self.width {
                Width::Full => d.w_full(),
                Width::Fixed(w) => d.w(w).flex_none(),
            })
            // `has-disabled:opacity-64` — o coss esmaece o conjunto inteiro em vez de trocar cor
            // por cor. Fica mais coerente e não exige um par "apagado" de cada token.
            // `has-disabled:opacity-64` — e **só quando a moldura é nossa** (é a sexta peça de
            // `superficie`; ver o comentário dela).
            //
            // ⚠️ Sem o gate, isto esmaece em DOBRO: quem hospeda o campo sem moldura (o
            // [`crate::input_group::InputGroup`]) esmaece a superfície dele pelo mesmo motivo, e as duas
            // multiplicam — 0,64² ≈ 0,41, um campo desabilitado quase ilegível. Na referência a classe
            // está na mesma lista que o `unstyled` apaga, então tirá-la daqui é o fiel além do correto.
            .when(dim_visible(self.disabled, self.unstyled), |d| d.opacity(0.64))
            .children(label_row)
            .child(frame_el)
            .children(support_row)
    }
}

#[cfg(test)]
// O clippy trata `assert!` sobre constante como erro provável, presumindo que quem escreveu quis
// testar algo variável. Aqui é o contrário: travar o valor que veio da referência É o propósito
// destes testes. Mesma decisão em todos os módulos portados do coss (ver `frame.rs`).
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    /// **O esmaecimento de desabilitado é a SEXTA peça da superfície — e sai com ela.**
    ///
    /// Sem isto, um campo sem moldura hospedado numa superfície que também esmaece (o
    /// [`crate::input_group::InputGroup`]) esmaece em DOBRO: 0,64 × 0,64 ≈ 0,41, quase ilegível. A
    /// referência põe a classe na mesma lista que o `unstyled` apaga, então o certo e o fiel coincidem.
    ///
    /// Este teste existe porque a correção do bug ficou **sem guarda**: na primeira versão ela era um
    /// `&&` inline no render, e reverter o gate não quebrava teste nenhum.
    #[test]
    fn o_esmaecimento_de_desabilitado_sai_com_a_moldura() {
        assert!(dim_visible(true, false), "com moldura própria, esmaece");
        assert!(!dim_visible(true, true), "sem moldura, quem esmaece é o hospedeiro");
        assert!(!dim_visible(false, false), "habilitado nunca esmaece");
        assert!(!dim_visible(false, true));
        // A composição que o bug produzia, em número — pra o próximo leitor não ter que refazer a conta.
        let dobro = 0.64_f32 * 0.64;
        assert!(dobro < 0.45, "0,64² = {dobro:.2}: é isto que o gate evita");
    }

    /// Precedência da borda, na ordem que os seletores `has-*` do coss impõem: inválido+foco é o
    /// estado mais específico, depois inválido, depois foco, depois repouso.
    ///
    /// `disabled` de propósito NÃO aparece: no coss ele não troca cor de borda — o conjunto todo
    /// recebe `opacity-64`. Se alguém reintroduzir um ramo de disabled aqui, este teste continua
    /// passando, mas o de opacidade abaixo documenta a intenção.
    /// **Desvio consciente do coss, travado aqui.**
    ///
    /// O original usa branco a 6% no bisel escuro. Nós usamos o DOBRO, porque a 6% o filete é
    /// imperceptível no nosso fundo. Este teste existe pra o desvio ser uma decisão registrada e
    /// não uma deriva: se alguém "corrigir" pra 6% achando que é erro de porte, ele falha e
    /// aponta pra cá.
    ///
    /// O tema CLARO segue fiel (preto 4%) — o desvio é só no escuro, onde o problema existia.
    #[test]
    fn bisel_escuro_e_o_dobro_da_referencia() {
        // Alfa de referência em 8 bits: 4% -> 10 (claro), 6% -> 15 (escuro).
        const REF_CLARO: f32 = 10.0 / 255.0;
        const REF_ESCURO: f32 = 15.0 / 255.0;

        let claro = FIELD_LIGHT.bevel.alpha();
        assert!(
            (claro - REF_CLARO).abs() < 1e-4,
            "o tema claro tem que seguir fiel à referência (preto 4%)"
        );

        let escuro = FIELD_DARK.bevel.alpha();
        assert!(
            (escuro - REF_ESCURO * 2.0).abs() < 1e-4,
            "o tema escuro é o DOBRO da referência; veio {escuro}"
        );
        assert!(escuro < 1.0, "ainda translúcido");
    }

    #[test]
    fn precedencia_da_cor_de_borda() {
        theme::set_theme(theme::ThemeMode::Dark);
        let f = field();

        assert_eq!(border_color_for(true, true), f.danger_border_focus);
        assert_eq!(border_color_for(true, false), f.danger_border);
        assert_eq!(border_color_for(false, true), f.ring);
        assert_eq!(border_color_for(false, false), f.border);

        // Inválido vence o foco: a validade é informação mais importante que "onde está o cursor".
        assert_ne!(border_color_for(true, true), f.ring);
    }

    /// As duas paletas do campo existem e diferem onde o coss manda diferir — em especial o fundo,
    /// que no escuro é translúcido (`bg-input/32`) e no claro é branco opaco.
    #[test]
    fn paletas_do_campo_diferem_entre_temas() {
        assert_ne!(FIELD_LIGHT.bg, FIELD_DARK.bg);
        assert_ne!(FIELD_LIGHT.text, FIELD_DARK.text);
        // O bisel troca de sentido: desce no claro, sobe no escuro.
        assert!(FIELD_LIGHT.bevel_dir > 0.0);
        assert!(FIELD_DARK.bevel_dir < 0.0);
    }

    /// `Validity` não pode expressar erro+sucesso ao mesmo tempo, e `.error()` depois de
    /// `.success()` simplesmente troca o estado (o último builder chamado ganha).
    #[test]
    fn validity_e_exclusiva() {
        // Chamar `.error()` depois de `.success()` TROCA o estado (o último builder ganha) —
        // não existe forma de ficar com os dois.
        let v = Validity::Error("nope".into());
        assert!(matches!(v, Validity::Error(_)));
        assert_eq!(v.message().map(|m| m.to_string()), Some("nope".to_string()));
        assert_eq!(Validity::Neutral.message(), None);
        assert_eq!(Validity::Neutral.color(), None);
    }

    /// **A convenção de cor da paleta do campo, decodificada de verdade.**
    ///
    /// Este é o teste que faltava. Todo valor de [`FieldPalette`] é `0xRRGGBBAA` e é consumido por
    /// `rgba`; um valor de 6 dígitos esquecido ali vira uma cor completamente diferente, sem erro de
    /// compilação. Foi o que aconteceu: `rgba(0xffffff)` é lido como `0x00FFFFFF` — ciano — e os
    /// campos ficaram ciano no tema claro.
    ///
    /// Em vez de comparar números com números (que não pegaria nada), decodifica e afirma o que a
    /// cor DEVE ser perceptualmente.
    #[test]
    fn paleta_do_campo_decodifica_pras_cores_pretendidas() {
        // Fundo do tema claro: branco OPACO.
        let bg: gpui::Rgba = FIELD_LIGHT.bg.hsla().into();
        assert_eq!((bg.r, bg.g, bg.b, bg.a), (1.0, 1.0, 1.0, 1.0), "claro: branco opaco");

        // Texto do tema claro: cinza MUITO escuro e opaco (#262626) — não um ciano.
        let txt: gpui::Rgba = FIELD_LIGHT.text.hsla().into();
        assert_eq!(txt.a, 1.0, "texto é opaco");
        assert!(txt.r < 0.2 && txt.g < 0.2 && txt.b < 0.2, "texto é quase preto");
        assert!(
            (txt.r - txt.g).abs() < 0.01 && (txt.g - txt.b).abs() < 0.01,
            "texto é NEUTRO: se r≠g≠b, o valor foi lido deslocado"
        );

        // Anel de foco: cinza neutro e opaco nos dois temas — o sintoma do bug era ele sair teal
        // (r=0), porque `0xa3a3a3` lido como rgba perde o canal vermelho.
        for (nome, ring) in [("claro", FIELD_LIGHT.ring), ("escuro", FIELD_DARK.ring)] {
            let c: gpui::Rgba = ring.hsla().into();
            assert_eq!(c.a, 1.0, "{nome}: anel opaco");
            assert!(c.r > 0.1, "{nome}: anel sem canal vermelho — leitura deslocada");
            assert!(
                (c.r - c.g).abs() < 0.01 && (c.g - c.b).abs() < 0.01,
                "{nome}: anel é cinza NEUTRO"
            );
        }

        // Erro: vermelho de verdade (r bem maior que g e b).
        for (nome, danger) in [("claro", FIELD_LIGHT.danger), ("escuro", FIELD_DARK.danger)] {
            let c: gpui::Rgba = danger.hsla().into();
            assert_eq!(c.a, 1.0, "{nome}: erro opaco");
            assert!(c.r > 0.7 && c.r > c.g + 0.3 && c.r > c.b + 0.3, "{nome}: erro é vermelho");
        }

        // Os tokens que DEVEM ser translúcidos continuam translúcidos.
        for (nome, c) in [
            ("borda claro", FIELD_LIGHT.border),
            ("borda escuro", FIELD_DARK.border),
            ("halo claro", FIELD_LIGHT.ring_glow),
            ("halo escuro", FIELD_DARK.ring_glow),
            ("bisel claro", FIELD_LIGHT.bevel),
            ("bisel escuro", FIELD_DARK.bevel),
            ("fundo escuro", FIELD_DARK.bg),
        ] {
            assert!(
                c.alpha() < 1.0,
                "{nome} tem que ser translúcido, veio com alfa {}",
                c.alpha()
            );
        }
    }

    /// O que o newtype [`Rgba8`] garante: as duas convenções não se cruzam. Um token de 6
    /// dígitos do tema **não compila** onde se espera cor do campo, e vice-versa — então a classe de
    /// bug que produziu campos ciano, anel teal e texto azul deixou de ser expressável.
    ///
    /// Isto não é verificável em runtime (é uma garantia de tipo); o teste existe pra registrar a
    /// intenção e falhar se alguém trocar o newtype por um `u32` cru:
    ///
    /// ```compile_fail
    /// let _ = gpui::rgb(empire_ui::input::FIELD_LIGHT.text);
    /// ```
    #[test]
    fn field_color_e_um_newtype_distinto_de_u32() {
        // Se `Rgba8` virar um alias de `u32`, esta comparação deixa de compilar por
        // ambiguidade ou o `assert_ne` abaixo perde sentido.
        assert_ne!(FIELD_LIGHT.text, FIELD_DARK.text);
        assert_eq!(Rgba8(0x262626ff), FIELD_LIGHT.text);
    }

    /// `opaque` eleva um token de 6 dígitos do tema à convenção do módulo, sem mexer no RGB.
    #[test]
    fn opaque_preserva_o_rgb_e_fixa_o_alfa() {
        let elevado: gpui::Rgba = opaque(0x2dd4bf).hsla().into();
        let original: gpui::Rgba = gpui::rgb(0x2dd4bf);
        assert_eq!(elevado.a, 1.0);
        assert!((elevado.r - original.r).abs() < 1e-6);
        assert!((elevado.g - original.g).abs() < 1e-6);
        assert!((elevado.b - original.b).abs() < 1e-6);
    }

    /// O bisel só aparece em REPOUSO — foco, inválido e desabilitado o apagam. É o
    /// `not-has-disabled:not-has-focus-visible:not-has-aria-invalid:before:shadow-*` do coss.
    #[test]
    fn bisel_so_aparece_em_repouso() {
        theme::set_theme(theme::ThemeMode::Dark);
        assert!(bevel_for(false, false, false, crate::group::Join::NONE).is_some(), "repouso: aparece");
        assert!(bevel_for(false, true, false, crate::group::Join::NONE).is_none(), "focado: some");
        assert!(bevel_for(false, false, true, crate::group::Join::NONE).is_none(), "inválido: some");
        assert!(bevel_for(true, false, false, crate::group::Join::NONE).is_none(), "desabilitado: some");
    }

    /// A pilha de sombras carrega **apenas** a sombra de repouso. O halo saiu daqui: como sombra
    /// ele vazava por cima do fundo translúcido do campo no tema escuro (ver [`ring_overlay`]).
    #[test]
    fn pilha_de_sombras_so_tem_a_sombra_de_repouso() {
        theme::set_theme(theme::ThemeMode::Dark);

        assert!(
            shadow_stack_for(true, false, 0.0, 1.0).is_empty(),
            "desabilitado: `shadow-none`"
        );
        // Repouso e válido: a sombra externa.
        let repouso = shadow_stack_for(false, false, 0.0, 1.0);
        assert_eq!(repouso.len(), 1);
        assert_eq!(repouso[0].offset.y, px(1.0), "shadow-xs desce 1px");
        assert_eq!(repouso[0].blur_radius, px(2.0));
        // Inválido: o coss tira a sombra de repouso mesmo sem foco.
        assert!(shadow_stack_for(false, true, 0.0, 1.0).is_empty());
        // Focado: sem sombra de repouso (quem aparece é o anel, que é outro elemento).
        assert!(shadow_stack_for(false, false, 0.0, 0.0).is_empty());
        // O `ring_k` não influencia mais nada aqui.
        assert_eq!(
            shadow_stack_for(false, false, 1.0, 1.0).len(),
            shadow_stack_for(false, false, 0.0, 1.0).len()
        );
    }

    /// O anel entra na árvore ao focar E permanece durante o desvanecer da saída — senão a
    /// transição não teria o que animar ao perder o foco. Desabilitado nunca mostra anel.
    #[test]
    fn anel_de_foco_aparece_ao_focar_e_durante_a_saida() {
        assert!(ring_visible(false, true, false), "focado");
        assert!(ring_visible(false, false, true), "desvanecendo na saída");
        assert!(!ring_visible(false, false, false), "em repouso, não");
        assert!(!ring_visible(true, true, false), "desabilitado, nunca");
        assert!(!ring_visible(true, false, true), "desabilitado, nem saindo");
    }

    /// O alfa escalado é o mecanismo do cross-fade: `k` multiplica, e valores fora de `[0,1]` são
    /// aparados (o `delta` do animador pode passar de 1 por arredondamento).
    #[test]
    fn alfa_escalado_multiplica_e_apara() {
        let meio = Rgba8(0xff000080);
        let cheio = meio.hsla().a;
        assert!((meio.scaled(1.0).a - cheio).abs() < 1e-6);
        assert!((meio.scaled(0.5).a - cheio * 0.5).abs() < 1e-6);
        assert_eq!(meio.scaled(0.0).a, 0.0);
        assert!((meio.scaled(3.0).a - cheio).abs() < 1e-6, "aparado em 1");
        assert_eq!(meio.scaled(-1.0).a, 0.0, "aparado em 0");
    }

    /// O contador conta CARACTERES, não bytes — senão acentos e emoji mentiriam o número.
    /// (Bug real e silencioso: "ação" tem 4 caracteres e 6 bytes.)
    #[test]
    fn contador_conta_caracteres_nao_bytes() {
        let texto = "ação";
        assert_eq!(texto.len(), 6, "bytes");
        assert_eq!(texto.chars().count(), 4, "caracteres — é este que o contador usa");

        let emoji = "oi 👋";
        assert_eq!(emoji.chars().count(), 4);
    }

    /// A cor do contador escala com a ocupação: neutro → aviso a partir de 90% → erro ao passar.
    #[test]
    fn cor_do_contador_por_ocupacao() {
        theme::set_theme(theme::ThemeMode::Dark);
        fn cor(count: usize, max: Option<usize>) -> u32 {
            match max {
                Some(m) if count > m => theme::DANGER(),
                Some(m) if m > 0 && count * 10 >= m * 9 => theme::WARNING(),
                _ => theme::TEXT_FAINT(),
            }
        }
        assert_eq!(cor(0, Some(10)), theme::TEXT_FAINT());
        assert_eq!(cor(8, Some(10)), theme::TEXT_FAINT());
        assert_eq!(cor(9, Some(10)), theme::WARNING(), "90% já avisa");
        assert_eq!(cor(10, Some(10)), theme::WARNING(), "no limite ainda é aviso");
        assert_eq!(cor(11, Some(10)), theme::DANGER(), "passou");
        assert_eq!(cor(999, None), theme::TEXT_FAINT(), "sem limite, sem alarme");
        // `max_len(0)` não pode dividir por zero nem virar aviso permanente.
        assert_eq!(cor(0, Some(0)), theme::TEXT_FAINT());
    }

    /// Altura e respiro crescem com o tamanho — um `Lg` mais baixo que um `Md` passaria batido no
    /// código e só apareceria na tela. As alturas são as do coss no breakpoint `sm:` (26/30/34).
    #[test]
    fn altura_e_respiro_crescem_com_o_tamanho() {
        let (sm, md, lg) = (InputSize::Sm, InputSize::Md, InputSize::Lg);

        // O `h-*` do coss é o MIOLO (fica no `<input>`); a caixa visível soma as duas bordas.
        assert_eq!(
            (sm.content_height(), md.content_height(), lg.content_height()),
            (26.0, 30.0, 34.0)
        );
        assert_eq!((sm.height(), md.height(), lg.height()), (28.0, 32.0, 36.0));
        for t in [sm, md, lg] {
            assert_eq!(
                t.height(),
                t.content_height() + 2.0,
                "a altura externa é miolo + 1px de borda em cima e embaixo"
            );
        }
        assert!(sm.height() < md.height() && md.height() < lg.height());
        assert!(sm.pad_x() < md.pad_x());
        assert_eq!(md.pad_x(), lg.pad_x(), "no coss o Md e o Lg têm o mesmo respiro");
        assert_eq!(InputSize::default(), InputSize::Md);
    }

    /// **O corpo do texto é o MESMO nos três tamanhos** (14px). No coss ele vem do wrapper
    /// (`sm:text-sm`) e as variantes de tamanho mudam só altura, respiro e line-height.
    ///
    /// Isto é contra-intuitivo — parece um esquecimento — então fica travado aqui pra ninguém
    /// "consertar" de volta pra uma escala por tamanho e sair do pixel-perfect.
    #[test]
    fn corpo_do_texto_nao_varia_com_o_tamanho() {
        assert_eq!(InputSize::Sm.text_size(), 14.0);
        assert_eq!(InputSize::Md.text_size(), 14.0);
        assert_eq!(InputSize::Lg.text_size(), 14.0);
    }

    /// O raio é `rounded-lg` (10px) igual nos três tamanhos — no coss o `rounded` está no wrapper,
    /// fora das variantes.
    #[test]
    fn raio_nao_varia_com_o_tamanho() {
        for t in [InputSize::Sm, InputSize::Md, InputSize::Lg] {
            assert_eq!(t.radius(), FIELD_RADIUS);
        }
    }

    // =============================================================================================
    // Textarea
    // =============================================================================================

    /// **Os números da referência, escritos como números.**
    ///
    /// O piso do miolo é `min-h-16.5 / 17.5 / 18.5` = **66 / 70 / 74**, e o respiro vertical é
    /// `py-[calc(--spacing(1)-1px)] / (1.5) / (2)` = **3 / 5 / 7**. Estão aqui LITERAIS de propósito: a
    /// implementação os DERIVA (linhas × entrelinha + respiro), e um teste que repetisse a derivação
    /// concordaria consigo mesmo mesmo com a entrelinha errada. Comparar com a referência é o único
    /// jeito de a derivação ser verificada em vez de assumida.
    #[test]
    fn piso_da_textarea_e_tres_linhas_mais_respiro() {
        let (sm, md, lg) = (InputSize::Sm, InputSize::Md, InputSize::Lg);

        assert_eq!(
            (sm.textarea_pad_y(), md.textarea_pad_y(), lg.textarea_pad_y()),
            (3.0, 5.0, 7.0),
            "py do coss: calc(--spacing(1|1.5|2) - 1px)"
        );
        assert_eq!(
            (
                sm.textarea_min_content_height(),
                md.textarea_min_content_height(),
                lg.textarea_min_content_height()
            ),
            (66.0, 70.0, 74.0),
            "min-h-16.5 / 17.5 / 18.5 do coss"
        );

        // E o que a derivação diz: os três pisos são a MESMA frase — três linhas mais o respiro.
        for t in [sm, md, lg] {
            assert_eq!(
                t.textarea_min_content_height(),
                TEXTAREA_ROWS as f32 * t.textarea_line_height() + 2.0 * t.textarea_pad_y(),
                "{t:?}: o piso é {TEXTAREA_ROWS} linhas + o respiro, não um número solto"
            );
        }
        assert!(sm.textarea_min_content_height() < md.textarea_min_content_height());
        assert!(md.textarea_min_content_height() < lg.textarea_min_content_height());
    }

    /// Piso EXTERNO = piso do miolo + as duas bordas de 1px → 68 / 72 / 76. Mesma relação (e mesmo
    /// porquê) do `content_height` × `height` do campo de uma linha.
    #[test]
    fn piso_externo_da_textarea_soma_as_duas_bordas() {
        for t in [InputSize::Sm, InputSize::Md, InputSize::Lg] {
            assert_eq!(t.textarea_min_height(), t.textarea_min_content_height() + 2.0);
        }
        assert_eq!(
            (
                InputSize::Sm.textarea_min_height(),
                InputSize::Md.textarea_min_height(),
                InputSize::Lg.textarea_min_height()
            ),
            (68.0, 72.0, 76.0)
        );
    }

    /// **A armadilha da entrelinha, com o número que sairia sem fixar.**
    ///
    /// Num campo de uma linha a entrelinha só desloca o glifo. Numa textarea ela MULTIPLICA: é a
    /// altura de cada linha, então ela decide o piso, o crescimento e quantas linhas cabem. O default
    /// do GPUI é `relative(1.618_034)` — sobre o corpo 14 dá **22,65**, e os três pisos do coss
    /// deixariam de caber três linhas.
    #[test]
    fn entrelinha_da_textarea_e_o_par_do_tailwind() {
        // O par do Tailwind pro `text-sm`: corpo 14, entrelinha 20.
        assert_eq!(InputSize::Md.text_size(), 14.0);
        assert_eq!(TEXTAREA_LINE_HEIGHT, 20.0);
        for t in [InputSize::Sm, InputSize::Md, InputSize::Lg] {
            assert_eq!(t.textarea_line_height(), 20.0, "{t:?}: o par não varia por tamanho");
        }

        // O que o GPUI usaria se ninguém fixasse a entrelinha.
        const RAZAO_DEFAULT_DO_GPUI: f32 = 1.618_034;
        let sem_fixar = InputSize::Md.text_size() * RAZAO_DEFAULT_DO_GPUI;
        assert!(
            (sem_fixar - 22.652_476).abs() < 1e-3,
            "o default do GPUI sobre 14px dá 22,65; veio {sem_fixar}"
        );

        // E o estrago: o piso de 70 do coss (3 linhas + 2×5) viraria ~78, e uma caixa de 70 caberia
        // 3,09 linhas em vez de 3 — meia linha aparecendo cortada na borda de baixo.
        let respiro = InputSize::Md.textarea_pad_y();
        let piso_torto = TEXTAREA_ROWS as f32 * sem_fixar + 2.0 * respiro;
        assert!(
            (piso_torto - 77.957_43).abs() < 1e-2,
            "sem fixar, o piso do Md sairia ~78; veio {piso_torto}"
        );
        assert!(
            piso_torto - InputSize::Md.textarea_min_content_height() > 7.0,
            "a diferença é de 8px — quase meia linha — e cresce a cada linha"
        );
        let linhas_no_piso = (InputSize::Md.textarea_min_content_height() - 2.0 * respiro) / sem_fixar;
        assert!(
            (linhas_no_piso - 3.0).abs() > 0.05,
            "com a entrelinha errada o piso do coss deixa de ser um número inteiro de linhas: \
             caberiam {linhas_no_piso}"
        );
    }

    /// **Os dois modelos de altura convergem.** Um piso de [`TEXTAREA_ROWS`] linhas é EXATAMENTE o
    /// `min-h-*` da referência, nos três tamanhos — é o que torna [`Input::rows`] e
    /// [`Input::textarea`] o mesmo pedido dito de duas formas, em vez de duas geometrias rivais.
    #[test]
    fn piso_por_linhas_bate_com_o_piso_da_referencia() {
        for t in [InputSize::Sm, InputSize::Md, InputSize::Lg] {
            assert_eq!(
                TextareaFloor::Rows(TEXTAREA_ROWS).min_content_height(t),
                TextareaFloor::Size.min_content_height(t),
                "{t:?}: rows({TEXTAREA_ROWS}) tem que dar o mesmo piso que o min-h do tamanho"
            );
            // E o mesmo do lado de fora — os dois ramos de `min_height` somam a MESMA borda.
            assert_eq!(
                TextareaFloor::Rows(TEXTAREA_ROWS).min_height(t),
                TextareaFloor::Size.min_height(t),
                "{t:?}: o piso EXTERNO também tem que coincidir"
            );
            assert_eq!(
                TextareaFloor::Size.min_height(t),
                TextareaFloor::Size.min_content_height(t) + 2.0,
                "{t:?}: externo = miolo + as duas bordas"
            );
        }
        // Uma linha a mais é uma entrelinha a mais — nada de escala mágica.
        assert_eq!(
            TextareaFloor::Rows(4).min_content_height(InputSize::Md)
                - TextareaFloor::Rows(3).min_content_height(InputSize::Md),
            TEXTAREA_LINE_HEIGHT
        );
        // `rows(0)` não pode virar caixa de altura zero (que fica inclicável): o mínimo é uma linha.
        assert_eq!(
            TextareaFloor::Rows(0).min_content_height(InputSize::Md),
            TextareaFloor::Rows(1).min_content_height(InputSize::Md)
        );
        assert_eq!(
            TextareaFloor::Rows(1).min_content_height(InputSize::Md),
            TEXTAREA_LINE_HEIGHT + 2.0 * InputSize::Md.textarea_pad_y()
        );
    }

    /// O respiro HORIZONTAL de uma textarea é o MESMO do campo de uma linha (11 / 9), não um valor
    /// próprio: conferido classe por classe no original, o `px-[calc(--spacing(3)-1px)]` do
    /// `textarea.tsx` é o mesmo do `input.tsx`. Se alguém inventar um `textarea_pad_x`, este teste
    /// documenta por que não deveria.
    #[test]
    fn respiro_horizontal_da_textarea_e_o_mesmo_do_campo() {
        assert_eq!(InputSize::Md.pad_x(), 11.0);
        assert_eq!(InputSize::Sm.pad_x(), 9.0);
        assert_eq!(InputSize::Lg.pad_x(), InputSize::Md.pad_x());
    }

    // =============================================================================================
    // A alça de redimensionar
    // =============================================================================================

    /// **A alça sai com a moldura — e é assim que o `resize-none` do coss é reproduzido de graça.**
    ///
    /// Os quatro gates, um a um. O do `unstyled` é o que importa mais: é ele que tira a alça de dentro
    /// do [`crate::input_group::InputGroup`], que é o ÚNICO lugar onde a referência escreve
    /// `resize-none` — e é o único lugar da lib que hospeda o campo sem moldura.
    #[test]
    fn a_alca_sai_com_a_moldura() {
        // O caso normal: textarea com moldura, habilitada, sem desligar nada.
        assert!(alca_visivel(true, false, false, false), "textarea comum tem alça");

        // Campo de UMA linha: não há altura pra arrastar.
        assert!(!alca_visivel(false, false, false, false), "uma linha não redimensiona");
        // Sem moldura (é o `input-group`): o `resize-none` da referência.
        assert!(!alca_visivel(true, true, false, false), "sem moldura, sem alça");
        // Desabilitado: quem não aceita edição não aceita redimensionamento.
        assert!(!alca_visivel(true, false, true, false), "desabilitado não redimensiona");
        // O desligamento explícito.
        assert!(!alca_visivel(true, false, false, true), "`resize_none()` desliga");

        // Qualquer combinação de negativas continua negativa — nenhum gate "cancela" o outro.
        assert!(!alca_visivel(false, true, true, true));
        assert!(!alca_visivel(true, true, true, false));
    }

    /// **A matemática do arraste**: mouse pra baixo cresce, pra cima encolhe, e o resultado nunca sai
    /// de `[piso, teto]`.
    ///
    /// Números redondos e independentes das constantes de propósito: o que se guarda aqui é a CONTA,
    /// não a geometria (essa está nos testes de piso e de teto).
    #[test]
    fn o_arraste_cresce_pra_baixo_e_para_nos_limites() {
        // Pra baixo cresce, na razão de 1px de mouse por 1px de caixa (a alça está na borda de baixo,
        // então a caixa segue o dedo sem fator de escala).
        assert_eq!(altura_apos_arraste(100.0, 40.0, 72.0, 400.0), 140.0);
        // Pra cima encolhe.
        assert_eq!(altura_apos_arraste(100.0, -20.0, 72.0, 400.0), 80.0);
        // Parado é parado.
        assert_eq!(altura_apos_arraste(100.0, 0.0, 72.0, 400.0), 100.0);

        // O piso segura: arrastar 500px pra cima para no piso, não em zero nem em negativo.
        assert_eq!(altura_apos_arraste(100.0, -500.0, 72.0, 400.0), 72.0);
        // O teto segura.
        assert_eq!(altura_apos_arraste(100.0, 5_000.0, 72.0, 400.0), 400.0);

        // Uma altura inicial JÁ fora dos limites é trazida pra dentro (é o caso de um `height()` do
        // call site menor que o piso: o primeiro arraste corrige em vez de propagar o absurdo).
        assert_eq!(altura_apos_arraste(10.0, 0.0, 72.0, 400.0), 72.0);
        assert_eq!(altura_apos_arraste(9_000.0, 0.0, 72.0, 400.0), 400.0);

        // E um teto ABAIXO do piso não derruba a app: `f32::clamp` entra em pânico com `min > max`, e o
        // guarda faz o campo parar no piso. (Sem o `piso.max(teto)`, esta linha é um panic.)
        assert_eq!(altura_apos_arraste(100.0, 0.0, 72.0, 10.0), 72.0);
    }

    /// **O teto do arraste é de 20 linhas, e é o mesmo tipo de conta do piso.**
    ///
    /// Os totais estão em LITERAL — 408 / 412 / 416 — e não recalculados a partir de
    /// [`ARRASTE_MAX_LINHAS`]: um teste que repetisse a fórmula concordaria consigo mesmo mesmo com a
    /// entrelinha errada, que é exatamente a armadilha que
    /// `entrelinha_da_textarea_e_o_par_do_tailwind` documenta.
    #[test]
    fn o_teto_do_arraste_e_de_vinte_linhas() {
        assert_eq!(ARRASTE_MAX_LINHAS, 20);

        // 20 × 20 + 2 × respiro + as duas bordas.
        assert_eq!(teto_do_arraste(InputSize::Md), 412.0, "20×20 + 2×5 + 2");
        assert_eq!(teto_do_arraste(InputSize::Sm), 408.0, "20×20 + 2×3 + 2");
        assert_eq!(teto_do_arraste(InputSize::Lg), 416.0, "20×20 + 2×7 + 2");

        for t in [InputSize::Sm, InputSize::Md, InputSize::Lg] {
            // O teto tem que estar ACIMA do piso nos três tamanhos, senão a alça não teria curso.
            assert!(
                teto_do_arraste(t) > t.textarea_min_height(),
                "{t:?}: o teto ({}) tem que ficar acima do piso ({})",
                teto_do_arraste(t),
                t.textarea_min_height()
            );
            // E teto e piso se medem do MESMO jeito: os dois contam as duas bordas. A diferença entre
            // eles é exatamente a diferença de LINHAS — nada de uma conta somar borda e a outra não.
            assert_eq!(
                teto_do_arraste(t) - t.textarea_min_height(),
                (ARRASTE_MAX_LINHAS - TEXTAREA_ROWS) as f32 * TEXTAREA_LINE_HEIGHT,
                "{t:?}: a diferença entre teto e piso é só o número de linhas"
            );
        }

        // O teto é mais generoso que o do exemplo de `growing` (10 linhas), que é o que justifica 20:
        // arrastar não pode ser mais restrito que o crescimento automático.
        assert!(ARRASTE_MAX_LINHAS > 10);
        // E não é sem teto: 20 linhas de 20px são 400px de conteúdo, mais da metade de uma janela 720.
        assert_eq!(ARRASTE_MAX_LINHAS as f32 * TEXTAREA_LINE_HEIGHT, 400.0);
        assert!(teto_do_arraste(InputSize::Md) < 720.0, "o teto cabe numa janela de 720");
    }

    /// **O glifo da alça existe no bundle.**
    ///
    /// ⚠️ Este é o teste que impede o pior modo de falha de um ícone no GPUI: um caminho errado **não
    /// dá erro** — a `AssetSource` devolve `None`, o `svg` não desenha nada, nada é logado, e a alça
    /// simplesmente não aparece na tela. (Já custou tempo nesta lib: `paperclip.svg` não existe no
    /// Iconoir, o nome é `attachment.svg`.)
    #[test]
    fn o_glifo_da_alca_esta_no_bundle() {
        // O caminho vem da CONSTANTE que o render usa (`ALCA_GLIFO`), e não repetido aqui: repetido, o
        // teste continuaria verde com o render pedindo outro arquivo.
        let bytes = crate::assets::lookup(ALCA_GLIFO)
            .unwrap_or_else(|| panic!("`{ALCA_GLIFO}` não está na tabela de assets embutidos"));
        assert!(!bytes.is_empty(), "o SVG não pode estar vazio");
        // E é um SVG de verdade, não outro arquivo com o nome certo.
        let texto = std::str::from_utf8(bytes).expect("SVG é texto");
        assert!(texto.contains("<svg"), "o conteúdo tem que ser um SVG");
        // Um caminho ERRADO devolve `None` em silêncio — é isto que o teste está protegendo.
        assert!(crate::assets::lookup("icons/resize_grip.svg").is_none());
    }

    /// **O alvo da alça cabe na moldura da textarea, nos três tamanhos — e é maior que um ícone de
    /// campo.**
    ///
    /// A moldura tem `overflow_hidden`: um alvo mais alto que o piso da caixa sairia recortado, e a
    /// parte recortada é área de pega que o usuário vê e não consegue usar.
    #[test]
    fn o_alvo_da_alca_cabe_no_piso_da_textarea() {
        assert_eq!(ALCA_FOLGA, 6.0, "a mesma folga do botãozinho de campo");
        for t in [InputSize::Sm, InputSize::Md, InputSize::Lg] {
            let alvo = t.icon() + ALCA_FOLGA;
            // O alvo é maior que um ícone de campo — é o que o torna um alvo e não um traço a acertar.
            assert!(alvo > t.icon(), "{t:?}: o alvo tem que ser maior que um ícone do campo");
            // E cabe no MIOLO da caixa no piso, com folga (o piso são três linhas de 20).
            assert!(
                alvo < t.textarea_min_content_height(),
                "{t:?}: o alvo ({alvo}) não cabe no piso do miolo ({})",
                t.textarea_min_content_height()
            );
            // Não cabe num campo de UMA linha — e é uma razão a mais pra a alça ser só de textarea.
            assert!(
                alvo > t.content_height() / 2.0,
                "{t:?}: o alvo ocupa mais de meia altura de um campo de uma linha"
            );
        }
        // Os três lados, em número: 19 / 21 / 23. Literais pelo mesmo motivo dos pisos — a
        // implementação DERIVA, e um teste que repetisse a derivação concordaria consigo mesmo.
        assert_eq!(
            (
                InputSize::Sm.icon() + ALCA_FOLGA,
                InputSize::Md.icon() + ALCA_FOLGA,
                InputSize::Lg.icon() + ALCA_FOLGA
            ),
            (19.0, 21.0, 23.0)
        );
    }

    /// **A tinta de repouso da alça é 40% do token de ícone.**
    ///
    /// Guardada como número porque foi pedido visual, e porque o aparo de [`Rgba8::scaled`] em `[0,1]`
    /// esconderia um valor absurdo. Mais apagada que a do ícone-alça do [`crate::scrub_input`] (metade)
    /// porque esta atravessa a área de texto em vez de ter um slot próprio.
    #[test]
    fn a_tinta_de_repouso_da_alca_e_quarenta_por_cento() {
        assert_eq!(ALCA_TINTA_REPOUSO, 0.4);
        assert!(ALCA_TINTA_REPOUSO > 0.0, "tinta invisível não é afordância");
        assert!(ALCA_TINTA_REPOUSO < 0.5, "mais discreta que a alça do scrub, que é metade");

        // E o alfa resultante é o do token escalado, não uma segunda cor escrita à mão.
        theme::set_theme(theme::ThemeMode::Dark);
        let token = opaque(theme::ICON());
        let repouso = token.scaled(ALCA_TINTA_REPOUSO);
        assert!((repouso.a - token.hsla().a * ALCA_TINTA_REPOUSO).abs() < 1e-6);
        assert!(repouso.a < token.hsla().a, "repouso é mais apagado que o hover");
    }

    /// **A altura arrastada tem precedência sobre o `height()` do call site, e os dois sobre o piso.**
    ///
    /// A função que o render chama é a MESMA que este teste chama — sem isso o teste seria uma cópia do
    /// `or_else` concordando consigo mesma, e inverter a ordem no render (deixar o `height()` vencer o
    /// arraste) não quebraria nada: a alça ficaria na tela sem efeito nenhum.
    #[test]
    fn o_arraste_vence_o_height_do_call_site() {
        // Só o piso: nenhum dos dois manda, e a moldura fica em `min_h` (o `field-sizing-content`).
        assert_eq!(altura_fixada_da_textarea(None, None), None);
        // Só o `height()`: a caixa é fixa no que o código pediu — e o `+2` é a tradução de MIOLO
        // (que é o que `Input::height` guarda) pra altura EXTERNA (que é o que a moldura usa).
        assert_eq!(altura_fixada_da_textarea(None, Some(198.0)), Some(200.0));
        // Só o arraste: a caixa é fixa no que o usuário arrastou, que JÁ é altura externa (sem `+2`).
        assert_eq!(altura_fixada_da_textarea(Some(150.0), None), Some(150.0));
        // Os DOIS: o arraste ganha.
        assert_eq!(
            altura_fixada_da_textarea(Some(150.0), Some(198.0)),
            Some(150.0),
            "a intenção explícita do usuário vence o default do código"
        );
    }
}

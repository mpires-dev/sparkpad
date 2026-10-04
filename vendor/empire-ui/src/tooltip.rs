//! `Tooltip` — a **etiqueta de ajuda** do `empire-ui`, com o visual do design system [coss][1].
//!
//! [1]: https://github.com/cosscom/coss — `apps/ui/registry/default/ui/tooltip.tsx`
//!
//! ```ignore
//! use empire_ui::menu::MenuSide;
//! use empire_ui::tooltip::Tooltip;
//! use empire_ui::Button;
//!
//! Tooltip::new("dica-salvar", "Salvar (⌘S)")
//!     .child(Button::icon("salvar", "iconoir/regular/floppy-disk.svg"))
//!
//! // Do lado, quando o de cima está ocupado (uma barra colada no topo da janela):
//! Tooltip::new("dica-lixeira", "Apagar")
//!     .side(MenuSide::Right)
//!     .child(Button::icon("apagar", "iconoir/regular/trash.svg"))
//! ```
//!
//! # Anatomia
//!
//! ```text
//!          ┌──────────────┐
//!          │ Salvar (⌘S)  │  ← o POPUP: 12/16, respiro 8/4, raio 8, borda 1px + fio de bisel
//!          └──────────────┘
//!                 ↕ 4px       ← `sideOffset={4}`
//!             ┌───────┐
//!             │  ⌘S   │       ← o GATILHO: o que você passou pro `.child(..)`
//!             └───────┘
//! ```
//!
//! # Por que NÃO a `.tooltip()` embutida do GPUI
//!
//! O GPUI já tem `div().tooltip(..)`, e ela **não** é o comportamento da referência. Duas diferenças,
//! as duas medidas na fonte do `gpui` 0.2.2:
//!
//! 1. **Ela ancora no MOUSE.** Em `window.rs`, `Window::prepaint_tooltip` monta a caixa em
//!    `Bounds::new(mouse_position + point(px(1.), px(1.)), tooltip_size)` — a etiqueta nasce colada na
//!    ponta do cursor e muda de lugar conforme o ponteiro anda dentro do gatilho. A referência ancora
//!    no **retângulo do gatilho** (`Positioner` com `side="top"`, `align="center"`, `sideOffset={4}`):
//!    a etiqueta fica parada, centralizada, 4px acima do botão. É a diferença entre uma legenda de
//!    cursor e uma etiqueta de componente.
//! 2. **Ela não tem atraso.** Não há nenhum `TOOLTIP_DELAY` no `gpui`: o `hover_listener` do
//!    `Interactivity` mostra a view no primeiro `MouseMoveEvent` que cai na hitbox. A referência
//!    espera **600ms** (o default do primitivo do Base UI) justamente pra o ponteiro poder atravessar
//!    uma barra de ferramentas inteira sem acender seis etiquetas no caminho.
//!
//! O teste de janela `o_popup_nasce_acima_do_gatilho_e_nao_no_cursor` é a **medição** dessa decisão:
//! com o ponteiro em (500, 500) e o gatilho em (200, 100), o popup é pintado com a base 4px acima do
//! gatilho e centralizado nele — a 300px de onde a `.tooltip()` embutida o teria posto.
//!
//! # O desenho: um `RenderOnce` com o tempo numa tabela
//!
//! O `Tooltip` **embrulha** o gatilho (`.child(..)`) em vez de ser uma entidade irmã dele. É o que
//! torna o call site uma linha só, e é o que a referência faz (`<Tooltip><TooltipTrigger>…`).
//!
//! Isso o obriga a ser um [`RenderOnce`], e um `RenderOnce` não tem estado — mas o atraso de 600ms é
//! estado, e é estado de TEMPO. A saída é a mesma do [`crate::button`]: uma tabela `thread_local`
//! indexada por [`ElementId`], e o [`gpui::Window::request_animation_frame`] como motor de frames
//! enquanto o relógio corre. Ver [`TOOLTIPS`].
//!
//! Três coisas moram nessa tabela, e cada uma resolve um problema que um `RenderOnce` não resolveria:
//!
//! | o que | por que na tabela |
//! |---|---|
//! | o retângulo do gatilho | é o "anchor rect" do `Positioner`, e só existe **depois** do prepaint |
//! | o hover (gatilho e popup) | é o que dispara o relógio, e sobrevive à reconstrução do elemento |
//! | o instante da última virada | é o relógio dos 600ms e do fade |
//!
//! (A caixa medida do popup também mora lá, mas essa é só observabilidade — ver [`measured_popup`].)
//!
//! ## O atraso de 600ms **paga** a armadilha do primeiro frame
//!
//! O `crate::menu` tem um aviso grande sobre isto: medir o popup por `canvas` e posicionar pela medida
//! erra o primeiro frame, porque a medida só existe depois do prepaint. Aqui o problema **não existe**,
//! e não por sorte: quando o popup finalmente é montado já passaram 600ms — ou seja dezenas de frames
//! em que o `canvas` do gatilho mediu e remediu o anchor rect. A posição do popup, no primeiro frame
//! em que ele aparece, já é a definitiva.
//!
//! A posição em si continua sendo a do [`crate::menu`]: [`anchor_point`] é a **única** aritmética de
//! `side`/`align`/offset da lib, e [`align_center_wrap`] é o **único** jeito de expressar
//! `align: center` (que não é um canto, e por isso sai por layout). Este módulo não tem conta de
//! posição própria — só a de largura disponível, que é o eixo que o menu não usava
//! ([`popup_max_width`]).
//!
//! # Onde ele funciona
//!
//! O popup vai dentro de [`gpui::deferred`]`(`[`gpui::anchored`]`(..))`, então ele **escapa do
//! recorte** de quem estiver por fora: um gatilho dentro de uma [`crate::ScrollArea`] rolada mostra a
//! etiqueta inteira, fora da caixa de rolagem, e não uma fatia dela. A prioridade é a **mesma** do
//! menu e do select (ver [`POPUP_PRIORITY`]).
//!
//! Isso é medido, e não presumido: o teste `tests_de_janela::a_etiqueta_escapa_do_recorte` planta o
//! gatilho dentro de uma caixa de 20×20 com `overflow_hidden` e confere que a etiqueta sai medindo a
//! altura e a largura inteiras. Encurralada na borda da janela ela **desliza** pra dentro em vez de
//! virar de lado (`tests_de_janela::encurralada_na_borda_a_etiqueta_gruda_na_janela`).
//!
//! # Diferenças em relação ao original — todas declaradas
//!
//! **Não reproduzível**
//!
//! - **`data-starting-style:scale-98` / `data-ending-style:scale-98`** (a metade de ESCALA da
//!   animação de entrada/saída): o GPUI não tem `transform` em `div` — não há como desenhar um
//!   elemento a 98% do tamanho sem refazer todo o layout dele em números, e um popup que muda de
//!   tamanho muda também o ponto de ancoragem. A metade de **opacidade** (`opacity-0`) é fiel e está
//!   implementada; ver [`FADE`] e [`popup_opacity`].
//! - **`data-instant:duration-0`**: o `data-instant` do Base UI marca as mudanças que **não** devem
//!   ser animadas — o caso é o ponteiro pular de um gatilho pro vizinho compartilhando o mesmo
//!   `TooltipCreateHandle`/`Provider`, em que a etiqueta se MOVE em vez de fechar e abrir. Aqui cada
//!   `Tooltip` é independente (ver "Superset consciente" abaixo, sobre o `Provider`), então não existe
//!   a transição que o `data-instant` desligaria.
//! - **`not-dark:bg-clip-padding`**: o GPUI pinta o fundo na border box e não tem `background-clip`.
//!   No tema claro a borda (`--border`, preto 8%) lê sobre o branco do próprio popup em vez de compor
//!   com o que houver atrás dele — na prática ela fica fixa em `#ebebeb`. Mesma diferença já
//!   documentada no [`crate::menu`], no [`crate::dialog`] e no [`crate::toast`].
//! - **`text-balance`**: não há balanceamento de linhas no GPUI. Afeta **onde** a linha quebra, não a
//!   altura da caixa. Mesma nota do [`crate::empty`].
//! - **`transition-[top,left,right,bottom,transform]` do `Positioner`**: o GPUI não interpola
//!   geometria. O popup aparece na posição final; ele não desliza de uma posição pra outra quando o
//!   gatilho se move. Na referência isso só é visível quando o `Positioner` troca de lado sozinho —
//!   o que aqui também não acontece, porque o `snap_to_window_with_margin` DESLIZA em vez de virar
//!   (a mesma escolha, e o mesmo motivo, do [`crate::menu`]).
//! - **O cross-fade de TEXTO do `Viewport`** (todo o bloco `**:data-current:…` /
//!   `**:data-previous:…`, mais o `overflow-clip` e o
//!   `w-[calc(var(--popup-width)-2*var(--viewport-inline-padding)-2px)]`): é a máquina que troca o
//!   texto **com o popup aberto**, esmaecendo o antigo por cima do novo enquanto a caixa muda de
//!   tamanho. Aqui o texto é uma [`gpui::SharedString`] fixa por frame; trocá-la troca a etiqueta no
//!   frame seguinte, sem cruzar as duas. Sem o cross-fade, o `overflow-clip` e a largura calculada do
//!   viewport não têm nada pra fazer — não estão omitidos, estão vazios.
//! - **`h-(--positioner-height)`/`w-(--positioner-width)` e `h-(--popup-height,auto)`/
//!   `w-(--popup-width,auto)`**: variáveis que o Base UI escreve no DOM pra a máquina de cross-fade
//!   acima ter uma caixa estável durante a troca. Sem cross-fade, o popup mede o conteúdo.
//! - **`origin-(--transform-origin)`**: `transform-origin` do `scale-98`, que não existe aqui.
//! - **`before:pointer-events-none` do bisel**: o overlay de bisel é um `div` sem listener nenhum,
//!   então não cria hitbox e não recebe ponteiro. Redundante aqui, não omitido.
//! - **`data-slot`**: atributo de DOM, usado no original pra estilizar de fora. Sem DOM, sem atributo.
//!
//! **Resolvido em número**
//!
//! - **`rounded-md`** = `calc(var(--radius) - 2px)` com `--radius: 0.625rem` = **8px**. Note que o
//!   popup do [`crate::menu`] é `rounded-lg` (10px): a etiqueta é deliberadamente menos redonda que o
//!   menu, e não é erro de porte. Ver [`RADIUS`].
//! - **`px-(--viewport-inline-padding) py-1`** com `--viewport-inline-padding: --spacing(2)` = **8px
//!   na horizontal, 4px na vertical**. Ver [`PAD_X`] e [`PAD_Y`].
//! - **`text-xs`** = **12px com entrelinha 16** — o par do Tailwind. ⚠️ Fixar a entrelinha é
//!   obrigatório: o default do GPUI é `relative(1.618_034)` (a razão de ouro, em
//!   `gpui/src/geometry.rs`), que daria uma linha de 19,42px e uma caixa de 29,42 em vez de **26**.
//!   Ver [`LINE_HEIGHT`] e o teste `a_entrelinha_e_16_e_nao_a_razao_de_ouro`.
//! - **`shadow-md/5`**: o `globals.css` do coss **não** redefine `--shadow-md`, então vale o default
//!   do Tailwind v4 — `0 4px 6px -1px black/10, 0 2px 4px -2px black/10` — e o `/5` troca o alfa das
//!   duas camadas por 5%. Mesma fonte de onde o [`crate::empty`] resolveu o `--shadow-sm` dele. Ver
//!   [`SHADOW_LAYERS`].
//! - **`before:rounded-[calc(var(--radius-md)-1px)]`** = 7px na referência, e **8px** aqui. Não é
//!   divergência: lá o pseudo-elemento fica por DENTRO da borda (padding box, raio − 1); aqui o
//!   overlay cobre a **border box** (`inset: -1px`), e o raio da border box é o da superfície. Ver
//!   [`bevel_overlay`].
//! - **Os atrasos** (600ms pra abrir, 0 pra fechar) e o **popup hoverable** são os defaults do
//!   `@base-ui/react/tooltip`, que a referência não sobrescreve. Ver [`OPEN_DELAY`],
//!   [`CLOSE_DELAY`] e [`Region`].
//!
//! **Valor deduzido**
//!
//! - **[`FADE`] = 150ms.** A referência declara `transition-[width,height,scale,opacity]` sem classe
//!   de duração, então vale o `--default-transition-duration` do Tailwind v4, que é 150ms. É o único
//!   número deste módulo que não saiu nem do `.tsx` nem da tabela de tokens.
//!
//! **Desvio consciente**
//!
//! - **No tema escuro o fio de bisel é o DOBRO da referência**: branco ~11,8% (alfa `0x1e`) em vez de
//!   6% (`0x0f`). A 6% o filete é imperceptível no nosso fundo escuro. É o mesmo desvio já vigente no
//!   [`crate::menu`], no [`crate::input`], no [`crate::card`] e no [`crate::select`] — e importa que
//!   os cinco sejam iguais, porque a etiqueta aparece encostada nos outros. Travado no teste
//!   `bisel_escuro_e_o_dobro_da_referencia`.
//! - **`side`/`align` são o [`MenuSide`]/[`MenuAlign`] do [`crate::menu`]**, não um par de enums
//!   próprios com os mesmos quatro nomes. O default aqui é [`MenuSide::Top`] (o da referência), e o do
//!   menu é `Bottom` — por isso [`DEFAULT_SIDE`] é explícito e não `MenuSide::default()`. Dois enums
//!   idênticos obrigariam a uma tradução entre eles pra chamar [`anchor_point`], e é exatamente numa
//!   tradução dessas que um `Left` viraria `Right` um dia.
//! - **O popup fica *hoverable* pelo retângulo dele, sem "safe polygon".** No Base UI a etiqueta
//!   hoverable sobrevive ao ponteiro atravessar o vão de 4px entre gatilho e popup. Aqui o vão é
//!   território de ninguém — mas atravessá-lo não fecha nada, porque **reentrar num popup ainda
//!   visível não paga o atraso de novo**: o relógio é rebobinado pro ponto do fade que produz a
//!   opacidade corrente (ver [`resume_elapsed`]). Na prática o efeito é o do original; o que falta é
//!   o polígono, não o comportamento.
//!
//! **Superset consciente**
//!
//! - O original declara cor de texto no popup (`text-popover-foreground`) e **herda** o resto. O GPUI
//!   não tem `inherit`, então o popup declara também o **corpo** e a **entrelinha** do texto
//!   (`text-xs`), que no original vêm da mesma classe. Sem isso a etiqueta sairia no corpo default da
//!   janela.
//! - **`min-width: 0` no viewport, que a referência não tem.** No CSS o `max-width` reflui o texto
//!   sozinho, porque o min-content de um parágrafo é a palavra mais longa. No GPUI o min-content de um
//!   texto é a **frase inteira numa linha**, então sem essa liberdade o `max-w-(--available-width)`
//!   apararia a caixa e deixaria o texto sair por fora dela. É uma declaração a mais pra chegar ao
//!   MESMO resultado — ver o comentário no [`popup_surface`], e o teste de janela
//!   `a_etiqueta_respeita_a_largura_disponivel`, que mede a etiqueta numa janela de 200px.
//!
//! **O que NÃO está aqui (declarado, não esquecido)**
//!
//! - **`TooltipProvider` e `TooltipCreateHandle`**: o `Provider` compartilha um relógio entre várias
//!   etiquetas (a segunda de uma barra abre sem atraso, porque a primeira "esquentou" o grupo) e o
//!   `createHandle` deixa abrir uma etiqueta de fora, por referência. Os dois exigem um estado
//!   COMPARTILHADO entre `Tooltip`s, e o que existe aqui é uma tabela por [`ElementId`] — cada
//!   etiqueta com o seu relógio. É a diferença mais visível deste porte: numa barra de ferramentas,
//!   cada botão cobra os seus 600ms.
//! - **`anchor` do `Positioner`** (ancorar em outro elemento que não o gatilho): ausente, como no
//!   [`crate::menu`].
//! - **`alignOffset`**: a referência não passa nenhum, então o valor é fixo em [`ALIGN_OFFSET`]. A
//!   [`anchor_point`] aceita o parâmetro; expor um `.align_offset(..)` aqui seria API que a referência
//!   não tem.
//! - **RTL**: [`MenuSide::Left`]/[`MenuSide::Right`] são o `inline-start`/`inline-end` resolvidos pra
//!   LTR. Mesma nota do [`crate::menu`].

use crate::color::Rgba8;
use crate::menu::{
    align_center_wrap, anchor_point, rect_of, MenuAlign, MenuSide, Rect, SIDE_OFFSET, WINDOW_MARGIN,
};
use crate::theme;
use gpui::{
    anchored, canvas, deferred, div, point, prelude::FluentBuilder as _, px, AnyElement, App,
    BoxShadow, Bounds, Div, ElementId, InteractiveElement, IntoElement, ParentElement, Pixels,
    RenderOnce, SharedString, StatefulInteractiveElement as _, Styled, Window,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::{Duration, Instant};

// =================================================================================================
// Paleta (valores do coss resolvidos)
// =================================================================================================
//
// Mesma disciplina de cor do menu, do card e do modal: TODO valor é `0xRRGGBBAA`, com o byte de alfa,
// SEMPRE — ver [`crate::color`]. Os tokens da [`crate::theme::Palette`] são `0xRRGGBB`, e misturar as
// duas convenções DESLOCA os canais e produz outra cor **sem erro de compilação** (`rgba(0xffffff)` é
// lido como `0x00FFFFFF`, ciano). Já custou três bugs visíveis nesta base. A única ponte é
// [`crate::color::opaque`].

/// Tokens visuais da etiqueta, por tema.
#[derive(Clone, Copy, Debug)]
struct TooltipPalette {
    /// Fundo do popup (`bg-popover`). **Opaco** nos dois temas — é o que torna a sombra externa
    /// segura (ver [`popup_shadow`]).
    popover_bg: Rgba8,
    /// Borda de 1px do popup (`border`, que no Tailwind do coss resolve pro `--border`).
    border: Rgba8,
    /// Cor do texto (`text-popover-foreground`).
    text: Rgba8,
    /// Fio de bisel de 1px sobre a borda. No claro é escuro e desce; no escuro é claro e sobe.
    bevel: Rgba8,
    /// Sentido do bisel: `+1` desce (fio na BASE), `-1` sobe (fio no TOPO). O sinal sai do
    /// deslocamento da sombra da referência (`0 1px` / `0 -1px`), e errar isso põe o filete no lado
    /// errado — o que lê como um popup iluminado por baixo.
    bevel_dir: f32,
    /// Cor das duas camadas da sombra externa (`shadow-md/5` = preto 5%).
    shadow: Rgba8,
}

/// Tema **claro**.
const TOOLTIP_LIGHT: TooltipPalette = TooltipPalette {
    popover_bg: Rgba8(0xffffffff), // --popover: white
    border: Rgba8(0x00000014),     // --border: preto 8%
    text: Rgba8(0x262626ff),       // --popover-foreground: neutral-800
    bevel: Rgba8(0x0000000a),      // preto 4% — fiel à referência
    bevel_dir: 1.0,
    shadow: Rgba8(0x0000000d), // preto 5%
};

/// Tema **escuro**.
const TOOLTIP_DARK: TooltipPalette = TooltipPalette {
    // --popover escuro = mix(background 96%, white) = #1d1d1d (o --background é #141414): a etiqueta
    // LEVANTA sobre o painel em vez de sumir nele.
    popover_bg: Rgba8(0x1d1d1dff),
    border: Rgba8(0xffffff0f), // --border: branco 6%
    text: Rgba8(0xf5f5f5ff),   // --popover-foreground: neutral-100
    // ⚠️ DESVIO CONSCIENTE do coss: o original usa branco a **6%** (alfa 15). Aqui é o DOBRO —
    // alfa 30 ≈ 11,8% — por decisão de design: a 6% o filete é imperceptível no nosso fundo escuro.
    // É o mesmo desvio já vigente no `menu.rs`, no `input.rs`, no `card.rs` e no `select.rs`; manter
    // os cinco iguais é o ponto — a etiqueta aparece encostada nos outros. NÃO "corrija" isto pra
    // 0x0f achando que é erro de porte; se a intenção mudar, mude junto o teste
    // `bisel_escuro_e_o_dobro_da_referencia`.
    bevel: Rgba8(0xffffff1e),
    bevel_dir: -1.0,
    shadow: Rgba8(0x0000000d), // preto 5% — igual no claro (o `/5` não muda com o tema)
};

/// A paleta da etiqueta no tema corrente.
fn palette() -> &'static TooltipPalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &TOOLTIP_DARK,
        theme::ThemeMode::Light => &TOOLTIP_LIGHT,
    }
}

// =================================================================================================
// Geometria
// =================================================================================================
//
// Os utilitários Tailwind do `tooltip.tsx` resolvidos em número. 1 unidade Tailwind = 4px;
// `--radius` = 0.625rem = 10px. Não há variante de breakpoint neste componente.

/// Raio do popup — `rounded-md` = `--radius-md` = `calc(var(--radius) - 2px)` = **8px**.
///
/// ⚠️ **Menos redondo que o popup do [`crate::menu`]**, que é `rounded-lg` = 10. Está no `.tsx`, e é
/// de propósito: a etiqueta é a menor superfície flutuante do sistema.
const RADIUS: f32 = 8.0;

/// Espessura da borda do popup — `border`.
const BORDER: f32 = 1.0;

/// Respiro **horizontal** do texto — `px-(--viewport-inline-padding)` com
/// `--viewport-inline-padding: --spacing(2)`, ou seja `2 × 4px`.
const PAD_X: f32 = 8.0;

/// Respiro **vertical** do texto — `py-1`.
const PAD_Y: f32 = 4.0;

/// Corpo do texto — `text-xs`.
const TEXT_SIZE: f32 = 12.0;

/// **Entrelinha** do texto — o `text-xs` do Tailwind é `12px/16px`.
///
/// ⚠️ **Isto NÃO é decoração.** O GPUI não usa a entrelinha do CSS; o default dele é `phi()`, a razão
/// de ouro (**1,618034** × o corpo, em `gpui/src/geometry.rs`). Sem fixar, a linha sairia com 19,42px
/// — 21% mais alta — e a caixa inteira com 29,42px em vez de **26**, ou seja 13% mais alta, sem nada
/// no código parecendo errado. Travado em `a_entrelinha_e_16_e_nao_a_razao_de_ouro` (que mostra os
/// dois números) e medido de verdade em
/// `tests_de_janela::o_popup_nasce_acima_do_gatilho_e_nao_no_cursor`.
const LINE_HEIGHT: f32 = 16.0;

/// A **altura** do popup de uma linha, derivada: as duas bordas, o `py-1` de cima e de baixo, e uma
/// linha de texto. Dá **26px**.
///
/// Não é uma medida copiada de lugar nenhum — é a soma do que as constantes acima afirmam, e existe
/// pra essa soma aparecer uma vez só.
///
/// É `#[cfg(test)]` de propósito: o layout **não** declara altura nenhuma (a caixa é o que o conteúdo
/// medir, como no original), então este valor não dirige pixel nenhum — ele dirige as *afirmações*.
/// Deixá-lo fora do build da lib é o que impede que ele vire, um dia, um `min_h` que a referência não
/// tem.
#[cfg(test)]
const POPUP_HEIGHT: f32 = 2.0 * BORDER + 2.0 * PAD_Y + LINE_HEIGHT;

/// As duas camadas do `--shadow-md` do Tailwind v4, em `(dy, blur, spread)`.
///
/// O coss **não** redefine `--shadow-md` no `globals.css`, então vale o default:
/// `0 4px 6px -1px …, 0 2px 4px -2px …`. A COR não está aqui porque o `/5` do `shadow-md/5` a
/// substitui inteira por preto 5% — ela é o `shadow` da [`TooltipPalette`].
const SHADOW_LAYERS: [(f32, f32, f32); 2] = [(4.0, 6.0, -1.0), (2.0, 4.0, -2.0)];

/// Lado default — o `side="top"` da referência.
///
/// ⚠️ **Explícito de propósito.** [`MenuSide::default()`] é `Bottom`, que é o default do *menu*: um
/// menu cresce pra baixo, uma etiqueta sobe. Escrever `MenuSide::default()` aqui pegaria o default
/// errado sem nenhum sinal.
const DEFAULT_SIDE: MenuSide = MenuSide::Top;

/// Alinhamento default — o `align="center"` da referência. Coincide com [`MenuAlign::default()`], e é
/// escrito por extenso pelo mesmo motivo do [`DEFAULT_SIDE`].
const DEFAULT_ALIGN: MenuAlign = MenuAlign::Center;

/// Deslocamento no eixo de **alinhamento**. A referência não passa `alignOffset`, então é 0 — e não
/// há setter (ver "O que NÃO está aqui", no doc do módulo).
const ALIGN_OFFSET: f32 = 0.0;

/// Piso da largura máxima do popup — o mesmo raciocínio (e o mesmo número) do `MIN_MAX_HEIGHT` do
/// [`crate::menu`]: sem piso, um gatilho colado na borda da janela abriria uma etiqueta de largura
/// ~0, e uma etiqueta ilegível é pior que uma que estoura 2px da margem.
const MIN_MAX_WIDTH: f32 = 96.0;

/// Prioridade do [`gpui::deferred`] do popup.
///
/// **A mesma do [`crate::menu`] e do [`crate::select`]**, porque na referência é o mesmo `z-50`: o
/// `.tsx` da etiqueta e o do menu escrevem os dois `z-50` no `Positioner`. Empatados na prioridade,
/// quem pinta por cima é quem vem depois na árvore — que é exatamente a regra do `z-index` igual no
/// CSS.
const POPUP_PRIORITY: usize = 1;

/// A largura máxima do popup — o `max-w-(--available-width)` da referência.
///
/// Simétrica do `popup_max_height` do [`crate::menu`], no outro eixo: nos lados **verticais**
/// (`top`/`bottom`) o popup pode usar a janela toda menos as margens, porque ele desliza livremente
/// na horizontal; nos lados **horizontais** (`left`/`right`) o espaço é o do lado do gatilho, e vale o
/// MAIOR dos dois — o `snap_to_window_with_margin` desliza o popup pra dentro da janela quando ele não
/// cabe no lado pedido, e limitá-lo ao espaço do lado pedido daria uma etiqueta de 20px de largura num
/// gatilho colado na borda.
///
/// ⚠️ O teto só faz o texto **quebrar** por causa do `min_w(0)` do viewport; sozinho ele apararia a
/// caixa e deixaria o texto sair por fora dela. O motivo está no comentário daquele `min_w`, em
/// [`Tooltip::render_popup`].
///
/// `pub(crate)` porque o [`crate::popover::Popover`] tem o MESMO `max-w-(--available-width)` no
/// `Positioner` dele, e a conta é a mesma — do mesmo jeito que o `popup_max_height` do
/// [`crate::menu`] serve os dois. Reimplementá-la lá daria dois lugares pra corrigir quando um dos
/// dois eixos estivesse errado.
pub(crate) fn popup_max_width(side: MenuSide, anchor: Rect, viewport_width: f32, side_offset: f32) -> f32 {
    let livre = if side.is_vertical() {
        viewport_width - 2.0 * WINDOW_MARGIN
    } else {
        let direita = viewport_width - (anchor.x + anchor.w) - side_offset - WINDOW_MARGIN;
        let esquerda = anchor.x - side_offset - WINDOW_MARGIN;
        direita.max(esquerda)
    };
    livre.max(MIN_MAX_WIDTH)
}

// =================================================================================================
// O relógio (atraso de abertura, fechamento e fade)
// =================================================================================================
//
// O `Tooltip` é um `RenderOnce`: ele não sabe que o ponteiro acabou de entrar no gatilho, nem há 600ms
// que ele entrou. A informação mora numa tabela por `ElementId` — o mesmo desenho do feedback de
// interação do `crate::button`, e pelo mesmo motivo (a UI roda numa thread só).
//
// O motor de frames é o `Window::request_animation_frame`: enquanto houver relógio correndo, o render
// pede o próximo frame. Não há elemento de animação envolvido — todo o progresso vem do tempo
// decorrido, então não há `delta` pra sincronizar nem id de animação pra invalidar.

/// Atraso pra **abrir** — o `delay` default do `@base-ui/react/tooltip`, que a referência não
/// sobrescreve.
///
/// É o número que separa uma etiqueta de uma cortina: com 600ms o ponteiro atravessa uma barra de
/// ferramentas inteira sem acender nada no caminho.
const OPEN_DELAY: Duration = Duration::from_millis(600);

/// Atraso pra **fechar** — o `closeDelay` default do Base UI: **zero**. Sair do gatilho começa a
/// fechar no mesmo instante, sem carência.
///
/// Ele dirige código (ver [`popup_opacity`]) em vez de ser só documentação: é o simétrico do
/// [`OPEN_DELAY`], e trocá-lo por 200ms basta pra a etiqueta ganhar carência de saída.
const CLOSE_DELAY: Duration = Duration::ZERO;

/// Duração do fade de entrada e de saída.
///
/// ⚠️ **Valor deduzido.** A referência escreve `transition-[…,opacity]` sem classe de duração, então
/// vale o `--default-transition-duration` do Tailwind v4 = 150ms. Se o coss redefinir esse token, é
/// aqui que se conserta.
const FADE: Duration = Duration::from_millis(150);

/// Teto da tabela de estado. Estourar só faz as próximas etiquetas daquela sessão abrirem com o
/// relógio zerado — degrada, não quebra. Mesma escolha do `INTERACTION_TABLE_CAP` do
/// [`crate::button`].
const TABLE_CAP: usize = 512;

/// Qual das duas regiões do componente o ponteiro tocou.
///
/// As duas contam: o popup da referência é **hoverable** (o default do Base UI), então o ponteiro pode
/// sair do gatilho e entrar na etiqueta sem que ela feche. É por isso que são dois campos e não um.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Region {
    /// O gatilho — o que o call site passou pro `.child(..)`.
    Trigger,
    /// O popup — a etiqueta em si.
    Popup,
}

/// O estado de uma etiqueta entre frames.
#[derive(Clone, Copy, Debug)]
struct TooltipState {
    /// Bounds do gatilho, medidos por `canvas` no prepaint — o "anchor rect" do `Positioner`.
    anchor: Option<Bounds<Pixels>>,
    /// Bounds do popup, medidos por `canvas` no último frame em que ele foi pintado. **Não**
    /// participa do posicionamento (ver o aviso do [`crate::menu`] sobre posicionar pela medida); é
    /// observabilidade, exposta por [`measured_popup`].
    popup: Option<Bounds<Pixels>>,
    /// O ponteiro está sobre o gatilho.
    on_trigger: bool,
    /// O ponteiro está sobre o popup.
    on_popup: bool,
    /// Instante da última virada de [`Self::hovered`] — é o relógio dos 600ms e do fade.
    since: Instant,
    /// A opacidade **congelada** no instante em que o ponteiro saiu. Zero quando a etiqueta não
    /// estava visível. É o que faz o fade de saída partir de onde o de entrada estava, sem salto.
    leave_from: f32,
}

impl Default for TooltipState {
    fn default() -> Self {
        Self {
            anchor: None,
            popup: None,
            on_trigger: false,
            on_popup: false,
            since: Instant::now(),
            leave_from: 0.0,
        }
    }
}

impl TooltipState {
    /// Se o ponteiro está em ALGUMA das duas regiões — a pergunta que o relógio responde.
    fn hovered(&self) -> bool {
        self.on_trigger || self.on_popup
    }
}

thread_local! {
    /// O estado de cada etiqueta, por [`ElementId`]. Ver o comentário de seção acima.
    static TOOLTIPS: RefCell<HashMap<ElementId, TooltipState>> = RefCell::new(HashMap::new());
}

/// Lê o estado de uma etiqueta (o default, se ela nunca foi tocada).
fn state(id: &ElementId) -> TooltipState {
    TOOLTIPS.with(|t| t.borrow().get(id).copied().unwrap_or_default())
}

/// Muta o estado de uma etiqueta, criando-o se preciso.
fn update(id: &ElementId, f: impl FnOnce(&mut TooltipState)) {
    TOOLTIPS.with(|t| {
        let mut t = t.borrow_mut();
        if t.len() > TABLE_CAP {
            t.clear();
        }
        let mut st = t.get(id).copied().unwrap_or_default();
        f(&mut st);
        t.insert(id.clone(), st);
    });
}

/// Registra o retângulo do gatilho. Chamado do prepaint do `canvas` de medida, a cada frame.
fn record_anchor(id: &ElementId, bounds: Bounds<Pixels>) {
    update(id, |st| st.anchor = Some(bounds));
}

/// Registra o retângulo do popup. Chamado do prepaint do `canvas` de medida do popup, nos frames em
/// que ele existe.
fn record_popup(id: &ElementId, bounds: Bounds<Pixels>) {
    update(id, |st| st.popup = Some(bounds));
}

/// Registra entrada/saída do ponteiro numa das duas regiões, e mexe o relógio quando isso muda o
/// [`TooltipState::hovered`].
///
/// As duas direções são costuradas pela **opacidade corrente**:
///
/// - **saindo**, ela é congelada em [`TooltipState::leave_from`], e o fade de saída parte dela — sem
///   isso, sair no meio do fade de ENTRADA daria um salto de brilho antes de a etiqueta sumir;
/// - **voltando**, o relógio é REBOBINADO pro ponto que produz essa mesma opacidade (ver
///   [`resume_elapsed`]), ou seja quem reentra numa etiqueta ainda visível **não paga os 600ms de
///   novo**. É o que faz o ponteiro atravessar o vão de 4px entre gatilho e popup sem piscar.
fn record_hover(id: &ElementId, region: Region, hovered: bool) {
    update(id, |st| {
        let antes = st.hovered();
        match region {
            Region::Trigger => st.on_trigger = hovered,
            Region::Popup => st.on_popup = hovered,
        }
        let depois = st.hovered();
        if antes == depois {
            return;
        }
        let agora = popup_opacity(antes, st.since.elapsed(), st.leave_from).unwrap_or(0.0);
        let (leave_from, pago) = if depois {
            (0.0, resume_elapsed(agora))
        } else {
            (agora, Duration::ZERO)
        };
        st.leave_from = leave_from;
        // `checked_sub` porque `Instant` é monotônico desde o boot: subtrair 750ms nos primeiros
        // instantes de vida do processo pode estourar pra trás em algumas plataformas.
        st.since = Instant::now()
            .checked_sub(pago)
            .unwrap_or_else(Instant::now);
    });
}

/// Quanto do relógio de abertura já está **pago** quando o ponteiro volta a uma etiqueta que ainda
/// está visível com opacidade `atual`.
///
/// Zero quando ela não estava visível (`atual == 0`): aí o ponteiro paga os 600ms inteiros, que é o
/// caso normal de entrar num gatilho. Acima disso, o relógio é posto no ponto do fade de entrada que
/// produz exatamente aquela opacidade — o que faz a reentrada ser **contínua**, sem piscar nem saltar.
fn resume_elapsed(atual: f32) -> Duration {
    if atual <= 0.0 {
        Duration::ZERO
    } else {
        OPEN_DELAY + FADE.mul_f32(atual.min(1.0))
    }
}

/// A opacidade com que o popup deve ser pintado, ou `None` quando ele **não** deve existir.
///
/// É a máquina de estado inteira, como função pura de três valores — por isso ela é testável sem
/// janela, sem GPU e sem dormir 600ms:
///
/// - **com o ponteiro em cima**, o `checked_sub` é o atraso de abertura: antes de [`OPEN_DELAY`] não
///   há popup nenhum (`None`, e não "um popup transparente" — um popup invisível ainda teria hitbox e
///   sombra). Depois dele, o fade de entrada;
/// - **sem o ponteiro**, o fade de saída parte da opacidade congelada, depois de [`CLOSE_DELAY`]. O
///   `>= FADE` é o que **encerra** a animação: sem ele o popup ficaria pra sempre em `Some(0.0)`, e o
///   [`needs_frame`] pediria frames pra sempre.
fn popup_opacity(hovered: bool, since: Duration, leave_from: f32) -> Option<f32> {
    if hovered {
        let entrando = since.checked_sub(OPEN_DELAY)?;
        Some((entrando.as_secs_f32() / FADE.as_secs_f32()).min(1.0))
    } else {
        let saindo = since.checked_sub(CLOSE_DELAY)?;
        if leave_from <= 0.0 || saindo >= FADE {
            return None;
        }
        Some(leave_from * (1.0 - saindo.as_secs_f32() / FADE.as_secs_f32()))
    }
}

/// Se ainda há relógio correndo, e portanto o render tem que pedir o próximo frame.
///
/// Uma etiqueta parada aberta sob o ponteiro devolve `false`: ela não queima frame nenhum. Só o
/// atraso e os dois fades pedem.
fn needs_frame(hovered: bool, since: Duration, leave_from: f32) -> bool {
    match popup_opacity(hovered, since, leave_from) {
        // Escondido: só há frame a pedir se o relógio dos 600ms está correndo.
        None => hovered,
        // Visível: pede enquanto o fade não terminou — entrando (`o < 1`) ou saindo (`!hovered`).
        Some(o) => !hovered || o < 1.0,
    }
}

// =================================================================================================
// Peças visuais (funções livres, pra serem testáveis sem construir um `Tooltip`)
// =================================================================================================

/// A sombra externa do popup — `shadow-md/5`: as duas camadas do `shadow-md` do Tailwind v4 com a cor
/// trocada por preto a 5% (é o que o `/5` faz).
///
/// Sombra EXTERNA atrás de um fundo OPACO: aqui a armadilha do [`gpui::Window::paint_shadows`] (que
/// **não** recorta a sombra pra fora do elemento, como o CSS faz) não morde — o retângulo cheio que o
/// GPUI pinta fica escondido pelo `--popover`, que é opaco nos dois temas.
fn popup_shadow() -> Vec<BoxShadow> {
    let cor = palette().shadow.hsla();
    SHADOW_LAYERS
        .iter()
        .map(|(dy, blur, spread)| BoxShadow {
            color: cor,
            offset: point(px(0.0), px(*dy)),
            blur_radius: px(*blur),
            spread_radius: px(*spread),
        })
        .collect()
}

/// A **superfície** do popup: tudo que é aparência, e nada de posição.
///
/// É uma função livre, e não um trecho do [`Tooltip::render_popup`], por um motivo bem concreto: ela
/// devolve um [`Div`], e um `Div` pode ser **inspecionado** por [`gpui::Styled::style`] num teste de
/// unidade — sem janela, sem GPU. Assim cada token do contrato (raio, borda, fundo, cor de texto,
/// corpo, entrelinha, opacidade, teto de largura, as duas camadas de sombra) é conferido no elemento
/// que vai pra tela, e não só na constante de onde ele saiu. Ver
/// `a_superficie_carrega_o_contrato_do_coss`.
///
/// ⚠️ Ela **não** tem `overflow_hidden`: recortaria o bisel, que é filho absoluto sobre a borda.
///
/// ⚠️ E tem `.id()`, que não é decoração: sem ele o GPUI não guarda o estado de hover do elemento
/// entre frames — a borda de SAÍDA do ponteiro nunca chegaria, e a etiqueta hoverable não fecharia
/// mais.
fn popup_surface(id: &ElementId, text: SharedString, max_w: f32, opacity: f32) -> gpui::Stateful<Div> {
    let p = palette();
    div()
        .id(child_id(id, "popup"))
        .relative()
        .flex()
        // `flex_none` porque o popup é quase sempre MAIOR que o container que o centraliza (que tem a
        // largura do gatilho — ver `crate::menu::align_center_wrap`): é ele que garante que o espaço
        // livre negativo transborde pros dois lados em vez de o popup ser encolhido pro tamanho do
        // gatilho. Mesma razão do popup do [`crate::menu`].
        //
        // ⚠️ Honestidade: nos harness de teste desta lib a linha é INERTE — o mínimo automático do
        // `taffy` já segura o popup no tamanho do conteúdo, então tirá-la não muda medida nenhuma. Ela
        // fica por ser o mecanismo declarado no módulo irmão; se um dia o layout do popup mudar, é a
        // primeira linha a reexaminar.
        .flex_none()
        // O `max-w-(--available-width)`. Ele só APARA a caixa; quem faz o texto quebrar dentro dela é
        // o `min_w(0)` do viewport, logo abaixo.
        .max_w(px(max_w))
        .bg(p.popover_bg.hsla())
        .border(px(BORDER))
        .border_color(p.border.hsla())
        .rounded(px(RADIUS))
        .shadow(popup_shadow())
        .text_size(px(TEXT_SIZE))
        // Ver [`LINE_HEIGHT`]: é ela que faz a caixa fechar em 26px em vez de 29,4.
        .line_height(px(LINE_HEIGHT))
        .text_color(p.text.hsla())
        // O `Viewport` da referência, reduzido ao que sobrou dele sem o cross-fade de texto:
        // `px-(--viewport-inline-padding) py-1`.
        //
        // ⚠️ **O `min_w(0)` não é detalhe: é o que faz o `max_w` acima significar algo.**
        //
        // O min-content de um texto no GPUI é a FRASE INTEIRA numa linha — o
        // `request_measured_layout` do texto só quebra quando recebe largura DEFINIDA, e
        // `AvailableSpace::MinContent` cai no braço `None` (ver `gpui/src/elements/text.rs`). Sem
        // `min_w(0)`, o tamanho mínimo automático deste item de flex é essa frase inteira, o `taffy`
        // não pode encolhê-lo, e o `max_w` do popup só apara a CAIXA: o texto continua numa linha só e
        // sai por fora da borda. No navegador isso não acontece porque o min-content de um parágrafo é
        // a palavra mais longa — e é essa liberdade que o `min_w(0)` recria.
        //
        // Mesma armadilha, e mesma cura, do [`crate::card`]. Travado no teste de janela
        // `a_etiqueta_respeita_a_largura_disponivel`, que mede a etiqueta numa janela de 200px.
        .child(div().px(px(PAD_X)).py(px(PAD_Y)).min_w(px(0.0)).child(text))
        .child(bevel_overlay())
        // Popup hoverable: o ponteiro dentro da etiqueta a mantém aberta.
        .on_hover({
            let id = id.clone();
            move |hovered, window, _cx| {
                record_hover(&id, Region::Popup, *hovered);
                window.refresh();
            }
        })
        // A metade REPRODUZÍVEL da animação da referência (a de escala não existe no GPUI — ver o doc
        // do módulo). `opacity(1.0)` seria um passe de pintura a mais por nada.
        .when(opacity < 1.0, |d| d.opacity(opacity))
}

/// O **fio de bisel** de 1px sobre a borda do popup.
///
/// ⚠️ **A técnica do coss não traduz pro GPUI.** Lá é um pseudo-elemento transparente com
/// `box-shadow: 0 ±1px <cor>`: só o filete que ESCAPA da caixa fica visível, porque o CSS nunca pinta
/// a sombra por baixo da border box de quem a projeta. O [`gpui::Window::paint_shadows`] insere a
/// sombra como um retângulo arredondado **completo**, sem recortar a área do próprio elemento — num
/// overlay transparente isso viraria uma lavagem de cor sobre a etiqueta INTEIRA.
///
/// Então o filete é desenhado como o que ele é: uma **borda de 1px num único lado** de um overlay
/// absoluto. Duas consequências que já custaram defeito nesta base (ver o mesmo bisel no
/// [`crate::menu`] e no [`crate::button`]):
///
/// - o overlay cobre a **BORDER box** (daí o `inset: -1px` a partir da padding box), e por isso o raio
///   dele é o da SUPERFÍCIE — **não** `raio − 1`, que é o que o `.tsx` diz porque lá o pseudo-elemento
///   fica por DENTRO da borda;
/// - o **lado** sai do SINAL do deslocamento da sombra: `0 1px` desce, então o fio é na BASE
///   (`border_b`); `0 -1px` sobe, então é no TOPO.
fn bevel_overlay() -> Div {
    let p = palette();
    let overlay = div()
        .absolute()
        .top(px(-BORDER))
        .left(px(-BORDER))
        .right(px(-BORDER))
        .bottom(px(-BORDER))
        .rounded(px(RADIUS))
        .border_color(p.bevel.hsla());
    if p.bevel_dir > 0.0 {
        overlay.border_b_1()
    } else {
        overlay.border_t_1()
    }
}

// =================================================================================================
// Observabilidade
// =================================================================================================

/// Os bounds do **gatilho** medidos no último frame — o "anchor rect" do `Positioner`.
///
/// `None` antes do primeiro prepaint daquele `id`. Existe pelo mesmo motivo do `Menu::popup_bounds`:
/// quem embute uma etiqueta pode querer saber onde ela se ancorou, e é contra esta medida que os
/// testes de janela conferem a geometria declarada nas constantes.
pub fn measured_anchor(id: &ElementId) -> Option<Bounds<Pixels>> {
    state(id).anchor
}

/// Os bounds do **popup** medidos no último frame em que ele foi pintado (a BORDER box dele).
///
/// `None` enquanto a etiqueta nunca abriu. Note que ela **não** participa do posicionamento: ver o
/// aviso do [`crate::menu`] sobre posicionar pela medida.
pub fn measured_popup(id: &ElementId) -> Option<Bounds<Pixels>> {
    state(id).popup
}

// =================================================================================================
// O componente
// =================================================================================================

/// A etiqueta de ajuda do coss, embrulhando o gatilho. Ver o doc do módulo.
///
/// É um **elemento de render** (`RenderOnce`): construa a cada frame. O `id` tem que ser **estável
/// entre frames e único na janela** — é a chave da tabela de estado ([`TOOLTIPS`]) e a identidade que
/// o GPUI usa pro estado de hover do elemento. Dois gatilhos com o mesmo `id` compartilhariam relógio
/// e anchor rect.
#[derive(IntoElement)]
pub struct Tooltip {
    id: ElementId,
    text: SharedString,
    side: MenuSide,
    align: MenuAlign,
    side_offset: f32,
    children: Vec<AnyElement>,
}

impl Tooltip {
    /// Uma etiqueta com este texto, nos defaults da referência: acima do gatilho
    /// ([`DEFAULT_SIDE`]), centralizada ([`DEFAULT_ALIGN`]), a 4px ([`SIDE_OFFSET`]).
    ///
    /// O gatilho vem depois, com [`ParentElement::child`] — o `Tooltip` é um embrulho, então ele
    /// aceita filhos como qualquer container.
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            side: DEFAULT_SIDE,
            align: DEFAULT_ALIGN,
            side_offset: SIDE_OFFSET,
            children: Vec::new(),
        }
    }

    /// De que lado do gatilho a etiqueta abre (default [`MenuSide::Top`]).
    pub fn side(mut self, side: MenuSide) -> Self {
        self.side = side;
        self
    }

    /// Como a etiqueta se alinha ao gatilho no eixo transversal (default [`MenuAlign::Center`]).
    pub fn align(mut self, align: MenuAlign) -> Self {
        self.align = align;
        self
    }

    /// Distância entre o gatilho e a etiqueta, em px — o `sideOffset` do `Positioner` (default 4).
    pub fn side_offset(mut self, offset: f32) -> Self {
        self.side_offset = offset;
        self
    }

    /// O texto da etiqueta.
    pub fn text(&self) -> &SharedString {
        &self.text
    }

    /// O popup, em `deferred(anchored(..))` pra ficar por cima de tudo, escapar do recorte de quem
    /// estiver por fora (uma [`crate::ScrollArea`], por exemplo) e não estourar a janela.
    fn render_popup(&self, anchor: Rect, opacity: f32, window: &Window) -> AnyElement {
        let (x, y, corner) = anchor_point(anchor, self.side, self.align, self.side_offset, ALIGN_OFFSET);
        let max_w = popup_max_width(
            self.side,
            anchor,
            f32::from(window.viewport_size().width),
            self.side_offset,
        );

        let surface = popup_surface(&self.id, self.text.clone(), max_w, opacity);

        // O wrap existe pra MEDIR: ele não tem borda nem respiro, então a caixa dele coincide com a
        // BORDER box do popup — medir por dentro da superfície daria a padding box, 2px menor.
        let wrap = div()
            .relative()
            .flex()
            .flex_none()
            .child(surface)
            .child(
                canvas(
                    {
                        let id = self.id.clone();
                        move |bounds, window: &mut Window, _cx| {
                            record_popup(&id, bounds);
                            // O prepaint é a única fonte de verdade que NÃO envelhece: se este popup
                            // ficou fora da árvore com o ponteiro dentro dele (o call site sumiu com
                            // a etiqueta, por exemplo), o `on_hover` nunca entregaria a borda de
                            // saída e o `on_popup` ficaria preso em `true`. Aqui a pergunta é
                            // respondida do zero, contra a posição real do ponteiro.
                            record_hover(
                                &id,
                                Region::Popup,
                                bounds.contains(&window.mouse_position()),
                            );
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );

        // `align: center` não é um canto: sai por LAYOUT, no container compartilhado com o
        // `crate::menu` — ver `crate::menu::align_center_wrap`.
        let child: AnyElement = if self.align == MenuAlign::Center {
            align_center_wrap(self.side, anchor, wrap.into_any_element()).into_any_element()
        } else {
            wrap.into_any_element()
        };

        deferred(
            anchored()
                .position(point(px(x), px(y)))
                // O canto do POPUP que encosta no ponto — é ele que expressa `side: top` e
                // `align: start`/`end` sem precisar do tamanho do popup.
                .anchor(corner)
                // O `collisionPadding` do `Positioner`: gruda na janela em vez de trocar de lado.
                // `side` aqui é uma decisão do call site, não uma preferência — e trocar de lado
                // sozinho poria a etiqueta debaixo do ponteiro. Mesma escolha do `crate::menu`.
                .snap_to_window_with_margin(px(WINDOW_MARGIN))
                .child(child),
        )
        .with_priority(POPUP_PRIORITY)
        .into_any_element()
    }
}

impl ParentElement for Tooltip {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

/// Um [`ElementId`] derivado do id da etiqueta, pros filhos que precisam de identidade própria.
fn child_id(id: &ElementId, nome: &'static str) -> ElementId {
    ElementId::NamedChild(Box::new(id.clone()), nome.into())
}

impl RenderOnce for Tooltip {
    fn render(mut self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let st = state(&self.id);
        let decorrido = st.since.elapsed();
        let hovered = st.hovered();
        let opacidade = popup_opacity(hovered, decorrido, st.leave_from);

        let popup = opacidade.map(|o| {
            self.render_popup(rect_of(st.anchor.unwrap_or_default()), o, window)
        });

        // Enquanto houver relógio correndo, pede o próximo frame. É o motor do atraso E dos dois
        // fades — não há elemento de animação, o progresso vem do tempo decorrido.
        if needs_frame(hovered, decorrido, st.leave_from) {
            window.request_animation_frame();
        }

        let filhos = std::mem::take(&mut self.children);

        // O wrap do gatilho — a peça que é MEDIDA, e portanto a que tem que ter exatamente o tamanho
        // do gatilho.
        //
        // ⚠️ **Por que ele não é a raiz.** O `display` default do GPUI é `Block`
        // (`gpui/src/style.rs`), então um elemento solto dentro de um container qualquer preenche a
        // largura dele — e `flex_none` não muda isso, porque `flex-*` só vale DENTRO de um pai flex.
        // Medido assim, o anchor rect saía com a largura da linha inteira (1720px num harness de
        // 1920) e o `align: center` centralizava a etiqueta no meio da JANELA, não no botão. A raiz
        // abaixo é um flex row com `items_start`, e é ela que faz este wrap fechar no conteúdo nos
        // dois eixos. Mesmo arranjo (e mesmo motivo) do wrap de gatilho do [`crate::menu`].
        let gatilho = div()
            // `.id()` não é decoração: é o que faz o GPUI guardar o estado de hover deste elemento
            // entre frames. Sem ele, `was_hovered` renasce `false` a cada frame e o `on_hover` nunca
            // entrega a borda de SAÍDA do ponteiro — a etiqueta abriria e não fecharia mais.
            .id(child_id(&self.id, "trigger"))
            // `relative` porque o canvas de medida é filho ABSOLUTO.
            //
            // Sem `flex_none`, ao contrário do wrap de gatilho do [`crate::menu`]: `flex-grow` já é 0
            // por default (então o wrap fecha no conteúdo, que é o que importa pra o anchor rect), e
            // fixar `flex-shrink: 0` aqui faria a etiqueta MUDAR o layout de quem a usa — numa linha
            // apertada os irmãos encolheriam e o gatilho embrulhado não, transbordando a caixa que o
            // call site lhe deu. Um embrulho deve ser transparente ao layout.
            .relative()
            .flex()
            .children(filhos)
            .child(
                canvas(
                    {
                        let id = self.id.clone();
                        move |bounds, _window, _cx| record_anchor(&id, bounds)
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .on_hover({
                let id = self.id.clone();
                move |hovered, window, _cx| {
                    record_hover(&id, Region::Trigger, *hovered);
                    window.refresh();
                }
            });

        div()
            .flex()
            // `items_start` pra o wrap do gatilho ter a ALTURA dele: o `stretch` default do flex o
            // esticaria, e o anchor rect sairia alto (o que empurraria a etiqueta pra cima do que
            // devia). Mesma linha, e mesmo motivo, da raiz do [`crate::menu`].
            .items_start()
            .child(gatilho)
            .children(popup)
    }
}

/// Recua o relógio de uma etiqueta como se `d` já tivesse passado.
///
/// É como os testes atravessam os 600ms sem dormir 600ms — e é o mesmo caminho que os testes de
/// interação do [`crate::button`] usam pra atravessar o fade deles. Compartilhado pelos dois módulos
/// de teste porque a operação é uma só.
#[cfg(test)]
fn recuar_relogio(id: &ElementId, d: Duration) {
    update(id, |st| {
        st.since = Instant::now().checked_sub(d).unwrap_or_else(Instant::now);
    });
}

#[cfg(test)]
// O clippy trata `assert!` sobre constante como erro provável, presumindo que quem escreveu quis
// testar algo variável. Aqui é o contrário: travar o valor que veio da referência É o propósito
// destes testes. Mesma decisão em todos os módulos portados do coss (ver `menu.rs`).
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    /// O `--radius` do coss (`0.625rem`), de onde saem todos os `--radius-*`.
    const COSS_RADIUS: f32 = 10.0;

    /// A entrelinha que o GPUI usaria sem [`LINE_HEIGHT`]: `phi()` = `relative(1.618_034)`, em
    /// `gpui/src/geometry.rs`.
    const PHI: f32 = 1.618_034;

    // ---------------------------------------------------------------------------------------------
    // Geometria e tokens, travados contra a referência
    // ---------------------------------------------------------------------------------------------

    /// **A geometria é a do `tooltip.tsx`.** Cada número aqui é a tradução de uma classe, e a conta
    /// que o justifica está do lado — não são medidas de captura de tela.
    #[test]
    fn a_geometria_e_a_do_coss() {
        // `rounded-md` = calc(--radius - 2px). E é MENOS que o `rounded-lg` do popup do menu.
        assert_eq!(RADIUS, COSS_RADIUS - 2.0, "rounded-md = --radius - 2");
        assert_eq!(RADIUS, 8.0);
        assert_eq!(BORDER, 1.0, "border");
        // `--viewport-inline-padding: --spacing(2)` = 2 × 4px, e `py-1` = 1 × 4px.
        assert_eq!(PAD_X, 2.0 * 4.0, "px-(--viewport-inline-padding)");
        assert_eq!(PAD_Y, 1.0 * 4.0, "py-1");
        assert_eq!(TEXT_SIZE, 12.0, "text-xs");
        // O `sideOffset={4}` da referência é o SIDE_OFFSET compartilhado com o menu.
        assert_eq!(SIDE_OFFSET, 4.0, "o sideOffset da referência é 4");
        // O `alignOffset` não existe na referência.
        assert_eq!(ALIGN_OFFSET, 0.0);
    }

    /// **A entrelinha do `text-xs` é 16, e não a razão de ouro** — a armadilha mais silenciosa deste
    /// porte.
    ///
    /// O par do Tailwind é `12px/16px`; o default do GPUI é `phi()` (1,618034 × o corpo). O teste
    /// mostra os DOIS números, porque o valor errado não parece errado em lugar nenhum do código: só
    /// aparece como uma etiqueta 13% mais alta que a do coss.
    #[test]
    fn a_entrelinha_e_16_e_nao_a_razao_de_ouro() {
        assert_eq!(LINE_HEIGHT, 16.0, "o par do `text-xs` no Tailwind é 12/16");

        // O que o GPUI faria sozinho:
        let sem_fixar = TEXT_SIZE * PHI;
        assert!(
            (sem_fixar - 19.416_408).abs() < 1e-3,
            "a linha default do GPUI a 12px é 19,42; veio {sem_fixar}"
        );
        assert!(
            sem_fixar > LINE_HEIGHT,
            "o default do GPUI é MAIOR — o erro infla, nunca aperta"
        );
        assert!(
            (sem_fixar / LINE_HEIGHT - 1.213_5).abs() < 1e-3,
            "a linha sairia 21% mais alta"
        );

        // E o que isso faz com a CAIXA, que é o que se vê:
        let caixa_sem_fixar = 2.0 * BORDER + 2.0 * PAD_Y + sem_fixar;
        assert!(
            (caixa_sem_fixar - 29.416_408).abs() < 1e-3,
            "a caixa sairia com 29,42px; veio {caixa_sem_fixar}"
        );
        assert!(
            (caixa_sem_fixar / POPUP_HEIGHT - 1.131_4).abs() < 1e-3,
            "ou seja 13% mais alta que os 26px do coss"
        );
    }

    /// **A altura do popup é a soma das partes, e dá 26px.** É o número que uma captura de tela do
    /// coss mede, e é o que o teste de janela confere contra a caixa MEDIDA.
    #[test]
    fn a_altura_do_popup_e_a_soma_das_partes() {
        assert_eq!(POPUP_HEIGHT, 2.0 * BORDER + 2.0 * PAD_Y + LINE_HEIGHT);
        assert_eq!(POPUP_HEIGHT, 26.0, "1+4+16+4+1");
    }

    /// **A convenção de cor da paleta, decodificada de verdade.**
    ///
    /// É o teste que pega a confusão de convenção: todo valor de [`TooltipPalette`] é `0xRRGGBBAA` e é
    /// consumido por `rgba`. Um valor de 6 dígitos esquecido ali vira uma cor completamente diferente
    /// **sem erro de compilação** (`rgba(0xffffff)` é lido como ciano). Isso já custou três bugs
    /// visíveis nesta base — então, em vez de comparar números com números (que não pegaria nada), o
    /// teste decodifica e afirma o que a cor DEVE ser perceptualmente.
    #[test]
    fn as_cores_decodificam_com_os_canais_no_lugar() {
        for (nome, p) in [("claro", &TOOLTIP_LIGHT), ("escuro", &TOOLTIP_DARK)] {
            let fundo: gpui::Rgba = p.popover_bg.hsla().into();
            assert_eq!(p.popover_bg.alpha(), 1.0, "{nome}: --popover é OPACO");
            assert!(
                (fundo.r - fundo.g).abs() < 0.01 && (fundo.g - fundo.b).abs() < 0.01,
                "{nome}: --popover é neutro"
            );
            let texto: gpui::Rgba = p.text.hsla().into();
            assert_eq!(p.text.alpha(), 1.0, "{nome}: o texto é opaco");
            assert!(
                (texto.r - texto.g).abs() < 0.01 && (texto.g - texto.b).abs() < 0.01,
                "{nome}: --popover-foreground é neutro"
            );
            // Contraste: o fundo e o texto têm que estar em pontas opostas da escala.
            assert!(
                (fundo.r - texto.r).abs() > 0.5,
                "{nome}: texto e fundo em pontas opostas"
            );
            // Borda e bisel são FIOS: translúcidos, senão viram traço em vez de nuance.
            assert!(p.border.alpha() < 0.15, "{nome}: --border é translúcida");
            assert!(p.bevel.alpha() < 0.15, "{nome}: o bisel é translúcido");
            assert!(p.shadow.alpha() < 0.1, "{nome}: a sombra é translúcida");
        }

        // O claro é claro e o escuro é escuro — se as duas paletas fossem trocadas, tudo acima
        // continuaria passando.
        let claro: gpui::Rgba = TOOLTIP_LIGHT.popover_bg.hsla().into();
        let escuro: gpui::Rgba = TOOLTIP_DARK.popover_bg.hsla().into();
        assert!(claro.r > 0.9, "--popover claro é branco");
        assert!(escuro.r < 0.2, "--popover escuro é quase preto");
    }

    /// **Os tokens são os do `globals.css`, resolvidos.** Confere os valores exatos contra a tabela do
    /// coss — inclusive o `--popover` escuro, que é uma MISTURA (`mix(--background 96%, white)`) e não
    /// um token literal.
    #[test]
    fn os_tokens_saem_da_tabela_do_coss() {
        assert_eq!(TOOLTIP_LIGHT.popover_bg, Rgba8(0xffffffff), "--popover: white");
        assert_eq!(
            TOOLTIP_LIGHT.text,
            Rgba8(0x262626ff),
            "--popover-foreground: neutral-800"
        );
        assert_eq!(
            TOOLTIP_DARK.text,
            Rgba8(0xf5f5f5ff),
            "--popover-foreground: neutral-100"
        );

        // `mix(in srgb, #141414 96%, white)` = 0x14 · 0,96 + 0xff · 0,04 = 29,4 -> 0x1d.
        let misturado = (0x14 as f32 * 0.96 + 255.0 * 0.04).round() as u32;
        assert_eq!(misturado, 0x1d, "a mistura do --popover escuro dá #1d");
        assert_eq!(TOOLTIP_DARK.popover_bg, Rgba8(0x1d1d1dff));

        // `--border`: preto 8% no claro (0,08 · 255 = 20,4 -> 20 = 0x14), branco 6% no escuro
        // (0,06 · 255 = 15,3 -> 15 = 0x0f).
        assert_eq!(TOOLTIP_LIGHT.border, Rgba8(0x00000014));
        assert_eq!(TOOLTIP_DARK.border, Rgba8(0xffffff0f));

        // A borda do popup é `--border`, e é IGUAL à do menu: os dois popups aparecem lado a lado.
        assert!(
            (TOOLTIP_LIGHT.border.alpha() - 20.0 / 255.0).abs() < 1e-4,
            "preto 8%"
        );
        assert!(
            (TOOLTIP_DARK.border.alpha() - 15.0 / 255.0).abs() < 1e-4,
            "branco 6%"
        );
    }

    /// **A sombra é o `shadow-md` do Tailwind v4 com alfa de 5%.**
    ///
    /// O coss não redefine `--shadow-md`, então as duas camadas são as do default; o `/5` troca a cor
    /// das duas por preto 5%. Trocar o `shadow-md` por `shadow-lg` (as camadas do menu) muda estes
    /// números — e a etiqueta passaria a flutuar mais alto que o próprio menu.
    #[test]
    fn a_sombra_e_o_shadow_md_com_alfa_de_cinco_por_cento() {
        assert_eq!(
            SHADOW_LAYERS,
            [(4.0, 6.0, -1.0), (2.0, 4.0, -2.0)],
            "0 4px 6px -1px, 0 2px 4px -2px"
        );
        // Preto 5%: 0,05 · 255 = 12,75 -> 13 = 0x0d.
        for (nome, p) in [("claro", &TOOLTIP_LIGHT), ("escuro", &TOOLTIP_DARK)] {
            assert!(
                (p.shadow.alpha() - 13.0 / 255.0).abs() < 1e-4,
                "{nome}: o `/5` do shadow-md/5 é preto 5%"
            );
        }

        // E a sombra construída tem as duas camadas, na ordem, com o dy certo.
        theme::set_theme(theme::ThemeMode::Dark);
        let camadas = popup_shadow();
        assert_eq!(camadas.len(), 2);
        for (camada, (dy, blur, spread)) in camadas.iter().zip(SHADOW_LAYERS) {
            assert_eq!(f32::from(camada.offset.y), dy);
            assert_eq!(f32::from(camada.offset.x), 0.0, "só desce, não desloca de lado");
            assert_eq!(f32::from(camada.blur_radius), blur);
            assert_eq!(f32::from(camada.spread_radius), spread);
        }
    }

    /// **O bisel escuro é o DOBRO da referência** — desvio consciente, e o mesmo dos outros quatro
    /// módulos da casa (ver o comentário em [`TOOLTIP_DARK`]).
    #[test]
    fn bisel_escuro_e_o_dobro_da_referencia() {
        // Alfa de referência em 8 bits: 4% -> 10 (claro), 6% -> 15 (escuro).
        const REF_CLARO: f32 = 10.0 / 255.0;
        const REF_ESCURO: f32 = 15.0 / 255.0;

        let claro = TOOLTIP_LIGHT.bevel.alpha();
        assert!(
            (claro - REF_CLARO).abs() < 1e-4,
            "o tema claro tem que seguir fiel à referência (preto 4%); veio {claro}"
        );
        let escuro = TOOLTIP_DARK.bevel.alpha();
        assert!(
            (escuro - REF_ESCURO * 2.0).abs() < 1e-4,
            "o tema escuro é o DOBRO da referência; veio {escuro}"
        );
        assert!(escuro < 1.0, "ainda translúcido");
    }

    /// **O fio de bisel troca de LADO com o tema.**
    ///
    /// O sinal sai do deslocamento da sombra da referência: `before:shadow-[0_1px_black/4%]` desce, e
    /// o filete fica na BASE; `dark:before:shadow-[0_-1px_white/6%]` sobe, e ele vai pro TOPO. É a
    /// diferença entre uma superfície iluminada de cima (certo) e uma iluminada de baixo (errado, e o
    /// olho percebe sem saber explicar).
    #[test]
    fn o_bisel_troca_de_lado_por_tema() {
        assert!(
            TOOLTIP_LIGHT.bevel_dir > 0.0,
            "claro: `0 1px` desce, filete na BASE"
        );
        assert!(
            TOOLTIP_DARK.bevel_dir < 0.0,
            "escuro: `0 -1px` sobe, filete no TOPO"
        );
        // E o sentido é o mesmo do menu e do botão — as três superfícies são iluminadas pela mesma
        // luz. Se um dia divergirem, é aqui que o teste avisa.
        assert_ne!(
            TOOLTIP_LIGHT.bevel_dir.signum(),
            TOOLTIP_DARK.bevel_dir.signum(),
            "os dois temas têm que apontar pra lados OPOSTOS"
        );
    }

    /// **A superfície que vai pra tela carrega o contrato do coss, token por token.**
    ///
    /// Os testes acima trancam as CONSTANTES; este tranca o elemento. É a diferença entre "o número
    /// certo está declarado" e "o número certo chega no `Div`" — um `.rounded(px(RADIUS_SM))` ou um
    /// `.bg(p.border.hsla())` trocado não mexe em constante nenhuma e passaria em todos os outros.
    ///
    /// A inspeção é o [`gpui::Styled::style`], que expõe o `StyleRefinement` do elemento: dá pra
    /// afirmar o pixel sem abrir janela.
    #[test]
    fn a_superficie_carrega_o_contrato_do_coss() {
        use gpui::{AbsoluteLength, DefiniteLength, Length};

        const TETO: f32 = 184.0;
        let px_len = |v: f32| Some(AbsoluteLength::Pixels(px(v)));

        for (modo, p) in [
            (theme::ThemeMode::Light, &TOOLTIP_LIGHT),
            (theme::ThemeMode::Dark, &TOOLTIP_DARK),
        ] {
            theme::set_theme(modo);
            let mut surface = popup_surface(&ElementId::from("tt-superficie"), "Salvar".into(), TETO, 0.5);
            let s = surface.style();

            // --- Forma ---------------------------------------------------------------------------
            let raios = s.corner_radii.clone();
            assert_eq!(raios.top_left, px_len(RADIUS), "{modo:?}: rounded-md = 8");
            assert_eq!(raios.bottom_right, px_len(RADIUS), "{modo:?}: nos quatro cantos");
            let bordas = s.border_widths.clone();
            assert_eq!(bordas.top, px_len(BORDER), "{modo:?}: border de 1px");
            assert_eq!(bordas.left, px_len(BORDER), "{modo:?}: nos quatro lados");
            assert_eq!(
                s.max_size.width,
                Some(Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(TETO))))),
                "{modo:?}: max-w-(--available-width)"
            );

            // --- Cor -----------------------------------------------------------------------------
            assert_eq!(
                s.background,
                Some(p.popover_bg.hsla().into()),
                "{modo:?}: bg-popover"
            );
            assert_eq!(s.border_color, Some(p.border.hsla()), "{modo:?}: a borda é --border");
            let texto = s.text.clone().expect("a superfície declara estilo de texto");
            assert_eq!(
                texto.color,
                Some(p.text.hsla()),
                "{modo:?}: text-popover-foreground"
            );

            // --- Texto: 12/16, o par do Tailwind -------------------------------------------------
            assert_eq!(texto.font_size, px_len(TEXT_SIZE), "{modo:?}: text-xs = 12");
            assert_eq!(
                texto.line_height,
                Some(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(LINE_HEIGHT)))),
                "{modo:?}: a entrelinha é DECLARADA em 16 — sem isso o GPUI usa a razão de ouro"
            );

            // --- Sombra e fade -------------------------------------------------------------------
            let sombras = s.box_shadow.clone().expect("shadow-md/5");
            assert_eq!(sombras.len(), SHADOW_LAYERS.len(), "{modo:?}: as duas camadas");
            assert_eq!(sombras[0].color, p.shadow.hsla(), "{modo:?}: preto 5%");
            assert_eq!(s.opacity, Some(0.5), "{modo:?}: o fade é PINTADO");

            // O respiro NÃO está aqui: ele é do viewport de dentro, e é medido no teste de janela
            // (a altura de 26px é `1 + PAD_Y + LINE_HEIGHT + PAD_Y + 1`).
            assert_eq!(s.padding.left, None, "{modo:?}: o respiro é do viewport, não da superfície");
        }

        // Opacidade cheia não gasta passe de pintura.
        let mut cheia = popup_surface(&ElementId::from("tt-superficie"), "Salvar".into(), TETO, 1.0);
        assert_eq!(
            cheia.style().opacity,
            None,
            "com o fade concluído não há por que pedir um passe de opacidade"
        );
    }

    /// **O fio de bisel é DESENHADO no lado do sinal** — não só declarado nele.
    ///
    /// O teste acima tranca o `bevel_dir`; este tranca a tradução dele em borda, que é onde o erro
    /// custa pixel: trocar o `border_b_1()` pelo `border_t_1()` no [`bevel_overlay`] não mexe em
    /// constante nenhuma e põe o filete na aresta errada nos DOIS temas de uma vez.
    ///
    /// De passagem, tranca também o **raio** do overlay: ele cobre a BORDER box (`inset: -1px`), então
    /// o raio dele é o da superfície — e **não** o `calc(--radius-md - 1px)` = 7 que o `.tsx` escreve
    /// (lá o pseudo-elemento fica por dentro da borda). Ver o doc de [`bevel_overlay`].
    #[test]
    fn o_fio_de_bisel_e_desenhado_no_lado_do_sinal() {
        for (modo, na_base) in [
            (theme::ThemeMode::Light, true),
            (theme::ThemeMode::Dark, false),
        ] {
            theme::set_theme(modo);
            let mut overlay = bevel_overlay();
            let estilo = overlay.style();
            let bordas = estilo.border_widths.clone();
            let raios = estilo.corner_radii.clone();

            if na_base {
                assert!(bordas.bottom.is_some(), "{modo:?}: `0 1px` desce — filete na BASE");
                assert!(bordas.top.is_none(), "{modo:?}: e nada no topo");
            } else {
                assert!(bordas.top.is_some(), "{modo:?}: `0 -1px` sobe — filete no TOPO");
                assert!(bordas.bottom.is_none(), "{modo:?}: e nada na base");
            }
            // Nunca nos lados: o filete é horizontal, não um contorno.
            assert!(bordas.left.is_none() && bordas.right.is_none(), "{modo:?}: só um lado");

            let esperado = gpui::AbsoluteLength::Pixels(px(RADIUS));
            for canto in [raios.top_left, raios.top_right, raios.bottom_left, raios.bottom_right] {
                assert_eq!(
                    canto,
                    Some(esperado),
                    "{modo:?}: o raio do overlay é o da SUPERFÍCIE ({RADIUS}), não {} do .tsx",
                    RADIUS - 1.0
                );
            }
        }
    }

    /// **A paleta segue o tema corrente.** Um `palette()` que devolvesse sempre a mesma passaria em
    /// todos os testes de cor acima.
    #[test]
    fn a_paleta_segue_o_tema() {
        theme::set_theme(theme::ThemeMode::Light);
        assert_eq!(palette().popover_bg, TOOLTIP_LIGHT.popover_bg);
        theme::set_theme(theme::ThemeMode::Dark);
        assert_eq!(palette().popover_bg, TOOLTIP_DARK.popover_bg);
    }

    // ---------------------------------------------------------------------------------------------
    // Posição (a aritmética compartilhada com o `crate::menu`, exercitada pelos defaults daqui)
    // ---------------------------------------------------------------------------------------------

    /// Um gatilho de teste longe da origem: uma conta que ignorasse a posição do gatilho passaria num
    /// retângulo em (0,0).
    const GATILHO: Rect = Rect {
        x: 200.0,
        y: 100.0,
        w: 40.0,
        h: 24.0,
    };

    /// **O default é ACIMA do gatilho, centralizado, a 4px.** É o `side="top" align="center"
    /// sideOffset={4}` da referência — e não o default do menu, que é `Bottom`.
    #[test]
    fn o_default_e_acima_do_gatilho_centralizado_a_quatro_px() {
        assert_eq!(DEFAULT_SIDE, MenuSide::Top, "a referência diz side=\"top\"");
        assert_eq!(DEFAULT_ALIGN, MenuAlign::Center, "align=\"center\"");
        assert_ne!(
            DEFAULT_SIDE,
            MenuSide::default(),
            "o default do MENU é Bottom — a etiqueta NÃO pode herdar esse"
        );

        let tt = Tooltip::new("t", "Salvar");
        assert_eq!(tt.side, DEFAULT_SIDE);
        assert_eq!(tt.align, DEFAULT_ALIGN);
        assert_eq!(tt.side_offset, SIDE_OFFSET);
        assert_eq!(tt.text().as_ref(), "Salvar");

        // E a conta: `side: top` põe a BASE do popup 4px acima do topo do gatilho. `align: center`
        // devolve o mesmo ponto/canto do `start` (a centralização é layout, não conta).
        let (x, y, corner) = anchor_point(GATILHO, DEFAULT_SIDE, DEFAULT_ALIGN, SIDE_OFFSET, ALIGN_OFFSET);
        assert_eq!(x, GATILHO.x);
        assert_eq!(y, GATILHO.y - 4.0, "4px ACIMA do topo do gatilho");
        assert_eq!(corner, gpui::Corner::BottomLeft, "é a BASE do popup que encosta");
    }

    /// **Os quatro lados saem da aritmética compartilhada.** Não é reteste do `crate::menu`: é a
    /// garantia de que este módulo chama aquela função com os argumentos na ordem certa — trocar
    /// `side` com `align` na chamada compila e põe a etiqueta do lado errado.
    #[test]
    fn os_quatro_lados_saem_do_anchor_point_compartilhado() {
        let offset = 4.0;
        let casos = [
            (MenuSide::Top, GATILHO.x, GATILHO.y - offset, gpui::Corner::BottomLeft),
            (
                MenuSide::Bottom,
                GATILHO.x,
                GATILHO.y + GATILHO.h + offset,
                gpui::Corner::TopLeft,
            ),
            (
                MenuSide::Right,
                GATILHO.x + GATILHO.w + offset,
                GATILHO.y,
                gpui::Corner::TopLeft,
            ),
            (MenuSide::Left, GATILHO.x - offset, GATILHO.y, gpui::Corner::TopRight),
        ];
        for (side, ex, ey, ecorner) in casos {
            let (x, y, corner) = anchor_point(GATILHO, side, MenuAlign::Center, offset, ALIGN_OFFSET);
            assert_eq!((x, y), (ex, ey), "{side:?}: ponto de ancoragem");
            assert_eq!(corner, ecorner, "{side:?}: canto do popup");
        }

        // `align: end` troca a ponta da aresta E o canto — o que prova que o `align` chega inteiro.
        let (x, _y, corner) = anchor_point(GATILHO, MenuSide::Top, MenuAlign::End, offset, ALIGN_OFFSET);
        assert_eq!(x, GATILHO.x + GATILHO.w, "align end: parte da aresta DIREITA");
        assert_eq!(corner, gpui::Corner::BottomRight);
    }

    /// **O `sideOffset` é o vão, e ele é configurável.** Um `side_offset` ignorado colaria a etiqueta
    /// no gatilho, e nada no visual denunciaria a causa.
    #[test]
    fn o_side_offset_e_o_vao_entre_gatilho_e_etiqueta() {
        let (_x, y4, _c) = anchor_point(GATILHO, MenuSide::Top, MenuAlign::Center, 4.0, ALIGN_OFFSET);
        let (_x, y12, _c) = anchor_point(GATILHO, MenuSide::Top, MenuAlign::Center, 12.0, ALIGN_OFFSET);
        assert_eq!(y4 - y12, 8.0, "8px a mais de offset sobem a etiqueta 8px");
        assert_eq!(Tooltip::new("t", "x").side_offset(12.0).side_offset, 12.0);
    }

    /// **A largura máxima é o espaço DISPONÍVEL** — o `max-w-(--available-width)` da referência.
    ///
    /// Nos lados verticais é a janela toda menos as margens (o popup desliza livre na horizontal); nos
    /// horizontais é o maior dos dois lados do gatilho, com um piso pra uma etiqueta colada na borda
    /// não nascer ilegível.
    #[test]
    fn a_largura_maxima_e_o_espaco_disponivel() {
        const JANELA: f32 = 1000.0;

        // Vertical: janela menos as duas margens, independente de onde está o gatilho.
        for side in [MenuSide::Top, MenuSide::Bottom] {
            let w = popup_max_width(side, GATILHO, JANELA, 4.0);
            assert_eq!(w, JANELA - 2.0 * WINDOW_MARGIN, "{side:?}: a janela menos as margens");
        }

        // Horizontal: o gatilho em x=200 tem 192px à esquerda e 748 à direita (com offset 4 e margem
        // 8) — vale o MAIOR.
        let direita = JANELA - (GATILHO.x + GATILHO.w) - 4.0 - WINDOW_MARGIN;
        let esquerda = GATILHO.x - 4.0 - WINDOW_MARGIN;
        assert_eq!((esquerda, direita), (188.0, 748.0));
        for side in [MenuSide::Left, MenuSide::Right] {
            assert_eq!(
                popup_max_width(side, GATILHO, JANELA, 4.0),
                direita,
                "{side:?}: o maior dos dois lados"
            );
        }

        // O piso: um gatilho colado na borda direita não deixa espaço nenhum de nenhum lado.
        let colado = Rect {
            x: 4.0,
            y: 100.0,
            w: 8.0,
            h: 24.0,
        };
        assert_eq!(
            popup_max_width(MenuSide::Right, colado, 24.0, 4.0),
            MIN_MAX_WIDTH,
            "o piso segura uma etiqueta encurralada"
        );
        assert_eq!(MIN_MAX_WIDTH, 96.0);
    }

    // ---------------------------------------------------------------------------------------------
    // O relógio
    // ---------------------------------------------------------------------------------------------

    /// **A etiqueta só existe depois de 600ms de ponteiro em cima.**
    ///
    /// Antes disso a função devolve `None`, e não `Some(0.0)`: um popup transparente ainda teria
    /// hitbox e sombra, e roubaria o hover de quem estivesse embaixo.
    #[test]
    fn o_popup_so_aparece_depois_de_seiscentos_ms() {
        assert_eq!(OPEN_DELAY, Duration::from_millis(600), "o delay default do Base UI");
        // ⚠️ Valor **deduzido**: o `--default-transition-duration` do Tailwind v4, porque a referência
        // escreve `transition-[…,opacity]` sem classe de duração. É o único número deste módulo que
        // não saiu do `.tsx` nem da tabela de tokens, e é esta linha que avisa se ele mudar.
        assert_eq!(FADE, Duration::from_millis(150), "a duração default de transição do Tailwind");

        assert_eq!(popup_opacity(true, Duration::ZERO, 0.0), None, "no instante 0");
        assert_eq!(
            popup_opacity(true, Duration::from_millis(599), 0.0),
            None,
            "1ms antes ainda não"
        );
        let no_limite = popup_opacity(true, OPEN_DELAY, 0.0).expect("aos 600ms já existe");
        assert_eq!(no_limite, 0.0, "e nasce transparente — é o `data-starting-style`");

        // O fade de entrada: metade do caminho na metade do tempo, cheio no fim, e nunca mais que 1.
        let meio = popup_opacity(true, OPEN_DELAY + FADE / 2, 0.0).expect("visível");
        assert!((meio - 0.5).abs() < 1e-3, "metade do fade; veio {meio}");
        assert_eq!(popup_opacity(true, OPEN_DELAY + FADE, 0.0), Some(1.0));
        assert_eq!(
            popup_opacity(true, OPEN_DELAY + FADE * 100, 0.0),
            Some(1.0),
            "parada, ela fica cheia — o fade não continua pra além de 1"
        );
    }

    /// **Fechar é imediato.** O `closeDelay` default do Base UI é zero: sair do gatilho começa a
    /// fechar no mesmo instante, e o que sobra é só o fade de saída.
    #[test]
    fn o_fechamento_e_imediato() {
        assert_eq!(CLOSE_DELAY, Duration::ZERO, "closeDelay = 0");

        // No instante da saída, a etiqueta está ainda com a opacidade que tinha.
        assert_eq!(popup_opacity(false, Duration::ZERO, 1.0), Some(1.0));
        let meio = popup_opacity(false, FADE / 2, 1.0).expect("ainda saindo");
        assert!((meio - 0.5).abs() < 1e-3, "metade do fade de saída; veio {meio}");
        // No fim do fade ela DEIXA DE EXISTIR — e não fica em `Some(0.0)`, que faria o
        // `needs_frame` pedir frames pra sempre.
        assert_eq!(popup_opacity(false, FADE, 1.0), None, "no fim do fade, sumiu");
        assert_eq!(popup_opacity(false, FADE * 10, 1.0), None);

        // Sem ponteiro e sem nada pra desvanecer (nunca abriu), não há popup em instante nenhum.
        assert_eq!(popup_opacity(false, Duration::ZERO, 0.0), None);
        assert_eq!(popup_opacity(false, OPEN_DELAY * 10, 0.0), None);
    }

    /// **A saída parte de onde a entrada estava.** Sair no meio do fade de entrada (opacidade 0,4)
    /// desvanece a partir de 0,4 — não de 1. Sem isto a etiqueta daria um salto de brilho justamente
    /// no frame em que o ponteiro sai dela.
    #[test]
    fn a_saida_parte_da_opacidade_congelada() {
        let inicio = popup_opacity(false, Duration::ZERO, 0.4).expect("visível");
        assert!((inicio - 0.4).abs() < 1e-6, "parte de 0,4; veio {inicio}");
        let meio = popup_opacity(false, FADE / 2, 0.4).expect("saindo");
        assert!((meio - 0.2).abs() < 1e-3, "metade de 0,4; veio {meio}");
        assert!(meio < inicio, "e só desce");
    }

    /// **Reentrar numa etiqueta ainda visível não paga o atraso de novo.**
    ///
    /// É o que faz o ponteiro atravessar o vão de 4px entre gatilho e popup sem a etiqueta piscar: o
    /// relógio é rebobinado pro ponto do fade que produz a opacidade corrente, em vez de zerado.
    #[test]
    fn reentrar_num_popup_visivel_nao_paga_o_delay_de_novo() {
        // Nunca esteve visível: paga tudo.
        assert_eq!(resume_elapsed(0.0), Duration::ZERO, "entrar do zero paga os 600ms");
        assert_eq!(popup_opacity(true, resume_elapsed(0.0), 0.0), None);

        // Estava cheia: volta cheia, no mesmo frame. (O `mul_f32` do fade erra na casa dos
        // nanossegundos, daí a tolerância em vez de igualdade.)
        assert!(
            (resume_elapsed(1.0).as_secs_f32() - (OPEN_DELAY + FADE).as_secs_f32()).abs() < 1e-4,
            "cheia = os 600ms + o fade inteiro; veio {:?}",
            resume_elapsed(1.0)
        );
        assert_eq!(popup_opacity(true, resume_elapsed(1.0), 0.0), Some(1.0));

        // Estava no meio do fade: volta no meio, e continua subindo daí.
        let voltou = popup_opacity(true, resume_elapsed(0.5), 0.0).expect("visível na volta");
        assert!((voltou - 0.5).abs() < 1e-3, "volta em 0,5; veio {voltou}");
        assert!(
            resume_elapsed(0.5) >= OPEN_DELAY,
            "o atraso está PAGO — a etiqueta não desaparece na volta"
        );

        // Acima de 1 (arredondamento) não vira tempo negativo nem futuro.
        assert_eq!(resume_elapsed(9.0), resume_elapsed(1.0), "aparado em 1");
    }

    /// **O frame é pedido só enquanto há o que mudar.** Uma etiqueta parada aberta sob o ponteiro não
    /// queima frame nenhum — e uma que já sumiu, muito menos. Sem esta economia o
    /// `request_animation_frame` seria um laço infinito de repaint.
    #[test]
    fn o_frame_e_pedido_so_enquanto_ha_o_que_mudar() {
        // Contando o atraso: pede.
        assert!(needs_frame(true, Duration::ZERO, 0.0));
        assert!(needs_frame(true, Duration::from_millis(599), 0.0));
        // Entrando (fade em curso): pede.
        assert!(needs_frame(true, OPEN_DELAY, 0.0));
        assert!(needs_frame(true, OPEN_DELAY + FADE / 2, 0.0));
        // Parada, aberta: NÃO pede.
        assert!(!needs_frame(true, OPEN_DELAY + FADE, 0.0));
        assert!(!needs_frame(true, OPEN_DELAY + FADE * 100, 0.0));
        // Saindo: pede até sumir.
        assert!(needs_frame(false, Duration::ZERO, 1.0));
        assert!(needs_frame(false, FADE / 2, 1.0));
        // Sumiu: NÃO pede.
        assert!(!needs_frame(false, FADE, 1.0));
        assert!(!needs_frame(false, Duration::ZERO, 0.0), "nunca abriu, nada a animar");
    }

    // ---------------------------------------------------------------------------------------------
    // A tabela de estado
    // ---------------------------------------------------------------------------------------------

    /// Um id de teste, com a tabela limpa.
    fn id_limpo(nome: &'static str) -> ElementId {
        let id = ElementId::from(nome);
        TOOLTIPS.with(|t| t.borrow_mut().remove(&id));
        id
    }

    /// A opacidade corrente de `id`, do jeito que o render a calcula.
    fn opacidade(id: &ElementId) -> Option<f32> {
        let st = state(id);
        popup_opacity(st.hovered(), st.since.elapsed(), st.leave_from)
    }

    /// Afirma que a etiqueta de `id` está na tela com opacidade ≈ `esperada`.
    ///
    /// A tolerância não é frouxidão: estes testes leem o relógio de VERDADE (`Instant::elapsed`), e os
    /// microssegundos que passam entre uma linha e a próxima já movem o fade — um `assert_eq!` contra
    /// `1.0` falha com `0.99995`.
    fn assert_opacidade(id: &ElementId, esperada: f32, contexto: &str) {
        let o = opacidade(id).unwrap_or_else(|| panic!("{contexto}: esperava a etiqueta na tela"));
        assert!(
            (o - esperada).abs() < 1e-3,
            "{contexto}: esperava ~{esperada}, veio {o}"
        );
    }

    /// **O ciclo de vida completo, pela tabela**: entrar no gatilho arma o relógio, os 600ms abrem, e
    /// sair fecha.
    #[test]
    fn a_tabela_arma_o_relogio_no_hover_do_gatilho() {
        let id = id_limpo("tt-ciclo");
        assert_eq!(opacidade(&id), None, "sem hover, sem etiqueta");

        record_hover(&id, Region::Trigger, true);
        assert!(state(&id).hovered());
        assert_eq!(opacidade(&id), None, "o relógio acabou de armar");

        recuar_relogio(&id, OPEN_DELAY + FADE);
        assert_opacidade(&id, 1.0, "passados os 600ms + fade");

        record_hover(&id, Region::Trigger, false);
        assert!(!state(&id).hovered());
        assert_eq!(
            state(&id).leave_from,
            1.0,
            "a saída congelou a opacidade que estava na tela"
        );
        assert_opacidade(&id, 1.0, "começa a sair de onde estava");
        recuar_relogio(&id, FADE);
        assert_eq!(opacidade(&id), None, "e no fim do fade sumiu");
    }

    /// **Sair no MEIO do fade de entrada congela a opacidade parcial**, e não a cheia.
    ///
    /// O teste `a_saida_parte_da_opacidade_congelada` prova que a função respeita o valor congelado;
    /// este prova que quem congela guarda o valor CERTO. Um `leave_from = 1.0` fixo passaria naquele e
    /// daria, aqui, um salto de brilho no frame em que o ponteiro sai.
    #[test]
    fn sair_no_meio_do_fade_congela_a_opacidade_parcial() {
        let id = id_limpo("tt-meio-do-fade");
        record_hover(&id, Region::Trigger, true);
        // Metade do fade de ENTRADA já correu: a etiqueta está a ~50%.
        recuar_relogio(&id, OPEN_DELAY + FADE / 2);
        assert_opacidade(&id, 0.5, "no meio do fade de entrada");

        record_hover(&id, Region::Trigger, false);
        let congelada = state(&id).leave_from;
        assert!(
            (congelada - 0.5).abs() < 1e-2,
            "congelou ~0,5, e não a opacidade cheia; veio {congelada}"
        );
        assert_opacidade(&id, 0.5, "e a saída parte de lá");
    }

    /// **O popup é hoverable: sair do gatilho PARA DENTRO da etiqueta não a fecha.**
    ///
    /// É o default do Base UI que a referência não sobrescreve. O teste roda a ordem de eventos mais
    /// hostil — o `false` do gatilho ANTES do `true` do popup, que é o que o GPUI entrega quando o
    /// ponteiro pula o vão num único movimento.
    #[test]
    fn o_popup_e_hoverable_e_o_vao_nao_fecha_a_etiqueta() {
        let id = id_limpo("tt-hoverable");
        record_hover(&id, Region::Trigger, true);
        recuar_relogio(&id, OPEN_DELAY + FADE);
        assert_opacidade(&id, 1.0, "aberta e parada");

        // A ordem ruim: o gatilho perde o ponteiro primeiro.
        record_hover(&id, Region::Trigger, false);
        // …e o popup o ganha no mesmo movimento.
        record_hover(&id, Region::Popup, true);

        assert!(state(&id).hovered(), "o popup segura a etiqueta aberta");
        assert_opacidade(&id, 1.0, "e ela NÃO volta a pagar os 600ms — não pisca");

        // Saindo do popup pra fora, fecha.
        record_hover(&id, Region::Popup, false);
        recuar_relogio(&id, FADE);
        assert_eq!(opacidade(&id), None);
    }

    /// **Registrar o mesmo hover duas vezes não reinicia o relógio.** O GPUI pode reemitir o mesmo
    /// valor; se cada emissão zerasse o relógio, a etiqueta nunca abriria enquanto o ponteiro se
    /// movesse dentro do gatilho — que é justamente o que ele faz.
    #[test]
    fn hover_repetido_nao_reinicia_o_relogio() {
        let id = id_limpo("tt-repetido");
        record_hover(&id, Region::Trigger, true);
        recuar_relogio(&id, OPEN_DELAY);
        let marca = state(&id).since;

        record_hover(&id, Region::Trigger, true); // mesma emissão
        assert_eq!(state(&id).since, marca, "o relógio não pode voltar pro zero");
        assert!(opacidade(&id).is_some(), "e a etiqueta continua aberta");
    }

    /// **O anchor rect e a caixa do popup são observáveis, e começam vazios.** É contra eles que os
    /// testes de janela conferem a geometria declarada.
    #[test]
    fn a_medida_do_gatilho_e_do_popup_e_observavel() {
        let id = id_limpo("tt-medida");
        assert_eq!(measured_anchor(&id), None);
        assert_eq!(measured_popup(&id), None);

        let caixa = Bounds {
            origin: point(px(GATILHO.x), px(GATILHO.y)),
            size: gpui::size(px(GATILHO.w), px(GATILHO.h)),
        };
        record_anchor(&id, caixa);
        assert_eq!(rect_of(measured_anchor(&id).expect("medido")), GATILHO);
        assert_eq!(measured_popup(&id), None, "o popup não foi pintado ainda");

        record_popup(&id, caixa);
        assert_eq!(measured_popup(&id), Some(caixa));
    }

    /// **A tabela tem teto, e estourar degrada em vez de quebrar.** Mesma regra do
    /// `INTERACTION_TABLE_CAP` do [`crate::button`]: no pior caso as próximas etiquetas abrem com o
    /// relógio zerado.
    #[test]
    fn a_tabela_tem_teto() {
        TOOLTIPS.with(|t| t.borrow_mut().clear());
        for i in 0..=TABLE_CAP + 1 {
            record_hover(&ElementId::from(i), Region::Trigger, true);
        }
        let tamanho = TOOLTIPS.with(|t| t.borrow().len());
        assert!(
            tamanho <= TABLE_CAP + 1,
            "a tabela não cresce sem limite; veio {tamanho}"
        );
        TOOLTIPS.with(|t| t.borrow_mut().clear());
    }

    /// **A prioridade do `deferred` é a mesma do menu**, porque na referência os dois são `z-50`.
    #[test]
    fn a_prioridade_do_deferred_e_a_do_menu() {
        assert_eq!(POPUP_PRIORITY, 1, "o mesmo `with_priority(1)` do menu e do select");
    }
}

#[cfg(test)]
// Mesmo motivo do módulo de testes acima: aqui também se afirma constante.
#[allow(clippy::assertions_on_constants)]
mod tests_de_janela {
    //! Os testes que precisam de uma JANELA de verdade: é onde a geometria declarada nas constantes é
    //! conferida contra a **medida**, e onde a ancoragem no gatilho é conferida contra a alternativa
    //! que este módulo recusou (a `.tooltip()` embutida do GPUI, que ancora no cursor).
    //!
    //! Uma nota sobre o tempo: o `Window::request_animation_frame` é um **no-op** na plataforma de
    //! teste (`TestWindow::on_request_frame` ignora o callback, em `gpui/src/platform/test/window.rs`),
    //! então nenhum frame chega sozinho aqui. Os testes recuam o relógio na tabela e pedem o redraw à
    //! mão — o que é mais rápido e mais determinístico que dormir 600ms, e é o mesmo caminho que os
    //! testes de interação do [`crate::button`] usam.

    use super::*;
    use gpui::{px, size, Context, Modifiers, Render, TestAppContext, VisualTestContext};

    /// Onde o gatilho é plantado dentro da janela. Longe da origem de propósito: uma conta que
    /// ignorasse a posição do gatilho passaria num harness em (0,0).
    const OFFSET_X: f32 = 200.0;
    /// Idem, no eixo vertical. Fundo o bastante pra a etiqueta caber ACIMA do gatilho sem o
    /// `snap_to_window_with_margin` interferir.
    const OFFSET_Y: f32 = 100.0;

    /// Tamanho do gatilho. Um `div` de tamanho fixo, e não um [`crate::Button`]: as afirmações de
    /// centralização ficam exatas, sem depender da métrica de outro componente.
    const TRIGGER_W: f32 = 40.0;
    /// Idem.
    const TRIGGER_H: f32 = 24.0;

    /// Onde o ponteiro é posto: longe do gatilho E longe de onde a etiqueta vai nascer. É o que dá
    /// sentido à afirmação "não é o cursor".
    const MOUSE: (f32, f32) = (500.0, 500.0);

    /// O id da etiqueta do harness.
    const TT: &str = "tt-janela";

    /// Altura da **barra** que hospeda a etiqueta — bem maior que o gatilho, de propósito.
    ///
    /// Uma etiqueta mora numa barra de ferramentas de altura fixa, e é esse arranjo que revela o
    /// `items_start` da raiz do [`Tooltip`]: sem ele, o `stretch` default do flex esticaria o wrap do
    /// gatilho pros 200px da barra, o anchor rect sairia com 200px de altura, e a etiqueta seria
    /// pintada 176px acima de onde devia. Num harness sem barra o defeito não aparece.
    const BARRA_H: f32 = 200.0;

    struct Harness;

    impl Render for Harness {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().pt(px(OFFSET_Y)).pl(px(OFFSET_X)).child(
                div().flex().h(px(BARRA_H)).child(
                    Tooltip::new(TT, "Salvar").child(
                        div()
                            .w(px(TRIGGER_W))
                            .h(px(TRIGGER_H))
                            .flex_none()
                            .child("⌘S"),
                    ),
                ),
            )
        }
    }

    /// Abre a janela com a tabela limpa e o tema escuro, e devolve o contexto visual.
    fn abrir(cx: &mut TestAppContext) -> (ElementId, VisualTestContext) {
        theme::set_theme(theme::ThemeMode::Dark);
        let id = ElementId::from(TT);
        TOOLTIPS.with(|t| t.borrow_mut().remove(&id));

        let window = cx.add_window(|_window, _cx| Harness);
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        (id, vcx)
    }

    /// Um redraw à mão (ver a nota sobre o tempo, no doc deste módulo).
    fn redesenhar(vcx: &mut VisualTestContext) {
        vcx.update(|window, _cx| window.refresh());
        vcx.run_until_parked();
    }

    /// Põe o relógio de `id` como se o ponteiro estivesse no gatilho há 600ms + fade — o estado
    /// "etiqueta aberta e parada".
    fn abrir_a_etiqueta(id: &ElementId) {
        update(id, |st| {
            st.on_trigger = true;
            st.leave_from = 0.0;
        });
        recuar_relogio(id, OPEN_DELAY + FADE);
    }

    /// **A etiqueta é pintada ACIMA do gatilho e centralizada nele — não no cursor — e a caixa dela
    /// tem os 26px declarados.**
    ///
    /// É a medição que justifica não usar a `.tooltip()` embutida do GPUI (ver o doc do módulo): ela
    /// poria a caixa em `mouse_position + (1,1)`, e o teste afirma o contrário, com o ponteiro a
    /// 300px de distância.
    ///
    /// A altura é o número que tranca a entrelinha: se o `line_height` voltasse pro default do GPUI
    /// (a razão de ouro), a caixa mediria 29,42 em vez de 26. Nenhuma leitura do `.tsx` pega isso —
    /// só a medida.
    #[gpui::test]
    fn o_popup_nasce_acima_do_gatilho_e_nao_no_cursor(cx: &mut TestAppContext) {
        let (id, mut vcx) = abrir(cx);

        // O ponteiro vai pra longe. O gatilho não é tocado, então o `on_hover` dele não dispara.
        vcx.simulate_mouse_move(point(px(MOUSE.0), px(MOUSE.1)), None, Modifiers::default());

        let ancora = rect_of(measured_anchor(&id).expect("o gatilho se mede no primeiro prepaint"));
        assert_eq!((ancora.x, ancora.y), (OFFSET_X, OFFSET_Y), "o anchor rect é o do gatilho");
        assert_eq!((ancora.w, ancora.h), (TRIGGER_W, TRIGGER_H));
        assert_eq!(measured_popup(&id), None, "sem hover, a etiqueta não existe");

        abrir_a_etiqueta(&id);
        redesenhar(&mut vcx);

        let caixa = measured_popup(&id).expect("aberta, a etiqueta se mede no prepaint dela");
        let (x, y) = (f32::from(caixa.origin.x), f32::from(caixa.origin.y));
        let (w, h) = (f32::from(caixa.size.width), f32::from(caixa.size.height));

        // --- A altura: a soma declarada, medida ---------------------------------------------------
        assert_eq!(h, POPUP_HEIGHT, "borda + py-1 + uma linha de 16 + py-1 + borda");
        assert_eq!(h, 26.0);

        // --- A posição: acima do gatilho, centralizada nele ---------------------------------------
        assert_eq!(
            y + h,
            ancora.y - SIDE_OFFSET,
            "a BASE da etiqueta fica 4px acima do topo do gatilho"
        );
        assert_eq!(
            x + w / 2.0,
            ancora.x + ancora.w / 2.0,
            "os centros horizontais coincidem — é a definição de align: center"
        );

        // --- E NÃO no cursor ---------------------------------------------------------------------
        //
        // A `.tooltip()` do GPUI põe a caixa em `mouse_position + (1,1)`; ver `prepaint_tooltip` em
        // `gpui/src/window.rs`.
        assert!(
            (x - (MOUSE.0 + 1.0)).abs() > 100.0 && (y - (MOUSE.1 + 1.0)).abs() > 100.0,
            "a etiqueta está a centenas de px do cursor, em ({x}, {y})"
        );

        // A etiqueta é mais larga que este gatilho de 40px, então `center` a faz transbordar pros
        // dois lados — o comportamento certo, e o motivo de o harness plantar o gatilho longe da
        // borda.
        assert!(w > TRIGGER_W, "o texto é mais largo que o gatilho, veio {w}");
        assert!(x < ancora.x, "e ela transborda pra esquerda também");
        assert!(x > WINDOW_MARGIN, "sem precisar grudar na janela neste harness");

        // --- O prepaint corrige o hover do popup contra a posição REAL do ponteiro ----------------
        //
        // Com o cursor longe, o popup não pode se declarar em hover (senão ele se seguraria aberto pra
        // sempre — ver o comentário do canvas de medida do popup).
        assert!(!state(&id).on_popup, "o cursor está longe: o popup não está em hover");

        // E com o cursor DENTRO dele, o prepaint marca — é o que faz a etiqueta hoverable sobreviver a
        // um `on_hover` que nunca entregou a borda de entrada.
        vcx.simulate_mouse_move(
            point(px(x + w / 2.0), px(y + h / 2.0)),
            None,
            Modifiers::default(),
        );
        redesenhar(&mut vcx);
        assert!(
            state(&id).on_popup,
            "o cursor dentro do popup: o prepaint marcou o hover dele"
        );
    }

    /// **O hover no gatilho arma o relógio, e a etiqueta não aparece antes dos 600ms.**
    ///
    /// É o atraso end-to-end: o ponteiro entra de verdade (um `MouseMoveEvent` na hitbox), o
    /// `on_hover` registra, um frame é desenhado — e não há etiqueta nenhuma nele. Só depois de o
    /// relógio ser recuado ela nasce.
    #[gpui::test]
    fn o_hover_no_gatilho_espera_os_seiscentos_ms(cx: &mut TestAppContext) {
        let (id, mut vcx) = abrir(cx);

        let centro = point(
            px(OFFSET_X + TRIGGER_W / 2.0),
            px(OFFSET_Y + TRIGGER_H / 2.0),
        );
        vcx.simulate_mouse_move(centro, None, Modifiers::default());

        assert!(state(&id).on_trigger, "o `on_hover` do gatilho registrou a entrada");
        assert_eq!(
            measured_popup(&id),
            None,
            "mas 0ms de hover não abre nada — é o delay de 600ms"
        );

        // O relógio corre.
        recuar_relogio(&id, OPEN_DELAY + FADE);
        redesenhar(&mut vcx);
        assert!(
            measured_popup(&id).is_some(),
            "passados os 600ms, a etiqueta abriu"
        );

        // E o ponteiro saindo do gatilho a fecha — sem carência (`closeDelay = 0`).
        vcx.simulate_mouse_move(point(px(MOUSE.0), px(MOUSE.1)), None, Modifiers::default());
        assert!(!state(&id).on_trigger, "o `on_hover` entregou a borda de SAÍDA");
        let st = state(&id);
        assert!(
            popup_opacity(st.hovered(), st.since.elapsed(), st.leave_from).is_some(),
            "no instante da saída ela ainda está na tela, desvanecendo"
        );
        recuar_relogio(&id, FADE);
        let st = state(&id);
        assert_eq!(
            popup_opacity(st.hovered(), st.since.elapsed(), st.leave_from),
            None,
            "e no fim do fade de saída, sumiu"
        );
    }

    /// **Um hover de popup PRESO na tabela é corrigido no prepaint dele.**
    ///
    /// O cenário real: o `Tooltip` sai da árvore com o ponteiro dentro da etiqueta (o painel que o
    /// hospedava fechou, por exemplo). O `on_hover` do popup morre junto e **nunca** entrega a borda de
    /// saída — o estado de hover do elemento renasce `false` a cada frame, então `is_hovered ==
    /// was_hovered` e o listener não é chamado. Sem correção, o `on_popup` ficaria `true` pra sempre e
    /// a etiqueta se seguraria aberta sozinha, com o ponteiro do outro lado da tela.
    ///
    /// Quem conserta é o `canvas` de medida do popup: ele pergunta ao [`gpui::Window::mouse_position`]
    /// se o ponteiro está DENTRO da caixa, e essa resposta não envelhece.
    ///
    /// O teste planta a inconsistência à mão porque é exatamente o que a árvore teria deixado lá.
    #[gpui::test]
    fn o_hover_preso_do_popup_e_corrigido_no_prepaint(cx: &mut TestAppContext) {
        let (id, mut vcx) = abrir(cx);

        // O ponteiro está longe de tudo.
        vcx.simulate_mouse_move(point(px(MOUSE.0), px(MOUSE.1)), None, Modifiers::default());

        // A inconsistência: a etiqueta está aberta *por causa do popup*, e o gatilho não tem ponteiro.
        update(&id, |st| {
            st.on_trigger = false;
            st.on_popup = true;
            st.leave_from = 0.0;
            st.since = Instant::now()
                .checked_sub(OPEN_DELAY + FADE)
                .unwrap_or_else(Instant::now);
        });
        assert!(state(&id).hovered(), "o estado preso mantém a etiqueta aberta");

        redesenhar(&mut vcx);

        assert!(
            !state(&id).on_popup,
            "o prepaint do popup viu o ponteiro fora e desmarcou o hover dele"
        );
        assert!(
            !state(&id).hovered(),
            "e sem nenhuma das duas regiões em hover, a etiqueta vai fechar"
        );
    }

    /// **A etiqueta escapa do recorte de quem está por fora.**
    ///
    /// O gatilho vai dentro de uma caixa com `overflow_hidden` MENOR que a etiqueta; se o popup fosse
    /// filho normal, ele seria recortado e a caixa medida sairia menor. Como ele é `deferred`, a
    /// medida é a inteira — é o que faz uma etiqueta funcionar dentro de uma [`crate::ScrollArea`].
    #[gpui::test]
    fn a_etiqueta_escapa_do_recorte(cx: &mut TestAppContext) {
        /// A caixa recortante: mais estreita e mais baixa que a etiqueta.
        const CAIXA: f32 = 20.0;

        struct Recortado;
        impl Render for Recortado {
            fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
                div().size_full().pt(px(OFFSET_Y)).pl(px(OFFSET_X)).child(
                    div()
                        .w(px(CAIXA))
                        .h(px(CAIXA))
                        .overflow_hidden()
                        .child(Tooltip::new(TT, "Uma etiqueta bem mais larga que a caixa").child(
                            div().w(px(TRIGGER_W)).h(px(TRIGGER_H)).flex_none(),
                        )),
                )
            }
        }

        theme::set_theme(theme::ThemeMode::Dark);
        let id = ElementId::from(TT);
        TOOLTIPS.with(|t| t.borrow_mut().remove(&id));

        let window = cx.add_window(|_window, _cx| Recortado);
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();

        abrir_a_etiqueta(&id);
        redesenhar(&mut vcx);

        let caixa = measured_popup(&id).expect("aberta");
        assert_eq!(
            f32::from(caixa.size.height),
            POPUP_HEIGHT,
            "a altura inteira, não a fatia de 20px"
        );
        assert!(
            f32::from(caixa.size.width) > CAIXA,
            "e a largura inteira, {}px, maior que os {CAIXA}px da caixa recortante",
            f32::from(caixa.size.width)
        );
    }

    /// **Encurralada na borda, a etiqueta GRUDA na janela em vez de sair dela.**
    ///
    /// É o `collisionPadding` do `Positioner`, que aqui é o `snap_to_window_with_margin`. O gatilho
    /// fica a 20px da borda esquerda e a etiqueta abre `side: left` — sem grudar, ela seria pintada
    /// inteira em x negativo, ou seja fora da tela.
    ///
    /// Note que ela **desliza**, e não vira pro outro lado: os dois são exclusivos no
    /// [`gpui::anchored`], e a escolha (e o motivo) são as mesmas do [`crate::menu`].
    #[gpui::test]
    fn encurralada_na_borda_a_etiqueta_gruda_na_janela(cx: &mut TestAppContext) {
        /// A que distância da borda esquerda o gatilho fica. Menos que a largura da etiqueta.
        const PERTO_DA_BORDA: f32 = 20.0;

        struct Encurralado;
        impl Render for Encurralado {
            fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
                div().size_full().pt(px(OFFSET_Y)).pl(px(PERTO_DA_BORDA)).child(
                    Tooltip::new(TT, "Apagar")
                        .side(MenuSide::Left)
                        .child(div().w(px(TRIGGER_W)).h(px(TRIGGER_H)).flex_none()),
                )
            }
        }

        theme::set_theme(theme::ThemeMode::Dark);
        let id = ElementId::from(TT);
        TOOLTIPS.with(|t| t.borrow_mut().remove(&id));

        let window = cx.add_window(|_window, _cx| Encurralado);
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();

        abrir_a_etiqueta(&id);
        redesenhar(&mut vcx);

        let caixa = measured_popup(&id).expect("aberta");
        let x = f32::from(caixa.origin.x);
        let w = f32::from(caixa.size.width);

        // Onde ela cairia se ninguém a segurasse: a borda direita dela 4px antes do gatilho.
        let sem_grudar = PERTO_DA_BORDA - SIDE_OFFSET - w;
        assert!(sem_grudar < 0.0, "o harness tem que ser apertado de verdade");
        assert_eq!(
            x, WINDOW_MARGIN,
            "grudou na janela guardando exatamente a margem (sem grudar seria x={sem_grudar})"
        );

        // E grudou **deslizando**, não virando: o `AnchoredFitMode::SwitchAnchor` default do
        // `anchored` — que é o que vale se alguém tirar o `snap_to_window_with_margin` — poria a
        // etiqueta à DIREITA do gatilho, e não colada na borda esquerda da janela.
        let se_virasse = PERTO_DA_BORDA + TRIGGER_W + SIDE_OFFSET;
        assert_ne!(
            x, se_virasse,
            "a etiqueta desliza pra dentro da janela; virar pro outro lado poria x em {se_virasse}"
        );
    }

    /// **A janela estreita não deixa a etiqueta estourar.** É o `max-w-(--available-width)` valendo de
    /// verdade: numa janela de 200px a etiqueta quebra em vez de sair pela borda.
    #[gpui::test]
    fn a_etiqueta_respeita_a_largura_disponivel(cx: &mut TestAppContext) {
        struct Estreito;
        impl Render for Estreito {
            fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
                div().size_full().pt(px(OFFSET_Y)).child(
                    Tooltip::new(TT, "Um texto longo o bastante pra não caber numa janela estreita")
                        .child(div().w(px(TRIGGER_W)).h(px(TRIGGER_H)).flex_none()),
                )
            }
        }

        theme::set_theme(theme::ThemeMode::Dark);
        let id = ElementId::from(TT);
        TOOLTIPS.with(|t| t.borrow_mut().remove(&id));

        let window = cx.add_window(|_window, _cx| Estreito);
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.simulate_resize(size(px(200.0), px(600.0)));
        vcx.run_until_parked();

        abrir_a_etiqueta(&id);
        redesenhar(&mut vcx);

        let caixa = measured_popup(&id).expect("aberta");
        let largura = f32::from(caixa.size.width);
        let teto = popup_max_width(DEFAULT_SIDE, rect_of(measured_anchor(&id).unwrap()), 200.0, SIDE_OFFSET);
        assert!(
            largura <= teto,
            "a etiqueta cabe no espaço disponível ({teto}px), veio {largura}"
        );
        assert!(
            f32::from(caixa.size.height) > POPUP_HEIGHT,
            "e o texto quebrou em mais de uma linha, em vez de estourar (w={largura}, h={}, teto={teto})",
            f32::from(caixa.size.height)
        );
    }
}

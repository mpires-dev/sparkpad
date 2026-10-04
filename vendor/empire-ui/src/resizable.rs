//! `Resizable` — painéis lado a lado com uma **divisa arrastável** entre eles.
//!
//! Porte do `resizable.tsx` do **lumiui** (`ResizableGroup` / `ResizablePanel` /
//! `ResizableSeparator`), que por sua vez embrulha o [`react-resizable-panels`] **v4**.
//!
//! [`react-resizable-panels`]: https://react-resizable-panels.vercel.app/
//!
//! # Este componente é CASCA, não motor
//!
//! Redimensionar painéis é um problema resolvido, e a solução já está nesta árvore: o fork
//! vendorizado do `gpui-component` traz `resizable/` — arraste, restrição de mínimo/máximo,
//! redistribuição em cascata quando um vizinho encosta no limite, e o reajuste proporcional quando a
//! janela muda de tamanho. **Nada disso é reimplementado aqui.** O que este módulo faz é vestir esse
//! motor com a aparência do lumiui e com os nossos tokens:
//!
//! | camada | de onde vem |
//! |---|---|
//! | arraste, cascata, mínimo/máximo, reajuste ao container | `gpui_component::resizable` (núcleo) |
//! | onde cada divisa CAI (a emenda entre dois painéis) | núcleo — nós só **medimos** ([`ancora`]) |
//! | linha do separador, puxador, cores, raio | **nosso** (este módulo) |
//! | navegação por teclado (setas, `Home`/`End`) | **nosso** (o núcleo não tem — ver abaixo) |
//!
//! # A unidade dos tamanhos é PIXEL
//!
//! Os exemplos do lumiui misturam `defaultSize={50}` com `minSize={100}` e `minSize={200}`, o que
//! sugere porcentagem no primeiro e pixel nos outros. **Não é ambíguo:** no
//! `react-resizable-panels` v4 a regra é explícita —
//!
//! > *"Numbers are interpreted as pixels (e.g. `minSize={200}` is 200 pixels). Strings without
//! > explicit units are interpreted as percentage (e.g. `defaultSize="50"` is 50 percent)."*
//! > — [`lib/components/panel/types.ts`], repetido em `README.md` e na página *Min/max sizes*.
//!
//! [`lib/components/panel/types.ts`]: https://github.com/bvaughn/react-resizable-panels/blob/main/lib/components/panel/types.ts
//!
//! Ou seja `defaultSize={50}` é **50 pixels**, não 50%. E isso casa com o núcleo daqui, que trabalha
//! em [`gpui::Pixels`] — não há conversão nenhuma a fazer. [`ResizablePanel::size`],
//! [`ResizablePanel::min_size`] e [`ResizablePanel::max_size`] recebem **pixels**, e é o que a nossa
//! API pública diz.
//!
//! # ⚠️ A INVERSÃO do `aria-orientation`
//!
//! É o erro mais fácil de cometer neste componente, e as classes da fonte convidam a ele:
//!
//! ```text
//! aria-[orientation=vertical]:w-px   aria-[orientation=vertical]:h-auto
//! aria-[orientation=horizontal]:h-px aria-[orientation=horizontal]:w-full
//! ```
//!
//! O `aria-orientation` do SEPARADOR é o **inverso** do `orientation` do GRUPO — está no
//! `Separator.tsx` da referência, em uma linha:
//!
//! ```text
//! const orientation = groupOrientation === "horizontal" ? "vertical" : "horizontal";
//! ```
//!
//! Num grupo `orientation="horizontal"` os painéis ficam lado a lado e a divisa entre eles é uma
//! linha **VERTICAL** — logo `aria-[orientation=vertical]:w-px` é o caso do grupo **horizontal**.
//! Ligar as classes na orientação do grupo inverte tudo: a linha nasce com a espessura no eixo
//! errado e o puxador sai girado.
//!
//! Aqui a inversão acontece **num lugar só**, em [`separator_metrics`], que é função pura e tem teste
//! próprio. Nenhum `if` de render lê `self.orientation` pra decidir medida: eles leem
//! [`SeparatorMetrics::aria`].
//!
//! # O `after:` da fonte, e a área de clique
//!
//! O separador da fonte é uma linha de 1px com um `::before`/`::after` invisível de 16px, que é a
//! área de clique de verdade. O GPUI não tem pseudo-elemento, então isso normalmente viraria um filho
//! absoluto invisível — **mas aqui não vira nada**, porque a área de arraste não é nossa: quem cria a
//! divisa arrastável é o núcleo, no `resize_handle` dele, e ela é `pub(crate)` no fork. Ver
//! "Não reproduzível" abaixo, onde a conta da fonte está escrita e a área real está medida.
//!
//! # Como o puxador fica POR CIMA sem `z-index`
//!
//! ## ⚠️ E por que NÃO com [`gpui::deferred`] — a regra vale pra lib inteira
//!
//! **[`gpui::deferred`] escapa da [`gpui::ContentMask`], logo não serve pra `z-index` dentro de
//! conteúdo rolável.** Não é opinião, é onde o GPUI pinta: um filho diferido é pintado em
//! `paint_deferred_draws` (`gpui-0.2.2/src/window.rs:2194`), **depois** de o passe principal ter
//! desempilhado a pilha de máscaras — então `window.content_mask()` cai no fallback dela, que é a
//! VIEWPORT inteira (`window.rs:2551`). O elemento sai com a geometria certa e o recorte errado.
//!
//! Isso é exatamente o que menu, popover e etiqueta QUEREM: são camadas flutuantes, precisam sair do
//! recorte de quem as contém, e o `deferred` continua certo pra elas. O separador **não** é camada
//! flutuante — ele vive dentro do grupo, e o grupo pode estar dentro de uma
//! [`crate::ScrollArea`]. Com `deferred` a linha e o puxador ignoravam o recorte do container rolável
//! e pintavam por cima do que estivesse embaixo (foi o defeito relatado: a linha atravessando a borda
//! do card e continuando num painel fixo). Amarrado em
//! [`tests_de_janela::o_separador_nao_pinta_fora_do_container_rolavel`].
//!
//! A regra curta: **`deferred` é pra ESCAPAR de uma hierarquia de recorte, não pra ordenar camadas
//! dentro dela.** Pra ordenar dentro dela o GPUI tem uma ferramenta só, e basta: **ordem de filhos —
//! o último pintado ganha.**
//!
//! ## O `z-10` da fonte é ordenação LOCAL
//!
//! Na fonte o puxador é `z-10` dentro do separador: ele desenha **sobre a linha do separador**, que é
//! o pai dele. Isso não precisa de mecanismo nenhum aqui — o puxador é filho da linha, e um filho
//! pinta depois do fundo do pai. Ponto.
//!
//! ## O que precisava de mecanismo era a alça do NÚCLEO
//!
//! O `ResizablePanel` do núcleo pendura o `resize_handle` dele — que pinta uma linha **opaca** de 1px
//! exatamente sobre a emenda — **depois** dos filhos do painel
//! (`vendor/gpui-component/src/resizable/panel.rs:303`). Medido, e não suposto: a faixa da alça vai
//! de −4 a +4 da divisa ([`tests_de_janela::a_faixa_de_arraste_do_nucleo_e_a_que_esta_declarada`]) e
//! a linha dela cai na caixa de conteúdo dessa faixa, ou seja em 0..1 — o mesmo pixel da nossa.
//! Então um separador que fosse filho do painel ficaria **embaixo** dela: a nossa cor e o realce de
//! `primary/40` sumiriam sob o `border` opaco do núcleo, e a linha dele atravessaria o puxador.
//!
//! A saída é ordem de filhos, no mesmo contexto de recorte: [`Resizable::render`] devolve um `div`
//! com **dois** filhos — o grupo (com todas as alças do núcleo dentro dele) e, DEPOIS, um separador
//! por divisa. Filho posterior pinta depois; e sendo filho normal de um `div` normal, ele herda a
//! `ContentMask` dos ancestrais. Amarrado em
//! [`tests_de_janela::o_separador_fica_por_cima_da_alca_do_nucleo`], que mede a ordem pela fila de
//! hitboxes — a alça é `occlude()`, e o `hit_test` do GPUI para no primeiro hitbox que bloqueia.
//!
//! O preço de sair do painel é que o separador perde a referência da emenda: o taffy o põe no canto
//! do GRUPO. Quem o leva até a divisa é [`Deslocado`], **no mesmo frame** — ver o doc dele.
//!
//! # Teclado
//!
//! O título do doc do lumiui diz *"with keyboard support"*, e o núcleo vendorizado **não tem**: os
//! três arquivos de `resizable/` não têm handle de foco, `on_key_down` nem ação — nem o
//! `resize_handle` nem o `ResizableState`. Então a navegação por teclado deste módulo é **nossa**, e
//! segue o contrato da referência (`onDocumentKeyDown.ts` do v4):
//!
//! | tecla | efeito |
//! |---|---|
//! | `←`/`→` (grupo horizontal), `↑`/`↓` (grupo vertical) | move a divisa **5 pontos percentuais** do grupo |
//! | `Home` | leva a divisa ao **mínimo** do painel de antes (`-100` pontos) |
//! | `End` | leva a divisa ao **máximo** do painel de antes (`+100` pontos) |
//! | seta do eixo cruzado | nada (é `no-op` na referência também) |
//!
//! O ponto delicado é *como* mexer nos tamanhos sem tocar no fork: no núcleo, `resize_panel` é
//! privada e não há setter público em [`ResizableState`]. O que existe de público é
//! [`ResizableState::sizes`] (leitura), `ResizableState::default()` e `ResizablePanel::size`, e o
//! núcleo documenta que o `size` declarado **vale quando o painel ainda não tem tamanho gravado**
//! (`"initial_size is Some and size is none, to use the initial size of the panel for first time
//! render"`). Então o teclado:
//!
//! 1. lê os tamanhos atuais por [`ResizableState::sizes`];
//! 2. calcula os novos com [`move_divider`] (função pura, testada);
//! 3. guarda-os e **reinicia** a entidade de estado (`*state = ResizableState::default()`);
//! 4. no frame seguinte cada painel entra com o tamanho novo pelo `size` declarado.
//!
//! Nada de `unsafe`, nada de patch no fork — só a API pública. O arraste continua sendo do núcleo, e
//! depois do primeiro frame pós-reinício o `size` declarado volta a ser ignorado, como o núcleo
//! manda. O que isso NÃO reusa é a cascata: ver "Desvio consciente".
//!
//! # Diferenças em relação ao original — todas declaradas
//!
//! ## Não reproduzível
//!
//! - **A área de clique de 16px, e a assimetria dela.** A fonte põe `after:inset-0` + `w-4` (16px) +
//!   `-left-1.5` (−6px) sobre uma linha de 1px: o alvo vai de **−6 a +10** em relação à linha, cujo
//!   centro está em **+0,5** — o alvo, centrado em **+2,0**, está **1,5px deslocado** pro lado de
//!   dentro do painel seguinte. Não há como reproduzir nem a largura nem a assimetria: a região que
//!   inicia o arraste é o `resize_handle` do núcleo, criado dentro do `ResizablePanel` dele, e tanto a
//!   função `resize_handle` quanto os `HANDLE_PADDING`/`HANDLE_SIZE` são `pub(crate)` — de fora do
//!   crate não se estende nem se reestiliza. A área real do núcleo está **medida** em
//!   [`tests_de_janela::a_faixa_de_arraste_do_nucleo_e_a_que_esta_declarada`] e escrita em
//!   [`HIT_START`]/[`HIT_SIZE`]: **8px, de −4 a +4**, praticamente centrada na linha (o centro da
//!   faixa cai em 0,0 contra os 0,5 do centro da linha). Custo pra reproduzir a fonte: patch no fork
//!   trocando `HANDLE_PADDING` por um parâmetro por eixo, ou expondo `resize_handle` — decisão do
//!   usuário, não deste porte.
//! - **`data-separator` completo.** A referência tem cinco estados (`disabled`, `inactive`, `hover`,
//!   `active`, `focus`); o lumiui só estiliza `active`/`inactive` e o `focus-visible`, e é o que está
//!   aqui. `hover` não muda nada na fonte (cai no `bg-border` de repouso), então não há o que portar,
//!   e `disabled` não existe na API do lumiui.
//! - **`collapsible` / `collapsedSize`.** O núcleo não colapsa: o `resize_panel` dele apara em
//!   `size_range.start` e para ali. A referência colapsa quando o arraste passa de **metade do
//!   `minSize`** — o que exige a posição do ponteiro DURANTE o arraste, que só existe dentro do
//!   `ResizePanelGroupElement` do fork. Dá pra fingir por cima (fixar `size_range` em `0..0`), mas aí
//!   o painel colapsado não reabre por arraste, o que é pior que não ter. Custo: um `collapsible:
//!   bool` + `collapsed_size: Pixels` no `ResizablePanel` do fork e um teste do limiar no
//!   `resize_panel`. Relatado, não patcheado.
//! - **`Enter` colapsa/restaura** e **`F6` cicla os separadores** (as duas outras teclas da
//!   referência). A primeira depende de `collapsible`; a segunda é redundante aqui, porque cada
//!   separador é uma parada de `Tab` (como na referência, que põe `tabIndex={0}` em todos).
//!
//! ## Resolvido em número
//!
//! - `w-px` / `h-px` (espessura da linha) → [`LINE`] = **1px**.
//! - `h-4` / `w-4` (o lado do puxador NO sentido da linha) → [`GRIP_ALONG`] = 4 × 4px = **16px**.
//! - `w-3` / `h-3` (o lado que ATRAVESSA a linha) → [`GRIP_ACROSS`] = 3 × 4px = **12px**.
//! - `size-3` (o ícone) → [`GRIP_ICON`] = **12px**.
//! - `rounded-xs` → `--radius-xs` do Tailwind v4 = `0.125rem` × 16px/rem = **2px** ([`GRIP_RADIUS`]).
//!   É o degrau que o Tailwind v4 acrescentou abaixo do `rounded-sm`, e nem o coss nem o lumiui o
//!   sobrescrevem (só redefinem `--radius-sm`/`-md`/`-lg`/`-xl` a partir do `--radius`), então vale o
//!   valor de fábrica.
//! - `bg-primary/40` → [`PRIMARY_ALPHA`] = **0,40**, aplicado com [`Rgba8::scaled`]. O `/N` do
//!   Tailwind v4 **multiplica** o alfa que o token já tem; aqui o `--primary` é opaco, então o
//!   produto dá 0,40 — mas a multiplicação está escrita como multiplicação, e não como um alfa
//!   cravado, pra a regra continuar certa se o token deixar de ser opaco.
//! - `-left-1.5` / `-top-1.5` da área de clique → **−6px**, e `after:w-4`/`after:h-4` → 16px. Números
//!   que ficaram só no doc, porque o alvo não é nosso (ver "Não reproduzível"): declará-los como
//!   `const` seria constante morta.
//! - Passo das setas → [`ARROW_STEP`] = **5 pontos percentuais** do grupo (`adjustLayoutForSeparator(el, ±5)`).
//!
//! ## Desvio consciente
//!
//! - **Cor, raio e borda são NOSSOS tokens**, como em todo porte desta lib — o componente vem de um
//!   design system diferente (lumiui) e tem que ler como irmão dos outros 39. `bg-border` e a borda do
//!   puxador viram o mesmo [`theme::Palette::border_divider`] que o [`crate::Group`] usa no separador
//!   dele; `bg-background` vira [`theme::BG_PANEL`]; `text-muted-foreground` vira
//!   [`theme::TEXT_MUTED`]; `text-foreground` vira [`theme::TEXT_VALUE`]; `bg-primary/40` vira
//!   [`theme::PRIMARY`] a 40%.
//! - **O separador não ocupa espaço no layout.** Na fonte ele é um irmão flex de 1px entre os painéis,
//!   e os painéis somam `100% − 1px × separadores`. No núcleo a divisa é absoluta e os painéis
//!   encostam — a linha é pintada SOBRE a emenda. Consequência visível: com dois painéis de 200px o
//!   grupo tem 400px, não 401px.
//! - **O `Home`/`End` e as setas movem só o PAR de vizinhos da divisa.** O arraste do núcleo cascateia
//!   (empurra o terceiro painel quando o segundo encosta no mínimo); o nosso [`move_divider`] troca
//!   tamanho apenas entre os painéis `ix` e `ix+1`. É o que se espera de uma seta, e evita duplicar a
//!   cascata do núcleo — que é exatamente o que este módulo não deve reescrever. Com dois painéis os
//!   dois comportamentos coincidem.
//! - **O mínimo default é 100px**, não `0%` como na referência: é o `PANEL_MIN_SIZE` do núcleo
//!   (`vendor/gpui-component/src/resizable/mod.rs:14`), e é o valor que vale quando ninguém chama
//!   [`ResizablePanel::min_size`]. Passar `min_size(0.0)` é permitido e chega ao núcleo, mas aí um
//!   painel de tamanho zero deixa de ganhar o `flex_none` do núcleo — então quem quer colapsar de
//!   verdade deve usar [`ResizablePanel::visible`].
//! - **O separador não é filho do painel, e sim irmão posterior ao grupo** (ver "Como o puxador fica
//!   POR CIMA"). Na fonte ele é filho do grupo, então isto até se aproxima dela; o que muda em
//!   relação a uma leitura ingênua do porte é que a posição dele vem de uma MEDIDA
//!   ([`Deslocado`]), e não do fluxo.
//! - **Esconder um painel esconde a divisa que vem ANTES dele.** A divisa é a borda de entrada do
//!   painel de depois dela (é onde o núcleo põe a alça), e um painel invisível não renderiza filho
//!   nenhum — nem a [`ancora`]. Sem âncora, sem divisa: o [`Deslocado`] não pinta o filho. Então
//!   `visible(false)` no painel do meio de três deixa uma divisa, não duas — o que é o desenho certo,
//!   e agora por construção nossa (antes era efeito colateral de o separador morar dentro do painel).
//! - **O separador não pinta anel de foco.** Não é omissão: a fonte também não — o
//!   `focus-visible:bg-primary/40` recolore a própria linha, e é isso que fazemos, com o
//!   [`crate::focus_ring`] decidindo se o foco veio do teclado. Um anel de 2px em volta de uma linha
//!   de 1px não é o desenho da referência.
//!
//! ## Superset consciente
//!
//! - Nada. A API pública tem exatamente as props do lumiui usadas nos exemplos (`orientation`,
//!   `withHandle`, `defaultSize`, `minSize`, `maxSize`), mais o [`Resizable::on_resize`] que o núcleo
//!   já oferecia.
//!
//! ## Ausente
//!
//! - `collapsible` / `collapsedSize` / `Enter` / `F6` — ver "Não reproduzível".
//! - `autoSaveId` (persistir o layout em `localStorage`). Não há equivalente de storage nesta lib, e
//!   quem quiser persistir tem o [`Resizable::on_resize`] com os tamanhos em pixel.
//! - `disabled` no separador (existe no `react-resizable-panels`, não na API do lumiui).
//!
//! ## Sem cobertura de teste — declarado
//!
//! - **A ordem de pintura ENTRE A LINHA E O PUXADOR.** É o `z-10` local da fonte, e depois da troca
//!   ele é só ordem de filhos: o puxador é filho da linha. Isso não é falsificável por teste — o
//!   `VisualTestContext` não dá acesso à lista de quads do frame, e não há como afirmar "este pixel é
//!   do puxador". O que dá pra afirmar, e está afirmado, é o degrau que de fato quebrava na tela: que
//!   o separador INTEIRO pinta depois da alça do núcleo
//!   ([`tests_de_janela::o_separador_fica_por_cima_da_alca_do_nucleo`], medido pela fila de hitboxes)
//!   e que ele não pinta fora do grupo
//!   ([`tests_de_janela::o_separador_nao_pinta_fora_do_container_rolavel`], medido pela
//!   [`gpui::ContentMask`] em vigor no paint).
//! - **A cor efetiva de cada estado.** As cores entram como `Hsla` no elemento e o teste não lê de
//!   volta o estilo pintado. O que os testes cobrem é a função que ESCOLHE ([`highlighted`]) e a
//!   aritmética do alfa. ⚠️ O que MUDOU aqui é que a cor voltou a ter efeito: com o separador embaixo
//!   da alça do núcleo, o `border` opaco dele cobriria a nossa linha e o realce de `primary/40` não
//!   apareceria — a cor certa dependia da ordem de pintura, e é a ordem que agora tem teste.
//! - **A prioridade que o `deferred` tinha.** Foi embora com ele: `SEPARATOR_PRIORITY` era a tradução
//!   do `z-10` contra o `z-50` dos popups, e sem `deferred` não há prioridade nenhuma pra ordenar —
//!   o separador pinta na ordem da árvore, dentro do grupo, e nunca por cima de um popup (que é
//!   diferido). A constante e o teste dela foram **removidos** em vez de virarem número morto.
//! - **O reajuste ao container e a cascata do arraste** — são do núcleo, e o núcleo tem os testes
//!   dele. Aqui só se testa que o arraste CHEGA nele (o teste de janela move a divisa de verdade).

use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

use gpui::{
    canvas, div, prelude::FluentBuilder as _, px, svg, AnyElement, App, AppContext as _, Bounds,
    Context, Element, ElementId, Entity, FocusHandle, GlobalElementId, InspectorElementId,
    InteractiveElement as _, IntoElement, KeyDownEvent, LayoutId, MouseDownEvent, MouseUpEvent,
    ParentElement, Pixels, RenderOnce, Styled, Window,
};
use gpui_component::resizable::{
    h_resizable, resizable_panel, v_resizable, ResizablePanel as CorePanel, ResizableState,
};

use crate::color::{opaque, Rgba8};
use crate::group::Orientation;
use crate::theme;

// =================================================================================================
// Medidas — "Resolvido em número" do doc do módulo
// =================================================================================================

/// Espessura da linha do separador — `w-px` / `h-px`.
const LINE: f32 = 1.0;

/// O lado do puxador **no sentido da linha** — `aria-vertical:h-4` / `aria-horizontal:w-4`.
const GRIP_ALONG: f32 = 16.0;

/// O lado do puxador que **atravessa** a linha — `aria-vertical:w-3` / `aria-horizontal:h-3`.
const GRIP_ACROSS: f32 = 12.0;

/// Lado do ícone do puxador — `size-3`.
const GRIP_ICON: f32 = 12.0;

/// Raio do puxador — `rounded-xs` = `--radius-xs` = `0.125rem` = 2px.
const GRIP_RADIUS: f32 = 2.0;

/// Alfa da linha realçada — o `/40` de `bg-primary/40`, aplicado por [`Rgba8::scaled`].
const PRIMARY_ALPHA: f32 = 0.40;

/// Passo das setas: **5 pontos percentuais** do tamanho do grupo.
const ARROW_STEP: f32 = 0.05;

/// Onde começa a faixa que inicia o arraste, em pixels **a partir da divisa**.
///
/// Não é escolha nossa: é a geometria do `resize_handle` do núcleo (`left: -HANDLE_PADDING`, com
/// `HANDLE_PADDING = 4px`). Está aqui porque o realce de "separador ativo" precisa acender
/// exatamente quando o arraste começa, e o estado interno do handle é privado. Medido em
/// [`tests_de_janela::a_faixa_de_arraste_do_nucleo_e_a_que_esta_declarada`].
const HIT_START: f32 = -4.0;

/// Largura da faixa que inicia o arraste — `HANDLE_PADDING` de cada lado da linha de 1px, que o
/// `box-sizing: border-box` do taffy espreme para `padding + border` = 8px.
const HIT_SIZE: f32 = 8.0;

/// Mínimo default de um painel — o `PANEL_MIN_SIZE` do núcleo
/// (`vendor/gpui-component/src/resizable/mod.rs:14`), repetido aqui porque é `pub(crate)` lá.
const MIN_SIZE_DEFAULT: f32 = 100.0;

/// O puxador de um separador **vertical** (grupo horizontal).
const GRIP_VERTICAL: &str = "icons/grip_vertical.svg";

/// O puxador de um separador **horizontal** (grupo vertical).
///
/// Na fonte não existe segundo ícone: é o mesmo `GripVerticalIcon` com `rotate-90`. O GPUI não tem
/// `transform` em `div` nem em `svg`, então o giro está no PATH — os mesmos seis pontos do
/// `grip_vertical.svg` transpostos.
const GRIP_HORIZONTAL: &str = "icons/grip_horizontal.svg";

// =================================================================================================
// Paleta
// =================================================================================================

/// Tokens do separador. Convenção da casa: `0xRRGGBBAA` (ver [`crate::color`]).
struct ResizablePalette {
    /// `bg-border` na linha **e** `border` na moldura do puxador — na fonte é o mesmo token, então
    /// aqui é o mesmo campo. É o [`theme::Palette::border_divider`], o mesmo tom que o
    /// [`crate::Group`] dá ao separador dele.
    border: Rgba8,
    /// `--primary` — a linha quando a divisa está sendo arrastada ou tem foco de teclado. Entra
    /// sempre multiplicada por [`PRIMARY_ALPHA`].
    primary: Rgba8,
    /// `bg-background` — o fundo do puxador, que é o que o faz "furar" a linha.
    grip_bg: Rgba8,
    /// `text-muted-foreground` — o ícone do puxador em repouso.
    grip_icon: Rgba8,
    /// `text-foreground` — o ícone com o separador realçado.
    grip_icon_on: Rgba8,
}

fn palette() -> ResizablePalette {
    ResizablePalette {
        // Os tokens `border_*` do tema já são de 8 dígitos (têm alfa), então entram direto — o
        // `opaque` é só pros de 6.
        border: Rgba8(theme::palette().border_divider),
        primary: opaque(theme::PRIMARY()),
        grip_bg: opaque(theme::BG_PANEL()),
        grip_icon: opaque(theme::TEXT_MUTED()),
        grip_icon_on: opaque(theme::TEXT_VALUE()),
    }
}

// =================================================================================================
// A inversão do `aria-orientation`, num lugar só
// =================================================================================================

/// As medidas de UM separador, resolvidas a partir da orientação do **grupo**.
///
/// Existe pra que a inversão do `aria-orientation` (ver o doc do módulo) aconteça uma vez, numa
/// função pura e testável, e não espalhada em `if`s de render — que é onde ela seria invisível.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct SeparatorMetrics {
    /// A orientação **ARIA** do separador: o INVERSO da do grupo.
    pub(crate) aria: Orientation,
    /// Largura do puxador, em px.
    pub(crate) grip_w: f32,
    /// Altura do puxador, em px.
    pub(crate) grip_h: f32,
    /// O SVG do puxador, já girado (o GPUI não gira `div`).
    pub(crate) grip_icon: &'static str,
}

/// Resolve as medidas do separador de um grupo de orientação `group`.
pub(crate) const fn separator_metrics(group: Orientation) -> SeparatorMetrics {
    match group {
        // Grupo HORIZONTAL = painéis lado a lado = divisa em pé. `aria-orientation="vertical"`.
        Orientation::Horizontal => SeparatorMetrics {
            aria: Orientation::Vertical,
            grip_w: GRIP_ACROSS,
            grip_h: GRIP_ALONG,
            grip_icon: GRIP_VERTICAL,
        },
        // Grupo VERTICAL = painéis empilhados = divisa deitada. `aria-orientation="horizontal"`.
        Orientation::Vertical => SeparatorMetrics {
            aria: Orientation::Horizontal,
            grip_w: GRIP_ALONG,
            grip_h: GRIP_ACROSS,
            grip_icon: GRIP_HORIZONTAL,
        },
    }
}

/// Se a linha do separador deve ser realçada (`bg-primary/40`).
///
/// Na fonte há duas regras com o MESMO efeito, e é por isso que elas cabem num predicado só:
/// `data-[separator='active']` (arrastando) e `focus-visible` (foco vindo do teclado). O segundo
/// passa pelo [`crate::focus_ring`], que é o `:focus-visible` desta lib — sem ele, clicar no
/// separador o focaria e a linha acenderia como se fosse teclado.
pub(crate) fn highlighted(active: bool, focused: bool, keyboard: bool) -> bool {
    active || (focused && keyboard)
}

// =================================================================================================
// O solver do teclado
// =================================================================================================

/// Tradução tecla → deslocamento da divisa, em **pixels**. `None` = a tecla não é nossa.
///
/// `aria` é a orientação do SEPARADOR (já invertida), e é ela que decide o par de setas: uma divisa
/// em pé (`aria` vertical, grupo horizontal) anda com `←`/`→`. A seta do eixo cruzado devolve `None`,
/// como na referência (*"Up/down are no-ops"* nos testes de integração dela).
///
/// `container` é o tamanho do grupo no eixo. As setas andam [`ARROW_STEP`] dele; `Home`/`End` são
/// `∓100` pontos percentuais na referência, o que satura em qualquer limite — aqui viram `∓container`,
/// que o [`move_divider`] apara no mínimo/máximo do painel.
pub(crate) fn key_delta(key: &str, aria: Orientation, container: f32) -> Option<f32> {
    let step = container * ARROW_STEP;
    match (aria, key) {
        (Orientation::Vertical, "left") => Some(-step),
        (Orientation::Vertical, "right") => Some(step),
        (Orientation::Horizontal, "up") => Some(-step),
        (Orientation::Horizontal, "down") => Some(step),
        (_, "home") => Some(-container),
        (_, "end") => Some(container),
        _ => None,
    }
}

/// Move a divisa `ix` — a que separa os painéis `ix` e `ix+1` — por `delta` pixels, e devolve os
/// tamanhos novos.
///
/// O par troca tamanho entre si: o que um ganha, o outro perde. Os limites dos **dois** entram na
/// mesma aparagem, porque crescer o de antes é encolher o de depois: o teto do painel `ix` é o menor
/// entre o `max` dele e `total − min` do vizinho.
///
/// Se as restrições forem impossíveis de satisfazer (`min` dos dois somando mais que o par tem),
/// devolve os tamanhos intactos — melhor não mexer que estourar num `clamp` invertido.
pub(crate) fn move_divider(
    sizes: &[Pixels],
    ranges: &[Range<f32>],
    ix: usize,
    delta: f32,
) -> Vec<Pixels> {
    let mut out = sizes.to_vec();
    if delta == 0.0 || ix + 1 >= sizes.len() || ranges.len() != sizes.len() {
        return out;
    }

    let (a, b) = (f32::from(sizes[ix]), f32::from(sizes[ix + 1]));
    let total = a + b;
    let (antes, depois) = (&ranges[ix], &ranges[ix + 1]);

    let piso = antes.start.max(total - depois.end);
    let teto = antes.end.min(total - depois.start);
    if piso > teto {
        return out;
    }

    let novo = (a + delta).clamp(piso, teto);
    out[ix] = px(novo);
    out[ix + 1] = px(total - novo);
    out
}

// =================================================================================================
// Estado
// =================================================================================================

#[cfg(test)]
thread_local! {
    /// **Sonda de teste** do RECORTE do separador.
    ///
    /// Grava, no paint do separador, o par `(retângulo do separador, retângulo da
    /// [`gpui::ContentMask`] em vigor)`. É a única forma de afirmar "o separador não pinta fora do
    /// grupo" sem acesso à cena: a máscara **é** o recorte que o rasterizador aplica, e o que
    /// aparece na tela é a interseção dos dois. Ver
    /// [`tests_de_janela::o_separador_nao_pinta_fora_do_container_rolavel`].
    static RECORTE_PROBE: std::cell::Cell<Option<(Bounds<Pixels>, Bounds<Pixels>)>> =
        const { std::cell::Cell::new(None) };

    /// **Sonda de teste** da ORDEM DE PINTURA contra a alça do núcleo.
    ///
    /// Grava se um [`gpui::Hitbox`] inserido de dentro do separador está `is_hovered` com o ponteiro
    /// sobre a divisa. A pergunta parece de mouse e é de ordem de pintura: a alça do núcleo é
    /// `occlude()`, ou seja um hitbox `BlockMouse`, e o `hit_test` do GPUI varre os hitboxes de
    /// TRÁS PRA FRENTE e **para** no primeiro que bloqueia (`gpui-0.2.2/src/window.rs:775`). Então o
    /// nosso só é alcançado se tiver sido inserido DEPOIS do da alça — e a inserção acontece no
    /// prepaint, na mesma ordem da árvore em que se pinta. Ver
    /// [`tests_de_janela::o_separador_fica_por_cima_da_alca_do_nucleo`].
    static ORDEM_PROBE: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };

    /// **Sonda de teste** do realce de arraste.
    ///
    /// O `active` vive num [`gpui::Window::use_keyed_state`], que é estado de ELEMENTO: o teste de
    /// janela não tem a chave nem a entidade, então não há como lê-lo de fora. Sem esta sonda o
    /// caminho "pressionar dentro da faixa do núcleo → separador ativo" ficaria sem cobertura, e é
    /// justamente ele que depende dos números medidos em [`HIT_START`]/[`HIT_SIZE`].
    static ACTIVE_PROBE: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

/// O estado que o [`Resizable`] guarda entre frames.
///
/// Um só, e não dois, porque [`gpui::Window::use_keyed_state`] guarda por CHAVE: duas chamadas com a
/// mesma chave e tipos diferentes se atropelariam.
struct ResizableStore {
    /// O estado do NÚCLEO (tamanhos e arraste). É nosso pra podermos reiniciá-lo no teclado.
    core: Entity<ResizableState>,
    /// Qual divisa está sendo arrastada — o `data-separator="active"` da fonte.
    active: Option<usize>,
    /// Os tamanhos que o teclado impôs, esperando o frame do reinício (ver o doc do módulo).
    keyboard: Option<Vec<Pixels>>,
    /// Um handle de foco por DIVISA (`panels − 1`).
    focus: Vec<FocusHandle>,
}

impl ResizableStore {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let core = cx.new(|_| ResizableState::default());
        // ⚠️ **Aqui NÃO vai um `cx.observe(&core, …)`**, e a ausência é deliberada.
        //
        // O núcleo observa o estado dele quando o cria sozinho (`use_keyed_state` no
        // `ResizablePanelGroup`), o que sugere que sem isso o arraste não repintaria. Não é o caso: o
        // GPUI **já** repinta a janela a cada `MouseMoveEvent` enquanto existe um arraste ativo, e no
        // `MouseUpEvent` (`gpui-0.2.2/src/window.rs:3716`, `if cx.has_active_drag()`). O teste
        // [`tests_de_janela::o_arraste_move_a_divisa_e_avisa_ao_soltar`] mede a divisa **com o botão
        // ainda apertado**, então é essa garantia que ele verifica.
        //
        // E observar teria um custo real: o `update_panel_size` do núcleo chama `cx.notify()` a CADA
        // frame (é o canvas de medição de cada painel). Encadear isso num `notify` da nossa view
        // deixaria a janela suja para sempre — uma bomba de redesenho, sem nada em troca.
        Self {
            core,
            active: None,
            keyboard: None,
            focus: Vec::new(),
        }
    }

    /// Garante um handle de foco por divisa. Cresce e encurta com a lista de painéis.
    fn sync_separators(&mut self, divisas: usize, cx: &mut Context<Self>) {
        while self.focus.len() < divisas {
            self.focus.push(cx.focus_handle());
        }
        self.focus.truncate(divisas);
    }

    /// Liga/desliga o realce de arraste. Só pede frame novo quando o valor MUDA — sem isso, cada
    /// `mouse up` da janela sujaria o grupo inteiro.
    fn set_active(&mut self, ix: Option<usize>, cx: &mut Context<Self>) {
        #[cfg(test)]
        ACTIVE_PROBE.with(|p| p.set(ix));
        if self.active != ix {
            self.active = ix;
            cx.notify();
        }
    }

    /// Aplica tamanhos vindos do teclado: guarda-os e **reinicia** o estado do núcleo, pra que o
    /// `size` declarado de cada painel volte a valer no próximo frame (ver o doc do módulo).
    fn apply_keyboard_sizes(&mut self, sizes: Vec<Pixels>, cx: &mut Context<Self>) {
        self.keyboard = Some(sizes);
        self.core.update(cx, |state, _| *state = ResizableState::default());
        cx.notify();
    }
}

// =================================================================================================
// ResizablePanel
// =================================================================================================

/// Um painel de um [`Resizable`]. Porte do `ResizablePanel` do lumiui.
///
/// **Todos os tamanhos são em PIXELS** — ver o doc do módulo pra por quê (é o que a referência faz
/// com prop numérica).
pub struct ResizablePanel {
    /// `defaultSize`. `None` = o núcleo distribui.
    size: Option<f32>,
    /// `minSize` / `maxSize`.
    range: Range<f32>,
    /// Se o painel é desenhado. `false` o remove sem tirá-lo da lista (o índice das divisas continua
    /// valendo).
    visible: bool,
    children: Vec<AnyElement>,
}

impl Default for ResizablePanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ResizablePanel {
    /// Um painel sem tamanho declarado: o núcleo lhe dá o que sobrar.
    pub fn new() -> Self {
        Self {
            size: None,
            range: MIN_SIZE_DEFAULT..f32::MAX,
            visible: true,
            children: Vec::new(),
        }
    }

    /// `defaultSize` — o tamanho inicial, em **pixels**.
    pub fn size(mut self, pixels: f32) -> Self {
        self.size = Some(pixels);
        self
    }

    /// `minSize` — o menor tamanho, em **pixels**. Default: [`MIN_SIZE_DEFAULT`].
    pub fn min_size(mut self, pixels: f32) -> Self {
        self.range.start = pixels;
        self
    }

    /// `maxSize` — o maior tamanho, em **pixels**. Default: sem teto.
    pub fn max_size(mut self, pixels: f32) -> Self {
        self.range.end = pixels;
        self
    }

    /// Se o painel é desenhado (default `true`).
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
}

impl ParentElement for ResizablePanel {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

// =================================================================================================
// Resizable (o grupo)
// =================================================================================================

/// Painéis redimensionáveis. Porte do `ResizableGroup` do lumiui.
///
/// É um **elemento de render** ([`RenderOnce`]): construa a cada frame.
///
/// ```ignore
/// Resizable::horizontal("editor")
///     .with_handle(true)
///     .child(ResizablePanel::new().size(240.0).min_size(180.0).child(arvore()))
///     .child(ResizablePanel::new().min_size(320.0).child(
///         Resizable::vertical("editor-direita")           // aninhado: grupo dentro de painel
///             .child(ResizablePanel::new().child(palco()))
///             .child(ResizablePanel::new().size(160.0).child(console())),
///     ))
/// ```
#[derive(IntoElement)]
pub struct Resizable {
    id: ElementId,
    orientation: Orientation,
    with_handle: bool,
    separator_hidden: bool,
    panels: Vec<ResizablePanel>,
    #[allow(clippy::type_complexity)]
    on_resize: Option<Box<dyn Fn(&[Pixels], &mut Window, &mut App) + 'static>>,
}

impl Resizable {
    /// Um grupo **horizontal** (painéis lado a lado) — o default da fonte.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            orientation: Orientation::Horizontal,
            with_handle: false,
            separator_hidden: false,
            panels: Vec::new(),
            on_resize: None,
        }
    }

    /// Atalho de [`Resizable::new`] — painéis lado a lado.
    pub fn horizontal(id: impl Into<ElementId>) -> Self {
        Self::new(id)
    }

    /// Painéis empilhados.
    pub fn vertical(id: impl Into<ElementId>) -> Self {
        Self::new(id).orientation(Orientation::Vertical)
    }

    /// `orientation`.
    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// `withHandle` — desenha o puxador de seis pontos no meio de cada divisa.
    /// Esconde a divisa **em repouso**, mantendo a área de arraste.
    ///
    /// Não existe na referência: é **superset consciente**, e nasceu de um caso real — a divisa entre a
    /// sidebar e o conteúdo do nosso storybook, onde uma linha de 1px atravessando o vão entre duas
    /// superfícies de fundos diferentes fica pior que nada. É o mesmo tratamento que o Zed dá à divisa
    /// de dock.
    ///
    /// ⚠️ **Esconder é "em repouso", não "sempre", e a diferença é deliberada.** O realce de
    /// `primary/40` continua aparecendo quando a divisa está focada por teclado ou sendo arrastada —
    /// sem isso, dar `Tab` até ela não teria retorno NENHUM na tela, e o componente perderia a
    /// acessibilidade que a gente construiu por cima do núcleo. Some também o puxador, porque puxador
    /// visível numa divisa invisível é contradição.
    pub fn separator_hidden(mut self, hidden: bool) -> Self {
        self.separator_hidden = hidden;
        self
    }

    pub fn with_handle(mut self, with_handle: bool) -> Self {
        self.with_handle = with_handle;
        self
    }

    /// Acrescenta um painel.
    pub fn child(mut self, panel: ResizablePanel) -> Self {
        self.panels.push(panel);
        self
    }

    /// Acrescenta vários painéis.
    pub fn children(mut self, panels: impl IntoIterator<Item = ResizablePanel>) -> Self {
        self.panels.extend(panels);
        self
    }

    /// Chamado quando o usuário **solta** a divisa, com os tamanhos em pixels na ordem dos painéis.
    ///
    /// É o gancho pra persistir layout (o `autoSaveId` da referência não tem equivalente aqui).
    pub fn on_resize(
        mut self,
        on_resize: impl Fn(&[Pixels], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_resize = Some(Box::new(on_resize));
        self
    }
}

impl RenderOnce for Resizable {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = separator_metrics(self.orientation);
        let store = window.use_keyed_state(self.id.clone(), cx, ResizableStore::new);

        let divisas = self.panels.len().saturating_sub(1);
        let (core, active, keyboard, focus) = store.update(cx, |s, cx| {
            s.sync_separators(divisas, cx);
            (
                s.core.clone(),
                s.active,
                s.keyboard.clone(),
                s.focus.clone(),
            )
        });

        // Os limites de cada painel, na forma que o solver do teclado consome.
        let ranges: Vec<Range<f32>> = self.panels.iter().map(|p| p.range.clone()).collect();

        let mut grupo = match self.orientation {
            Orientation::Horizontal => h_resizable(self.id.clone()),
            Orientation::Vertical => v_resizable(self.id.clone()),
        }
        .with_state(&core);

        if let Some(on_resize) = self.on_resize {
            grupo = grupo.on_resize(move |state, window, cx| {
                let sizes = state.read(cx).sizes().clone();
                on_resize(&sizes, window, cx);
            });
        }

        // Onde cada divisa é MEDIDA neste mesmo frame — ver [`ancora`] e [`Deslocado`].
        let ancoras: Ancoras = Rc::new(RefCell::new(vec![None; divisas]));

        let grupo = grupo.children(self.panels.into_iter().enumerate().map(|(ix, panel)| {
            // O tamanho declarado, ou o que o teclado impôs no frame do reinício.
            let declarado = keyboard
                .as_ref()
                .and_then(|v| v.get(ix).copied())
                .or_else(|| panel.size.map(px));

            let mut core_panel: CorePanel = resizable_panel()
                .visible(panel.visible)
                .size_range(px(panel.range.start)..px(panel.range.end));
            if let Some(size) = declarado {
                core_panel = core_panel.size(size);
            }
            let mut core_panel = core_panel.children(panel.children);

            // Dentro do painel fica só a ÂNCORA — o separador em si é irmão POSTERIOR ao grupo (ver
            // "Como o puxador fica POR CIMA"). A divisa `ix-1` é a borda de entrada do painel `ix`,
            // que é também onde o núcleo pendura a alça dele.
            if ix > 0 {
                core_panel = core_panel.child(ancora(&ancoras, ix - 1));
            }
            core_panel
        }));

        // `focus` tem exatamente uma entrada por divisa — é o que o `sync_separators` garante — então
        // iterar por ele é iterar pelas divisas.
        let mut separadores = Vec::with_capacity(focus.len());
        for (ix, handle) in focus.iter().enumerate() {
            separadores.push(separador(
                ix,
                metrics,
                puxador_visivel(self.with_handle, self.separator_hidden),
                self.separator_hidden,
                active == Some(ix),
                handle.clone(),
                core.clone(),
                store.clone(),
                ranges.clone(),
                ancoras.clone(),
                window,
            ));
        }

        // ⚠️ **A ORDEM DESTES DOIS FILHOS É O `z-index` DO COMPONENTE.** O grupo primeiro (e com ele
        // as alças do núcleo, cada uma pintando a linha opaca dela como ÚLTIMO filho do painel dela),
        // os separadores depois. Sendo filhos normais de um `div` normal, eles herdam a `ContentMask`
        // dos ancestrais — é justamente o que o `deferred` não fazia.
        //
        // O `relative` é o que faz o `absolute` de cada separador se medir contra o GRUPO. É de lá que
        // sai a extensão no eixo que ATRAVESSA a divisa (`top_0().bottom_0()`); a posição NO eixo da
        // divisa é o [`Deslocado`] que resolve.
        div()
            .relative()
            .size_full()
            .child(grupo)
            .children(separadores)
    }
}

// =================================================================================================
// A âncora e o deslocamento — como o separador sai do painel sem perder a divisa de vista
// =================================================================================================

/// Onde cada divisa foi MEDIDA no frame corrente. Uma entrada por divisa; `None` = o painel de
/// depois dela não renderizou (está invisível), logo não há divisa pra desenhar.
type Ancoras = Rc<RefCell<Vec<Option<Bounds<Pixels>>>>>;

/// A **âncora** da divisa `ix`: um canvas invisível dentro do painel de DEPOIS dela, que grava onde
/// o taffy pôs esse painel.
///
/// É a única coisa que este módulo ainda põe dentro do painel do núcleo, e ela não pinta nada. O
/// canto superior esquerdo do painel **é** a divisa: para um grupo horizontal ele dá o `x` da
/// emenda, e para um vertical o `y` — nos dois casos com a mesma expressão, porque é o mesmo canto.
fn ancora(ancoras: &Ancoras, ix: usize) -> impl IntoElement {
    let ancoras = ancoras.clone();
    canvas(
        move |bounds, _, _| {
            if let Some(slot) = ancoras.borrow_mut().get_mut(ix) {
                *slot = Some(bounds);
            }
        },
        |_, _, _, _| {},
    )
    .absolute()
    .size_full()
}

/// Desloca o filho até a âncora da divisa `ix`, **no mesmo frame** em que ela foi medida.
///
/// # Por que um elemento, e não um `left(px(x))`
///
/// O separador precisa de duas coisas que não cabem no mesmo lugar da árvore: ser pintado DEPOIS do
/// grupo (senão a alça do núcleo pinta a linha dela em cima dele) e ficar EM CIMA da emenda entre
/// dois painéis (que só o taffy sabe onde é). Um `div` irmão do grupo resolve a primeira e não a
/// segunda: o `left` de um `absolute` é decidido no `render`, e ali só existem as medidas do frame
/// ANTERIOR — durante um arraste isso é a linha do núcleo andando na hora e a nossa um frame atrás.
///
/// A ordem das fases do GPUI resolve isso sem medida velha: o `request_layout` da árvore INTEIRA
/// acontece antes de qualquer `prepaint`, e o `prepaint` corre na ordem da árvore. Então quando o
/// `prepaint` deste elemento roda, as âncoras — que estão nos painéis, ANTES na árvore — já
/// gravaram as medidas DESTE frame. O que falta é mover o filho até lá, e é só isso que este
/// elemento faz: [`gpui::Window::with_element_offset`], que é o mesmo mecanismo com que o próprio
/// GPUI posiciona alças de arraste.
///
/// **Ele não muda o contexto de recorte.** O filho é prepintado e pintado dentro da pilha de
/// máscaras do frame, no lugar dele na árvore — ao contrário de [`gpui::deferred`], que pinta em
/// `paint_deferred_draws`, depois de a pilha ter sido desempilhada.
///
/// Sem âncora (painel invisível) o filho não é prepintado **nem** pintado: uma divisa cujo painel de
/// depois não existe não tem onde ficar.
struct Deslocado {
    filho: Option<AnyElement>,
    ancoras: Ancoras,
    ix: usize,
}

impl IntoElement for Deslocado {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Deslocado {
    type RequestLayoutState = ();
    /// Se o filho foi prepintado — o `paint` não pode pintar o que não prepintou.
    type PrepaintState = bool;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        // Devolve o `LayoutId` do PRÓPRIO filho, como o `Deferred` do GPUI faz: o taffy então
        // posiciona o filho pelo estilo dele (absoluto, no canto do grupo) e o `bounds` que chega no
        // `prepaint` é exatamente esse retângulo — de onde sai o delta.
        (self.filho.as_mut().unwrap().request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        let Some(alvo) = self.ancoras.borrow().get(self.ix).copied().flatten() else {
            return false;
        };
        let delta = alvo.origin - bounds.origin;
        window.with_element_offset(delta, |window| {
            self.filho.as_mut().unwrap().prepaint(window, cx);
        });
        true
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        prepintado: &mut bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        if *prepintado {
            self.filho.as_mut().unwrap().paint(window, cx);
        }
    }
}

// =================================================================================================
// O separador
// =================================================================================================

/// A casca visual de UMA divisa: a linha, o puxador e as teclas.
///
/// Mora **fora** do painel, como irmão posterior ao grupo, e é levado até a emenda pelo
/// [`Deslocado`] — ver o doc dele e "Como o puxador fica POR CIMA" no doc do módulo. Dentro dele a
/// ordem dos filhos é o `z-10` da fonte: a linha pinta o fundo dela, e o puxador, sendo filho, pinta
/// depois.
///
/// As medidas continuam sendo escritas como se o pai fosse o painel (`top_0().bottom_0()`), e
/// continuam certas: o irmão do grupo tem o retângulo do GRUPO, e no eixo que ATRAVESSA a divisa o
/// painel e o grupo têm a mesma extensão.
/// O puxador sai ou não.
///
/// É função e não um `&&` no call site pra o teste poder APONTAR pra ela: escrita inline, a expressão
/// só seria alcançável por teste de janela lendo a cena, e um teste que repetisse `with_handle &&
/// !hidden` estaria verificando a si mesmo. Mesma decisão do `dim_visible` do [`crate::input`].
fn puxador_visivel(with_handle: bool, separator_hidden: bool) -> bool {
    with_handle && !separator_hidden
}

/// A cor da linha da divisa.
///
/// Extraída do render porque é decisão, e decisão embutida num `if` de render é inalcançável por teste.
/// Três casos e não dois: `hidden` só apaga o REPOUSO — realçado (focado por teclado ou sendo
/// arrastado) continua acendendo, senão `separator_hidden` custaria a acessibilidade que o teclado
/// deste módulo construiu. Ver [`Resizable::separator_hidden`].
fn cor_da_linha(realcado: bool, hidden: bool, p: &ResizablePalette) -> gpui::Hsla {
    if realcado {
        p.primary.scaled(PRIMARY_ALPHA)
    } else if hidden {
        gpui::transparent_black()
    } else {
        p.border.hsla()
    }
}

#[allow(clippy::too_many_arguments)]
fn separador(
    ix: usize,
    metrics: SeparatorMetrics,
    with_handle: bool,
    separator_hidden: bool,
    active: bool,
    focus: FocusHandle,
    core: Entity<ResizableState>,
    store: Entity<ResizableStore>,
    ranges: Vec<Range<f32>>,
    ancoras: Ancoras,
    window: &mut Window,
) -> AnyElement {
    let p = palette();
    // Cada separador é uma parada de `Tab`, como na referência (`tabIndex={0}` em todos). O
    // `tab_stop` tem que estar no handle ANTES do `track_focus`, porque é o `track_focus` que copia
    // o valor pra tabela de paradas do frame (a mesma armadilha documentada no `toggle_group`).
    let focus = focus.tab_index(0).tab_stop(true);
    let realcado = highlighted(active, focus.is_focused(window), crate::focus_ring::visible());

    let linha = div()
        .track_focus(&focus)
        .absolute()
        .flex()
        .items_center()
        .justify_center()
        .bg(cor_da_linha(realcado, separator_hidden, &p))
        .map(|d| match metrics.aria {
            // Divisa em pé: espessura no X, esticada no Y (`w-px h-auto`).
            Orientation::Vertical => d.top_0().bottom_0().left_0().w(px(LINE)),
            // Divisa deitada: espessura no Y, largura cheia (`h-px w-full`).
            Orientation::Horizontal => d.left_0().right_0().top_0().h(px(LINE)),
        })
        .on_key_down({
            let core = core.clone();
            let store = store.clone();
            move |event: &KeyDownEvent, window, cx| {
                let sizes = core.read(cx).sizes().clone();
                let container: f32 = sizes.iter().copied().map(f32::from).sum();
                let Some(delta) = key_delta(event.keystroke.key.as_str(), metrics.aria, container)
                else {
                    return;
                };
                let novos = move_divider(&sizes, &ranges, ix, delta);
                // A tecla é nossa mesmo quando ela não move nada (a divisa já está no limite):
                // deixar propagar faria a seta rolar o container atrás.
                cx.stop_propagation();
                crate::focus_ring::keyboard_used(window);
                if novos != sizes {
                    store.update(cx, |s, cx| s.apply_keyboard_sizes(novos, cx));
                }
            }
        })
        // A sonda do estado ATIVO. Não é uma área de clique: o arraste é do núcleo (ver "Não
        // reproduzível"). Ela só escuta a janela pra saber quando a faixa do núcleo foi pressionada,
        // porque o `active` interno do `resize_handle` é privado.
        .child(
            canvas(
                |bounds, _, _| bounds,
                move |_, bounds, window, _cx| {
                    #[cfg(test)]
                    RECORTE_PROBE
                        .with(|p| p.set(Some((bounds, window.content_mask().bounds))));
                    window.on_mouse_event({
                        let store = store.clone();
                        move |event: &MouseDownEvent, phase, window, cx| {
                            if !phase.bubble() || !bounds.contains(&event.position) {
                                return;
                            }
                            // Clicar não deve acender o realce de FOCO (o `track_focus` foca no
                            // mouse down) — é o que o `:focus-visible` da casa resolve.
                            crate::focus_ring::pointer_used(window);
                            store.update(cx, |s, cx| s.set_active(Some(ix), cx));
                        }
                    });
                    window.on_mouse_event({
                        let store = store.clone();
                        move |_: &MouseUpEvent, phase, _, cx| {
                            if phase.bubble() {
                                store.update(cx, |s, cx| s.set_active(None, cx));
                            }
                        }
                    });
                },
            )
            .absolute()
            .map(|c| match metrics.aria {
                Orientation::Vertical => c.top_0().bottom_0().left(px(HIT_START)).w(px(HIT_SIZE)),
                Orientation::Horizontal => c.left_0().right_0().top(px(HIT_START)).h(px(HIT_SIZE)),
            }),
        )
        .when(with_handle, |d| d.child(puxador(metrics, realcado, &p)));

    // A sonda da ORDEM DE PINTURA. Tem que nascer DENTRO do separador, porque o que se mede é a
    // posição do separador na fila de hitboxes do frame — ver [`ORDEM_PROBE`].
    #[cfg(test)]
    let linha = linha.child(sonda_de_ordem(metrics));

    Deslocado {
        filho: Some(linha.into_any_element()),
        ancoras,
        ix,
    }
    .into_any_element()
}

/// A sonda da ordem de pintura: um hitbox do TAMANHO DA FAIXA DA ALÇA, inserido de dentro do
/// separador, que grava em [`ORDEM_PROBE`] se ele é alcançável pelo ponteiro.
#[cfg(test)]
fn sonda_de_ordem(metrics: SeparatorMetrics) -> impl IntoElement {
    canvas(
        // O hitbox entra no PREPAINT, que é onde o GPUI monta a fila que o `hit_test` varre.
        |bounds, window, _| window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal),
        |_, hitbox, window, _| ORDEM_PROBE.with(|p| p.set(Some(hitbox.is_hovered(window)))),
    )
    .absolute()
    .map(|c| match metrics.aria {
        Orientation::Vertical => c.top_0().bottom_0().left(px(HIT_START)).w(px(HIT_SIZE)),
        Orientation::Horizontal => c.left_0().right_0().top(px(HIT_START)).h(px(HIT_SIZE)),
    })
}

/// O puxador de seis pontos, centrado na linha.
///
/// O `flex_none` não é detalhe: o pai é a linha de **1px**, e sem ele o flex encolheria o puxador
/// até caber. Com ele, o puxador transborda e o `justify_center` do pai o centra na divisa — que é
/// exatamente o que o `flex items-center justify-center` da fonte faz, sem margem negativa nenhuma.
fn puxador(metrics: SeparatorMetrics, realcado: bool, p: &ResizablePalette) -> impl IntoElement {
    div()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .w(px(metrics.grip_w))
        .h(px(metrics.grip_h))
        .rounded(px(GRIP_RADIUS))
        .border_1()
        .border_color(p.border.hsla())
        .bg(p.grip_bg.hsla())
        .child(
            svg()
                .path(metrics.grip_icon)
                .size(px(GRIP_ICON))
                .text_color(if realcado {
                    p.grip_icon_on.hsla()
                } else {
                    p.grip_icon.hsla()
                }),
        )
}

// =================================================================================================
// Testes
// =================================================================================================

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    /// **`separator_hidden` apaga só o REPOUSO.** Os três casos, e o do meio é a decisão: divisa
    /// escondida que está focada ou sendo arrastada continua acendendo, senão `Tab` até ela não daria
    /// retorno nenhum na tela.
    #[test]
    fn a_divisa_escondida_ainda_acende_quando_realcada() {
        let p = palette();

        let repouso_visivel = cor_da_linha(false, false, &p);
        let repouso_escondido = cor_da_linha(false, true, &p);
        let realcado_visivel = cor_da_linha(true, false, &p);
        let realcado_escondido = cor_da_linha(true, true, &p);

        assert_eq!(repouso_escondido.a, 0.0, "escondida em repouso é transparente");
        assert!(repouso_visivel.a > 0.0, "visível em repouso pinta a borda");
        assert_eq!(
            realcado_escondido, realcado_visivel,
            "realçada, escondida e visível são a MESMA cor — é o que preserva a acessibilidade"
        );
        assert!(realcado_escondido.a > 0.0, "e ela não é transparente");
    }

    /// `separator_hidden` também tira o puxador: puxador visível numa divisa invisível é contradição.
    #[test]
    fn esconder_a_divisa_tira_o_puxador() {
        let escondido = Resizable::horizontal("t").with_handle(true).separator_hidden(true);
        assert!(escondido.with_handle, "o pedido de quem chama é preservado no campo…");
        assert!(escondido.separator_hidden);

        // …e é [`puxador_visivel`] que decide. A tabela-verdade inteira, chamando a função de
        // verdade — a primeira versão deste teste repetia a expressão `with_handle && !hidden` e por
        // isso sobrevivia a remover o gate do render.
        assert!(!puxador_visivel(true, true), "pedido, mas a divisa está escondida");
        assert!(puxador_visivel(true, false), "pedido, divisa visível");
        assert!(!puxador_visivel(false, true));
        assert!(!puxador_visivel(false, false));
    }

    /// **A INVERSÃO.** O `aria-orientation` do separador é o contrário do `orientation` do grupo — e
    /// é o erro nº 1 deste componente (ver o doc do módulo). Se alguém "simplificar"
    /// [`separator_metrics`] devolvendo a orientação recebida, este teste cai.
    #[test]
    fn a_orientacao_aria_do_separador_e_a_inversa_da_do_grupo() {
        assert_eq!(
            separator_metrics(Orientation::Horizontal).aria,
            Orientation::Vertical,
            "grupo horizontal (painéis lado a lado) → divisa EM PÉ"
        );
        assert_eq!(
            separator_metrics(Orientation::Vertical).aria,
            Orientation::Horizontal,
            "grupo vertical (painéis empilhados) → divisa DEITADA"
        );
    }

    /// O lado LONGO do puxador acompanha a linha, e o CURTO a atravessa: `aria-vertical:h-4 w-3`,
    /// `aria-horizontal:h-3 w-4`. Trocar os dois campos de lugar num dos braços deixa o puxador
    /// deitado numa divisa em pé — passa desapercebido no código e é óbvio na tela.
    #[test]
    fn o_lado_longo_do_puxador_acompanha_a_linha() {
        let em_pe = separator_metrics(Orientation::Horizontal);
        assert_eq!((em_pe.grip_w, em_pe.grip_h), (12.0, 16.0), "w-3 h-4");

        let deitada = separator_metrics(Orientation::Vertical);
        assert_eq!((deitada.grip_w, deitada.grip_h), (16.0, 12.0), "w-4 h-3");
    }

    /// **O ícone acompanha a orientação, e os dois arquivos EXISTEM.**
    ///
    /// Na fonte há um SVG só, girado com `rotate-90`; o GPUI não gira `div` nem `svg`, então são dois
    /// arquivos. E caminho de ícone errado **falha em silêncio** no render do GPUI (o
    /// `AssetSource::load` devolve `Ok(None)` e nada é pintado, sem log nenhum), então o teste resolve
    /// os dois caminhos pela tabela de verdade.
    #[test]
    fn o_icone_do_puxador_acompanha_a_orientacao_e_os_arquivos_existem() {
        assert_eq!(
            separator_metrics(Orientation::Horizontal).grip_icon,
            "icons/grip_vertical.svg"
        );
        assert_eq!(
            separator_metrics(Orientation::Vertical).grip_icon,
            "icons/grip_horizontal.svg"
        );
        for path in [GRIP_VERTICAL, GRIP_HORIZONTAL] {
            let bytes = crate::assets::lookup(path)
                .unwrap_or_else(|| panic!("ícone ausente da tabela de assets: {path}"));
            assert!(!bytes.is_empty(), "SVG vazio: {path}");
        }
    }

    /// As medidas, contra a ARITMÉTICA da folha e não contra elas mesmas.
    #[test]
    fn as_medidas_sao_as_da_folha() {
        assert_eq!(LINE, 1.0, "w-px / h-px");
        assert_eq!(GRIP_ALONG, 4.0 * 4.0, "h-4 / w-4 = 4 passos de 4px");
        assert_eq!(GRIP_ACROSS, 3.0 * 4.0, "w-3 / h-3 = 3 passos de 4px");
        assert_eq!(GRIP_ICON, 3.0 * 4.0, "size-3");
        assert_eq!(GRIP_RADIUS, 0.125 * 16.0, "rounded-xs = --radius-xs = 0.125rem");
        assert_eq!(PRIMARY_ALPHA, 40.0 / 100.0, "bg-primary/40");
        assert_eq!(ARROW_STEP, 5.0 / 100.0, "as setas andam 5 pontos percentuais");
    }

    /// O mínimo default é o do NÚCLEO (`PANEL_MIN_SIZE`), e é o que um painel sem `min_size` leva.
    #[test]
    fn o_minimo_default_e_o_do_nucleo() {
        assert_eq!(MIN_SIZE_DEFAULT, 100.0, "PANEL_MIN_SIZE do fork");
        let p = ResizablePanel::new();
        assert_eq!(p.range.start, MIN_SIZE_DEFAULT);
        assert_eq!(p.range.end, f32::MAX, "sem teto por default");
        assert!(p.size.is_none(), "sem defaultSize: o núcleo distribui");
        assert!(p.visible, "visível por default");
    }

    /// Os construtores gravam onde deviam — em PIXELS, que é a unidade resolvida (ver o doc).
    #[test]
    fn os_construtores_do_painel_gravam_em_pixels() {
        let p = ResizablePanel::new()
            .size(50.0)
            .min_size(200.0)
            .max_size(400.0)
            .visible(false);
        assert_eq!(p.size, Some(50.0), "o defaultSize dos exemplos são 50 PIXELS");
        assert_eq!(p.range, 200.0..400.0);
        assert!(!p.visible);
    }

    /// O grupo nasce HORIZONTAL (o default da fonte) e os atalhos escolhem o eixo.
    #[test]
    fn o_grupo_nasce_horizontal() {
        assert_eq!(Resizable::new("g").orientation, Orientation::Horizontal);
        assert_eq!(
            Resizable::horizontal("g").orientation,
            Orientation::Horizontal
        );
        assert_eq!(Resizable::vertical("g").orientation, Orientation::Vertical);
        assert!(!Resizable::new("g").with_handle, "withHandle é opt-in");
    }

    /// **As setas do eixo do GRUPO movem a divisa; as do eixo cruzado não fazem nada.**
    ///
    /// O `aria` aqui já é o invertido, então uma divisa `Vertical` (grupo horizontal) anda com
    /// `←`/`→`. Se a inversão vazasse pra cá, o grupo horizontal responderia a `↑`/`↓`.
    #[test]
    fn as_setas_do_eixo_do_grupo_movem_a_divisa() {
        // Divisa EM PÉ (grupo horizontal): eixo X.
        assert_eq!(key_delta("left", Orientation::Vertical, 400.0), Some(-20.0));
        assert_eq!(key_delta("right", Orientation::Vertical, 400.0), Some(20.0));
        assert_eq!(key_delta("up", Orientation::Vertical, 400.0), None);
        assert_eq!(key_delta("down", Orientation::Vertical, 400.0), None);

        // Divisa DEITADA (grupo vertical): eixo Y.
        assert_eq!(key_delta("up", Orientation::Horizontal, 400.0), Some(-20.0));
        assert_eq!(key_delta("down", Orientation::Horizontal, 400.0), Some(20.0));
        assert_eq!(key_delta("left", Orientation::Horizontal, 400.0), None);
        assert_eq!(key_delta("right", Orientation::Horizontal, 400.0), None);
    }

    /// `Home`/`End` são `∓100` pontos percentuais na referência: valem nos dois eixos e saturam.
    #[test]
    fn home_e_end_valem_nos_dois_eixos_e_saturam() {
        for aria in [Orientation::Vertical, Orientation::Horizontal] {
            assert_eq!(key_delta("home", aria, 400.0), Some(-400.0));
            assert_eq!(key_delta("end", aria, 400.0), Some(400.0));
        }
        assert_eq!(
            key_delta("k", Orientation::Vertical, 400.0),
            None,
            "tecla alheia"
        );
    }

    /// **A divisa troca tamanho entre os DOIS vizinhos, e o par conserva o total.** Se o solver
    /// mexesse só no painel de antes, o grupo cresceria a cada seta.
    #[test]
    fn a_divisa_troca_tamanho_entre_os_dois_vizinhos() {
        let sizes = vec![px(200.0), px(200.0)];
        let ranges = vec![100.0..f32::MAX, 100.0..f32::MAX];
        let novos = move_divider(&sizes, &ranges, 0, 50.0);
        assert_eq!(novos, vec![px(250.0), px(150.0)]);
        assert_eq!(
            f32::from(novos[0]) + f32::from(novos[1]),
            400.0,
            "o par conserva o total"
        );
    }

    /// O **mínimo do vizinho** limita o crescimento: quem cresce só pode tomar o que o outro tem de
    /// folga. Sem essa ponta do `clamp`, uma seta esmagaria o vizinho abaixo do mínimo dele.
    #[test]
    fn o_minimo_do_vizinho_apara_o_crescimento() {
        let sizes = vec![px(200.0), px(200.0)];
        let ranges = vec![100.0..f32::MAX, 100.0..f32::MAX];
        assert_eq!(
            move_divider(&sizes, &ranges, 0, 150.0),
            vec![px(300.0), px(100.0)],
            "o vizinho para no mínimo dele, e não em 50"
        );
    }

    /// E o **máximo do próprio** também apara.
    #[test]
    fn o_maximo_do_proprio_apara_o_crescimento() {
        let sizes = vec![px(200.0), px(200.0)];
        let ranges = vec![100.0..250.0, 100.0..f32::MAX];
        assert_eq!(
            move_divider(&sizes, &ranges, 0, 100.0),
            vec![px(250.0), px(150.0)]
        );
    }

    /// `Home`/`End` chegam como `∓container` e o `clamp` os leva exatamente ao limite — é assim que
    /// as duas teclas funcionam sem um caminho próprio no solver.
    #[test]
    fn o_delta_saturado_leva_a_divisa_ao_limite() {
        let sizes = vec![px(200.0), px(200.0)];
        let ranges = vec![100.0..f32::MAX, 100.0..f32::MAX];
        assert_eq!(
            move_divider(&sizes, &ranges, 0, -400.0),
            vec![px(100.0), px(300.0)],
            "Home: o painel de antes vai ao mínimo DELE"
        );
        assert_eq!(
            move_divider(&sizes, &ranges, 0, 400.0),
            vec![px(300.0), px(100.0)],
            "End: e ao máximo que o vizinho permite"
        );
    }

    /// Restrições impossíveis (os dois mínimos somando mais que o par tem) devolvem os tamanhos
    /// intactos. Sem o guarda, o `clamp` receberia `min > max` e **entraria em pânico**.
    #[test]
    fn restricoes_impossiveis_devolvem_os_tamanhos_intactos() {
        let sizes = vec![px(200.0), px(200.0)];
        let ranges = vec![300.0..f32::MAX, 300.0..f32::MAX];
        assert_eq!(move_divider(&sizes, &ranges, 0, 10.0), sizes);
    }

    /// A ÚLTIMA divisa não existe (não há vizinho depois), e um `ranges` de tamanho errado é bug de
    /// quem chama: nos dois casos nada muda, em vez de indexar fora.
    #[test]
    fn divisa_inexistente_e_ranges_desalinhados_nao_mexem() {
        let sizes = vec![px(200.0), px(200.0)];
        let ranges = vec![100.0..f32::MAX, 100.0..f32::MAX];
        assert_eq!(
            move_divider(&sizes, &ranges, 1, 10.0),
            sizes,
            "não há divisa 1"
        );
        assert_eq!(
            move_divider(&sizes, &ranges[..1], 0, 10.0),
            sizes,
            "ranges curto"
        );
        assert_eq!(move_divider(&sizes, &ranges, 0, 0.0), sizes, "delta zero");
    }

    /// **O terceiro painel não se mexe.** É o desvio declarado em relação ao arraste do núcleo (que
    /// cascateia): a seta move só o PAR da divisa.
    #[test]
    fn a_seta_nao_toca_nos_outros_paineis() {
        let sizes = vec![px(200.0), px(200.0), px(200.0)];
        let ranges = vec![100.0..f32::MAX, 100.0..f32::MAX, 100.0..f32::MAX];
        assert_eq!(
            move_divider(&sizes, &ranges, 0, 50.0),
            vec![px(250.0), px(150.0), px(200.0)]
        );
    }

    /// **O realce acende arrastando OU com foco de teclado — e não com foco de mouse.** O terceiro
    /// caso é o que o [`crate::focus_ring`] existe pra impedir: sem ele, clicar no separador o
    /// focaria e a linha ficaria acesa depois de soltar.
    #[test]
    fn o_realce_acende_arrastando_ou_com_foco_de_teclado() {
        assert!(highlighted(true, false, false), "arrastando");
        assert!(highlighted(false, true, true), "foco vindo do teclado");
        assert!(!highlighted(false, true, false), "foco vindo do MOUSE: apagado");
        assert!(
            !highlighted(false, false, true),
            "teclado, mas o foco não é daqui: apagado"
        );
        assert!(!highlighted(false, false, false), "repouso");
    }

    /// O `/40` do Tailwind v4 **multiplica** o alfa. Com o `--primary` opaco o produto é 0,40; o que
    /// o teste proíbe é substituir o alfa em vez de multiplicá-lo — o que quebraria no dia em que o
    /// token deixasse de ser opaco.
    #[test]
    fn o_alfa_do_realce_e_multiplicativo() {
        theme::set_theme(theme::ThemeMode::Dark);
        let p = palette();
        assert_eq!(p.primary.alpha(), 1.0, "--primary é opaco");
        assert!((p.primary.scaled(PRIMARY_ALPHA).a - PRIMARY_ALPHA).abs() < 1e-6);

        // O mesmo escalar sobre um token JÁ translúcido tem que compor, não cravar.
        let translucido = Rgba8(0xf3f3f380);
        let base = translucido.hsla().a;
        assert!((translucido.scaled(PRIMARY_ALPHA).a - base * PRIMARY_ALPHA).abs() < 1e-6);
    }

    /// A linha e a moldura do puxador saem do MESMO token, como na fonte (`bg-border` e `border`), e
    /// é o mesmo tom que o [`crate::Group`] dá ao separador dele — o que faz o componente ler como
    /// irmão dos outros.
    #[test]
    fn a_linha_usa_o_token_de_divisoria_do_tema() {
        for modo in [theme::ThemeMode::Dark, theme::ThemeMode::Light] {
            theme::set_theme(modo);
            assert_eq!(palette().border, Rgba8(theme::palette().border_divider));
        }
        theme::set_theme(theme::ThemeMode::Dark);
    }
}

/// Os testes que precisam de uma JANELA — porque o que se quer verificar é a **solda** com o núcleo
/// vendorizado: que o arraste chega nele, onde fica a faixa que o inicia, e que a volta pelo teclado
/// (que reinicia a entidade de estado) produz de fato os tamanhos novos. Nada disso é observável numa
/// função pura: depende do layout do taffy e dos eventos de verdade.
///
/// ⚠️ Todos envolvem a raiz em [`gpui_component::Root`]: o clique atravessa o núcleo do
/// `gpui-component`, que faz `unwrap` nas camadas de popover/modal dele (`root.rs:268`). Sem o `Root`
/// o teste estoura ali, e o pânico não diz que o que falta é isto.
#[cfg(test)]
mod tests_de_janela {
    use super::*;
    use gpui::{
        point, Bounds, Modifiers, MouseButton, Render, StatefulInteractiveElement as _,
        TestAppContext, VisualTestContext,
    };
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Um painel do harness.
    #[derive(Clone, Copy)]
    struct Spec {
        size: Option<f32>,
        min: f32,
        visible: bool,
    }

    impl Spec {
        fn nova(min: f32) -> Self {
            Self {
                size: None,
                min,
                visible: true,
            }
        }

        fn oculta(mut self) -> Self {
            self.visible = false;
            self
        }
    }

    /// Onde os painéis do harness gravam os bounds que o taffy lhes deu, a cada frame.
    type Sondas = Rc<RefCell<Vec<Option<Bounds<Pixels>>>>>;

    /// O canvas-sonda que mede o painel `ix`. É `size_full`, então mede o painel inteiro.
    fn sonda(sondas: &Sondas, ix: usize) -> impl IntoElement {
        let sondas = sondas.clone();
        canvas(
            move |bounds, _, _| {
                sondas.borrow_mut()[ix] = Some(bounds);
            },
            |_, _, _, _| {},
        )
        .size_full()
    }

    struct Harness {
        orientation: Orientation,
        specs: Vec<Spec>,
        sondas: Sondas,
        /// Os tamanhos que o `on_resize` entregou, uma entrada por vez que a divisa foi solta.
        soltas: Rc<RefCell<Vec<Vec<Pixels>>>>,
    }

    impl Render for Harness {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let sondas = self.sondas.clone();
            let soltas = self.soltas.clone();
            Resizable::new("grupo")
                .orientation(self.orientation)
                .with_handle(true)
                .on_resize(move |sizes, _, _| soltas.borrow_mut().push(sizes.to_vec()))
                .children(
                    self.specs
                        .iter()
                        .copied()
                        .enumerate()
                        .map(|(ix, spec)| {
                            let mut p = ResizablePanel::new()
                                .min_size(spec.min)
                                .visible(spec.visible);
                            if let Some(s) = spec.size {
                                p = p.size(s);
                            }
                            p.child(sonda(&sondas, ix))
                        })
                        .collect::<Vec<_>>(),
                )
        }
    }

    /// O tamanho do GRUPO no eixo, somando os painéis.
    ///
    /// Medido, e não constante: a janela dos testes tem o tamanho default do `TestAppContext`, e
    /// **não se pode redimensioná-la antes do primeiro frame**. Um `simulate_resize` depois dele
    /// dispara o reajuste proporcional do núcleo, que reescala os painéis pela razão que eles tinham —
    /// ou seja, apaga o tamanho declarado. Foi exatamente o que fez o teste dos grupos aninhados
    /// falhar dizendo que `size(150.0)` valia 100.
    fn container(sondas: &Sondas, orientation: Orientation) -> f32 {
        (0..sondas.borrow().len())
            .map(|ix| medida(sondas, ix, orientation))
            .sum()
    }

    #[allow(clippy::type_complexity)]
    fn abrir(
        cx: &mut TestAppContext,
        orientation: Orientation,
        specs: Vec<Spec>,
    ) -> (
        Sondas,
        Rc<RefCell<Vec<Vec<Pixels>>>>,
        VisualTestContext,
    ) {
        cx.update(|cx| {
            // O `resize_handle` do núcleo lê `cx.theme()`: sem isto, o primeiro frame estoura.
            gpui_component::init(cx);
            crate::focus_ring::init(cx);
        });
        let sondas: Sondas = Rc::new(RefCell::new(vec![None; specs.len()]));
        let soltas: Rc<RefCell<Vec<Vec<Pixels>>>> = Rc::new(RefCell::new(Vec::new()));
        let (s, so) = (sondas.clone(), soltas.clone());
        let window = cx.add_window(move |window, cx| {
            gpui_component::Root::new(
                cx.new(|_| Harness {
                    orientation,
                    specs,
                    sondas: s,
                    soltas: so,
                }),
                window,
                cx,
            )
        });
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        (sondas, soltas, vcx)
    }

    /// O tamanho do painel `ix` no eixo do grupo.
    fn medida(sondas: &Sondas, ix: usize, orientation: Orientation) -> f32 {
        let b = sondas.borrow()[ix].expect("a sonda mediu o painel");
        match orientation {
            Orientation::Horizontal => f32::from(b.size.width),
            Orientation::Vertical => f32::from(b.size.height),
        }
    }

    /// A posição da divisa entre os painéis 0 e 1, e um ponto no meio do eixo transversal.
    fn divisa(sondas: &Sondas, orientation: Orientation) -> (f32, f32) {
        let b = sondas.borrow()[0].expect("a sonda mediu o painel");
        match orientation {
            Orientation::Horizontal => (
                f32::from(b.origin.x + b.size.width),
                f32::from(b.origin.y + b.size.height / 2.0),
            ),
            Orientation::Vertical => (
                f32::from(b.origin.y + b.size.height),
                f32::from(b.origin.x + b.size.width / 2.0),
            ),
        }
    }

    fn ponto(coord: f32, transversal: f32, orientation: Orientation) -> gpui::Point<Pixels> {
        match orientation {
            Orientation::Horizontal => point(px(coord), px(transversal)),
            Orientation::Vertical => point(px(transversal), px(coord)),
        }
    }

    /// Aperta o botão em `origem` e arrasta até `origem + avanco`, **sem soltar**.
    ///
    /// São DOIS `mouse move`: o primeiro passa do limiar de arraste do GPUI e faz o núcleo gravar
    /// qual divisa está em uso; o segundo é o que redimensiona, porque o handler de movimento do
    /// núcleo lê esse índice no PAINT anterior. Com um movimento só, nada acontece — e isso já
    /// pareceu "o arraste não funciona".
    ///
    /// Não solta de propósito: é entre o movimento e o `mouse up` que se pode verificar se a janela
    /// repinta DURANTE o arraste (ver [`o_arraste_move_a_divisa_e_avisa_ao_soltar`]).
    fn arrastar_ate(
        vcx: &mut VisualTestContext,
        orientation: Orientation,
        origem: f32,
        transversal: f32,
        avanco: f32,
    ) {
        let p = |c: f32| ponto(c, transversal, orientation);
        vcx.simulate_mouse_down(p(origem), MouseButton::Left, Modifiers::default());
        vcx.run_until_parked();
        vcx.simulate_mouse_move(p(origem + 6.0), MouseButton::Left, Modifiers::default());
        vcx.run_until_parked();
        vcx.simulate_mouse_move(p(origem + avanco), MouseButton::Left, Modifiers::default());
        vcx.run_until_parked();
    }

    fn soltar(
        vcx: &mut VisualTestContext,
        orientation: Orientation,
        onde: f32,
        transversal: f32,
    ) {
        vcx.simulate_mouse_up(
            ponto(onde, transversal, orientation),
            MouseButton::Left,
            Modifiers::default(),
        );
        vcx.run_until_parked();
    }

    fn dois_paineis() -> Vec<Spec> {
        vec![Spec::nova(100.0), Spec::nova(100.0)]
    }

    /// **O arraste CHEGA no núcleo.** É o teste que prova que este módulo é casca e não motor: o
    /// componente não implementa arraste nenhum, e a divisa se mexe. O `on_resize` disparando uma vez
    /// só amarra a outra ponta — ele é o hook de "soltou", não de "está arrastando".
    #[gpui::test]
    fn o_arraste_move_a_divisa_e_avisa_ao_soltar(cx: &mut TestAppContext) {
        let orientation = Orientation::Horizontal;
        let (sondas, soltas, mut vcx) = abrir(cx, orientation, dois_paineis());

        let antes = medida(&sondas, 0, orientation);
        let total = container(&sondas, orientation);
        let (d, y) = divisa(&sondas, orientation);
        assert!(
            (antes - total / 2.0).abs() < 1.0,
            "dois painéis sem tamanho declarado dividem o grupo ao meio; o 0 mediu {antes} de {total}"
        );

        arrastar_ate(&mut vcx, orientation, d, y, 100.0);

        // ⚠️ A medição é feita AINDA COM O BOTÃO APERTADO, e é isso que dá dente ao teste: o núcleo
        // avisa a mudança chamando `cx.notify()` na entidade DELE, e sem o `cx.observe` do
        // [`ResizableStore::new`] ninguém sabe. Medindo depois do `mouse up`, o `window.refresh()` que
        // o próprio handle do núcleo faz ao soltar mascararia a falta — a divisa só pularia no fim do
        // arraste, que é o defeito que o observador existe pra impedir.
        let depois = medida(&sondas, 0, orientation);
        assert!(
            depois > antes + 50.0,
            "a divisa devia ter andado ~100px DURANTE o arraste; o painel 0 foi de {antes} para {depois}"
        );
        assert!(
            (medida(&sondas, 1, orientation) - (total - depois)).abs() < 1.5,
            "o vizinho devolve o que o outro tomou"
        );
        assert!(soltas.borrow().is_empty(), "o on_resize não dispara ARRASTANDO");

        soltar(&mut vcx, orientation, d + 100.0, y);
        assert_eq!(soltas.borrow().len(), 1, "o on_resize dispara ao SOLTAR, uma vez");
        assert_eq!(
            soltas.borrow()[0].len(),
            2,
            "e entrega um tamanho por painel, em pixels"
        );
    }

    /// Um arraste isolado, numa janela nova, começando a `offset` pixels da divisa. Devolve se ele
    /// moveu alguma coisa — é a régua com que se MEDE a faixa de arraste do núcleo.
    fn arraste_pega(cx: &mut TestAppContext, offset: f32) -> bool {
        let orientation = Orientation::Horizontal;
        let (sondas, _, mut vcx) = abrir(cx, orientation, dois_paineis());
        let antes = medida(&sondas, 0, orientation);
        let (d, y) = divisa(&sondas, orientation);
        arrastar_ate(&mut vcx, orientation, d + offset, y, 80.0);
        soltar(&mut vcx, orientation, d + offset + 80.0, y);
        (medida(&sondas, 0, orientation) - antes).abs() > 1.0
    }

    /// **O `min_size` da nossa API chega ao `size_range` do núcleo.**
    ///
    /// A aparagem que só o núcleo pode fazer: um arraste que passa MUITO do mínimo do vizinho tem que
    /// parar nele. O teclado não serve pra provar isto — os limites dele são aparados pelo nosso
    /// [`move_divider`], então ele passaria mesmo com o `size_range` não sendo passado adiante.
    #[gpui::test]
    fn o_arraste_para_no_minimo_declarado_do_vizinho(cx: &mut TestAppContext) {
        let orientation = Orientation::Horizontal;
        let (sondas, _, mut vcx) = abrir(
            cx,
            orientation,
            vec![Spec::nova(180.0), Spec::nova(260.0)],
        );
        let total = container(&sondas, orientation);
        let (d, y) = divisa(&sondas, orientation);

        // Empurra a divisa pra muito além do que o mínimo do vizinho permite.
        arrastar_ate(&mut vcx, orientation, d, y, total);
        soltar(&mut vcx, orientation, d + total, y);

        let vizinho = medida(&sondas, 1, orientation);
        assert!(
            (vizinho - 260.0).abs() < 1.5,
            "o vizinho tinha que parar nos 260px declarados; ficou em {vizinho}"
        );
    }

    /// **A faixa que inicia o arraste é a do NÚCLEO, e é esta.** Ver "Não reproduzível" no doc do
    /// módulo: a área de 16px assimétrica da fonte não é alcançável de fora do fork, então o que se
    /// pode fazer é MEDIR a de verdade e declará-la — que é o que [`HIT_START`]/[`HIT_SIZE`] são, e o
    /// que o realce de "separador ativo" usa pra acender na hora certa.
    #[gpui::test]
    fn a_faixa_de_arraste_do_nucleo_e_a_que_esta_declarada(cx: &mut TestAppContext) {
        assert!(arraste_pega(cx, 0.0), "sobre a linha");
        assert!(arraste_pega(cx, HIT_START + 0.5), "na ponta de dentro da faixa");
        assert!(
            arraste_pega(cx, HIT_START + HIT_SIZE - 0.5),
            "na outra ponta da faixa"
        );
        assert!(
            !arraste_pega(cx, HIT_START - 2.0),
            "2px antes da faixa já não pega"
        );
        assert!(
            !arraste_pega(cx, HIT_START + HIT_SIZE + 2.0),
            "2px depois da faixa já não pega"
        );
    }

    /// **Pressionar dentro da faixa acende o realce, e soltar apaga.** É o `data-separator="active"`
    /// da fonte, e o caminho depende dos números medidos no teste de cima — se a faixa declarada
    /// deslizar, o realce acende fora de hora.
    #[gpui::test]
    fn o_botao_apertado_na_faixa_acende_o_realce(cx: &mut TestAppContext) {
        let orientation = Orientation::Horizontal;
        let (sondas, _, mut vcx) = abrir(cx, orientation, dois_paineis());
        let (d, y) = divisa(&sondas, orientation);
        ACTIVE_PROBE.with(|p| p.set(None));

        vcx.simulate_mouse_down(
            ponto(d + 20.0, y, orientation),
            MouseButton::Left,
            Modifiers::default(),
        );
        vcx.run_until_parked();
        assert_eq!(
            ACTIVE_PROBE.with(|p| p.get()),
            None,
            "longe da divisa não acende nada"
        );
        vcx.simulate_mouse_up(
            ponto(d + 20.0, y, orientation),
            MouseButton::Left,
            Modifiers::default(),
        );
        vcx.run_until_parked();

        vcx.simulate_mouse_down(ponto(d, y, orientation), MouseButton::Left, Modifiers::default());
        vcx.run_until_parked();
        assert_eq!(
            ACTIVE_PROBE.with(|p| p.get()),
            Some(0),
            "na divisa, o separador 0 fica ativo"
        );

        vcx.simulate_mouse_up(ponto(d, y, orientation), MouseButton::Left, Modifiers::default());
        vcx.run_until_parked();
        assert_eq!(ACTIVE_PROBE.with(|p| p.get()), None, "soltar apaga");
    }

    /// **A seta move a divisa, e o passo é 5% do grupo.**
    ///
    /// É o teste que prova o caminho inteiro do teclado — o que o núcleo não tem: foco no separador,
    /// tecla, solver, reinício da entidade de estado e tamanho novo chegando ao layout. Se o reinício
    /// não funcionasse (por exemplo se o `size` declarado deixasse de valer no primeiro frame), a
    /// divisa não andaria um pixel.
    #[gpui::test]
    fn a_seta_move_a_divisa_cinco_por_cento_do_grupo(cx: &mut TestAppContext) {
        let orientation = Orientation::Horizontal;
        let (sondas, _, mut vcx) = abrir(cx, orientation, dois_paineis());

        let antes = medida(&sondas, 0, orientation);
        let container = antes + medida(&sondas, 1, orientation);

        vcx.update(|window, _cx| window.focus_next());
        vcx.run_until_parked();
        vcx.simulate_keystrokes("right");
        vcx.run_until_parked();

        let esperado = antes + container * ARROW_STEP;
        let depois = medida(&sondas, 0, orientation);
        assert!(
            (depois - esperado).abs() < 1.0,
            "a seta devia levar o painel 0 de {antes} para {esperado}; foi para {depois}"
        );

        vcx.simulate_keystrokes("left left");
        vcx.run_until_parked();
        let volta = medida(&sondas, 0, orientation);
        assert!(
            (volta - (antes - container * ARROW_STEP)).abs() < 1.0,
            "e as setas andam pros dois lados; ficou em {volta}"
        );
    }

    /// **`Home` e `End` levam a divisa aos limites — e os limites são os que `min_size` declarou.**
    ///
    /// Os dois mínimos são de propósito DIFERENTES do default do núcleo (100): assim o teste também
    /// prova que o `min_size` da nossa API chega ao `size_range` do núcleo. Com 100 nos dois, tirar o
    /// `.size_range(...)` do render não quebraria nada — o default do núcleo é o mesmo número.
    #[gpui::test]
    fn home_e_end_levam_a_divisa_aos_limites_declarados(cx: &mut TestAppContext) {
        let orientation = Orientation::Horizontal;
        let (sondas, _, mut vcx) = abrir(
            cx,
            orientation,
            vec![Spec::nova(180.0), Spec::nova(260.0)],
        );
        vcx.update(|window, _cx| window.focus_next());
        vcx.run_until_parked();

        let total = container(&sondas, orientation);
        vcx.simulate_keystrokes("home");
        vcx.run_until_parked();
        let minimo = medida(&sondas, 0, orientation);
        assert!(
            (minimo - 180.0).abs() < 1.0,
            "Home leva o painel 0 ao mínimo DELE (180); ficou em {minimo}"
        );

        vcx.simulate_keystrokes("end");
        vcx.run_until_parked();
        let maximo = medida(&sondas, 0, orientation);
        assert!(
            (maximo - (total - 260.0)).abs() < 1.5,
            "End o leva até onde o mínimo do VIZINHO permite ({}); ficou em {maximo}",
            total - 260.0
        );
    }

    /// **A seta do eixo cruzado não faz nada** — é `no-op` na referência também, e aqui a decisão
    /// mora no [`key_delta`], que lê a orientação ARIA (invertida). Se a inversão vazasse, um grupo
    /// horizontal andaria com `↑`/`↓`.
    #[gpui::test]
    fn a_seta_do_eixo_cruzado_nao_mexe(cx: &mut TestAppContext) {
        let orientation = Orientation::Horizontal;
        let (sondas, _, mut vcx) = abrir(cx, orientation, dois_paineis());
        vcx.update(|window, _cx| window.focus_next());
        vcx.run_until_parked();

        let antes = medida(&sondas, 0, orientation);
        vcx.simulate_keystrokes("down up down");
        vcx.run_until_parked();
        assert_eq!(
            medida(&sondas, 0, orientation),
            antes,
            "num grupo horizontal, ↑/↓ não movem a divisa"
        );
    }

    /// O mesmo componente num grupo **VERTICAL**: a divisa anda com `↑`/`↓` e o arraste é no eixo Y.
    /// É o outro braço da inversão, e sem ele o teste da inversão seria só de função pura.
    #[gpui::test]
    fn no_grupo_vertical_a_divisa_anda_no_eixo_y(cx: &mut TestAppContext) {
        let orientation = Orientation::Vertical;
        let (sondas, _, mut vcx) = abrir(cx, orientation, dois_paineis());

        let antes = medida(&sondas, 0, orientation);
        vcx.update(|window, _cx| window.focus_next());
        vcx.run_until_parked();
        vcx.simulate_keystrokes("down");
        vcx.run_until_parked();
        let depois = medida(&sondas, 0, orientation);
        assert!(depois > antes + 1.0, "↓ cresce o painel de cima");

        vcx.simulate_keystrokes("right left");
        vcx.run_until_parked();
        assert_eq!(
            medida(&sondas, 0, orientation),
            depois,
            "e ←/→ não fazem nada num grupo vertical"
        );
    }

    /// **Um painel invisível não desenha nem ocupa espaço**, e os visíveis ficam com o grupo inteiro.
    ///
    /// A sonda mora nos FILHOS do painel, e o núcleo não renderiza filho nenhum de um painel
    /// invisível — então "a sonda do painel oculto nunca mediu" é exatamente a afirmação que se quer.
    /// É também o caminho que este porte recomenda pra colapsar de verdade, já que o `collapsible` da
    /// referência não está no núcleo (ver "Ausente" no doc do módulo).
    #[gpui::test]
    fn um_painel_invisivel_nao_desenha_nem_ocupa_espaco(cx: &mut TestAppContext) {
        let orientation = Orientation::Horizontal;
        let (sondas, _, _vcx) = abrir(
            cx,
            orientation,
            vec![
                Spec::nova(100.0),
                Spec::nova(100.0).oculta(),
                Spec::nova(100.0),
            ],
        );

        assert!(
            sondas.borrow()[1].is_none(),
            "o painel oculto não renderizou filho nenhum, logo a sonda dele não mediu"
        );
        let a = medida(&sondas, 0, orientation);
        let b = medida(&sondas, 2, orientation);
        assert!(
            a > 1.0 && b > 1.0,
            "os dois visíveis ficaram com área: {a} e {b}"
        );
        assert!(
            (a - b).abs() < 1.5,
            "e dividem o grupo entre si, sem reservar nada pro oculto: {a} contra {b}"
        );
    }

    // =============================================================================================
    // A ORDEM DE PINTURA e o RECORTE — os dois invariantes que o `deferred` misturava
    // =============================================================================================

    /// **O SEPARADOR PINTA POR CIMA DA ALÇA DO NÚCLEO** — que é o que o `z-10` da fonte precisa, e a
    /// razão pela qual o separador não pode morar dentro do painel.
    ///
    /// O `ResizablePanel` do núcleo pendura a alça dele DEPOIS dos filhos
    /// (`vendor/gpui-component/src/resizable/panel.rs:303`), e a alça pinta uma linha **opaca** de
    /// 1px exatamente sobre a emenda. Um separador que fosse filho do painel ficaria embaixo dessa
    /// linha: a nossa cor (e o realce de foco) desapareceriam e a linha do núcleo atravessaria o
    /// puxador. Por isso o separador é irmão POSTERIOR ao grupo inteiro.
    ///
    /// A afirmação passa pela fila de hitboxes porque é a mesma fila que a ordem de pintura alimenta:
    /// a alça é `occlude()` (`BlockMouse`) e o `hit_test` para nela — logo o nosso hitbox só é
    /// alcançado se tiver entrado depois. Ver [`ORDEM_PROBE`].
    #[gpui::test]
    fn o_separador_fica_por_cima_da_alca_do_nucleo(cx: &mut TestAppContext) {
        let orientation = Orientation::Horizontal;
        let (sondas, _, mut vcx) = abrir(cx, orientation, dois_paineis());
        let (d, y) = divisa(&sondas, orientation);

        ORDEM_PROBE.with(|p| p.set(None));
        // Sem botão apertado: é hover, não arraste.
        vcx.simulate_mouse_move(ponto(d, y, orientation), None, Modifiers::default());
        vcx.run_until_parked();
        // ⚠️ O `mouse_hit_test` é recalculado no FIM do `draw` (`window.rs:2065`), então a sonda —
        // que lê durante o paint — só vê o resultado do movimento no frame SEGUINTE.
        vcx.update(|window, _cx| window.refresh());
        vcx.run_until_parked();

        assert_eq!(
            ORDEM_PROBE.with(|p| p.get()),
            Some(true),
            "com o ponteiro sobre a divisa, o separador tem que ser alcançável — se ele estiver \
             ATRÁS da alça `occlude()` do núcleo, o `hit_test` para na alça e isto dá `false`, que é \
             o mesmo que dizer que a linha do núcleo pinta em cima do puxador"
        );
    }

    /// **O separador pousa SOBRE a emenda, e não no canto do grupo.** É o que o [`Deslocado`] existe
    /// pra fazer: o separador é irmão do grupo, então o taffy o põe no canto do grupo, e quem o leva
    /// até a divisa é o deslocamento pela âncora — no MESMO frame, senão ele ficaria um frame atrás
    /// da linha do núcleo durante o arraste.
    ///
    /// Tirar o `with_element_offset` do [`Deslocado`] deixa o separador em `x = 0`; é isto que cai.
    #[gpui::test]
    fn o_separador_pousa_sobre_a_emenda_dos_paineis(cx: &mut TestAppContext) {
        let orientation = Orientation::Horizontal;
        RECORTE_PROBE.with(|p| p.set(None));
        let (sondas, _, _vcx) = abrir(cx, orientation, dois_paineis());
        let (d, _) = divisa(&sondas, orientation);

        let (separador, _) = RECORTE_PROBE
            .with(|p| p.get())
            .expect("o separador foi pintado");
        // A sonda é a faixa de arraste: começa em `HIT_START` px da divisa (ver [`HIT_START`]).
        assert!(
            (f32::from(separador.origin.x) - (d + HIT_START)).abs() < 0.5,
            "o separador tinha que estar sobre a emenda (x = {}); ficou em {:?}",
            d + HIT_START,
            separador.origin
        );
        // E no eixo que ATRAVESSA a divisa ele cobre o painel inteiro — o que é o mesmo que dizer
        // que o `top_0().bottom_0()` continua valendo depois de o separador sair do painel.
        let painel = sondas.borrow()[1].expect("a sonda mediu o painel de depois da divisa");
        assert!(
            (f32::from(separador.size.height) - f32::from(painel.size.height)).abs() < 0.5,
            "a linha tinha que ter a altura do painel ({:?}); veio {:?}",
            painel.size.height,
            separador.size.height
        );
    }

    // =============================================================================================
    // O RECORTE — o invariante que o `deferred` quebrava
    // =============================================================================================

    /// Altura da janela de rolagem do harness [`Rolavel`].
    const JANELA_H: f32 = 120.0;

    /// Altura do conteúdo dentro dela — **cinco vezes** a janela, pra o grupo (e portanto a linha do
    /// separador, que é `top_0 bottom_0`) sobrar MUITO pra fora do recorte.
    const CONTEUDO_H: f32 = 600.0;

    /// Um [`Resizable`] dentro de um container **rolável** mais baixo que o conteúdo — a situação da
    /// captura do defeito: a linha do separador atravessava a borda do card e continuava no painel
    /// fixo de baixo.
    struct Rolavel {
        /// Onde o container rolável grava o retângulo dele.
        janela: Rc<RefCell<Option<Bounds<Pixels>>>>,
    }

    impl Render for Rolavel {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let janela = self.janela.clone();
            div()
                .id("janela-de-rolagem")
                .absolute()
                .left_0()
                .top_0()
                .w(px(400.0))
                .h(px(JANELA_H))
                // É ESTE `overflow_*_scroll` que instala a `ContentMask` do recorte.
                .overflow_y_scroll()
                .child(
                    canvas(
                        move |bounds, _, _| *janela.borrow_mut() = Some(bounds),
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .size_full(),
                )
                .child(
                    div().w_full().h(px(CONTEUDO_H)).child(
                        Resizable::horizontal("rolavel")
                            .with_handle(true)
                            .child(ResizablePanel::new())
                            .child(ResizablePanel::new()),
                    ),
                )
        }
    }

    /// **O SEPARADOR NÃO PINTA FORA DO GRUPO.** É o invariante que o [`gpui::deferred`] quebrava, e
    /// o defeito que o usuário viu: a linha e o puxador pintando por cima de um painel FIXO, fora da
    /// área de rolagem.
    ///
    /// A causa era mecânica, e não de posicionamento: um filho diferido é pintado em
    /// `paint_deferred_draws`, **depois** de a pilha de máscaras do frame ter sido desempilhada
    /// (`gpui-0.2.2/src/window.rs:2194`) — então `window.content_mask()` cai no fallback, que é a
    /// VIEWPORT inteira (`window.rs:2551`). O separador ganhava a geometria certa e o recorte
    /// errado.
    ///
    /// O que se afirma aqui é o que o rasterizador de fato desenha: a interseção do retângulo do
    /// separador com a máscara em vigor no paint dele. Com `deferred` a máscara é a viewport toda e a
    /// interseção desce até os 600px do conteúdo; sem ele a máscara é a do container e a interseção
    /// para nos 120px da janela.
    #[gpui::test]
    fn o_separador_nao_pinta_fora_do_container_rolavel(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::focus_ring::init(cx);
        });
        RECORTE_PROBE.with(|p| p.set(None));
        let janela: Rc<RefCell<Option<Bounds<Pixels>>>> = Rc::new(RefCell::new(None));
        let j = janela.clone();
        let window = cx.add_window(move |window, cx| {
            gpui_component::Root::new(cx.new(|_| Rolavel { janela: j }), window, cx)
        });
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();

        let recorte = janela
            .borrow()
            .expect("o container rolável mediu o retângulo dele");
        let (separador, mascara) = RECORTE_PROBE
            .with(|p| p.get())
            .expect("o separador foi pintado (senão não há o que verificar)");

        // ⚠️ Sem esta primeira afirmação o teste seria VAZIO: se o separador couber na janela de
        // rolagem, qualquer máscara o contém e o `deferred` passaria.
        assert!(
            f32::from(separador.bottom()) > f32::from(recorte.bottom()) + 1.0,
            "o separador ({:?}) tem que sobrar pra fora da janela de rolagem ({:?}) — é o que faz \
             o recorte importar",
            separador,
            recorte
        );

        let visivel = separador.intersect(&mascara);
        assert!(
            f32::from(visivel.bottom()) <= f32::from(recorte.bottom()) + 0.5
                && f32::from(visivel.top()) >= f32::from(recorte.top()) - 0.5
                && f32::from(visivel.left()) >= f32::from(recorte.left()) - 0.5
                && f32::from(visivel.right()) <= f32::from(recorte.right()) + 0.5,
            "o separador pintou FORA do container rolável: o que sobra depois da ContentMask é \
             {visivel:?}, e a janela é {recorte:?} (máscara em vigor: {mascara:?})"
        );
    }

    /// O harness dos grupos ANINHADOS: horizontal ⊃ vertical ⊃ horizontal, com uma sonda em cada
    /// painel folha.
    struct Aninhado {
        sondas: Sondas,
    }

    impl Render for Aninhado {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let s = self.sondas.clone();
            Resizable::horizontal("n1")
                .with_handle(true)
                .child(ResizablePanel::new().size(150.0).child(sonda(&s, 0)))
                .child(
                    ResizablePanel::new().child(
                        Resizable::vertical("n2")
                            .with_handle(true)
                            .child(ResizablePanel::new().child(sonda(&s, 1)))
                            .child(ResizablePanel::new().size(140.0).child(
                                Resizable::horizontal("n3")
                                    .child(ResizablePanel::new().child(sonda(&s, 2)))
                                    .child(ResizablePanel::new().child(sonda(&s, 3))),
                            )),
                    ),
                )
        }
    }

    /// **Três níveis de grupos aninhados renderizam.** É o exemplo mais duro da fonte, e o que mais
    /// facilmente estoura: cada grupo tem estado próprio, e o `ResizablePanel` do núcleo faz
    /// `expect("BUG: The `index` of ResizablePanel should be one of in `state`")` se dois grupos
    /// dividirem a mesma entidade — o que aconteceria se a chave do estado não fosse o `id` do grupo.
    #[gpui::test]
    fn tres_niveis_de_grupos_aninhados_renderizam(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::focus_ring::init(cx);
        });
        let sondas: Sondas = Rc::new(RefCell::new(vec![None; 4]));
        let s = sondas.clone();
        let window = cx.add_window(move |window, cx| {
            gpui_component::Root::new(cx.new(|_| Aninhado { sondas: s }), window, cx)
        });
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();

        for ix in 0..4 {
            let b = sondas.borrow()[ix].expect("todo painel folha foi medido");
            assert!(
                f32::from(b.size.width) > 1.0 && f32::from(b.size.height) > 1.0,
                "o painel {ix} saiu com área zero: {:?}",
                b.size
            );
        }

        let esquerda = sondas.borrow()[0].unwrap();
        assert!(
            (f32::from(esquerda.size.width) - 150.0).abs() < 1.0,
            "o tamanho declarado do painel externo vale (150px); veio {:?}",
            esquerda.size
        );

        // O grupo do terceiro nível divide a LARGURA do painel que o contém, não a do grupo de fora:
        // os dois painéis folha somam o que sobrou depois dos 150px do painel esquerdo.
        let (a, b) = (sondas.borrow()[2].unwrap(), sondas.borrow()[3].unwrap());
        let dentro = f32::from(a.size.width) + f32::from(b.size.width);
        let fora = f32::from(esquerda.size.width) + dentro;
        assert!(
            (dentro - (fora - 150.0)).abs() < 1.5,
            "o grupo aninhado ficou preso ao painel dele; somou {dentro} de {fora}"
        );
        // E o do segundo nível divide a ALTURA, com os 140px declarados no painel de baixo.
        let alto = sondas.borrow()[1].unwrap();
        assert!(
            (f32::from(a.size.height) - 140.0).abs() < 1.0,
            "o painel de baixo do grupo vertical vale os 140px declarados; veio {:?}",
            a.size
        );
        assert!(
            f32::from(alto.size.height) > 140.0,
            "e o de cima fica com o resto da altura; veio {:?}",
            alto.size
        );
    }
}

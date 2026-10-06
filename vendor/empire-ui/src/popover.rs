//! `Popover` — a **camada flutuante de conteúdo livre** do `empire-ui`, com o visual do design
//! system [coss][1].
//!
//! [1]: https://github.com/cosscom/coss — `apps/ui/registry/default/ui/popover.tsx`
//!
//! ```ignore
//! use empire_ui::popover::{Popover, PopoverDescription, PopoverTitle};
//!
//! let pop = cx.new(|cx| {
//!     Popover::new(cx)
//!         .trigger_button("Dimensões")
//!         .content(|_window, _cx| {
//!             gpui::div()
//!                 .flex()
//!                 .flex_col()
//!                 .gap(gpui::px(8.0))
//!                 .child(PopoverTitle::new("Dimensões"))
//!                 .child(PopoverDescription::new("Largura e altura da composição."))
//!                 .into_any_element()
//!         })
//! });
//! ```
//!
//! # Anatomia
//!
//! ```text
//! ┌──────────────┐
//! │  Dimensões ⌄ │                  ← o GATILHO (`PopoverTrigger`): um `Button`, ou o que você montar
//! └──────────────┘
//!        ↕ 4px                       ← `sideOffset={4}`
//!  ┌────────────────────────────┐
//!  │                            │   ← respiro 16px em volta de TUDO (ver [`PAD`])
//!  │  Dimensões                 │   ← `PopoverTitle`       (18px, semibold, leading-none)
//!  │  Largura e altura da        │
//!  │  composição.                │   ← `PopoverDescription` (14/20, --muted-foreground)
//!  │                            │
//!  └────────────────────────────┘
//!   └──── a largura do conteúdo ──┘   ← `w-(--popup-width,auto)`, com o teto de
//!                                       `max-w-(--available-width)`
//! ```
//!
//! # O que é DELE, e o que é emprestado
//!
//! O popover é **a mesma superfície flutuante do [`crate::menu`]**: `bg-popover`, borda `--border`,
//! `rounded-lg`, `shadow-lg/5` e o fio de bisel. Então ele não tem paleta própria, nem posicionador
//! próprio, nem sombra própria, nem bisel próprio — tudo isso é importado do `menu`:
//!
//! | o que | de onde |
//! |---|---|
//! | as cores (`--popover`, `--border`, `--foreground`, `--muted-foreground`, bisel, sombra) | [`crate::menu::palette`] |
//! | `side`/`align`/offsets → (ponto, canto) | [`crate::menu::anchor_point`] |
//! | `align: center` (que não é um canto) | [`crate::menu::align_center_wrap`] |
//! | `max-h-(--available-height)` | [`crate::menu::popup_max_height`] |
//! | `max-w-(--available-width)` | [`crate::tooltip::popup_max_width`] |
//! | `shadow-lg/5` | [`crate::menu::popup_shadow`] |
//! | o fio de bisel | [`crate::menu::bevel_overlay`] |
//! | a guarda do clique no gatilho | [`crate::menu::outside_click_closes`] |
//! | `--radius-lg`, `border`, `text-sm` (14/20), `sideOffset`, `collisionPadding` | as constantes do `menu` |
//! | a bissecção de uma `cubic-bezier` do CSS | [`crate::dialog::cubic_bezier`] |
//! | a modalidade de `:focus-visible` | [`crate::focus_ring`] |
//!
//! O que é **dele**: abrir por **clique** num gatilho (e não por hover, como o
//! [`crate::tooltip::Tooltip`]), o respiro de 16px, e a transição de entrada.
//!
//! ## Por que não é o `Menu` com outro conteúdo
//!
//! O [`crate::menu::Menu`] é uma lista de comandos: ele tem item destacado, navegação por seta,
//! `Home`/`End`, checkbox/radio e a regra de "ação fecha, marcação não". O popover não tem nada disso
//! — o conteúdo dele é arbitrário (um formulário, um calendário, um bloco de texto) e o teclado dele
//! é só `Escape`. Enfiar conteúdo livre no `Menu` significaria carregar toda aquela máquina de
//! seleção inerte; o que os dois compartilham de verdade é a **superfície** e o **posicionador**, e é
//! exatamente isso que este módulo importa em vez de copiar.
//!
//! # Como ele fica por cima de tudo
//!
//! Igual ao menu e à etiqueta: [`gpui::deferred`]`(`[`gpui::anchored`]`(..))`. Não há portal no GPUI
//! (ver o doc do [`crate::menu`] pra a discussão inteira), então os `bounds` do gatilho são medidos
//! por um [`gpui::canvas`] no prepaint, e o `anchored` resolve a posição conhecendo o tamanho do
//! popup na hora certa. A prioridade é a **mesma** dos irmãos (ver [`POPUP_PRIORITY`]) — na
//! referência os três são `z-50`.
//!
//! ⚠️ **Nunca posicione pela medida do popup.** O aviso está inteiro no doc do [`crate::menu`], e
//! aqui ele vale com força: diferente do [`crate::tooltip::Tooltip`], que só monta o popup depois de
//! 600ms (ou seja, dezenas de frames de medida), o popover monta o dele **no mesmo frame** do clique.
//! A [`Popover::popup_bounds`] existe só como observabilidade e não realimenta posição nenhuma.
//!
//! # A animação, e o que dela cabe no GPUI
//!
//! A fonte declara **duas** classes de animação no `Popup`, e nada mais:
//!
//! ```text
//! transition-[width,height,scale,opacity]
//! data-starting-style:scale-98  data-starting-style:opacity-0
//! ```
//!
//! Ou seja: ao **montar**, o popup parte de 98% de escala e opacidade 0, e transita pro repouso
//! (100%/1). São **dois** efeitos, e eles não se traduzem igual:
//!
//! - **fade** (`opacity-0` → repouso): reproduzido. Ver [`Anim::opacity`].
//! - **escala** (`scale-98` → `scale-100`, com `origin-(--transform-origin)`): **não reproduzível**.
//!   Não há `transform` em `div` no GPUI, e "desenhar a 98%" exigiria refazer o layout inteiro em
//!   números — e um popup que muda de tamanho muda também o ponto de ancoragem.
//!
//! ⚠️ **Não há animação de SAÍDA.** O `Popup` da fonte declara só `data-starting-style`; o
//! `data-ending-style` aparece no arquivo **apenas** no `Viewport`, e ali é pro cross-fade dos FILHOS
//! (`**:data-current:data-ending-style:opacity-0`), não pro popup. Sem estilo de saída, o Base UI não
//! tem o que transitar quando fecha: o popup **desmonta na hora**. É o que está implementado — ver
//! [`Anim::mounted`].
//!
//! O `popover.tsx` do shadcn, esse sim, tem saída (`data-[state=closed]:animate-out fade-out-0`).
//! **Não** foi adotada: a fonte não é silenciosa aqui, ela diz o que anima e o que não anima, e o
//! critério deste porte é que onde a fonte fala, ela ganha (ver a tabela abaixo). Se um dia a casa
//! quiser o fade de saída, ele volta como superset declarado — é uma condição em [`Anim::mounted`] e
//! um sinal em [`Anim::opacity`], não um redesenho.
//!
//! # Diferenças em relação ao original — todas declaradas
//!
//! **Duas referências, e onde elas divergem**
//!
//! O `popover.tsx` do coss **não** estende o do shadcn: ele é escrito sobre
//! `@base-ui/react/popover`, não sobre o Radix. Então as duas fontes discordam em três pontos, e a
//! escolha aqui foi consciente:
//!
//! | | coss (Base UI) — **a fonte** | shadcn (Radix) | aqui |
//! |---|---|---|---|
//! | raio | `rounded-lg` (10) | `rounded-md` (8) | **10** |
//! | sombra | `shadow-lg/5` | `shadow-md` | **`shadow-lg/5`** |
//! | largura | `w-(--popup-width,auto)` + `max-w-(--available-width)` | `w-72` (288) | **conteúdo**, com o teto da largura disponível |
//! | animação de entrada | `scale-98` + `opacity-0`, `transition` | `zoom-in-95` + `fade-in-0` + `slide-in-from-*-2` | **só o fade** (escala não é reproduzível) |
//! | animação de saída | nenhuma (só `data-starting-style`) | `fade-out-0` + `zoom-out-95` | **nenhuma** — desmonta na hora |
//!
//! **O critério é um só: onde a fonte fala, ela ganha; o shadcn só entra onde a fonte é silenciosa.**
//! E nos quatro pontos acima a fonte **não** é silenciosa — ela diz `auto`, diz `scale`, e diz o que
//! não anima. Por isso não há `w-72` aqui, não há `slide-in-from-*`, e não há fade de saída: nenhum dos
//! três existe no `popover.tsx` do coss.
//!
//! (Onde o shadcn de fato entra: nada, neste componente. Ele fica na tabela porque a comparação é
//! informativa — e porque foi lendo o shadcn que estes três números entraram por engano numa versão
//! anterior deste módulo.)
//!
//! **Não reproduzível**
//!
//! - **`data-starting-style:scale-98`** (a metade de ESCALA da entrada): sem `transform` em `div`. Ver
//!   a seção da animação, acima. A metade de **opacidade** é fiel e está implementada.
//! - **`origin-(--transform-origin)`**: é o `transform-origin` da escala que não existe.
//! - **`transition-[width,height,…]`** — as partes `width` e `height` da lista: o GPUI não interpola
//!   geometria, então uma troca de conteúdo que mude o tamanho do popup salta em vez de crescer. Só a
//!   parte `opacity` da lista é reproduzida.
//! - **`transition-[top,left,right,bottom,transform]` do `Positioner`**: mesmo motivo. O popup não
//!   desliza de uma posição pra outra quando o gatilho se move; ele aparece na posição final.
//! - **`data-instant:transition-none`**: marca as mudanças que não devem ser animadas, e o caso é
//!   justamente a transição de posição acima, que aqui não existe. Não há o que desligar.
//! - **`not-dark:bg-clip-padding`**: o GPUI pinta o fundo na border box e não tem `background-clip`.
//!   No tema claro a borda (`--border`, preto 8%) lê sobre o branco do próprio popup em vez de compor
//!   com o que houver atrás. Mesma diferença já declarada no [`crate::menu`], no [`crate::dialog`] e
//!   no [`crate::tooltip`].
//! - **O cross-fade de conteúdo do `Viewport`** (todo o bloco `**:data-current:…` /
//!   `**:data-previous:…`, mais o `overflow-clip` e o
//!   `w-[calc(var(--popup-width)-2*var(--viewport-inline-padding)-2px)]`): é a máquina que troca o
//!   conteúdo **com o popup aberto**, esmaecendo o antigo por cima do novo enquanto a caixa muda de
//!   tamanho. Aqui o conteúdo é remontado pelo `render`, e trocá-lo troca o que está na tela no frame
//!   seguinte, sem cruzar os dois. Sem o cross-fade, o `overflow-clip` e a largura calculada do
//!   viewport não têm nada pra fazer — não estão omitidos, estão vazios. Mesma nota do
//!   [`crate::tooltip`].
//! - **`h-(--positioner-height)`/`w-(--positioner-width)` e `h-(--popup-height,auto)`/
//!   `w-(--popup-width,auto)`**: variáveis que o Base UI escreve no DOM pra a máquina de cross-fade
//!   acima ter uma caixa estável durante a troca. Sem cross-fade, o popup mede o conteúdo.
//! - **`before:pointer-events-none`** do bisel: o overlay de bisel é um `div` sem listener nenhum,
//!   então não cria hitbox. Redundante aqui, não omitido.
//! - **`outline-none`**: não há outline no GPUI pra suprimir. (E o popup não desenha anel: quem usa o
//!   [`crate::focus_ring`] é o gatilho, por dentro do [`crate::Button`].)
//! - **`data-slot`**: atributo de DOM, usado no original pra estilizar de fora. Sem DOM, sem
//!   atributo.
//!
//! **Resolvido em número**
//!
//! - **`rounded-lg`** = `--radius-lg` = **10px** — o [`crate::menu::RADIUS`], importado e não
//!   redeclarado.
//! - **O respiro do `Viewport` é 16px nos dois eixos, e a conta é POR EIXO** — a fonte **não** escreve
//!   um `p-4` uniforme, escreve `px-(--viewport-inline-padding) py-4` com
//!   `[--viewport-inline-padding:--spacing(4)]`. Os dois eixos coincidem, mas por caminhos diferentes,
//!   e ambos passam pelo mesmo token:
//!
//!   | eixo | classe | conta | px |
//!   |---|---|---|---|
//!   | horizontal | `px-(--viewport-inline-padding)` = `--spacing(4)` | `4 × var(--spacing)` = `4 × 0.25rem` = `1rem` | **16** |
//!   | vertical | `py-4` | `calc(var(--spacing) * 4)` = `4 × 0.25rem` = `1rem` | **16** |
//!
//!   `--spacing: 0.25rem` medido em `tailwindcss@4.3.3/theme.css:325`, e `1rem` = 16px na raiz default.
//!   Por os dois fecharem em 16 é que [`PAD`] pode ser um número só — se um dia o
//!   `--viewport-inline-padding` do coss mudar, este módulo precisa de dois.
//! - **`shadow-lg/5`**: as duas camadas do `--shadow-lg` do Tailwind v4 com a cor trocada por preto
//!   5% — o [`crate::menu::popup_shadow`], importado.
//! - **`sideOffset={4}`** = o [`crate::menu::SIDE_OFFSET`]; **`alignOffset={0}`** = [`ALIGN_OFFSET`];
//!   **`side="bottom"`**, **`align="center"`** = os defaults do [`MenuSide`]/[`MenuAlign`].
//! - **`before:rounded-[calc(var(--radius-lg)-1px)]`** = 9px na referência, e **10px** aqui. Não é
//!   divergência: lá o pseudo-elemento fica por DENTRO da borda (padding box, raio − 1); aqui o
//!   overlay cobre a **border box** (`inset: -1px`). Explicado inteiro no
//!   [`crate::menu::bevel_overlay`].
//! - **A duração e a curva da entrada saem do `transition-[…]` da fonte**, que não traz classe de
//!   `duration-*` nem de `ease-*` — então valem os dois defaults do Tailwind v4, medidos em
//!   `tailwindcss@4.3.3/theme.css:492-493`:
//!
//!   ```text
//!   --default-transition-duration: 150ms;
//!   --default-transition-timing-function: cubic-bezier(0.4, 0, 0.2, 1);
//!   ```
//!
//!   Ver [`DURATION`] e [`EASE`]. ⚠️ Essa curva **não** é a `ease-in-out` do CSS
//!   (`cubic-bezier(.42,0,.58,1)`) que o [`crate::dialog`] usa, nem a `ease` (`.25,.1,.25,1)`) que o
//!   `tw-animate-css` do shadcn usaria: é o token que o Tailwind **chama** de `--ease-in-out` e que
//!   vale por default em todo `transition` sem easing explícito. Três curvas parecidas com nomes
//!   colididos — a conta acima é a que vale aqui.
//! - **`data-starting-style:opacity-0`** = opacidade inicial **0**, transitando pro repouso (1).
//! - **`PopoverTitle`** = `font-semibold text-lg leading-none`: **18px**, peso 600, entrelinha
//!   `1 × 18` = **18**. ⚠️ O par default do `text-lg` no Tailwind é `18/28`; o `leading-none` o
//!   sobrepõe. Ver [`TITLE_SIZE`] e [`TITLE_LINE_HEIGHT`].
//! - **`PopoverDescription`** = `text-muted-foreground text-sm`: **14/20**, o
//!   [`crate::menu::TEXT_SIZE`]/[`crate::menu::TEXT_LINE_HEIGHT`], na cor `muted` da paleta do menu.
//! - **`max-h-(--available-height)` + `overflow-y-auto`** = o [`crate::menu::popup_max_height`] mais
//!   uma área rolável — o mesmo arranjo da lista do menu, e pelo mesmo motivo: quem rola é o
//!   conteúdo, porque a superfície **não** pode ter `overflow_hidden` (recortaria o bisel).
//! - **`max-w-(--available-width)`** = o [`crate::tooltip::popup_max_width`], que é o simétrico exato
//!   do `popup_max_height` no outro eixo e já existia na lib. Nos lados verticais é a janela toda menos
//!   as duas margens de colisão; nos horizontais, o maior dos dois espaços ao lado do gatilho, com um
//!   piso pra um popover encurralado não nascer ilegível.
//! - **`w-(--popup-width,auto)`** = **largura de conteúdo**, que é o default e não tem constante.
//!
//!   ⚠️ **"Conteúdo" aqui é `max-content`, não `min-content`** — e a diferença já custou um defeito
//!   nesta base. O `width: auto` do CSS num elemento posicionado é **shrink-to-fit**:
//!
//!   ```text
//!   min( max(min-content, disponível), max-content )
//!   ```
//!
//!   O *preferido* é o `max-content` (a frase inteira numa linha), e o `min-content` (a palavra mais
//!   longa) é só o PISO. Uma versão anterior deste módulo entregava o piso: a superfície era
//!   `.flex().flex_col()`, e numa coluna a largura do filho é o eixo TRANSVERSAL — o
//!   `align-items: stretch` default a amarrava à largura do container de centralização, que é a largura
//!   do GATILHO. O texto media contra ~66px e quebrava; num título de uma palavra, quebrava **letra por
//!   letra** (medido: `34×196` num popup que devia ser `132×52` — 9 linhas pra "Dimensões"). A fonte
//!   escreve `relative flex …` no `Popup`, ou seja uma LINHA, e é numa linha que a largura do filho é o
//!   eixo principal e o tamanho base sai do conteúdo. Ver o comentário no [`popup_surface`].
//!
//!   Os testes que faltavam eram os do **piso**: `o_titulo_de_uma_linha_nao_quebra` e
//!   `a_largura_acompanha_o_conteudo_e_a_frase_nao_quebra`. Os antigos só olhavam o teto, e todos usavam
//!   `div` de largura declarada — num `div` fixo `min-content == max-content`, então **nenhum deles
//!   podia ver o defeito**. Os novos usam TEXTO e afirmam a ALTURA (a contagem de linhas), que não
//!   depende da métrica de fonte da plataforma.
//!
//! **Desvio consciente**
//!
//! - **No tema escuro o fio de bisel é o DOBRO da referência** (branco ~11,8%, alfa `0x1e`, em vez de
//!   6%). Não é uma decisão deste módulo: ele importa a paleta do [`crate::menu`], onde o desvio está
//!   declarado e travado por teste. Os cinco módulos da casa (`menu`, `input`, `card`, `select`,
//!   `tooltip`) usam o mesmo valor, e o popover aparece encostado neles.
//! - **`side`/`align` são o [`MenuSide`]/[`MenuAlign`] do [`crate::menu`]**, não um par de enums
//!   próprios com os mesmos quatro nomes. Dois enums idênticos obrigariam a uma tradução entre eles
//!   pra chamar [`crate::menu::anchor_point`], e é exatamente numa tradução dessas que um `Left`
//!   viraria `Right` um dia. Mesma decisão do [`crate::tooltip`].
//! - **Encurralado na borda, o popup DESLIZA pra dentro da janela em vez de virar de lado.** Os dois
//!   são exclusivos no [`gpui::anchored`] (o `fit_mode` é um campo só) — a discussão inteira está em
//!   [`crate::menu`]. Aqui, como lá, `side` é uma decisão do call site.
//! - **[`Popover::width`] existe, e a fonte não tem largura fixa.** É um **escape** explícito, não o
//!   default: sem chamada nenhuma o popup mede o conteúdo, como a fonte manda. O setter existe porque
//!   no CSS quem quer largura fixa passa `className="w-72"` e aqui não há cascata de classes. Quando
//!   usado, ele conviveu com o teto de [`crate::tooltip::popup_max_width`] de propósito — uma largura
//!   pedida nunca deve empurrar o popover pra fora da janela.
//!
//! **Superset consciente**
//!
//! - **O popup declara o corpo e a entrelinha do texto** (`text-sm`, 14/20), que o original **herda**
//!   do documento. O GPUI não tem `inherit` de CSS, e sem corpo declarado o conteúdo sairia no
//!   default da janela — 16px com a entrelinha da razão de ouro (25,89px). 14px é o corpo do
//!   `PopoverHeader` do shadcn e o do item do menu, então é o valor que não surpreende.
//! - **`min_w(0)` na área de conteúdo**, que a referência não tem. No CSS o min-content de um
//!   parágrafo é a palavra mais longa, e o texto reflui sozinho; no GPUI o min-content de um texto é a
//!   **frase inteira numa linha**, então sem essa liberdade o conteúdo empurraria a caixa em vez de
//!   quebrar. Mesma armadilha, e mesma cura, do [`crate::card`] e do [`crate::tooltip`]. É o que faz o
//!   [`Popover::width`] refluir o texto em vez de deixá-lo sair pela borda — travado em
//!   `com_largura_pedida_o_texto_reflui_dentro_dela`.
//! - **A altura do `Viewport` sai do `stretch`, e não de um `h-full` declarado.** A fonte escreve
//!   `size-full` (largura E altura a 100%); aqui a largura é o `w_full` (que é o que faz o viewport
//!   preencher uma largura PEDIDA), e a altura vem do `align-items: stretch` default da linha de flex —
//!   a superfície tem um filho em fluxo só. Um `h_full` explícito seria uma porcentagem contra uma
//!   altura indefinida, que é justamente a armadilha que o `crate::table` pagou.
//! - **`occlude()` no popup**: no GPUI o hit test acumula TODOS os hitboxes sob o cursor, não só o de
//!   cima — sem isto o clique atravessaria o popover e chegaria no que está atrás dele.
//!
//! **O que NÃO está aqui (declarado, não esquecido)**
//!
//! - **`tooltipStyle` do coss** — a variante booleana do `PopoverPopup`, que troca a superfície inteira:
//!   `w-fit text-balance rounded-md text-xs shadow-md/5
//!   before:rounded-[calc(var(--radius-md)-1px)]`, e no `Viewport`
//!   `py-1 [--viewport-inline-padding:--spacing(2)]`. Isso é, número por número, o
//!   [`crate::tooltip::Tooltip`], que já existe nesta lib com exatamente esses valores: raio **8**
//!   (`rounded-md`), `text-xs` **12/16**, `shadow-md/5`, e respiro **8×4** (`--spacing(2)` = 8 na
//!   horizontal, `py-1` = 4 na vertical). Reproduzi-lo aqui seria duas verdades pra uma etiqueta. Quem
//!   quer o `tooltipStyle` usa o `Tooltip`.
//!
//!   ⚠️ É por isso que o `py-1` e o `p-2` que aparecem na lista de classes do arquivo **não** são peças
//!   que ficaram de fora: o `py-1` é deste ramo `tooltipStyle` (portado no `Tooltip`), e o `p-2` é do
//!   ramo `has-data-[slot=calendar]` (logo abaixo, alcançável por [`Popover::padding`]). O único
//!   respiro do popover em si é o `px-(--viewport-inline-padding) py-4` = 16px.
//! - **`PopoverClose` como elemento**: o fechamento é [`Popover::close`], chamado do `on_click` do que
//!   você montar no conteúdo. Na referência o `Close` não tem estilo nenhum (é só um `<button>` que
//!   fecha), então não há pixel a portar.
//! - **`PopoverAnchor` / o `anchor` do `Positioner`** (ancorar em outro elemento que não o gatilho):
//!   ausente, como no [`crate::menu`] e no [`crate::tooltip`].
//! - **`PopoverCreateHandle`** (abrir/fechar de fora, por referência): o equivalente aqui é ter o
//!   [`gpui::Entity`] em mão e chamar [`Popover::set_open`] — que é o que a lib inteira faz. O
//!   `createHandle` existe no React porque lá não se tem referência ao estado; aqui se tem.
//! - **`PopoverHeader` do shadcn** (`flex flex-col gap-1 text-sm`): não existe no coss, e é um
//!   `div().flex().flex_col().gap(px(4.0))` — não vale um tipo.
//! - **`has-data-[slot=calendar]:rounded-xl` e `has-data-[slot=calendar]:p-2` do coss** (o popover que
//!   hospeda um calendário arredonda mais e respira menos): depende de o popup inspecionar o
//!   `data-slot` dos DESCENDENTES, que é DOM. O respiro é alcançável por [`Popover::padding`]; o raio
//!   não tem setter, porque `rounded-xl` (14) num popover de conteúdo genérico seria uma segunda
//!   verdade de superfície sem ninguém pedindo.
//! - **O foco automático no conteúdo ao abrir** (o Radix e o Base UI movem o foco pra dentro do popup,
//!   e o devolvem ao gatilho ao fechar): ausente. O foco fica no gatilho, e o `Escape` chega ao
//!   popover porque o `on_key_down` é ouvido pela RAIZ — o evento sobe do elemento focado pelos
//!   ancestrais. A consequência declarada: um campo de texto dentro do popover precisa de clique (ou
//!   de foco explícito pelo call site) pra receber o teclado.
//! - **`modal` do Radix** (travar a rolagem e o foco atrás do popup): ausente, e de propósito — o
//!   popover não é modal. Quem quer modalidade usa o [`crate::dialog`] ou o [`crate::sheet`].
//! - **RTL**: [`MenuSide::Left`]/[`MenuSide::Right`] são o `inline-start`/`inline-end` resolvidos pra
//!   LTR. Mesma nota do [`crate::menu`].
//!
//! **Sem cobertura de teste — declarado**
//!
//! - **A opacidade PINTADA durante a animação.** O teste `a_superficie_carrega_o_contrato` confere que
//!   a opacidade chega ao `Div` (inspecionando o `StyleRefinement`), mas ninguém lê o framebuffer:
//!   não há como afirmar, nesta lib, que o pixel saiu a 50%.
//! - **Três declarações de layout são INERTES hoje, e isso é medido, não suposto**: o `flex_none` da
//!   superfície, o `flex_none` do wrap de medida, e o `w_full` do viewport. Tirar qualquer uma das três
//!   não muda medida nenhuma em nenhum teste — quem segura o popup no tamanho do conteúdo é o mínimo
//!   automático do `taffy`. As três ficam por fidelidade (o `size-full` do `Viewport` está no `.tsx`) e
//!   por serem o mecanismo declarado no [`crate::tooltip`], que tem a mesma nota. No caso do `w_full` eu
//!   **tentei** escrever o teste que lhe daria dentes (contar as linhas em que o texto quebra dentro de
//!   uma largura pedida) e ele passava com e sem a linha: era um teste vazio, e foi removido em vez de
//!   mantido pra inflar o número.
//! - **Os lados HORIZONTAIS não têm teste de texto próprio, e a razão é uma medida.** Escrevi um
//!   (`side: left`/`right` com título de uma palavra) e depois procurei a mutação que o mataria: nem
//!   trocar a superfície pra coluna, nem dar `w(anchor.w)` ao ramo horizontal do
//!   [`crate::menu::align_center_wrap`] o derrubava. Numa LINHA de flex a largura é o eixo principal e o
//!   mínimo automático do `taffy` a mantém no max-content **nos dois eixos** — então o conserto vale
//!   igual pros quatro lados, e o teste não distinguia nada. Removido em vez de mantido sem dentes; o
//!   caminho horizontal continua exercitado por `encurralado_na_borda_o_popup_gruda_na_janela`, que é
//!   quem pega o `side` não chegando ao container de centralização.
//! - **A `escala` que não existe**, obviamente: não há o que testar num efeito não reproduzido.
//! - **A bissecção da bezier em si** não é reafirmada aqui: ela é a do
//!   [`crate::dialog::cubic_bezier`], que tem os testes dela lá. O que este módulo testa é que os
//!   pontos de controle são os do default do Tailwind e que a curva resultante difere das outras duas
//!   curvas de nome parecido que circulam nesta base.

use crate::menu::{
    align_center_wrap, anchor_point, bevel_overlay, outside_click_closes, palette, popup_max_height,
    popup_shadow, rect_of, MenuAlign, MenuSide, BORDER, RADIUS, SIDE_OFFSET, TEXT_LINE_HEIGHT,
    TEXT_SIZE, WINDOW_MARGIN,
};
use crate::tooltip::popup_max_width;
use crate::{Button, ButtonVariant};
use gpui::{
    anchored, canvas, deferred, div, point, prelude::FluentBuilder as _, px, AnyElement, App, Bounds,
    Context, Div, EventEmitter, FocusHandle, Focusable, FontWeight, InteractiveElement, IntoElement,
    KeyDownEvent, MouseDownEvent, ParentElement, Pixels, Render, RenderOnce, ScrollHandle,
    SharedString, StatefulInteractiveElement as _, Styled, Window,
};
use std::time::{Duration, Instant};

// =================================================================================================
// Eventos
// =================================================================================================

/// O que o [`Popover`] emite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopoverEvent {
    /// O popup abriu (`true`) ou fechou (`false`) — o `onOpenChange` da referência.
    ///
    /// Emitido no instante da decisão, e **não** no fim da animação: quem assina quer saber que o
    /// usuário fechou o popover, não que o fade terminou.
    OpenChange(bool),
}

// =================================================================================================
// Geometria
// =================================================================================================
//
// Os utilitários Tailwind do `popover.tsx` resolvidos em número. 1 unidade Tailwind = 4px
// (`var(--spacing)`); `--radius` = 0.625rem = 10px. Note que a superfície (raio, borda, sombra,
// cores) NÃO tem constante aqui: ela é importada do `crate::menu`, que é o mesmo `bg-popover`.

// ⚠️ **Não há constante de largura**, e isso é a fonte falando: o `PopoverPopup` é
// `w-(--popup-width,auto)`, ou seja largura de CONTEÚDO, limitada por `max-w-(--available-width)`. Uma
// versão anterior deste módulo tinha um `WIDTH: f32 = 288.0` ("`w-72`") que **não existe no
// `popover.tsx`** — ele veio do shadcn, que é outro componente. Se você for adicionar um default de
// largura, confira a fonte primeiro.

/// Respiro em volta do conteúdo — **16px**.
///
/// ⚠️ A fonte **não** escreve um `p-4` uniforme: ela escreve
/// `px-(--viewport-inline-padding) py-4` com `[--viewport-inline-padding:--spacing(4)]`. São dois
/// caminhos que caem no mesmo número, e é por caírem no mesmo número que aqui cabe UMA constante:
///
/// | eixo | classe | conta | px |
/// |---|---|---|---|
/// | horizontal | `px-(--viewport-inline-padding)` = `--spacing(4)` | `4 × 0.25rem` = `1rem` | 16 |
/// | vertical | `py-4` | `calc(var(--spacing) * 4)` = `4 × 0.25rem` = `1rem` | 16 |
///
/// `--spacing: 0.25rem` medido em `tailwindcss@4.3.3/theme.css:325`. Se o coss um dia mudar o
/// `--viewport-inline-padding`, os eixos divergem e este módulo passa a precisar de dois números.
const PAD: f32 = 16.0;

/// Corpo do [`PopoverTitle`] — `text-lg` = `1.125rem` = **18px**.
const TITLE_SIZE: f32 = 18.0;

/// **Entrelinha** do [`PopoverTitle`] — `leading-none` = `line-height: 1`, ou seja `1 × 18` = **18**.
///
/// ⚠️ O par default do `text-lg` no Tailwind é `18px/28px`; a classe `leading-none` o **sobrepõe**.
/// Sem declarar, o GPUI usaria `phi()` (a razão de ouro, 1,618034 × o corpo) = 29,12px — 62% mais
/// alta que os 18 do coss, e nada no código pareceria errado.
const TITLE_LINE_HEIGHT: f32 = TITLE_SIZE;

/// Deslocamento no eixo de **alinhamento**. A fonte passa `alignOffset = 0`, mas o popover expõe o
/// setter ([`Popover::align_offset`]) porque o `Positioner` o aceita e um popover preso num canto de
/// painel às vezes precisa dos 2px.
const ALIGN_OFFSET: f32 = 0.0;

/// Prioridade do [`gpui::deferred`] do popup.
///
/// **A mesma do [`crate::menu`], do [`crate::select`] e do [`crate::tooltip`]**, porque na referência
/// é o mesmo `z-50`. Empatados na prioridade, quem pinta por cima é quem vem depois na árvore — que é
/// exatamente a regra do `z-index` igual no CSS.
const POPUP_PRIORITY: usize = 1;

// =================================================================================================
// A animação (o `transition-[…]` + `data-starting-style` da fonte, reduzido ao que o GPUI pinta)
// =================================================================================================
//
// A fonte declara `transition-[width,height,scale,opacity]` no `Popup`, e como estado inicial de
// montagem `data-starting-style:scale-98 data-starting-style:opacity-0`. Não há `data-ending-style` no
// `Popup` — ele aparece só no `Viewport`, e ali é pro cross-fade dos FILHOS. Logo: **entrada animada,
// saída instantânea**. Ver o doc do módulo.

/// Duração da entrada.
///
/// **Medido**, não deduzido: a fonte escreve `transition-[…]` **sem** classe `duration-*`, então vale o
/// `--default-transition-duration` do Tailwind v4 = **150ms**
/// (`tailwindcss@4.3.3/theme.css:492`).
const DURATION: Duration = Duration::from_millis(150);

/// Os pontos de controle da curva da entrada.
///
/// **Medido**: a fonte escreve `transition-[…]` **sem** classe `ease-*`, então vale o
/// `--default-transition-timing-function` do Tailwind v4 = `cubic-bezier(0.4, 0, 0.2, 1)`
/// (`tailwindcss@4.3.3/theme.css:493`).
///
/// ⚠️ **Três curvas de nome colidido circulam nesta base — esta é a terceira, e não é nenhuma das
/// outras duas:**
///
/// | quem | pontos | onde |
/// |---|---|---|
/// | `ease-in-out` do **CSS** | `.42, 0, .58, 1` | [`crate::dialog`] (`duration-200 ease-in-out`) |
/// | `ease` do **CSS** | `.25, .1, .25, 1` | o `tw-animate-css` do shadcn (que este módulo NÃO usa) |
/// | default de `transition` do **Tailwind** | `.4, 0, .2, 1` | **aqui** |
///
/// O Tailwind chama o terceiro de `--ease-in-out`, o que é justamente a armadilha: ele não é o
/// `ease-in-out` do CSS. Trocar um pelo outro não quebra nada visível num teste de constante — só muda
/// o ritmo da abertura.
const EASE: (f32, f32, f32, f32) = (0.4, 0.0, 0.2, 1.0);

/// A curva da entrada avaliada em `t` ∈ `[0,1]`.
///
/// A bissecção é a do [`crate::dialog::cubic_bezier`], importada e não reescrita: a conta de inverter
/// `x(u) = t` numa bezier do CSS é uma só nesta lib.
fn ease(t: f32) -> f32 {
    let (x1, y1, x2, y2) = EASE;
    crate::dialog::cubic_bezier(x1, y1, x2, y2, t)
}

/// O relógio da **entrada** — o `transition-[…,opacity]` + `data-starting-style:opacity-0` da fonte.
///
/// Está num tipo próprio (e não solto no [`Popover`]) porque é a única parte da abertura que é
/// **pura**: o [`Popover`] carrega um [`FocusHandle`], que só existe com uma [`App`] viva, e isso
/// tornaria a máquina de estados intestável sem abrir janela. Mesma ideia do `Fade` do
/// [`crate::dialog`], mas **mais simples**, e a simplicidade é a fonte falando:
///
/// - o modal tem entrada E saída, então precisa guardar "de que opacidade a transição partiu" pra
///   inverter no meio sem salto;
/// - o popover **não tem saída** (ver o doc do módulo): fechar desmonta. Então não há transição a
///   inverter, e o estado inteiro é **um instante**: quando o popup montou. Fechado é `None`.
///
/// Reabrir depois de fechar recomeça a entrada do zero, e isso não é uma decisão nossa: no navegador o
/// elemento é REMOVIDO da árvore e re-inserido, então o `data-starting-style` se aplica de novo, do
/// começo. Aqui sai igual, e de graça.
#[derive(Clone, Copy, Debug, Default)]
struct Anim {
    /// Quando o popup montou. `None` = fechado, nada na árvore.
    since: Option<Instant>,
}

impl Anim {
    /// Se o popup está aberto — que aqui é o mesmo que "montado", já que não há saída animada.
    fn open(&self) -> bool {
        self.since.is_some()
    }

    /// Quanto da entrada já correu, em `[0,1]`. Fechado, é 0.
    fn progress(&self) -> f32 {
        match self.since {
            None => 0.0,
            Some(start) => (start.elapsed().as_secs_f32() / DURATION.as_secs_f32()).clamp(0.0, 1.0),
        }
    }

    /// A opacidade VISÍVEL agora: 0 no frame da montagem (o `data-starting-style:opacity-0`), 1 no fim
    /// da transição.
    ///
    /// ⚠️ **As duas pontas são exatas, e não é preciosismo.** A bissecção de [`ease`] devolve ~7e-13 de
    /// resíduo nas pontas, e um `0,999_999_999_999` no repouso fazia o
    /// `when(opacity < 1.0, …)` do [`popup_surface`] pedir um passe de opacidade **pra sempre** num
    /// popover parado — invisível na tela, permanente no perfil de pintura.
    fn opacity(&self) -> f32 {
        let t = self.progress();
        if t <= 0.0 {
            0.0
        } else if t >= 1.0 {
            1.0
        } else {
            ease(t)
        }
    }

    /// Troca o estado. Devolve `false` se ele já era esse (e então nada acontece — nem evento, nem
    /// reinício de transição, que é o que faz um segundo `open()` não piscar o popup).
    fn set(&mut self, open: bool) -> bool {
        if self.open() == open {
            return false;
        }
        self.since = open.then(Instant::now);
        true
    }

    /// Se ainda há transição em curso — é o que decide pedir o próximo frame.
    ///
    /// Aberto e assentado, isto é `false`: um popover parado não queima frame nenhum.
    fn animating(&self) -> bool {
        self.since.is_some_and(|start| start.elapsed() < DURATION)
    }

    /// Se o popup deve estar MONTADO.
    ///
    /// **É o próprio `open`**, e é aqui que a ausência de animação de saída vive: fechado, o render não
    /// monta nada — nem pintura, nem hitbox. (Se um dia a casa quiser o fade de saída do shadcn, é esta
    /// função que passa a ser `open || opacidade > 0`, e a [`Self::opacity`] que ganha o sinal.)
    ///
    /// ⚠️ Nota de verificação: escrever `self.open() || self.opacity() > 0.0` aqui é hoje um mutante
    /// **equivalente** — fechar zera o relógio, então `opacity()` devolve exatamente 0 e o segundo termo
    /// nunca dispara. Só é equivalente porque a [`Self::opacity`] apara as pontas: com o resíduo de
    /// ~7e-13 da bissecção, esse mesmo `> 0.0` deixaria o popup montado **pra sempre** depois do
    /// primeiro fechamento. É o tipo de defeito que nenhum teste de constante pega, e é por isso que a
    /// exatidão das pontas é testada.
    fn mounted(&self) -> bool {
        self.open()
    }

    /// Põe a transição no fim, sem esperar. Só em teste: o progresso vem de [`Instant`] (relógio de
    /// parede), que o executor de teste do GPUI não adianta.
    #[cfg(test)]
    fn settle(&mut self) {
        if self.since.is_some() {
            self.since = Instant::now().checked_sub(DURATION);
        }
    }
}

// =================================================================================================
// Teclado
// =================================================================================================

/// O que uma tecla faz num popover — `Some(novo estado)`, ou `None` se a tecla não é dele.
///
/// - **fechado**: `Enter`/`Space` abrem. É a semântica de botão do `PopoverTrigger` (na referência ele
///   é um `<button>`, e o navegador o ativa por tecla; o [`crate::Button`] desta lib **não** trata
///   tecla, então quem trata é o popover — mesma divisão do [`crate::menu`]).
/// - **aberto**: `Escape` fecha. E só: o conteúdo é arbitrário, então setas e `Enter` pertencem a
///   quem estiver dentro do popup, não ao popover.
///
/// Função livre porque é a decisão inteira do teclado: dentro do `if` de um handler ela seria
/// inalcançável por teste.
fn key_action(open: bool, key: &str) -> Option<bool> {
    match (open, key) {
        (true, "escape") => Some(false),
        (false, "enter" | "space") => Some(true),
        _ => None,
    }
}

// =================================================================================================
// Peças visuais (funções livres, pra serem testáveis sem construir um `Popover`)
// =================================================================================================

/// A **superfície** do popup: tudo que é aparência, e nada de posição.
///
/// É uma função livre, e não um trecho do [`Popover::render_popup`], por um motivo bem concreto: ela
/// devolve um [`Div`], e um `Div` pode ser **inspecionado** por [`gpui::Styled::style`] num teste de
/// unidade — sem janela, sem GPU. Assim cada token do contrato chega ao elemento que vai pra tela, e
/// não só à constante de onde ele saiu (ver `a_superficie_carrega_o_contrato`).
///
/// ⚠️ Ela **não** tem `overflow_hidden`: recortaria o bisel, que é filho absoluto sobre a borda. Quem
/// rola é a área de conteúdo, e o bisel é irmão dela. Mesma regra do popup do [`crate::menu`].
fn popup_surface(width: Option<f32>, max_width: f32, opacity: f32) -> Div {
    let p = palette();
    let surface = div()
        .relative()
        // `occlude`: captura o mouse, não vaza clique pro conteúdo atrás (no GPUI o hit test acumula
        // todos os hitboxes sob o cursor).
        .occlude()
        // ⚠️ **Uma LINHA de flex, e não uma coluna — isto não é cosmético.**
        //
        // A fonte escreve `relative flex …` no `Popup` (row), com o `Viewport` de `size-full` dentro. Uma
        // versão anterior daqui escrevia `.flex().flex_col()`, e o `flex_col` **quebrava a largura**: num
        // container de coluna a largura do filho é o eixo TRANSVERSAL, então o `align-items: stretch`
        // default o amarrava à largura do container de centralização — que é a largura do GATILHO (ver
        // `crate::menu::align_center_wrap`, que declara `w(anchor.w)`). O texto então media contra ~66px
        // e quebrava; com um título de uma palavra, quebrava **letra por letra** (medido: 34×196 num
        // popup que devia ser 132×52 — 9 linhas pra "Dimensões").
        //
        // Numa LINHA a largura do filho é o eixo PRINCIPAL, e o tamanho base dele sai do conteúdo
        // (max-content) em vez do espaço disponível — que é o `shrink-to-fit` do CSS,
        // `min(max(min-content, disponível), max-content)`, e é o que `w-(--popup-width,auto)` pede.
        //
        // Uma coluna aqui nunca teve função: a superfície tem UM filho em fluxo (o viewport); o bisel é
        // absoluto. Quem empilha o conteúdo é o `flex_col` do viewport, que continua lá.
        //
        // Travado em `tests_de_janela::o_titulo_de_uma_linha_nao_quebra` e
        // `a_largura_acompanha_o_conteudo_e_a_frase_nao_quebra`, que medem a ALTURA (ou seja, a contagem
        // de linhas) e por isso não dependem da métrica de fonte da plataforma.
        .flex()
        // `flex_none` porque o popup é quase sempre MAIOR que o container que o centraliza: é ele que
        // garante que o espaço livre negativo transborde pros dois lados em vez de o popup ser encolhido
        // pro tamanho do gatilho.
        //
        // ⚠️ Honestidade, medida por mutação: **hoje esta linha é INERTE** — e a do `wrap` de medida
        // também. Tirar qualquer uma das duas não muda medida nenhuma em nenhum dos testes: quem segura
        // o popup no tamanho do conteúdo é o **mínimo automático do `taffy`**, não o `flex-shrink`. As
        // duas ficam porque são o mecanismo declarado no módulo irmão (o [`crate::tooltip`] tem a mesma
        // linha e a mesma nota) e porque são a primeira coisa a reexaminar se o encadeamento de
        // containers do popup mudar.
        .flex_none()
        // O `max-w-(--available-width)` do `Positioner`. Ele só APARA a caixa; quem faz o conteúdo
        // caber dentro dela é o `min_w(0)` da área de conteúdo. Vale TAMBÉM quando há largura pedida
        // por [`Popover::width`]: uma largura de escape não deve empurrar o popover pra fora da janela.
        .max_w(px(max_width))
        .bg(p.popover_bg.hsla())
        .border(px(BORDER))
        .border_color(p.border.hsla())
        .rounded(px(RADIUS))
        // Sombra EXTERNA atrás de um fundo OPACO: aqui a armadilha do `paint_shadows` (que não recorta
        // a sombra pra fora do elemento, como o CSS faz) não morde — o retângulo cheio fica escondido
        // pelo `--popover`.
        .shadow(popup_shadow())
        // `text-popover-foreground`. O corpo e a entrelinha são superset declarado (ver o doc do
        // módulo): o original herda os dois do documento, e o GPUI não tem `inherit`.
        .text_color(p.text.hsla())
        .text_size(px(TEXT_SIZE))
        .line_height(px(TEXT_LINE_HEIGHT))
        // A metade REPRODUZÍVEL da entrada (a de escala não existe no GPUI — ver o doc do módulo).
        // `opacity(1.0)` seria um passe de pintura a mais por nada.
        .when(opacity < 1.0, |d| d.opacity(opacity));
    // Sem largura pedida, NADA é declarado: um item de flex sem `w` fecha no conteúdo, que é
    // exatamente o `w-(--popup-width,auto)` da fonte.
    match width {
        Some(w) => surface.w(px(w)),
        None => surface,
    }
}

/// O `PopoverTitle` da referência — `font-semibold text-lg leading-none`.
///
/// Função livre pelo mesmo motivo do [`popup_surface`]: um [`Div`] é inspecionável em teste de
/// unidade, e o [`PopoverTitle`] (que é um [`RenderOnce`]) não é.
fn title_element(text: SharedString) -> Div {
    div()
        .text_size(px(TITLE_SIZE))
        // Ver [`TITLE_LINE_HEIGHT`]: `leading-none` é 1 × o corpo, e sem declarar o GPUI usaria a
        // razão de ouro (29,12px em vez de 18).
        .line_height(px(TITLE_LINE_HEIGHT))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(palette().text.hsla())
        .child(text)
}

/// O `PopoverDescription` da referência — `text-muted-foreground text-sm`.
fn description_element(text: SharedString) -> Div {
    div()
        .text_size(px(TEXT_SIZE))
        .line_height(px(TEXT_LINE_HEIGHT))
        .text_color(palette().muted.hsla())
        .child(text)
}

// =================================================================================================
// O componente
// =================================================================================================

/// A assinatura de um gatilho montado pelo call site (ver [`Popover::trigger`]). Fica num alias
/// porque inline ela é complexa demais até pro clippy.
type TriggerRender = dyn Fn(bool, &mut Window, &mut Context<Popover>) -> AnyElement;

/// A assinatura do conteúdo do popup (ver [`Popover::content`]).
///
/// É um `Fn` e não um `AnyElement` guardado porque o conteúdo é **remontado a cada frame**, como todo
/// elemento do GPUI: um `AnyElement` só pode ser consumido uma vez, e o popup é redesenhado a cada
/// passo do fade.
type ContentRender = dyn Fn(&mut Window, &mut Context<Popover>) -> AnyElement;

/// O que renderiza o gatilho.
enum PopoverTrigger {
    /// Um [`Button`] com rótulo (o embutido).
    Button(SharedString),
    /// Um [`Button`] só de ícone, quadrado.
    IconButton(SharedString),
    /// Qualquer elemento, montado pelo call site. Recebe `open` (pra o gatilho poder reagir a estar
    /// aberto, o `data-popup-open` da referência).
    Custom(Box<TriggerRender>),
}

/// O popover (gatilho + camada flutuante de conteúdo livre). Ver o doc do módulo.
///
/// É um `Entity` (view) que mantém o seu estado (aberto, animação) e **emite** [`PopoverEvent`]. Quem
/// usa assina (`cx.subscribe`) e age.
pub struct Popover {
    /// A animação, e com ela o estado "aberto".
    anim: Anim,
    smooth: bool,
    fade: crate::motion::Tween,
    /// O gatilho.
    trigger: PopoverTrigger,
    /// O conteúdo do popup. `None` = popup vazio (só a superfície e o respiro).
    content: Option<Box<ContentRender>>,
    side: MenuSide,
    align: MenuAlign,
    side_offset: f32,
    align_offset: f32,
    /// Largura FIXA do popup, se o call site pediu uma. `None` — o **default** — é o
    /// `w-(--popup-width,auto)` da fonte: a caixa mede o conteúdo. Ver [`Popover::width`].
    width: Option<f32>,
    /// Respiro em volta do conteúdo.
    padding: f32,
    /// Bounds do gatilho, medidos por `canvas` no prepaint — o "anchor rect" do `Positioner`, e o
    /// retângulo em que um mouse-down **não** conta como "clique fora" (ver
    /// [`crate::menu::outside_click_closes`]).
    trigger_bounds: Bounds<Pixels>,
    /// A caixa do popup, medida por `canvas` no prepaint — a BORDER box, em pixels de janela.
    ///
    /// **Não** participa do posicionamento (ver o aviso no doc do módulo). É observabilidade.
    popup_bounds: Option<Bounds<Pixels>>,
    /// Id estável desta entidade (pra `div().id(..)` único na árvore).
    id: u64,
    focus_handle: FocusHandle,
    /// Rolagem do conteúdo. Persiste entre renders: sem ela, cada `cx.notify()` devolveria o conteúdo
    /// pro topo no meio da rolagem.
    scroll: ScrollHandle,
}

impl Popover {
    /// Cria um popover fechado, nos defaults da fonte: abaixo do gatilho, centralizado, a 4px, com a
    /// largura do **conteúdo** (limitada pela largura disponível da janela) e 16px de respiro.
    ///
    /// O gatilho embutido é um [`Button`] rotulado `Popover` — troque com [`Self::trigger_button`] ou
    /// [`Self::trigger`]. O conteúdo vem de [`Self::content`].
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            anim: Anim::default(),
            smooth:false,fade:crate::motion::Tween::new(0.),
            trigger: PopoverTrigger::Button(SharedString::from("Popover")),
            content: None,
            side: MenuSide::default(),
            align: MenuAlign::default(),
            side_offset: SIDE_OFFSET,
            align_offset: ALIGN_OFFSET,
            // O `w-(--popup-width,auto)` da fonte: a caixa mede o conteúdo. Uma versão anterior deste
            // módulo nascia com 288px ("`w-72`"), que é do shadcn e NÃO existe no `popover.tsx`.
            width: None,
            padding: PAD,
            trigger_bounds: Bounds::default(),
            popup_bounds: None,
            id: cx.entity_id().as_u64(),
            focus_handle: cx.focus_handle(),
            scroll: ScrollHandle::new(),
        }
    }

    /// Close immediately when another popup takes its place.
    pub fn dismiss(&mut self, cx: &mut Context<Self>) {
        self.set_open(false,cx);self.fade=crate::motion::Tween::new(0.);cx.notify();
    }
    /// Enable interruptible entrance and exit fades. Off by default.
    pub fn motion(mut self, enabled: bool) -> Self {self.smooth=enabled;self}

    pub fn trigger_button(mut self, label: impl Into<SharedString>) -> Self {
        self.trigger = PopoverTrigger::Button(label.into());
        self
    }

    /// Gatilho = um [`Button`] quadrado só com este ícone (ex.: `"iconoir/regular/settings.svg"`).
    pub fn trigger_icon_button(mut self, path: impl Into<SharedString>) -> Self {
        self.trigger = PopoverTrigger::IconButton(path.into());
        self
    }

    /// Gatilho = o que você montar. Recebe `open` e devolve o elemento.
    ///
    /// O clique é tratado pelo **wrap** que o `Popover` põe em volta, não pelo seu elemento: não
    /// registre `on_click` pra abrir, ou o popover abre e fecha no mesmo clique.
    pub fn trigger(
        mut self,
        render: impl Fn(bool, &mut Window, &mut Context<Self>) -> AnyElement + 'static,
    ) -> Self {
        self.trigger = PopoverTrigger::Custom(Box::new(render));
        self
    }

    /// O **conteúdo** do popup — o `children` do `PopoverPopup` da referência.
    ///
    /// É uma closure porque o conteúdo é remontado a cada frame (ver [`ContentRender`]). O `Context`
    /// dá acesso a `cx.listener(..)`, então o conteúdo pode fechar o popover:
    ///
    /// ```ignore
    /// .content(|_window, cx| {
    ///     Button::new("ok", "Fechar")
    ///         .on_click(cx.listener(|this, _e, _window, cx| this.close(cx)))
    ///         .into_any_element()
    /// })
    /// ```
    pub fn content(
        mut self,
        render: impl Fn(&mut Window, &mut Context<Self>) -> AnyElement + 'static,
    ) -> Self {
        self.content = Some(Box::new(render));
        self
    }

    /// De que lado do gatilho o popup abre (default [`MenuSide::Bottom`]).
    pub fn side(mut self, side: MenuSide) -> Self {
        self.side = side;
        self
    }

    /// Como o popup se alinha ao gatilho no eixo transversal (default [`MenuAlign::Center`]).
    pub fn align(mut self, align: MenuAlign) -> Self {
        self.align = align;
        self
    }

    /// Distância entre o gatilho e o popup, em px — o `sideOffset` (default 4).
    pub fn side_offset(mut self, offset: f32) -> Self {
        self.side_offset = offset;
        self
    }

    /// Deslocamento no eixo de **alinhamento** (default 0). Positivo empurra pro fim do eixo.
    pub fn align_offset(mut self, offset: f32) -> Self {
        self.align_offset = offset;
        self
    }

    /// Fixa a largura do popup, em px — **um escape, não o default.**
    ///
    /// Sem esta chamada o popup mede o **conteúdo**, que é o `w-(--popup-width,auto)` da fonte. O
    /// setter existe porque no CSS quem quer largura fixa passa `className="w-72"`, e aqui não há
    /// cascata de classes.
    ///
    /// A largura pedida continua limitada pela largura DISPONÍVEL da janela (o
    /// `max-w-(--available-width)`): pedir 900px numa janela de 400 não põe o popover fora da tela.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Respiro em volta do conteúdo, em px (default 16 — ver [`PAD`]).
    ///
    /// Existe pro caso que a fonte resolve com `has-data-[slot=calendar]:p-2` — um popover que hospeda
    /// um calendário respira 8px em vez de 16, porque o calendário já tem o respiro dele.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Se o popup está aberto.
    ///
    /// Aqui isto é o mesmo que "está na tela": a fonte não tem animação de saída, então fechar
    /// desmonta no mesmo frame (ver o doc do módulo).
    pub fn is_open(&self) -> bool {
        self.anim.open()
    }

    /// Abre ou fecha o popup. **Emite** [`PopoverEvent::OpenChange`] se mudou.
    pub fn set_open(&mut self, open: bool, cx: &mut Context<Self>) {
        if !self.anim.set(open) {
            return;
        }
        self.fade.set(if open {1.} else {0.},160);
        cx.emit(PopoverEvent::OpenChange(open));
        cx.notify();
    }

    /// Abre o popup.
    pub fn open(&mut self, cx: &mut Context<Self>) {
        self.set_open(true, cx);
    }

    /// Fecha o popup — o `PopoverClose` da referência, do lado de cá.
    pub fn close(&mut self, cx: &mut Context<Self>) {
        self.set_open(false, cx);
    }

    /// Alterna o popup (o clique no gatilho).
    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        self.set_open(!self.anim.open(), cx);
    }

    /// Onde o popup foi medido no último frame em que esteve na tela (BORDER box, em pixels de
    /// janela). `None` enquanto ele nunca abriu.
    ///
    /// ⚠️ É observabilidade, e **não** realimenta posição nenhuma — ver o aviso no doc do módulo.
    pub fn popup_bounds(&self) -> Option<Bounds<Pixels>> {
        self.popup_bounds
    }

    /// Os bounds do **gatilho** medidos no último frame — o "anchor rect" do `Positioner`.
    pub fn trigger_bounds(&self) -> Bounds<Pixels> {
        self.trigger_bounds
    }

    /// O teclado. Ouvido pela RAIZ (o evento sobe do elemento focado pelos ancestrais), então um
    /// listener cobre o gatilho e o popup. A decisão está em [`key_action`].
    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(novo) = key_action(self.anim.open(), event.keystroke.key.as_str()) else {
            return;
        };
        // Chegou aqui: a tecla é do popover, ou seja o usuário está operando pelo teclado. O
        // `focus_ring::init` já marcaria isso; marcar aqui faz o anel funcionar mesmo num app que
        // esqueceu de inicializar o crate.
        crate::focus_ring::keyboard_used(window);
        self.set_open(novo, cx);
    }

    /// O gatilho, dentro do wrap que mede os bounds e trata o clique.
    fn render_trigger(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content: AnyElement = match &self.trigger {
            // O `focus_handle` vai pro Button de propósito: é ELE que fica na ordem de tabulação e
            // desenha o próprio anel `focus-visible` (ver `crate::focus_ring`), então o popover não
            // precisa reinventar anel nenhum — e a referência não dá estilo nenhum ao
            // `PopoverTrigger`, o visual dele é do botão.
            PopoverTrigger::Button(label) => Button::new(("popover-trigger", self.id), label.clone())
                .variant(ButtonVariant::Outline)
                .focus(&self.focus_handle)
                .into_any_element(),
            PopoverTrigger::IconButton(path) => {
                Button::icon(("popover-trigger", self.id), path.clone())
                    .variant(ButtonVariant::Outline)
                    .focus(&self.focus_handle)
                    .into_any_element()
            }
            PopoverTrigger::Custom(render) => render(self.anim.open(), window, cx),
        };

        div()
            .id(("popover-trigger-wrap", self.id))
            // `relative` porque o canvas de medida é filho ABSOLUTO; `flex_none` pra o wrap ter a
            // largura do gatilho e não a da linha inteira — a largura errada aqui desalinharia
            // `align: center`, que mede o gatilho.
            .relative()
            .flex()
            .flex_none()
            .child(content)
            .child(
                canvas(
                    {
                        let view = cx.entity();
                        move |bounds, _window, cx| {
                            view.update(cx, |this, _| this.trigger_bounds = bounds);
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .on_click(cx.listener(|this, _e, window, cx| {
                // Abrir com o mouse não deve acender anel (ver `crate::focus_ring`).
                crate::focus_ring::pointer_used(window);
                this.toggle(cx);
            }))
    }

    /// O popup — em `deferred(anchored(..))` pra ficar por cima de tudo, escapar do recorte de quem
    /// estiver por fora e não estourar a janela.
    fn render_popup(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // O "anchor rect" do `Positioner`: os bounds medidos do gatilho.
        let anchor = rect_of(self.trigger_bounds);
        let viewport = window.viewport_size();
        let max_h = popup_max_height(
            self.side,
            anchor,
            f32::from(viewport.height),
            self.side_offset,
        );
        // O `max-w-(--available-width)`, do [`crate::tooltip`] — o simétrico exato do `max_h` acima.
        let max_w = popup_max_width(self.side, anchor, f32::from(viewport.width), self.side_offset);
        let (x, y, corner) = anchor_point(
            anchor,
            self.side,
            self.align,
            self.side_offset,
            self.align_offset,
        );

        let content = self.content.as_ref().map(|render| render(window, cx));

        let surface = popup_surface(self.width, max_w, if self.smooth {self.fade.value()} else {self.anim.opacity()})
            .when(!self.anim.open(), |d| d.capture_any_mouse_down(cx.listener(|this,event:&MouseDownEvent,_,cx| {if this.popup_bounds.is_some_and(|b|b.contains(&event.position)) {cx.stop_propagation();}})))
            .child(
                // O `Viewport` da fonte: `px-(--viewport-inline-padding) py-4`,
                // `max-h-(--available-height)` e `overflow-y-auto`. Quem rola é ESTE div, e não a
                // superfície: a superfície não pode ter `overflow_hidden` (recortaria o bisel, que é
                // irmão deste).
                div()
                    .id(("popover-viewport", self.id))
                    .flex()
                    // O empilhamento vertical do conteúdo vive AQUI — e é por isso que a superfície pode
                    // ser uma linha (ver o comentário no [`popup_surface`]).
                    .flex_col()
                    // A metade horizontal do `size-full` da fonte. A metade vertical não é declarada —
                    // vem do `stretch` da linha (ver o doc do módulo).
                    //
                    // ⚠️ Honestidade, medida por mutação: **esta linha é INERTE hoje**, igual ao par de
                    // `flex_none`. Tentei dar dentes a ela com um teste (conteúdo `w_full` dentro de uma
                    // largura pedida, contando as linhas em que o texto quebra) e o teste passava COM e
                    // SEM o `w_full` daqui — ou seja era um teste vazio, e foi removido em vez de
                    // mantido. Ela fica por fidelidade ao `size-full` do `Viewport` da fonte, e porque o
                    // caso em que ela importaria (um filho de porcentagem sob uma largura PEDIDA) é
                    // pequeno demais pra apostar contra. Ver "Sem cobertura de teste — declarado".
                    .w_full()
                    // Ver o doc do módulo: sem `min_w(0)` o min-content de um texto no GPUI é a frase
                    // inteira numa linha, e o conteúdo empurraria a caixa em vez de quebrar.
                    .min_w(px(0.0))
                    .p(px(self.padding))
                    .max_h(px(max_h))
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .children(content),
            )
            .child(bevel_overlay())
            .on_mouse_down_out(cx.listener(|this, e: &MouseDownEvent, _window, cx| {
                let pos = e.position;
                // A guarda é a do menu: o clique no GATILHO é, do ponto de vista do popup, um clique
                // fora — sem ela o popup fecharia aqui e o `on_click` do gatilho o reabriria no mesmo
                // clique, e o botão nunca fecharia o próprio popover.
                if outside_click_closes(
                    rect_of(this.trigger_bounds),
                    f32::from(pos.x),
                    f32::from(pos.y),
                ) {
                    this.set_open(false, cx);
                }
            }));

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
                        let view = cx.entity();
                        move |bounds, _window, cx| {
                            view.update(cx, |this, _| this.popup_bounds = Some(bounds));
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
        // `crate::menu` e o `crate::tooltip` — ver `crate::menu::align_center_wrap`.
        let child: AnyElement = if self.align == MenuAlign::Center {
            align_center_wrap(self.side, anchor, wrap.into_any_element()).into_any_element()
        } else {
            wrap.into_any_element()
        };

        deferred(
            anchored()
                .position(point(px(x), px(y)))
                // O canto do POPUP que encosta no ponto — é ele que expressa `align: start`/`end` e
                // `side: top`/`left` sem precisar do tamanho do popup.
                .anchor(corner)
                // O `collisionPadding` do `Positioner`: gruda na janela em vez de trocar de lado.
                // `side` aqui é uma decisão do call site, não uma preferência. Mesma escolha (e mesmo
                // motivo) do `crate::menu`.
                .snap_to_window_with_margin(px(WINDOW_MARGIN))
                .child(child),
        )
        .with_priority(POPUP_PRIORITY)
    }

    /// Põe a transição de entrada no fim, sem esperar. Só em teste — ver [`Anim::settle`].
    #[cfg(test)]
    fn settle(&mut self, cx: &mut Context<Self>) {
        self.anim.settle();
        cx.notify();
    }
}

impl EventEmitter<PopoverEvent> for Popover {}

impl Focusable for Popover {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Popover {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Enquanto a transição de entrada andar, pede o próximo frame — é o motor do fade, já que não
        // há elemento de animação e o progresso vem do tempo decorrido.
        //
        // `request_animation_frame` (e não o `on_next_frame` do `crate::dialog`) porque ele notifica a
        // view CORRENTE, que aqui é este próprio `Popover`: é o popover que monta o popup, não uma
        // view hospedeira. Ele é um no-op na plataforma de TESTE — ver a nota dos testes de janela.
        if self.anim.animating() || (self.smooth && self.fade.moving()) {
            window.request_animation_frame();
        }

        // Com o gatilho embutido, o `focus_handle` está no `Button` (que o rastreia e desenha o anel).
        // Com gatilho customizado ninguém o rastreia, então a RAIZ rastreia: senão o popover sairia da
        // ordem de tabulação e o teclado não chegaria nele.
        let custom_trigger = matches!(self.trigger, PopoverTrigger::Custom(_));
        let trigger = self.render_trigger(window, cx);

        let mut root = div()
            .id(("popover", self.id))
            .relative()
            .flex()
            // `items_start` pra o wrap do gatilho ter a ALTURA dele (o `stretch` do flex esticaria o
            // wrap, e a medida do anchor rect sairia alta).
            .items_start()
            .when(custom_trigger, |d| d.track_focus(&self.focus_handle))
            // O teclado é ouvido pela RAIZ: o evento sobe do elemento focado pelos ancestrais, então
            // um listener cobre o gatilho (fechado) e o conteúdo (aberto).
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                this.on_key(e, window, cx);
            }))
            .child(trigger);

        // Fechado não monta NADA: nem pintura, nem hitbox (o popup é `occlude()`). A fonte não tem
        // animação de saída, então isto acontece no MESMO frame do fechamento — ver [`Anim::mounted`].
        if self.anim.mounted() || (self.smooth && self.fade.moving()) {
            root = root.child(self.render_popup(window, cx));
        }

        root
    }
}

// =================================================================================================
// Título e descrição
// =================================================================================================

/// O **título** de um popover — `font-semibold text-lg leading-none` na referência.
///
/// É um elemento de render, não um `Entity`: construa a cada frame, dentro do
/// [`Popover::content`].
#[derive(IntoElement)]
pub struct PopoverTitle {
    text: SharedString,
}

impl PopoverTitle {
    /// Um título com este texto.
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self { text: text.into() }
    }
}

impl RenderOnce for PopoverTitle {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        title_element(self.text)
    }
}

/// A **descrição** de um popover — `text-muted-foreground text-sm` na referência.
#[derive(IntoElement)]
pub struct PopoverDescription {
    text: SharedString,
}

impl PopoverDescription {
    /// Uma descrição com este texto.
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self { text: text.into() }
    }
}

impl RenderOnce for PopoverDescription {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        description_element(self.text)
    }
}

#[cfg(test)]
// O clippy trata `assert!` sobre constante como erro provável, presumindo que quem escreveu quis
// testar algo variável. Aqui é o contrário: travar o valor que veio da referência É o propósito
// destes testes. Mesma decisão em todos os módulos portados do coss (ver `menu.rs`).
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use crate::theme;

    /// A unidade de espaçamento do Tailwind — `var(--spacing)` = `0.25rem` = 4px, medido em
    /// `tailwindcss@4.3.3/theme.css:325`.
    const SPACING: f32 = 4.0;

    /// Um teto de largura disponível qualquer, pros testes de superfície. Não é um número da
    /// referência — ele vem do [`crate::tooltip::popup_max_width`] em runtime.
    const TETO: f32 = 784.0;

    /// A entrelinha que o GPUI usaria sem declarar: `phi()` = `relative(1.618_034)`, em
    /// `gpui/src/geometry.rs`.
    const PHI: f32 = 1.618_034;

    /// Um retângulo de gatilho qualquer, pros testes de posição: 100×32 em (200, 300). Longe da
    /// origem de propósito — uma conta que ignorasse a posição do gatilho passaria em (0,0).
    const GATILHO: crate::menu::Rect = crate::menu::Rect {
        x: 200.0,
        y: 300.0,
        w: 100.0,
        h: 32.0,
    };

    /// Um popup que montou há `ms` milissegundos — é como os testes atravessam a entrada sem dormir.
    fn abrindo(ms: u64) -> Anim {
        Anim {
            since: Instant::now().checked_sub(Duration::from_millis(ms)),
        }
    }

    // ---------------------------------------------------------------------------------------------
    // Geometria
    // ---------------------------------------------------------------------------------------------

    /// **A geometria é a da fonte, com a conta do lado.**
    ///
    /// Cada número aqui é a tradução de uma classe do `popover.tsx`, e a conta está escrita em vez de o
    /// literal aparecer duas vezes: `assert_eq!(PAD, 16.0)` sozinho não protege nada, mas
    /// `assert_eq!(PAD, 4.0 * SPACING)` amarra o número à escala de onde ele veio.
    #[test]
    fn a_geometria_e_a_da_fonte() {
        // O respiro do `Viewport`, POR EIXO — a fonte não escreve `p-4` uniforme:
        //   horizontal: `px-(--viewport-inline-padding)` com `--viewport-inline-padding: --spacing(4)`
        //   vertical:   `py-4`
        // Os dois caem em `4 × var(--spacing)`, e é por isso que aqui cabe uma constante só.
        assert_eq!(PAD, 4.0 * SPACING, "--spacing(4) na horizontal E py-4 na vertical");
        assert_eq!(PAD, 16.0);
        // `text-lg` = 1.125rem, e `leading-none` = 1 × o corpo.
        assert_eq!(TITLE_SIZE, 1.125 * 16.0, "text-lg");
        assert_eq!(TITLE_SIZE, 18.0);
        assert_eq!(TITLE_LINE_HEIGHT, TITLE_SIZE, "leading-none = line-height 1");
        // Os offsets: o `sideOffset={4}` compartilhado, e nenhum alignOffset.
        assert_eq!(SIDE_OFFSET, 4.0, "sideOffset={{4}}");
        assert_eq!(ALIGN_OFFSET, 0.0, "a fonte passa alignOffset = 0");
    }

    /// **A entrelinha do título é 18, e não a razão de ouro** — a armadilha mais silenciosa deste
    /// porte, e a que nenhuma leitura do `.tsx` pega.
    ///
    /// O `leading-none` do coss é `line-height: 1`; o default do GPUI é `phi()` (1,618034 × o corpo).
    /// O teste mostra os DOIS números, porque o valor errado não parece errado em lugar nenhum do
    /// código: ele só aparece como um título 62% mais alto que o da referência.
    #[test]
    fn a_entrelinha_do_titulo_e_um_e_nao_a_razao_de_ouro() {
        let sem_declarar = TITLE_SIZE * PHI;
        assert!(
            (sem_declarar - 29.124_612).abs() < 1e-3,
            "a linha default do GPUI a 18px é 29,12; veio {sem_declarar}"
        );
        assert!(
            sem_declarar > TITLE_LINE_HEIGHT,
            "o default do GPUI é MAIOR — o erro infla, nunca aperta"
        );
        assert!(
            (sem_declarar / TITLE_LINE_HEIGHT - 1.618_034).abs() < 1e-3,
            "62% mais alta, que é exatamente a razão de ouro sobre um `leading-none`"
        );

        // E o par do corpo do texto (`text-sm`) é o do menu, importado: 14/20, e NÃO 14 × phi.
        assert_eq!(TEXT_SIZE, 14.0, "text-sm");
        assert_eq!(TEXT_LINE_HEIGHT, 20.0, "o par do text-sm no Tailwind é 14/20");
        assert!(TEXT_SIZE * PHI > TEXT_LINE_HEIGHT);
    }

    // ---------------------------------------------------------------------------------------------
    // Reuso: o popover NÃO tem superfície própria
    // ---------------------------------------------------------------------------------------------

    /// **A superfície do popover é a do [`crate::menu`], token por token.**
    ///
    /// É o teste que impede a regressão mais provável deste módulo: alguém declara um
    /// `POPOVER_LIGHT`/`POPOVER_DARK` com os mesmos nove campos do menu, e a partir daí os dois
    /// popups do design system divergem no primeiro ajuste que alguém fizer num só. Aqui a afirmação
    /// é de IDENTIDADE contra a paleta do menu — se o popover ganhar cores próprias, isto falha.
    #[test]
    fn o_popover_nao_tem_paleta_nem_superficie_propria() {
        for modo in [theme::ThemeMode::Light, theme::ThemeMode::Dark] {
            theme::set_theme(modo);
            let p = palette();
            let mut surface = popup_surface(None, TETO, 1.0);
            let s = surface.style();

            assert_eq!(s.background, Some(p.popover_bg.hsla().into()), "{modo:?}: bg-popover");
            assert_eq!(s.border_color, Some(p.border.hsla()), "{modo:?}: --border");
            let texto = s.text.clone().expect("a superfície declara estilo de texto");
            assert_eq!(texto.color, Some(p.text.hsla()), "{modo:?}: text-popover-foreground");

            // A sombra é a do menu — as duas camadas do `shadow-lg/5`, e não um par próprio.
            let sombras = s.box_shadow.clone().expect("shadow-lg/5");
            assert_eq!(sombras, popup_shadow(), "{modo:?}: a sombra É a do menu");
            assert_eq!(sombras.len(), 2, "{modo:?}: shadow-lg tem duas camadas");
        }
    }

    /// **A superfície que vai pra tela carrega o contrato, e não só as constantes.**
    ///
    /// O teste acima tranca as CORES; este tranca a forma, a largura e o fade. É a diferença entre "o
    /// número certo está declarado" e "o número certo chega no `Div`" — um `.rounded(px(8.0))`
    /// trocado não mexe em constante nenhuma e passaria em todos os outros.
    #[test]
    fn a_superficie_carrega_o_contrato() {
        use gpui::{AbsoluteLength, DefiniteLength, Length};

        theme::set_theme(theme::ThemeMode::Dark);
        let px_len = |v: f32| Some(AbsoluteLength::Pixels(px(v)));
        let comprimento = |v: f32| {
            Some(Length::Definite(DefiniteLength::Absolute(
                AbsoluteLength::Pixels(px(v)),
            )))
        };

        let mut surface = popup_surface(None, TETO, 0.5);
        let s = surface.style();

        // --- Forma: o raio e a borda do MENU (10 e 1), e não o `rounded-md` do shadcn ------------
        let raios = s.corner_radii.clone();
        assert_eq!(raios.top_left, px_len(RADIUS), "rounded-lg = --radius-lg");
        assert_eq!(raios.bottom_right, px_len(RADIUS), "nos quatro cantos");
        assert_eq!(raios.top_left, px_len(10.0), "e o número é 10, não os 8 do shadcn");
        let bordas = s.border_widths.clone();
        assert_eq!(bordas.top, px_len(BORDER), "border de 1px");
        assert_eq!(bordas.left, px_len(BORDER), "nos quatro lados");

        // --- Largura: NENHUMA declarada por default ----------------------------------------------
        //
        // É o `w-(--popup-width,auto)` da fonte. Uma versão anterior deste módulo declarava 288px aqui
        // (o `w-72` do shadcn, que não existe no `popover.tsx`); esta asserção é o que impede a volta.
        assert_eq!(
            s.size.width, None,
            "sem `.width(..)` a caixa mede o CONTEÚDO — não há largura declarada"
        );
        // O teto da largura disponível, esse sim, está sempre lá.
        assert_eq!(
            s.max_size.width,
            comprimento(TETO),
            "max-w-(--available-width)"
        );

        // --- Texto: 14/20, DECLARADO (o original herda) -------------------------------------------
        let texto = s.text.clone().expect("estilo de texto");
        assert_eq!(texto.font_size, px_len(TEXT_SIZE), "text-sm");
        assert_eq!(
            texto.line_height,
            Some(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(
                TEXT_LINE_HEIGHT
            )))),
            "a entrelinha é DECLARADA em 20 — sem isso o GPUI usa a razão de ouro"
        );

        // --- O fade é PINTADO --------------------------------------------------------------------
        assert_eq!(s.opacity, Some(0.5), "a opacidade da entrada chega ao Div");

        // O respiro NÃO está aqui: ele é da área de conteúdo, e é medido no teste de janela.
        assert_eq!(s.padding.left, None, "o respiro é do viewport, não da superfície");

        // Opacidade cheia não gasta passe de pintura.
        let mut cheia = popup_surface(None, TETO, 1.0);
        assert_eq!(
            cheia.style().opacity,
            None,
            "com a entrada concluída não há por que pedir um passe de opacidade"
        );

        // `.width(..)` é o escape: aí a largura é declarada, e o teto CONTINUA lá.
        const PEDIDA: f32 = 200.0;
        let mut fixa = popup_surface(Some(PEDIDA), TETO, 1.0);
        let s = fixa.style();
        assert_eq!(s.size.width, comprimento(PEDIDA), "a largura pedida chega ao Div");
        assert_eq!(
            s.max_size.width,
            comprimento(TETO),
            "e o teto da janela não é abandonado por causa dela"
        );
    }

    /// **O título e a descrição são o `PopoverTitle`/`PopoverDescription` da referência.**
    ///
    /// Inclui o que os separa: o título é 18/18 semibold na cor do texto; a descrição é 14/20 no peso
    /// normal e na cor `--muted-foreground`. Trocar as duas cores de lugar não mexe em constante
    /// nenhuma.
    #[test]
    fn o_titulo_e_a_descricao_seguem_a_referencia() {
        use gpui::{AbsoluteLength, DefiniteLength};

        for modo in [theme::ThemeMode::Light, theme::ThemeMode::Dark] {
            theme::set_theme(modo);
            let p = palette();

            let mut titulo = title_element("Dimensões".into());
            let s = titulo.style();
            let t = s.text.clone().expect("o título declara estilo de texto");
            assert_eq!(t.font_size, Some(AbsoluteLength::Pixels(px(TITLE_SIZE))), "{modo:?}: text-lg");
            assert_eq!(
                t.line_height,
                Some(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(
                    TITLE_LINE_HEIGHT
                )))),
                "{modo:?}: leading-none"
            );
            assert_eq!(t.font_weight, Some(FontWeight::SEMIBOLD), "{modo:?}: font-semibold");
            assert_eq!(t.color, Some(p.text.hsla()), "{modo:?}: o título é --foreground");

            let mut desc = description_element("Largura e altura.".into());
            let s = desc.style();
            let d = s.text.clone().expect("a descrição declara estilo de texto");
            assert_eq!(d.font_size, Some(AbsoluteLength::Pixels(px(TEXT_SIZE))), "{modo:?}: text-sm");
            assert_eq!(
                d.line_height,
                Some(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(
                    TEXT_LINE_HEIGHT
                )))),
                "{modo:?}: o par 14/20"
            );
            assert_eq!(d.font_weight, None, "{modo:?}: a descrição não muda o peso");
            assert_eq!(d.color, Some(p.muted.hsla()), "{modo:?}: --muted-foreground");

            // E os dois são cores DIFERENTES — se a descrição usasse a do título, a hierarquia
            // visual do popover desapareceria.
            assert_ne!(p.text, p.muted, "{modo:?}: o muted é mais apagado que o texto");
        }
    }

    // ---------------------------------------------------------------------------------------------
    // A animação
    // ---------------------------------------------------------------------------------------------

    /// **A duração e a curva são os DEFAULTS do `transition` do Tailwind, e não uma das outras duas
    /// curvas de nome parecido.**
    ///
    /// Medidos, não deduzidos: a fonte escreve `transition-[width,height,scale,opacity]` sem classe de
    /// `duration-*` nem de `ease-*`, então valem `--default-transition-duration: 150ms` e
    /// `--default-transition-timing-function: cubic-bezier(0.4, 0, 0.2, 1)`
    /// (`tailwindcss@4.3.3/theme.css:492-493`).
    ///
    /// A parte que importa é a última: **três curvas com nomes colididos circulam nesta base**, e a
    /// diferença entre elas não aparece em nenhum teste de constante — só no ritmo da abertura. O teste
    /// afirma que esta é a do Tailwind comparando com as outras duas pelo VALOR em `t = 0,25`.
    #[test]
    fn a_duracao_e_a_curva_sao_os_defaults_do_tailwind() {
        assert_eq!(
            DURATION,
            Duration::from_millis(150),
            "--default-transition-duration"
        );
        assert_eq!(
            EASE,
            (0.4, 0.0, 0.2, 1.0),
            "--default-transition-timing-function: cubic-bezier(0.4, 0, 0.2, 1)"
        );

        // As pontas são exatas, e o meio é monotônico.
        assert!(ease(0.0).abs() < 1e-4, "em 0 a curva vale 0");
        assert!((ease(1.0) - 1.0).abs() < 1e-4, "em 1 vale 1");
        let mut anterior = -1.0;
        for i in 0..=20 {
            let v = ease(i as f32 / 20.0);
            assert!(v >= anterior - 1e-6, "a curva não pode voltar atrás em t={i}/20");
            anterior = v;
        }

        // --- E ela NÃO é nenhuma das outras duas -------------------------------------------------
        //
        // As três avaliadas no primeiro quarto. Se alguém trocar os pontos de controle por qualquer um
        // dos outros dois pares, uma destas asserções cai.
        let nossa = ease(0.25);
        let ease_in_out_do_css = crate::dialog::cubic_bezier(0.42, 0.0, 0.58, 1.0, 0.25);
        let ease_do_css = crate::dialog::cubic_bezier(0.25, 0.1, 0.25, 1.0, 0.25);
        assert!(
            (nossa - ease_in_out_do_css).abs() > 0.02,
            "a nossa ({nossa}) NÃO é a `ease-in-out` do CSS ({ease_in_out_do_css}) que o dialog usa"
        );
        assert!(
            (nossa - ease_do_css).abs() > 0.02,
            "nem a `ease` do CSS ({ease_do_css}) que o tw-animate-css do shadcn usaria"
        );
        // E o FORMATO: `cubic-bezier(.4,0,.2,1)` é um ease-in-out — arranca devagar (o `y1 = 0` puxa a
        // curva pra baixo no início) e freia no fim. Uma reta, ou uma curva de `ease-out`, quebraria
        // uma destas duas.
        assert!(nossa < 0.25, "arranque LENTO: atrás da reta no primeiro quarto; veio {nossa}");
        let tres_quartos = ease(0.75);
        assert!(
            tres_quartos > 0.75,
            "e adiante da reta no último quarto; veio {tres_quartos}"
        );

        // Aparada nas duas pontas: o `progress` vem de tempo decorrido e pode passar de 1.
        assert_eq!(ease(2.0), ease(1.0), "aparada em 1");
        assert_eq!(ease(-1.0), ease(0.0), "aparada em 0");
    }

    /// **Fechado não monta nada.** Um popup invisível que ainda engolisse cliques (ele é `occlude()`)
    /// seria pior que não ter popover.
    #[test]
    fn fechado_nao_monta_nada() {
        let a = Anim::default();
        assert!(!a.open(), "nasce fechado");
        assert_eq!(a.opacity(), 0.0);
        assert!(!a.mounted(), "nada na tela");
        assert!(!a.animating(), "e nenhum frame a pedir");
    }

    /// **A entrada percorre 0 → 1 nos 150ms, e para lá.**
    ///
    /// O `== 1.0` no fim não é folga: sem a aparadura o popup ficaria pra sempre subindo, e o
    /// [`Anim::animating`] pediria frames pra sempre.
    #[test]
    fn a_entrada_percorre_a_opacidade_e_para() {
        let inicio = abrindo(0);
        assert!(
            inicio.opacity() < 0.05,
            "nasce transparente — é o `data-starting-style:opacity-0`; veio {}",
            inicio.opacity()
        );
        assert!(inicio.mounted(), "mas já está na árvore, no frame da montagem");
        assert!(inicio.animating());

        let meio = abrindo(75);
        let o = meio.opacity();
        assert!(o > 0.3 && o < 1.0, "a meio caminho está entre as pontas; veio {o}");
        assert!(o > inicio.opacity(), "e subiu");

        let fim = abrindo(150);
        assert_eq!(fim.opacity(), 1.0, "aos 150ms está cheio");
        assert!(!fim.animating(), "e não pede mais frame — um popover parado não queima frame");
        assert_eq!(abrindo(10_000).opacity(), 1.0, "parado, fica cheio");
        assert!(!abrindo(10_000).animating());
    }

    /// **Não há animação de SAÍDA: fechar desmonta no mesmo instante.**
    ///
    /// ⚠️ Isto é a fonte falando, e é a diferença mais fácil de "consertar" por engano. O `Popup` do
    /// `popover.tsx` declara só `data-starting-style` (`scale-98`/`opacity-0`); o `data-ending-style`
    /// aparece no arquivo apenas no `Viewport`, e ali é pro cross-fade dos FILHOS. Sem estilo de saída
    /// o Base UI não tem o que transitar, e o popup sai da árvore de uma vez.
    ///
    /// O shadcn, esse sim, tem `data-[state=closed]:animate-out fade-out-0` — e é dele que viria a
    /// tentação. Se alguém adicionar o fade de saída, este teste falha e aponta pra cá.
    #[test]
    fn nao_ha_animacao_de_saida() {
        let mut a = abrindo(150);
        assert!(a.mounted() && a.opacity() == 1.0, "aberta e assentada");

        assert!(a.set(false), "fechar muda o estado");
        assert!(!a.open());
        assert!(
            !a.mounted(),
            "e DESMONTA no mesmo instante — a fonte não tem animação de saída"
        );
        assert!(!a.animating(), "logo não há frame nenhum a pedir depois de fechar");
        assert_eq!(a.opacity(), 0.0);
    }

    /// **Reabrir recomeça a entrada do ZERO, e reabrir o que já está aberto não pisca.**
    ///
    /// O primeiro não é escolha nossa: sem animação de saída, fechar REMOVE o popup da árvore, e montar
    /// de novo aplica o `data-starting-style` de novo — do começo, como no navegador. O segundo é o que
    /// impede um `open()` redundante de reiniciar o fade de um popover que já está na tela.
    #[test]
    fn reabrir_recomeca_a_entrada_e_o_estado_repetido_e_inerte() {
        let mut a = abrindo(150);
        assert_eq!(a.opacity(), 1.0);

        // Fecha e reabre: a entrada parte do zero.
        assert!(a.set(false));
        assert!(a.set(true), "reabrir muda o estado");
        assert!(
            a.opacity() < 0.05,
            "a entrada recomeça transparente; veio {}",
            a.opacity()
        );
        assert!(a.animating(), "e o relógio volta a correr");

        // Repetir o MESMO estado é inerte — nem evento, nem reinício de transição.
        let mut assentada = abrindo(150);
        let marca = assentada.since;
        assert!(!assentada.set(true), "o estado já era esse");
        assert_eq!(assentada.since, marca, "o relógio não pode voltar pro zero");
        assert_eq!(assentada.opacity(), 1.0, "e a opacidade não pisca");

        // Fechar o que já está fechado, idem.
        let mut fechada = Anim::default();
        assert!(!fechada.set(false));
        assert_eq!(fechada.since, None);
    }

    // ---------------------------------------------------------------------------------------------
    // Teclado e defaults
    // ---------------------------------------------------------------------------------------------

    /// **`Enter`/`Space` abrem, `Escape` fecha, e nada mais é do popover.**
    ///
    /// O conteúdo é arbitrário: setas, `Home`/`End` e o próprio `Enter` com o popup ABERTO pertencem a
    /// quem estiver dentro dele. Um popover que engolisse o `Enter` de um campo de texto seria
    /// inutilizável.
    #[test]
    fn o_teclado_do_popover_e_so_abrir_e_fechar() {
        assert_eq!(key_action(false, "enter"), Some(true), "enter abre");
        assert_eq!(key_action(false, "space"), Some(true), "space abre");
        assert_eq!(key_action(true, "escape"), Some(false), "escape fecha");

        // Aberto, o `Enter` NÃO é do popover.
        assert_eq!(key_action(true, "enter"), None, "aberto, o enter é do conteúdo");
        assert_eq!(key_action(true, "space"), None);
        // Fechado, o `Escape` não tem o que fechar.
        assert_eq!(key_action(false, "escape"), None);
        // E as teclas do MENU não são do popover: aqui não há item a destacar.
        for tecla in ["down", "up", "home", "end", "tab", "a"] {
            assert_eq!(key_action(true, tecla), None, "`{tecla}` aberto não é do popover");
            assert_eq!(key_action(false, tecla), None, "`{tecla}` fechado não é do popover");
        }
    }

    /// **Os defaults de posição são os das referências**: `side="bottom"`, `align="center"`,
    /// `sideOffset={4}`. Um default diferente muda o lugar de TODO popover da app.
    #[test]
    fn os_defaults_de_posicao_sao_os_das_referencias() {
        assert_eq!(MenuSide::default(), MenuSide::Bottom, "side=\"bottom\"");
        assert_eq!(MenuAlign::default(), MenuAlign::Center, "align=\"center\"");

        // E a conta: `side: bottom` põe o TOPO do popup 4px abaixo da base do gatilho.
        let (x, y, corner) =
            anchor_point(GATILHO, MenuSide::default(), MenuAlign::default(), SIDE_OFFSET, ALIGN_OFFSET);
        assert_eq!(x, GATILHO.x, "align: center devolve o mesmo ponto do start");
        assert_eq!(y, GATILHO.y + GATILHO.h + 4.0, "4px ABAIXO da base do gatilho");
        assert_eq!(corner, gpui::Corner::TopLeft, "é o TOPO do popup que encosta");
    }

    /// **A prioridade do `deferred` é a dos irmãos**, porque na referência os três são `z-50`.
    #[test]
    fn a_prioridade_do_deferred_e_a_dos_irmaos() {
        assert_eq!(POPUP_PRIORITY, 1, "o mesmo `with_priority(1)` do menu, do select e do tooltip");
    }

    /// A altura máxima do popup é a do menu, importada — o `max-h-(--available-height)`.
    ///
    /// Não reteste do menu: o que se afirma é que ESTE módulo chama aquela função e que ela responde
    /// ao espaço livre (um `max_h` fixo, ou um que ignorasse a janela, passaria em tudo o mais).
    #[test]
    fn a_altura_maxima_vem_do_menu_e_segue_o_espaco_livre() {
        let alto = popup_max_height(MenuSide::Bottom, GATILHO, 1200.0, SIDE_OFFSET);
        let baixo = popup_max_height(MenuSide::Bottom, GATILHO, 600.0, SIDE_OFFSET);
        assert!(alto > baixo, "janela maior, mais altura disponível ({alto} vs {baixo})");
        assert_eq!(
            alto,
            1200.0 - (GATILHO.y + GATILHO.h) - SIDE_OFFSET - WINDOW_MARGIN,
            "é o espaço abaixo do gatilho menos o offset e a margem de colisão"
        );
    }

    /// Os defaults do construtor — o que um call site herda sem escrever nada.
    ///
    /// Roda sem janela porque o `Popover` inteiro precisaria de um `Context`; os setters são conferidos
    /// contra a caixa MEDIDA em `tests_de_janela::os_setters_chegam_ao_popup`.
    #[test]
    fn os_defaults_do_construtor_sao_os_da_fonte() {
        assert_eq!(PAD, 16.0, "o respiro default");
        assert_eq!(ALIGN_OFFSET, 0.0);
        // O `Anim` default é o estado do construtor: fechado, nada montado.
        let a = Anim::default();
        assert!(!a.open() && !a.mounted());
    }
}

#[cfg(test)]
// Mesmo motivo do módulo de testes acima: aqui também se afirma constante.
#[allow(clippy::assertions_on_constants)]
mod tests_de_janela {
    //! Os testes que precisam de uma JANELA de verdade: é onde a geometria declarada nas constantes é
    //! conferida contra a **medida**, e onde o clique é conferido contra o que ele realmente acerta.
    //! Nenhum teste puro pega um popup ancorado 100px pra esquerda.
    //!
    //! Uma nota sobre o tempo: o `Window::request_animation_frame` é um **no-op** na plataforma de
    //! teste (`TestWindow::on_request_frame` ignora o callback, em
    //! `gpui/src/platform/test/window.rs`), então nenhum frame chega sozinho aqui. Os testes assentam
    //! a animação à mão ([`Popover::settle`]) e pedem o redraw — o que é mais rápido e mais
    //! determinístico que dormir 150ms, e é o mesmo caminho do [`crate::tooltip`] e do
    //! [`crate::dialog`].

    use super::*;
    use crate::theme;
    use gpui::{AppContext as _, Modifiers, TestAppContext, VisualTestContext};
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Onde o gatilho é plantado dentro da janela. Longe da origem de propósito: um `anchor_point` que
    /// ignorasse a posição do gatilho passaria num harness em (0,0).
    const OFFSET_X: f32 = 200.0;
    /// Idem, no eixo vertical.
    const OFFSET_Y: f32 = 100.0;

    /// Altura do conteúdo do harness — um `div` de tamanho FIXO, e não texto: assim a caixa do popup
    /// é uma soma exata das constantes, sem depender da métrica de fonte da plataforma.
    const CONTEUDO_H: f32 = 60.0;

    /// Largura do conteúdo do harness, quando o teste quer uma. Escolhida **longe de 288** de propósito:
    /// é contra ela que se afirma que o popup mede o conteúdo e não o `w-72` do shadcn.
    const CONTEUDO_W: f32 = 120.0;

    /// Um container que planta o popover num ponto conhecido da janela.
    struct Harness {
        pop: gpui::Entity<Popover>,
    }

    impl Render for Harness {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .pt(px(OFFSET_Y))
                .pl(px(OFFSET_X))
                .child(self.pop.clone())
        }
    }

    /// O conteúdo do harness: um `div` de altura fixa.
    ///
    /// `largura` `Some(w)` = largura própria, que é o caso NORMAL agora (a caixa do popup mede o
    /// conteúdo); `None` = `w_full`, pros testes que fixam a largura do popup com
    /// [`Popover::width`] e querem o conteúdo preenchendo o que sobrar.
    fn conteudo(largura: Option<f32>) -> AnyElement {
        let d = div().h(px(CONTEUDO_H)).flex_none();
        match largura {
            Some(w) => d.w(px(w)).into_any_element(),
            None => d.w_full().into_any_element(),
        }
    }

    /// Abre a janela e devolve o popover, os eventos capturados e o contexto visual.
    ///
    /// `preparar` recebe o `Popover` recém-construído, pra cada teste escolher lado, alinhamento e
    /// largura sem uma função de seis parâmetros.
    #[allow(clippy::type_complexity)]
    fn montar(
        cx: &mut TestAppContext,
        preparar: impl FnOnce(Popover) -> Popover + 'static,
    ) -> (
        gpui::Entity<Popover>,
        Rc<RefCell<Vec<PopoverEvent>>>,
        VisualTestContext,
    ) {
        // Conteúdo de largura própria: é o caso normal agora que a caixa do popup MEDE o conteúdo.
        montar_com(cx, Some(CONTEUDO_W), preparar)
    }

    /// Idem, escolhendo a largura do conteúdo.
    #[allow(clippy::type_complexity)]
    fn montar_com(
        cx: &mut TestAppContext,
        conteudo_w: Option<f32>,
        preparar: impl FnOnce(Popover) -> Popover + 'static,
    ) -> (
        gpui::Entity<Popover>,
        Rc<RefCell<Vec<PopoverEvent>>>,
        VisualTestContext,
    ) {
        theme::set_theme(theme::ThemeMode::Dark);
        let eventos: Rc<RefCell<Vec<PopoverEvent>>> = Rc::new(RefCell::new(Vec::new()));
        let capturados = eventos.clone();

        let window = cx.add_window(move |_window, cx| {
            let pop = cx.new(|cx| {
                preparar(
                    Popover::new(cx)
                        .trigger_button("Abrir")
                        .content(move |_w, _cx| conteudo(conteudo_w)),
                )
            });
            cx.subscribe(&pop, move |_this, _p, ev: &PopoverEvent, _cx| {
                capturados.borrow_mut().push(*ev);
            })
            .detach();
            Harness { pop }
        });

        let harness = window.root(cx).expect("view raiz");
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let pop = vcx.read(|cx| harness.read(cx).pop.clone());
        (pop, eventos, vcx)
    }

    /// O centro do gatilho, em coordenadas da janela.
    fn centro_do_gatilho(
        pop: &gpui::Entity<Popover>,
        vcx: &mut VisualTestContext,
    ) -> gpui::Point<Pixels> {
        vcx.read(|cx| {
            let b = pop.read(cx).trigger_bounds();
            point(
                b.origin.x + b.size.width / 2.0,
                b.origin.y + b.size.height / 2.0,
            )
        })
    }

    /// Clica no gatilho e deixa a janela parada.
    fn clicar_no_gatilho(pop: &gpui::Entity<Popover>, vcx: &mut VisualTestContext) {
        let g = centro_do_gatilho(pop, vcx);
        vcx.simulate_click(g, Modifiers::default());
        vcx.run_until_parked();
    }

    /// Assenta a animação e redesenha — o popup passa a estar no lugar final, com opacidade cheia.
    fn assentar(pop: &gpui::Entity<Popover>, vcx: &mut VisualTestContext) {
        vcx.update(|_window, cx| pop.update(cx, |p, cx| p.settle(cx)));
        vcx.run_until_parked();
    }

    /// A caixa medida do popup.
    fn caixa(pop: &gpui::Entity<Popover>, vcx: &mut VisualTestContext) -> Bounds<Pixels> {
        vcx.read(|cx| pop.read(cx).popup_bounds().expect("o popup se mede no prepaint dele"))
    }

    /// O anchor rect medido.
    fn ancora(pop: &gpui::Entity<Popover>, vcx: &mut VisualTestContext) -> crate::menu::Rect {
        vcx.read(|cx| rect_of(pop.read(cx).trigger_bounds()))
    }

    /// **Fechado, o popover não monta popup nenhum** — nem pintura, nem hitbox.
    #[gpui::test]
    fn fechado_nao_monta_popup(cx: &mut TestAppContext) {
        let (pop, eventos, mut vcx) = montar(cx, |p| p);
        assert!(!vcx.read(|cx| pop.read(cx).is_open()));
        assert_eq!(
            vcx.read(|cx| pop.read(cx).popup_bounds()),
            None,
            "sem clique, o popup nunca foi medido"
        );
        // O gatilho, sim, é medido no primeiro prepaint — é o anchor rect.
        let g = ancora(&pop, &mut vcx);
        assert_eq!((g.x, g.y), (OFFSET_X, OFFSET_Y), "o anchor rect é o do gatilho");
        assert!(g.w > 0.0 && g.h > 0.0);
        assert!(eventos.borrow().is_empty(), "e nenhum evento");
    }

    /// **A geometria declarada bate com a MEDIDA, px a px, nos DOIS eixos — e a largura é a do
    /// CONTEÚDO.**
    ///
    /// A caixa é a soma de tudo que este módulo afirma: as duas bordas e o respiro de 16px em volta do
    /// conteúdo. Se o respiro virasse 24, ou a borda desaparecesse, estes números mudam — e são os
    /// números que a captura de tela mede.
    ///
    /// ⚠️ A asserção de largura é a que trava a correção mais importante deste módulo: o popup mede o
    /// `conteúdo + 2×respiro + 2×borda`, e **não** os 288px do `w-72` do shadcn, que não existem na
    /// fonte. `CONTEUDO_W` é deliberadamente longe de 288 pra a diferença ser inequívoca.
    #[gpui::test]
    fn o_popup_nasce_com_a_geometria_declarada(cx: &mut TestAppContext) {
        let (pop, _ev, mut vcx) = montar_com(cx, Some(CONTEUDO_W), |p| p);
        clicar_no_gatilho(&pop, &mut vcx);
        assert!(vcx.read(|cx| pop.read(cx).is_open()), "o clique abre o popover");
        assentar(&pop, &mut vcx);

        let b = caixa(&pop, &mut vcx);
        assert_eq!(
            f32::from(b.size.height),
            2.0 * BORDER + 2.0 * PAD + CONTEUDO_H,
            "a altura é borda + 16 + conteúdo + 16 + borda"
        );
        assert_eq!(f32::from(b.size.height), 94.0);
        assert_eq!(
            f32::from(b.size.width),
            2.0 * BORDER + 2.0 * PAD + CONTEUDO_W,
            "e a LARGURA segue a mesma regra: é o conteúdo, não uma classe de largura"
        );
        assert_eq!(f32::from(b.size.width), 154.0);
        assert_ne!(
            f32::from(b.size.width),
            288.0,
            "e em particular NÃO é o `w-72` do shadcn, que não existe no popover.tsx"
        );
    }

    /// **Numa janela estreita a largura para no espaço DISPONÍVEL** — o `max-w-(--available-width)`.
    ///
    /// Sem o teto, um conteúdo largo empurraria o popover pra fora da tela. O teste mede numa janela de
    /// 200px, contra o mesmo [`crate::tooltip::popup_max_width`] que o render usa.
    #[gpui::test]
    fn a_largura_para_no_espaco_disponivel(cx: &mut TestAppContext) {
        /// Bem mais largo que a janela do teste.
        const CONTEUDO_LARGO: f32 = 2000.0;
        /// A janela apertada.
        const JANELA_W: f32 = 200.0;

        // O harness padrão planta o gatilho em x=200, que numa janela de 200px fica FORA dela — então o
        // popover é aberto por API. O que este teste exercita é o teto de largura, não o clique.
        let (pop, _ev, mut vcx) = montar_com(cx, Some(CONTEUDO_LARGO), |p| p);
        vcx.simulate_resize(gpui::size(px(JANELA_W), px(600.0)));
        vcx.run_until_parked();

        vcx.update(|_window, cx| pop.update(cx, |p, cx| p.open(cx)));
        assentar(&pop, &mut vcx);

        let g = ancora(&pop, &mut vcx);
        let largura = f32::from(caixa(&pop, &mut vcx).size.width);
        let teto = popup_max_width(MenuSide::Bottom, g, JANELA_W, SIDE_OFFSET);
        assert_eq!(largura, teto, "a caixa para exatamente no teto disponível");
        assert!(
            largura < CONTEUDO_LARGO,
            "e MUITO menor que o conteúdo ({CONTEUDO_LARGO}px)"
        );
        assert!(largura <= JANELA_W, "cabe na janela de {JANELA_W}px");
    }

    /// Monta o popover com conteúdo de **TEXTO** (e não um `div` de tamanho fixo), numa janela larga.
    ///
    /// ⚠️ Esta é a diferença que faltava na primeira versão do módulo: todos os outros testes de
    /// geometria usam `div` de largura declarada, e por isso **nenhum deles vê** o que o texto faz. Um
    /// `div` de 120px tem min-content = max-content = 120; um texto tem min-content (a palavra mais
    /// longa) MUITO menor que o max-content (a frase inteira), e é aí que "ajustar ao conteúdo" e
    /// "encolher ao mínimo" deixam de ser a mesma coisa.
    #[allow(clippy::type_complexity)]
    fn montar_com_texto(
        cx: &mut TestAppContext,
        conteudo: impl Fn() -> AnyElement + 'static,
    ) -> (gpui::Entity<Popover>, VisualTestContext) {
        theme::set_theme(theme::ThemeMode::Dark);
        let window = cx.add_window(move |_window, cx| {
            let pop =
                cx.new(|cx| Popover::new(cx).trigger_button("Abrir").content(move |_w, _cx| conteudo()));
            Harness { pop }
        });
        let harness = window.root(cx).expect("view raiz");
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        // Janela LARGA: o teto de `max-w-(--available-width)` não pode ser o que limita nada aqui.
        vcx.simulate_resize(gpui::size(px(1400.0), px(900.0)));
        vcx.run_until_parked();
        let pop = vcx.read(|cx| harness.read(cx).pop.clone());

        clicar_no_gatilho(&pop, &mut vcx);
        assentar(&pop, &mut vcx);
        (pop, vcx)
    }

    /// **Uma linha de texto que CABE na janela ocupa UMA linha — o popup ajusta ao conteúdo.**
    ///
    /// ⚠️ Este é o teste do PISO da largura, e é o que faltava. O `a_largura_para_no_espaco_disponivel`
    /// só olha o teto, então um popup colapsado no mínimo passa por ele.
    ///
    /// A asserção é a **altura**, e não a largura, de propósito: a altura de uma linha é
    /// [`TITLE_LINE_HEIGHT`], um número que este módulo declara, então a conta não depende da métrica de
    /// fonte da plataforma. Se o popup encolher ao min-content, o título quebra (no limite, no meio da
    /// palavra) e a altura vira um múltiplo disto.
    ///
    /// O `width: auto` do CSS num elemento posicionado é **shrink-to-fit** =
    /// `min(max(min-content, disponível), max-content)`: o preferido é o **max-content**. É essa a conta
    /// que o popup tem que reproduzir.
    #[gpui::test]
    fn o_titulo_de_uma_linha_nao_quebra(cx: &mut TestAppContext) {
        let (pop, mut vcx) = montar_com_texto(cx, || {
            PopoverTitle::new("Dimensões").into_any_element()
        });

        let b = caixa(&pop, &mut vcx);
        let altura = f32::from(b.size.height);
        let uma_linha = 2.0 * BORDER + 2.0 * PAD + TITLE_LINE_HEIGHT;
        assert_eq!(
            altura, uma_linha,
            "o título tem que caber em UMA linha (borda + 16 + {TITLE_LINE_HEIGHT} + 16 + borda = \
             {uma_linha}); veio {altura}, ou seja {:.1} linhas — o popup encolheu ao min-content em vez \
             de ajustar ao conteúdo",
            (altura - 2.0 * BORDER - 2.0 * PAD) / TITLE_LINE_HEIGHT
        );
    }

    /// **Uma frase que cabe na janela ocupa UMA linha, e o popup fica MAIS LARGO que com uma palavra
    /// só.**
    ///
    /// O teste acima pega a quebra; este pega o colapso: se a largura fosse fixa (ou mínima), a frase
    /// longa mediria o MESMO que a curta. A comparação entre os dois popups é font-independente — não
    /// afirma pixel de texto nenhum, só que um conteúdo mais largo produz uma caixa mais larga.
    #[gpui::test]
    fn a_largura_acompanha_o_conteudo_e_a_frase_nao_quebra(cx: &mut TestAppContext) {
        /// Uma frase longa, mas folgadamente menor que a janela de 1400px do harness.
        const FRASE: &str = "Largura sai do conteúdo, como na fonte.";

        let (curto, mut vcx) = montar_com_texto(cx, || {
            PopoverDescription::new("Ok").into_any_element()
        });
        let estreito = f32::from(caixa(&curto, &mut vcx).size.width);

        let (longo, mut vcx) = montar_com_texto(cx, || {
            PopoverDescription::new(FRASE).into_any_element()
        });
        let b = caixa(&longo, &mut vcx);
        let (largo, altura) = (f32::from(b.size.width), f32::from(b.size.height));

        let uma_linha = 2.0 * BORDER + 2.0 * PAD + TEXT_LINE_HEIGHT;
        assert_eq!(
            altura, uma_linha,
            "a frase cabe em UMA linha numa janela de 1400px; veio {altura} ({:.1} linhas) — sinal de \
             que ela quebrou palavra por palavra",
            (altura - 2.0 * BORDER - 2.0 * PAD) / TEXT_LINE_HEIGHT
        );
        assert!(
            largo > estreito,
            "e a caixa da frase ({largo}) tem que ser mais larga que a da palavra só ({estreito}) — \
             se forem iguais, a largura não vem do conteúdo"
        );
    }

    /// **O conteúdo típico de um popover (título + descrição) mede a soma exata das partes.**
    ///
    /// Este é o caso que a captura de tela pegou e que os testes não pegavam: título e descrição de
    /// texto, um sob o outro, com um vão entre eles. Os números relatados eram largura ~98, altura ~276
    /// e "Dimens/ões" quebrado no meio da palavra; a asserção abaixo é a mesma caixa, medida.
    ///
    /// A altura é a soma declarada — duas bordas, dois respiros, uma linha de título, o vão e uma linha
    /// de descrição —, então ela prova as DUAS coisas de uma vez: que nada quebrou e que o empilhamento
    /// vertical continua sendo o do `flex_col` do viewport (a superfície virou linha, o viewport não).
    #[gpui::test]
    fn titulo_e_descricao_juntos_medem_a_soma_das_partes(cx: &mut TestAppContext) {
        /// O vão entre título e descrição, escolhido pelo call site (a fonte não tem um).
        const GAP: f32 = 8.0;
        /// As strings exatas do relato.
        const TITULO: &str = "Dimensões";
        const DESCRICAO: &str = "Largura sai do conteúdo, como na fonte — nada de w-72.";

        let (pop, mut vcx) = montar_com_texto(cx, || {
            div()
                .flex()
                .flex_col()
                .gap(px(GAP))
                .child(PopoverTitle::new(TITULO))
                .child(PopoverDescription::new(DESCRICAO))
                .into_any_element()
        });

        let b = caixa(&pop, &mut vcx);
        let (largura, altura) = (f32::from(b.size.width), f32::from(b.size.height));

        let esperada = 2.0 * BORDER + 2.0 * PAD + TITLE_LINE_HEIGHT + GAP + TEXT_LINE_HEIGHT;
        assert_eq!(
            altura, esperada,
            "borda + 16 + título({TITLE_LINE_HEIGHT}) + vão({GAP}) + descrição({TEXT_LINE_HEIGHT}) + \
             16 + borda = {esperada}; veio {altura} — se for muito maior, alguma das duas linhas \
             quebrou (o relato original media ~276)"
        );
        assert_eq!(altura, 80.0);

        // E a largura acompanha a linha mais longa (a descrição), muito acima da largura do gatilho —
        // o relato original media ~98, que era a largura do BOTÃO.
        let g = ancora(&pop, &mut vcx);
        assert!(
            largura > 2.0 * g.w,
            "a caixa ({largura}) tem que ser bem mais larga que o gatilho ({}) — a largura vem do \
             conteúdo, não do container de centralização",
            g.w
        );
    }

    /// **Com largura PEDIDA, o texto reflui DENTRO dela** — a caixa não estoura nem colapsa.
    ///
    /// Fecha o outro lado do par: os dois testes acima provam que sem `.width(..)` a caixa segue o
    /// conteúdo; este prova que COM `.width(..)` é o conteúdo que segue a caixa. É o que o `size-full`
    /// do `Viewport` da fonte faz, e depende do `min_w(0)` — sem ele o texto sairia por fora da borda
    /// em vez de quebrar (a mesma armadilha do `crate::card` e do `crate::tooltip`).
    #[gpui::test]
    fn com_largura_pedida_o_texto_reflui_dentro_dela(cx: &mut TestAppContext) {
        /// Estreito o bastante pra a frase não caber numa linha.
        const PEDIDA: f32 = 120.0;
        const FRASE: &str = "Largura sai do conteúdo, como na fonte.";

        theme::set_theme(theme::ThemeMode::Dark);
        let window = cx.add_window(move |_window, cx| {
            let pop = cx.new(|cx| {
                Popover::new(cx)
                    .trigger_button("Abrir")
                    .width(PEDIDA)
                    .content(|_w, _cx| PopoverDescription::new(FRASE).into_any_element())
            });
            Harness { pop }
        });
        let harness = window.root(cx).expect("view raiz");
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.simulate_resize(gpui::size(px(1400.0), px(900.0)));
        vcx.run_until_parked();
        let pop = vcx.read(|cx| harness.read(cx).pop.clone());
        clicar_no_gatilho(&pop, &mut vcx);
        assentar(&pop, &mut vcx);

        let b = caixa(&pop, &mut vcx);
        let (largura, altura) = (f32::from(b.size.width), f32::from(b.size.height));

        assert_eq!(
            largura, PEDIDA,
            "a largura é a PEDIDA, mesmo com a frase querendo mais — a caixa não estoura"
        );
        let uma_linha = 2.0 * BORDER + 2.0 * PAD + TEXT_LINE_HEIGHT;
        assert!(
            altura > uma_linha,
            "e o texto QUEBROU dentro dela (altura {altura} > {uma_linha}); se não quebrasse, ele \
             estaria saindo por fora da borda"
        );
        // Mas quebrou em poucas linhas, não letra por letra: a frase tem ~38 caracteres, então uma
        // quebra por caractere daria dezenas de linhas.
        let linhas = (altura - 2.0 * BORDER - 2.0 * PAD) / TEXT_LINE_HEIGHT;
        assert!(
            linhas <= 6.0,
            "quebrou em {linhas} linhas — mais que isso é quebra por caractere, não por palavra"
        );
    }

    /// **O popup é PINTADO centralizado no gatilho, 4px abaixo dele.**
    ///
    /// Aqui não há modelo nenhum: a caixa é a que o `canvas` mediu no prepaint, ou seja onde o popup
    /// realmente está. É o teste que prova que a centralização por layout (o container do tamanho do
    /// gatilho com `justify-center`, importado do menu) faz o que o `align: center` da referência pede.
    #[gpui::test]
    fn o_popup_centraliza_no_gatilho_quatro_px_abaixo(cx: &mut TestAppContext) {
        let (pop, _ev, mut vcx) = montar(cx, |p| p);
        clicar_no_gatilho(&pop, &mut vcx);
        assentar(&pop, &mut vcx);

        let g = ancora(&pop, &mut vcx);
        let b = caixa(&pop, &mut vcx);
        let (x, y) = (f32::from(b.origin.x), f32::from(b.origin.y));
        let w = f32::from(b.size.width);

        assert_eq!(
            x + w / 2.0,
            g.x + g.w / 2.0,
            "os centros horizontais coincidem — é a definição de align: center"
        );
        assert_eq!(y, g.y + g.h + SIDE_OFFSET, "4px abaixo da base do gatilho");
        // 288px é bem mais largo que este gatilho, então `center` o faz transbordar pros dois lados.
        assert!(x < g.x, "um popup mais largo que o gatilho transborda dos dois lados");
        assert!(x > WINDOW_MARGIN, "sem precisar grudar na janela neste harness");
    }

    /// **A entrada NÃO desloca o popup: ele é pintado no lugar final desde o primeiro frame.**
    ///
    /// ⚠️ É a medição da correção. A fonte não tem `slide-in-from-*` — a entrada dela é `scale-98` +
    /// `opacity-0`, e a escala não é reproduzível no GPUI. Uma versão anterior deste módulo deslocava o
    /// popup 8px "vindo do lado do gatilho", número que veio do shadcn e **não existe no
    /// `popover.tsx`**. Este teste é o que impede a volta: a caixa medida no primeiro frame pintado tem
    /// que ser IDÊNTICA à caixa assentada.
    #[gpui::test]
    fn a_entrada_nao_desloca_o_popup(cx: &mut TestAppContext) {
        let (pop, _ev, mut vcx) = montar_com(cx, Some(CONTEUDO_W), |p| p);
        clicar_no_gatilho(&pop, &mut vcx);

        // Primeiro frame pintado: a transição de entrada acabou de começar.
        let entrando = caixa(&pop, &mut vcx);
        assentar(&pop, &mut vcx);
        let assentada = caixa(&pop, &mut vcx);

        assert_eq!(
            entrando.origin, assentada.origin,
            "a entrada é só opacidade: a POSIÇÃO não muda entre o primeiro frame e o repouso"
        );
        assert_eq!(
            entrando.size, assentada.size,
            "e o TAMANHO também não (a escala não é reproduzível, então nem é tentada)"
        );

        // E a posição é a do `anchor_point` puro, sem resíduo de deslocamento nenhum.
        let g = ancora(&pop, &mut vcx);
        assert_eq!(f32::from(assentada.origin.y), g.y + g.h + SIDE_OFFSET);
    }

    /// **`side: top` abre pra CIMA do gatilho.** Sem isto, um `side` ignorado passaria em todos os
    /// outros testes de posição, que usam o default.
    #[gpui::test]
    fn com_side_top_o_popup_abre_acima(cx: &mut TestAppContext) {
        let (pop, _ev, mut vcx) = montar_com(cx, Some(CONTEUDO_W), |p| p.side(MenuSide::Top));
        clicar_no_gatilho(&pop, &mut vcx);
        assentar(&pop, &mut vcx);
        let b = caixa(&pop, &mut vcx);
        let y_final = f32::from(b.origin.y);

        // É a BASE do popup que encosta 4px antes do topo do gatilho.
        let g = ancora(&pop, &mut vcx);
        assert_eq!(
            y_final + f32::from(b.size.height),
            g.y - SIDE_OFFSET,
            "side top: a base do popup fica 4px acima do topo do gatilho"
        );
        assert!(y_final < g.y, "e o popup inteiro está ACIMA do gatilho");
    }

    /// **O clique no gatilho FECHA o popover aberto** (e não fecha-e-reabre), e o clique fora também
    /// fecha.
    ///
    /// O primeiro caso é a armadilha que a guarda importada do menu resolve: o clique no gatilho é, do
    /// ponto de vista do popup, um clique FORA — então o `on_mouse_down_out` fecharia, e o `on_click`
    /// do gatilho reabriria no mesmo clique.
    #[gpui::test]
    fn o_gatilho_fecha_o_proprio_popover_e_o_clique_fora_tambem(cx: &mut TestAppContext) {
        let (pop, eventos, mut vcx) = montar(cx, |p| p);

        clicar_no_gatilho(&pop, &mut vcx);
        assentar(&pop, &mut vcx);
        clicar_no_gatilho(&pop, &mut vcx);
        assert!(
            !vcx.read(|cx| pop.read(cx).is_open()),
            "o segundo clique no gatilho FECHA — se reabrisse, o botão nunca fecharia o popover"
        );
        assert_eq!(
            &*eventos.borrow(),
            &[PopoverEvent::OpenChange(true), PopoverEvent::OpenChange(false)],
            "abriu uma vez e fechou uma vez"
        );

        // Clique bem longe: fecha.
        clicar_no_gatilho(&pop, &mut vcx);
        assentar(&pop, &mut vcx);
        vcx.simulate_click(point(px(5.0), px(5.0)), Modifiers::default());
        vcx.run_until_parked();
        assert!(!vcx.read(|cx| pop.read(cx).is_open()), "clique fora fecha");
    }

    /// **O clique DENTRO do popup não fecha o popover.**
    ///
    /// É o que separa um popover de conteúdo de um menu: o usuário precisa poder mexer no que está
    /// dentro dele sem que a camada desapareça. Se o `occlude()` ou a hitbox do popup faltarem, o
    /// clique vaza pro `on_mouse_down_out` e fecha.
    #[gpui::test]
    fn o_clique_dentro_do_popup_nao_fecha(cx: &mut TestAppContext) {
        let (pop, eventos, mut vcx) = montar(cx, |p| p);
        clicar_no_gatilho(&pop, &mut vcx);
        assentar(&pop, &mut vcx);

        let b = caixa(&pop, &mut vcx);
        let centro = point(
            b.origin.x + b.size.width / 2.0,
            b.origin.y + b.size.height / 2.0,
        );
        vcx.simulate_click(centro, Modifiers::default());
        vcx.run_until_parked();

        assert!(
            vcx.read(|cx| pop.read(cx).is_open()),
            "clicar no conteúdo NÃO fecha o popover"
        );
        assert_eq!(
            &*eventos.borrow(),
            &[PopoverEvent::OpenChange(true)],
            "e não emite nada de novo"
        );
    }

    /// **O clique no popup NÃO atravessa pro que está atrás dele** — é o `occlude()`.
    ///
    /// ⚠️ Este teste existe porque o de cima (`o_clique_dentro_do_popup_nao_fecha`) **não** cobre o
    /// `occlude()`: quem impede o popover de fechar ali é o próprio `on_mouse_down_out`, que só dispara
    /// fora dos bounds do elemento — tirar o `occlude()` e aquele teste continua passando (medido por
    /// mutação). O que o `occlude()` faz é outra coisa: no GPUI o hit test **acumula** TODOS os
    /// hitboxes sob o cursor, não só o de cima, então sem ele o clique chega TAMBÉM em quem está
    /// embaixo do popup — um botão da app, por exemplo, apertado por acidente através de um popover
    /// aberto.
    #[gpui::test]
    fn o_clique_no_popup_nao_atravessa_pro_fundo(cx: &mut TestAppContext) {
        use std::cell::Cell;

        struct ComFundo {
            pop: gpui::Entity<Popover>,
            /// Quantas vezes o que está ATRÁS do popup recebeu um mouse-down.
            fundo: Rc<Cell<usize>>,
        }
        impl Render for ComFundo {
            fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
                let contador = self.fundo.clone();
                div()
                    .size_full()
                    .relative()
                    // O "fundo": cobre a janela toda e conta cliques. É o que um botão da app seria.
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .size_full()
                            .on_mouse_down(gpui::MouseButton::Left, move |_e, _window, _cx| {
                                contador.set(contador.get() + 1);
                            }),
                    )
                    .child(
                        div()
                            .absolute()
                            .top(px(OFFSET_Y))
                            .left(px(OFFSET_X))
                            .child(self.pop.clone()),
                    )
            }
        }

        theme::set_theme(theme::ThemeMode::Dark);
        let fundo = Rc::new(Cell::new(0usize));
        let contador = fundo.clone();
        let window = cx.add_window(move |_window, cx| {
            let pop = cx
                .new(|cx| Popover::new(cx).trigger_button("Abrir").content(|_w, _cx| conteudo(Some(CONTEUDO_W))));
            ComFundo {
                pop,
                fundo: contador,
            }
        });
        let harness = window.root(cx).expect("view raiz");
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let pop = vcx.read(|cx| harness.read(cx).pop.clone());

        // Antes de abrir, um clique no fundo chega nele — é o controle do experimento: sem esta
        // asserção, um fundo que nunca contasse nada faria a asserção final passar por engano.
        vcx.simulate_click(point(px(600.0), px(400.0)), Modifiers::default());
        vcx.run_until_parked();
        assert_eq!(fundo.get(), 1, "o fundo recebe clique quando não há nada na frente");

        vcx.update(|_window, cx| pop.update(cx, |p, cx| p.open(cx)));
        assentar(&pop, &mut vcx);

        let b = caixa(&pop, &mut vcx);
        let centro = point(
            b.origin.x + b.size.width / 2.0,
            b.origin.y + b.size.height / 2.0,
        );
        vcx.simulate_click(centro, Modifiers::default());
        vcx.run_until_parked();

        assert_eq!(
            fundo.get(),
            1,
            "o clique parou no popup: o fundo NÃO recebeu nada de novo"
        );
        assert!(vcx.read(|cx| pop.read(cx).is_open()), "e o popover continua aberto");
    }

    /// **O teclado abre, o `Escape` fecha, e é o teclado que acende o anel de foco.**
    ///
    /// O `focus-visible` da referência: o clique no gatilho não pode acender anel (defeito já relatado
    /// nesta base), e a primeira tecla acende.
    #[gpui::test]
    fn o_teclado_abre_o_escape_fecha_e_o_anel_acende(cx: &mut TestAppContext) {
        let (pop, eventos, mut vcx) = montar(cx, |p| p);

        // Foca o gatilho por CLIQUE (e fecha o popover que o clique abriu): o anel não acende.
        clicar_no_gatilho(&pop, &mut vcx);
        clicar_no_gatilho(&pop, &mut vcx);
        assert!(
            !crate::focus_ring::visible(),
            "clicar não acende o anel (a referência é focus-visible, não focus)"
        );

        // `Enter` com o popover fechado ABRE.
        vcx.simulate_keystrokes("enter");
        vcx.run_until_parked();
        assert!(vcx.read(|cx| pop.read(cx).is_open()), "enter abre");
        assert!(crate::focus_ring::visible(), "o teclado acende o anel");
        assentar(&pop, &mut vcx);
        assert!(vcx.read(|cx| pop.read(cx).popup_bounds()).is_some());

        // `Escape` fecha.
        vcx.simulate_keystrokes("escape");
        vcx.run_until_parked();
        assert!(!vcx.read(|cx| pop.read(cx).is_open()), "escape fecha");
        assert_eq!(
            &eventos.borrow()[2..],
            &[PopoverEvent::OpenChange(true), PopoverEvent::OpenChange(false)]
        );
    }

    /// **Os setters chegam ao popup medido**: largura, respiro e alinhamento.
    ///
    /// É o par de janela do teste puro dos defaults: aqui se confere que `.width(..)`, `.padding(..)` e
    /// `.align(..)` mudam a CAIXA, e não só um campo. A largura pedida é o **escape** — o default é o
    /// conteúdo, conferido em `o_popup_nasce_com_a_geometria_declarada`.
    #[gpui::test]
    fn os_setters_chegam_ao_popup(cx: &mut TestAppContext) {
        /// Uma largura pedida, diferente do que o conteúdo mediria.
        const LARGURA: f32 = 240.0;
        const RESPIRO: f32 = 8.0;

        let (pop, _ev, mut vcx) = montar(cx, |p| {
            p.width(LARGURA).padding(RESPIRO).align(MenuAlign::End)
        });
        clicar_no_gatilho(&pop, &mut vcx);
        assentar(&pop, &mut vcx);

        let g = ancora(&pop, &mut vcx);
        let b = caixa(&pop, &mut vcx);
        assert_eq!(f32::from(b.size.width), LARGURA, "a largura pedida vence o conteúdo");
        assert_ne!(
            f32::from(b.size.width),
            2.0 * BORDER + 2.0 * RESPIRO + CONTEUDO_W,
            "e é ela, não a medida do conteúdo — senão o setter seria inerte"
        );
        assert_eq!(
            f32::from(b.size.height),
            2.0 * BORDER + 2.0 * RESPIRO + CONTEUDO_H,
            "o respiro pedido, dos dois lados"
        );
        assert_eq!(
            f32::from(b.origin.x) + f32::from(b.size.width),
            g.x + g.w,
            "align: end junta as bordas DIREITAS"
        );
    }

    /// **Um conteúdo alto NÃO estoura a janela: ele para na altura disponível e rola.**
    ///
    /// É o `max-h-(--available-height)` + `overflow-y-auto` do `Viewport` da referência, medido. Um
    /// `max_h` fixo (ou ausente) passaria em todos os outros testes — nenhum deles usa conteúdo alto —,
    /// e só apareceria como um popover saindo pela borda de baixo da tela.
    #[gpui::test]
    fn o_popup_nao_passa_da_altura_disponivel(cx: &mut TestAppContext) {
        /// Conteúdo bem mais alto que qualquer janela do harness.
        const CONTEUDO_ALTO: f32 = 2000.0;

        struct Alto {
            pop: gpui::Entity<Popover>,
        }
        impl Render for Alto {
            fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
                div()
                    .size_full()
                    .pt(px(OFFSET_Y))
                    .pl(px(OFFSET_X))
                    .child(self.pop.clone())
            }
        }

        theme::set_theme(theme::ThemeMode::Dark);
        let window = cx.add_window(move |_window, cx| {
            let pop = cx.new(|cx| {
                Popover::new(cx).trigger_button("Abrir").content(|_w, _cx| {
                    div()
                        .w(px(CONTEUDO_W))
                        .h(px(CONTEUDO_ALTO))
                        .flex_none()
                        .into_any_element()
                })
            });
            Alto { pop }
        });
        let harness = window.root(cx).expect("view raiz");
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.simulate_resize(gpui::size(px(800.0), px(600.0)));
        vcx.run_until_parked();
        let pop = vcx.read(|cx| harness.read(cx).pop.clone());

        clicar_no_gatilho(&pop, &mut vcx);
        assentar(&pop, &mut vcx);

        let g = ancora(&pop, &mut vcx);
        let altura = f32::from(caixa(&pop, &mut vcx).size.height);
        // O teto é o do `crate::menu::popup_max_height`, aplicado à área de conteúdo — a caixa é ele
        // mais as duas bordas.
        let teto = popup_max_height(MenuSide::Bottom, g, 600.0, SIDE_OFFSET);
        assert_eq!(
            altura,
            teto + 2.0 * BORDER,
            "a caixa é a altura disponível mais as duas bordas"
        );
        assert!(
            altura < CONTEUDO_ALTO,
            "e MUITO menor que o conteúdo ({CONTEUDO_ALTO}px), que passou a rolar"
        );
        assert!(altura < 600.0, "cabe na janela de 600px");
    }

    /// **Encurralado na borda, o popup GRUDA na janela em vez de sair dela.**
    ///
    /// É o `collisionPadding` do `Positioner`, que aqui é o `snap_to_window_with_margin`. O gatilho fica
    /// a 20px da borda esquerda e o popover abre `side: left` — sem grudar, ele seria pintado inteiro em
    /// x negativo, ou seja fora da tela.
    ///
    /// Note que ele **desliza**, e não vira pro outro lado: os dois são exclusivos no
    /// [`gpui::anchored`], e a escolha (e o motivo) são as mesmas do [`crate::menu`]. A última asserção
    /// é o que separa os dois comportamentos — o `AnchoredFitMode::SwitchAnchor` default, que é o que
    /// vale se alguém tirar o `snap_to_window_with_margin`, poria o popup à DIREITA do gatilho.
    #[gpui::test]
    fn encurralado_na_borda_o_popup_gruda_na_janela(cx: &mut TestAppContext) {
        /// A que distância da borda esquerda o gatilho fica. Muito menos que a largura do popup.
        const PERTO_DA_BORDA: f32 = 20.0;

        struct Encurralado {
            pop: gpui::Entity<Popover>,
        }
        impl Render for Encurralado {
            fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
                div()
                    .size_full()
                    .pt(px(OFFSET_Y))
                    .pl(px(PERTO_DA_BORDA))
                    .child(self.pop.clone())
            }
        }

        theme::set_theme(theme::ThemeMode::Dark);
        let window = cx.add_window(move |_window, cx| {
            let pop = cx.new(|cx| {
                Popover::new(cx)
                    .trigger_button("Abrir")
                    .side(MenuSide::Left)
                    .content(|_w, _cx| conteudo(Some(CONTEUDO_W)))
            });
            Encurralado { pop }
        });
        let harness = window.root(cx).expect("view raiz");
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let pop = vcx.read(|cx| harness.read(cx).pop.clone());

        clicar_no_gatilho(&pop, &mut vcx);
        assentar(&pop, &mut vcx);

        let g = ancora(&pop, &mut vcx);
        let b = caixa(&pop, &mut vcx);
        let (x, w) = (f32::from(b.origin.x), f32::from(b.size.width));

        // Onde ele cairia se ninguém o segurasse: a borda direita dele 4px antes do gatilho.
        let sem_grudar = g.x - SIDE_OFFSET - w;
        assert!(sem_grudar < 0.0, "o harness tem que ser apertado de verdade");
        assert_eq!(
            x, WINDOW_MARGIN,
            "grudou na janela guardando exatamente a margem (sem grudar seria x={sem_grudar})"
        );

        let se_virasse = g.x + g.w + SIDE_OFFSET;
        assert_ne!(
            x, se_virasse,
            "o popup desliza pra dentro da janela; virar pro outro lado poria x em {se_virasse}"
        );
    }

    /// **O popup escapa do recorte de quem está por fora.**
    ///
    /// O gatilho vai dentro de uma caixa com `overflow_hidden` MENOR que o popup; se ele fosse filho
    /// normal, seria recortado e a caixa medida sairia menor. Como é `deferred`, a medida é a inteira —
    /// é o que faz um popover funcionar dentro de uma [`crate::ScrollArea`].
    #[gpui::test]
    fn o_popup_escapa_do_recorte(cx: &mut TestAppContext) {
        /// A caixa recortante: bem menor que o popup.
        const CAIXA: f32 = 24.0;

        struct Recortado {
            pop: gpui::Entity<Popover>,
        }
        impl Render for Recortado {
            fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
                div().size_full().pt(px(OFFSET_Y)).pl(px(OFFSET_X)).child(
                    div()
                        .w(px(CAIXA))
                        .h(px(CAIXA))
                        .overflow_hidden()
                        .child(self.pop.clone()),
                )
            }
        }

        theme::set_theme(theme::ThemeMode::Dark);
        let window = cx.add_window(move |_window, cx| {
            let pop = cx
                .new(|cx| Popover::new(cx).trigger_button("Abrir").content(|_w, _cx| conteudo(Some(CONTEUDO_W))));
            Recortado { pop }
        });
        let harness = window.root(cx).expect("view raiz");
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let pop = vcx.read(|cx| harness.read(cx).pop.clone());

        // O gatilho está recortado, então clicar nele pelo centro medido pode cair fora da caixa —
        // aqui o popover é aberto por API, que é o que este teste quer exercitar (o recorte, não o
        // clique).
        vcx.update(|_window, cx| pop.update(cx, |p, cx| p.open(cx)));
        assentar(&pop, &mut vcx);

        let b = caixa(&pop, &mut vcx);
        assert_eq!(
            f32::from(b.size.height),
            2.0 * BORDER + 2.0 * PAD + CONTEUDO_H,
            "a altura INTEIRA, não a fatia de {CAIXA}px"
        );
        assert!(
            f32::from(b.size.width) > CAIXA,
            "e a largura inteira, {}px, maior que os {CAIXA}px da caixa recortante",
            f32::from(b.size.width)
        );
    }

    /// **Fechar desmonta o popup no MESMO frame** — a fonte não tem animação de saída.
    ///
    /// A prova não é a medida (a [`Popover::popup_bounds`] é observabilidade e guarda o último frame em
    /// que o popup existiu), é a **hitbox**: com o popup fora da árvore, um clique no lugar exato onde
    /// ele estava deixa de ser absorvido e passa a chegar no que está atrás. É a mesma técnica do
    /// `o_clique_no_popup_nao_atravessa_pro_fundo`, invertida.
    ///
    /// ⚠️ Se alguém adicionar o fade de saída do shadcn, o popup continua montado por 150ms depois do
    /// fechamento, o clique é absorvido pelo `occlude()` dele, o fundo não conta, e este teste falha.
    #[gpui::test]
    fn fechar_desmonta_o_popup_no_mesmo_frame(cx: &mut TestAppContext) {
        use std::cell::Cell;

        struct ComFundo {
            pop: gpui::Entity<Popover>,
            fundo: Rc<Cell<usize>>,
        }
        impl Render for ComFundo {
            fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
                let contador = self.fundo.clone();
                div()
                    .size_full()
                    .relative()
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .size_full()
                            .on_mouse_down(gpui::MouseButton::Left, move |_e, _window, _cx| {
                                contador.set(contador.get() + 1);
                            }),
                    )
                    .child(
                        div()
                            .absolute()
                            .top(px(OFFSET_Y))
                            .left(px(OFFSET_X))
                            .child(self.pop.clone()),
                    )
            }
        }

        theme::set_theme(theme::ThemeMode::Dark);
        let fundo = Rc::new(Cell::new(0usize));
        let contador = fundo.clone();
        let window = cx.add_window(move |_window, cx| {
            let pop = cx.new(|cx| {
                Popover::new(cx)
                    .trigger_button("Abrir")
                    .content(|_w, _cx| conteudo(Some(CONTEUDO_W)))
            });
            ComFundo {
                pop,
                fundo: contador,
            }
        });
        let harness = window.root(cx).expect("view raiz");
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let pop = vcx.read(|cx| harness.read(cx).pop.clone());

        vcx.update(|_window, cx| pop.update(cx, |p, cx| p.open(cx)));
        assentar(&pop, &mut vcx);
        let antes = caixa(&pop, &mut vcx);
        let centro = point(
            antes.origin.x + antes.size.width / 2.0,
            antes.origin.y + antes.size.height / 2.0,
        );

        // Fecha e redesenha **sem assentar nada**: não há transição de saída a atravessar.
        vcx.update(|_window, cx| pop.update(cx, |p, cx| p.close(cx)));
        vcx.run_until_parked();
        assert!(!vcx.read(|cx| pop.read(cx).is_open()));

        let antes_do_clique = fundo.get();
        vcx.simulate_click(centro, Modifiers::default());
        vcx.run_until_parked();
        assert_eq!(
            fundo.get(),
            antes_do_clique + 1,
            "o clique no lugar do popup chegou no FUNDO: o popup já saiu da árvore no mesmo frame \
             do fechamento (se houvesse fade de saída, o occlude() dele teria absorvido)"
        );
        assert!(
            !vcx.read(|cx| pop.read(cx).is_open()),
            "e o popover continua fechado"
        );
    }
}

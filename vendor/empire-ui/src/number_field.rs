//! `number_field` — o **campo numérico com botões de passo** do [coss][1], sobre o primitivo
//! [NumberField do Base UI][2].
//!
//! [1]: https://github.com/cosscom/coss/blob/main/packages/ui/src/components/number-field.tsx
//! [2]: https://github.com/mui/base-ui/tree/master/packages/react/src/number-field
//!
//! ```ignore
//! // uma vez, no `new` da sua view:
//! let quantidade = cx.new(|cx| {
//!     NumberField::new(Some(1.0), window, cx)
//!         .min(0.0)
//!         .max(99.0)
//!         .step(1.0)
//! });
//! let _sub = cx.subscribe(&quantidade, |this, _campo, ev: &NumberFieldEvent, cx| match ev {
//!     // a cada tecla parseável e a cada passo
//!     NumberFieldEvent::Change(v) => this.previa(*v, cx),
//!     // no blur e no fim de um passo
//!     NumberFieldEvent::Commit(v) => this.salvar(*v, cx),
//! });
//!
//! // a cada render: `.child(self.quantidade.clone())`
//! ```
//!
//! # A anatomia
//!
//! Uma moldura só — a do [`crate::Input`] — com **três** filhos em linha: o botão de menos, o campo
//! de texto e o botão de mais. Os botões vão de borda a borda na vertical, têm o respiro horizontal
//! do campo e arredondam **só os dois cantos que encostam na moldura**; o campo cresce no meio.
//!
//! ```text
//! ┌──────┬───────────────┬──────┐
//! │  −   │      42       │  +   │   ← 28 / 32 / 36 de altura externa (Sm / Md / Lg)
//! └──────┴───────────────┴──────┘
//!    9/11      cresce       9/11
//! ```
//!
//! Os botões **não recebem foco** (`tabIndex={-1}` no primitivo): o controle é **uma** parada de
//! `Tab`, e quem tem o foco é sempre o campo de texto. Clicar num botão foca o campo, pra o usuário
//! poder continuar pelas setas — é o `inputRef.current?.focus()` do `useNumberFieldStepperButton`.
//!
//! # O valor é `Option<f32>`, e isso é da referência
//!
//! `NumberField.Root` tem `value: number | null`, e o `null` não é um detalhe: campo vazio **é** um
//! estado, distinto de zero. Digitar até esvaziar o campo produz `None`; ↑ num campo vazio semeia
//! **0** (e o clampa), que é o `incrementValue` do primitivo ao pé da letra.
//!
//! # A matemática — pura, testada, e transcrita do primitivo
//!
//! A ordem é a do `toValidatedNumber` do Base UI:
//!
//! ```text
//! 1. snap ao passo   (só se `snap_on_step` E o passo não for zero)
//! 2. clamp em [min, max]   (só se a interação clampa — ver `allow_out_of_range`)
//! ```
//!
//! O `clamp` é o **`clamp_js`** da casa ([`crate::meter::clamp_js`], `max(min, min(v, max))`) — o
//! MESMO `internals/clamp.ts` que o primitivo do medidor e o do number field importam lá. **Não** é
//! o [`f32::clamp`], que entra em pânico com `min > max`.
//!
//! Duas coisas do `toValidatedNumber` **colapsam** aqui, e cada uma tem teste que prova a álgebra em
//! vez de código que a repete:
//!
//! - a base do snap é sempre `min ?? 0`. A condição de lá
//!   (`small || minWithDefault === MIN_SAFE_INTEGER ? minWithZeroDefault : minWithDefault`) tem um
//!   operando **inerte**: com `min` declarado os dois ramos são o mesmo número, e sem `min` o segundo
//!   termo do `||` já basta. O que o `small` (o passo do `alt`) de fato decide é o arredondamento —
//!   ele alinha pro múltiplo mais PRÓXIMO, enquanto o passo normal alinha direcionalmente. Ver
//!   `a_base_do_snap_e_sempre_o_minimo_ou_zero`;
//! - o `removeFloatingPointErrors` e o segundo clamp que vem depois dele saem: em `f32` o primeiro é
//!   a identidade (ver "Não reproduzível") e, sendo identidade, o segundo reclampa um número que já
//!   está na faixa.
//!
//! ## Os casos degenerados, todos decididos
//!
//! | caso | resultado | de onde vem |
//! |---|---|---|
//! | valor abaixo de `min` | `min` | o clamp |
//! | valor acima de `max` | `max` | idem |
//! | `min == max` | esse valor | idem |
//! | **`min > max`** (faixa invertida) | **sempre `min`** | `max(min, min(v, max))`: o `min` interno desce pra `max`, o `max` externo sobe pra `min` — o piso vence |
//! | `min`/`max` ausentes | `∓`[`SAFE_INTEGER`] | `min ?? Number.MIN_SAFE_INTEGER` do primitivo |
//! | **`step == 0`** | ↑/↓ não mudam nada (`v + 0` = `v`) e o **snap é desligado** | o `step !== 0` do `toValidatedNumber` |
//! | **`step < 0`** | ↑ **diminui** e ↓ aumenta — o sinal entra em `v + passo × direção`; o snap usa `abs(step)` de tamanho e `signum(step)` de sentido | transcrito, e a referência não o proíbe |
//! | texto que **não parseia** | valor **inalterado**; o texto volta ao formatado no render seguinte | `parsedValue === null → return` |
//! | texto **vazio** | valor `None` | `inputValue.trim() === '' → setValue(null)` |
//! | `"inf"` / `"NaN"` no texto | **recusados** ([`crate::scrub_input::parse_finite`]) | nosso, e obrigatório — ver abaixo |
//! | valor `None` + ↑/↓ | semeia **0**, clampado, **sem** snap | `incrementValue` |
//! | `allow_out_of_range` + digitação | **não** clampa | `shouldClampValue = !allowOutOfRange \|\| !isInputReason` |
//! | `allow_out_of_range` + ↑/↓/botão | clampa | idem — passo sempre clampa |
//! | valor no limite | o botão daquele lado **para de agir** (e não muda de aparência) | `isAtBoundary` |
//! | valor no teto **e** `step < 0` | o `+` continua desligado, **embora apertá-lo baixasse** o valor | `isAtBoundary` compara `value >= max` e não olha o sinal do passo. É a única configuração em que o gate é observável — com passo positivo o clamp já torna o passo um no-op |
//!
//! ⚠️ **Por que NaN tem que morrer no parse.** Em JS `Math.max(min, Math.min(NaN, max))` propaga
//! `NaN`. Em Rust é o **contrário**: `f32::min` devolve *o outro operando* quando um é `NaN`, então
//! `clamp_js(NaN, 0, 100)` devolve **100** — um valor plausível, silencioso e errado. Por isso todo
//! caminho de entrada (parse, [`NumberField::set_value`], os builders) passa por um guarda de
//! finitude, e o teste `nan_nao_atravessa_o_clamp` trava a armadilha.
//!
//! # Teclado
//!
//! | tecla | efeito |
//! |---|---|
//! | `↑` / `↓` | ± `step` (default **1**) |
//! | `shift`+`↑` / `↓` | ± `large_step` (default **10**) |
//! | `alt`+`↑` / `↓` | ± `small_step` (default **0,1**) |
//! | `Home` | vai pro `min` — **só se `min` foi declarado**; sem ele a tecla segue pro campo e move o cursor |
//! | `End` | vai pro `max` — idem |
//! | `PageUp` / `PageDown` | **nada**, de propósito — ver "Ausente" |
//!
//! ## Como as teclas chegam aqui
//!
//! O [`InputState`] do núcleo empilha o contexto de teclas `Input` e o `gpui_component::init` vincula
//! `up`→`MoveUp`, `shift-up`→`SelectUp`, `home`→`MoveHome`… Num campo de UMA linha esses handlers
//! existem e **não** propagam (`MoveUp` faz `return` cedo e a ação morre ali), então nem um
//! `on_key_down` nem um `capture_action` do núcleo alcançariam a tecla — e `SelectUp`/`SelectDown`
//! nem são tipos públicos (`pub(crate) mod actions` no `gpui-component`).
//!
//! A saída é a que o próprio `NumberInput` do núcleo usa: **ações próprias**, vinculadas num contexto
//! próprio. A precedência é o detalhe que decide se funciona, e ela é medida, não presumida
//! (`gpui::Keymap::bindings_for_input`, 0.2.2): os bindings que casam são ordenados por
//! **profundidade** do casamento no *stack* de contextos e, em empate, por **ordem de registro
//! decrescente**. O contexto deste componente é o do WRAPPER, portanto mais raso que o `Input` do
//! campo — um predicado `"NumberField"` perderia. Com `"NumberField > Input"` o predicado casa no
//! MESMO nó que `"Input"` (mesma profundidade), o empate cai na ordem de registro, e [`init`] roda
//! depois do `gpui_component::init`. Ver [`PREDICADO`].
//!
//! E quando o limite não existe, `Home`/`End` fazem `cx.propagate()`: o laço de
//! `Window::dispatch_key_event` segue pro binding seguinte e o `MoveHome` do núcleo move o cursor —
//! que é exatamente o "let the browser handle it" da referência.
//!
//! # Por que isto NÃO é um [`crate::input_group::InputGroup`]
//!
//! O grupo é o candidato óbvio: superfície única, um anel, uma parada de `Tab` — tudo o que este
//! componente precisa. Ele não serve, e por dois números medidos:
//!
//! 1. **O respiro do addon não é o respiro do botão de passo.** Um addon é conteúdo INSET: o
//!    [`crate::input_group::addon_lead_pad`] dá `pad_x − puxão`, ou seja **3px** pro `Md` com um
//!    [`crate::input_group::AddonItemKind::Control`] dentro. No number field o botão **é** o
//!    respiro: ele encosta na borda e carrega os 11px inteiros por dentro, de modo que o glifo caia
//!    a 12px da borda externa E o realce de hover cubra o segmento todo. Absorver os 3px na
//!    padding do botão acerta o glifo e erra o hover — sobra um filete de 3px de superfície não
//!    realçada entre a borda e o preenchimento, e o `rounded-s-[calc(var(--radius-lg)-1px)]` do
//!    original deixa de encaixar na curva interna da borda.
//! 2. **O campo hospedado encolhe do lado do addon.** O [`crate::input_group::field_pad`] troca os
//!    11px por **8** quando há addon inline (é o `**:[input]:ps-2` da referência do `input-group`).
//!    O `number-field.tsx` mantém `px-[calc(--spacing(3)-1px)]` no input, sem exceção — porque lá o
//!    `NumberFieldGroup` é uma superfície IRMÃ do `input-group`, não uma instância dele.
//!
//! Fazer o grupo caber pediria duas escapatórias novas na API dele, e a segunda seria um addon com
//! fundo e raio próprios — literalmente o que o doc do [`crate::input_group`] proíbe ("Um addon não
//! é um filho costurado: ele não tem borda, não tem fundo, não tem raio"). Então a moldura é montada
//! aqui, com **zero** número e **zero** cor de superfície próprios: borda, fundo, sombra, bisel,
//! anel, raio e o desfoque por clique-fora vêm todos de [`crate::input`], exatamente como o
//! [`crate::input_group`] faz. Se aparecer aqui um `Rgba8` de borda ou um `10.0` de raio, este porte
//! errou.
//!
//! O que o grupo daria de graça e que sai de graça aqui também: **uma** parada de `Tab` (os botões
//! não têm `track_focus`, então não existem pro `Tab`) e **um** anel (o do campo hospedado está
//! desligado por [`crate::Input::unstyled`]; quem desenha é este módulo, com
//! [`crate::input::ring_overlay`]).
//!
//! ⚠️ O anel é o do CAMPO, e não o [`crate::focus_ring`]: a referência pede `focus-within:ring-[3px]`
//! e não `focus-visible:`, ou seja ele acende em qualquer foco, inclusive de clique. É a mesma
//! decisão (e o mesmo código) do [`crate::Input`] e do [`crate::input_group`]; o
//! [`crate::focus_ring`] é pros componentes que a referência gateia em `:focus-visible`.
//!
//! # O que vem do resto da casa
//!
//! - **[`crate::Input`]**: o campo inteiro, em [`crate::Input::unstyled`] — miolo, IME, seleção,
//!   undo, e as métricas de [`InputSize`] (miolo 26/30/34, respiro 9/11/11, corpo 14). Nenhuma
//!   sobrescrita de métrica: os números do `number-field.tsx` **são** os do `InputSize`, conferidos
//!   classe por classe (ver "Resolvido em número").
//! - **[`crate::scrub_input`]**: a formatação numérica ([`crate::scrub_input::format_value`], casas
//!   decimais derivadas do passo) e o parse ([`crate::scrub_input::parse_finite`], com a recusa de
//!   `inf`/`NaN`). Não há uma segunda gramática de número nesta casa.
//! - **[`crate::meter`]**: o [`crate::meter::clamp_js`] e o
//!   [`crate::meter::tabular_nums`] — os dois vêm do mesmo lugar na referência (`internals/clamp.ts`
//!   e a classe `tabular-nums`), então são compartilhados aqui como são compartilhados lá.
//!
//! # Diferenças em relação ao original — todas declaradas
//!
//! **Não reproduzível**
//!
//! - ⚠️ **`text-center` no campo.** É a maior divergência de pixel deste porte, e é do núcleo: não
//!   existe alinhamento de texto no campo do `gpui-component` (grep de `text_align`/`TextAlign` em
//!   `vendor/gpui-component/src/input/` não devolve nada; o elemento calcula a origem do glifo a
//!   partir da borda ESQUERDA das próprias bounds). O número fica encostado à esquerda da caixa que
//!   cresce, em vez de centrado nela. A caixa em si está no lugar certo — o que se perde é a posição
//!   horizontal dos dígitos dentro dela.
//! - **`justify-between` no grupo.** Sem efeito observável: o filho do meio é `grow`, então não
//!   sobra espaço livre pra distribuir. Não está escrito, pra não haver estilo morto.
//! - **`removeFloatingPointErrors`.** É uma limpeza de ruído binário na **15ª casa significativa**
//!   (`parseFloat(v.toPrecision(15))`), com um teto de desvio de `1e-10`. Em `f32` há ~7,2 casas
//!   significativas: um `f32` impresso com 15 dígitos e relido volta EXATAMENTE o mesmo `f32`, então
//!   a limpeza é a função identidade — e o teto de `1e-10` é ~800× menor que um ULP de `f32` em
//!   torno de 1,0 (1,19e-7). Transcrever isto seria código morto. Travado em
//!   `a_limpeza_de_ruido_da_referencia_e_identidade_em_f32`.
//! - **`transition-colors`** no hover dos botões: o GPUI não tem transição declarativa de estilo. A
//!   cor troca de um frame pro outro. (O `transition-shadow` do anel, ao contrário, EXISTE — vem do
//!   cross-fade feito à mão no [`crate::input`].)
//! - **`pointer-coarse:after:min-h-11`** (o alvo de 44px pra toque nos botões): o GPUI 0.2.2 não
//!   distingue a granularidade do ponteiro, e uma janela de desktop não tem toque.
//! - **`data-slot`**, **`aria-*`**, **`aria-roledescription="Number field"`**, `role`: o GPUI 0.2.2
//!   não tem árvore de acessibilidade (verificado no `Cargo.toml` e em `styled.rs`/`window.rs` da
//!   0.2.2; o que existe é [`gpui::FocusHandle`] e `tab_stop`). Mesma constatação do
//!   [`crate::meter`].
//! - **`not-dark:bg-clip-padding`**: recorte de fundo por box não existe no GPUI.
//! - **O cursor do scrub** (`NumberField.ScrubAreaCursor` + `CursorGrowIcon`): a referência trava o
//!   ponteiro (`requestPointerLock`) e desenha um SVG que segue um cursor virtual. No GPUI há
//!   [`gpui::CursorStyle`] — um cursor do SISTEMA, não um desenho. Ver "Ausente" pro scrub em si.
//! - **`Math.sign(0) === 0`**: o `signum` do Rust devolve `1.0` pra `0.0` (e `-1.0` pra `-0.0`).
//!   Inalcançável aqui — o snap só roda com `step != 0` (o mesmo guarda da referência), e é o único
//!   lugar que usa o sinal.
//! - **`Math.round` arredonda meio pra +∞; `f32::round` arredonda meio pra longe do zero.** Diverge
//!   só quando `raw_steps / step` cai exatamente em `−n,5` no snap `nearest` (o ramo do `alt`). Fica
//!   declarado em vez de "corrigido": a correção seria uma terceira regra de arredondamento nesta
//!   casa.
//!
//! **Resolvido em número** (`--spacing` = 4px; as variantes `sm:` do Tailwind são as que valem, porque
//! uma janela de desktop está sempre acima do breakpoint de 640px — mesma decisão do resto da lib)
//!
//! - **`sm:h-6.5 / sm:h-7.5 / sm:h-8.5`** no input → 26 / 30 / 34, que **são** os
//!   [`InputSize::content_height`] da casa; com as duas bordas de 1px da moldura, 28 / 32 / 36 de
//!   altura externa ([`InputSize::height`]). Travado em `a_altura_e_a_do_campo_da_casa`, e medido em
//!   janela nos três tamanhos. ⚠️ A moldura **não declara altura** — o `NumberFieldGroup` do coss
//!   também não; ela vem do campo, como lá.
//! - **`px-[calc(--spacing(3)-1px)]`** = **11** e **`in-data-[size=sm]:px-[calc(--spacing(2.5)-1px)]`**
//!   = **9**, nos botões E no input → são os [`InputSize::pad_x`], não constantes novas. O `-1px`
//!   desconta a borda, pro recuo total a partir da borda externa fechar em 12.
//! - **`rounded-s-[calc(var(--radius-lg)-1px)]`** / `rounded-e-*` → `10 − 1` = **9**, e só nos DOIS
//!   cantos que encostam na moldura. Ver [`RAIO_DESCONTO`]. O 10 vem de
//!   [`crate::input::FIELD_RADIUS`].
//! - **`sm:[&_svg]:size-4`** → **16** ([`LADO_DO_GLIFO`]). A classe base `size-4.5` (18) é o ramo
//!   `max-sm:`.
//! - **`sm:text-sm`** → o PAR do Tailwind, **(14, 20)**. Fixar a entrelinha é obrigatório: o default
//!   do GPUI é `relative(1.618_034)`, que sobre 14 dá **22,65**. Ver [`CORPO_DO_TEXTO`] e
//!   [`ENTRELINHA`].
//! - **`tabular-nums`** → a feature OpenType `tnum`, aplicada na MOLDURA e herdada pelo miolo do
//!   núcleo (que lê `window.text_style()`). Reusa [`crate::meter::tabular_nums`].
//! - **`data-disabled:opacity-64`** → [`OPACIDADE_DESABILITADO`]. O campo hospedado **não** esmaece a
//!   si mesmo (é o que [`crate::Input::unstyled`] desliga), então não há multiplicação de opacidade —
//!   o mesmo cuidado do [`crate::input_group`].
//! - **`hover:bg-accent`** → o `--accent` da referência, que resolve pro MESMO valor do `--secondary`
//!   (preto/branco a 4%). Ver [`NF_CLARO`]/[`NF_ESCURO`].
//! - **`text-foreground`** nos glifos → o `--foreground` do PRÓPRIO campo
//!   ([`crate::input::FieldPalette::text`]), não uma cópia. Mesma decisão do diamante do
//!   [`crate::scrub_input`].
//! - **`shadow-xs/5`**, **`ring-ring/24`**, **`border-input`**, **`bg-background`/`dark:bg-input/32`**,
//!   **`has-aria-invalid:border-destructive/36`**, o bisel do `before:` → todos de
//!   [`crate::input`], sem uma linha de superfície aqui. ⚠️ No Tailwind v4 o `/N` **multiplica** o
//!   alfa; onde isso aparece, quem já resolveu foi o [`crate::input`] (`dark:bg-input/32` = branco a
//!   8% × 0,32 ≈ 2,5%, o `0xffffff07` da paleta dele).
//! - **`step = 1`**, **`smallStep = 0.1`**, **`largeStep = 10`** → [`PASSO`],
//!   [`PASSO_PEQUENO`], [`PASSO_GRANDE`].
//! - **`min ?? Number.MIN_SAFE_INTEGER`** → [`SAFE_INTEGER`] (2⁵³−1, que em `f32` arredonda pro
//!   próximo par — irrelevante, é um limite de "sem limite").
//! - **`STEP_EPSILON_FACTOR = 1e-10`** → **reescalado**, ver "Desvio consciente".
//!
//! **Desvio consciente**
//!
//! - **A tolerância do snap é em ULPs de `f32`, não o `1e-10` do primitivo.** Ver
//!   [`TOLERANCIA_DO_SNAP`]: transcrito ao pé da letra o número seria constante morta (`1.0f32 + 1e-10
//!   == 1.0f32`), e removido de vez o `floor` cairia um passo atrás no primeiro múltiplo que o `f32`
//!   não representa exato. O que se reescala é a MAGNITUDE, na unidade que existe em `f32`.
//! - **O glifo é o Iconoir, não o lucide.** O `MinusIcon`/`PlusIcon` do lucide é traço 2 num `viewBox`
//!   24, ou seja **1,33px** de tinta a 16px. O par da casa (`iconoir/regular/minus.svg` e `plus.svg`)
//!   é traço 1,5 no mesmo `viewBox` → **1,0px**, que é a regra escrita no [`crate::input`] (o doc de
//!   `ALCA_GLIFO`: "a regra da casa não é o número, é o RESULTADO: 1,0px de tinta"). O par
//!   `icons/minus.svg` + `icons/plus.svg`, que já existe no bundle, **não** serve: o primeiro é
//!   `viewBox` 16 traço 1,5 (**1,5px**) e o segundo `viewBox` 24 traço 2 (**1,33px**) — dois pesos
//!   diferentes dentro de UM controle. Travado em `o_menos_e_o_mais_tem_o_mesmo_peso_de_traco`.
//! - **`shift`+`Home` / `shift`+`End` NÃO pulam pro limite.** Na referência o `shiftKey` não está na
//!   lista de modificadores que dão bypass, então `shift+Home` cai no ramo do limite e o
//!   `preventDefault` come a seleção. Aqui a tecla fica com o campo (seleciona até o começo/fim da
//!   linha): o `shift` como "estender seleção" é convenção de campo de texto mais profunda do que a
//!   combinação que a referência não distingue de propósito, e os modificadores que o desenho DO
//!   number field usa são `alt` e `shift` **nas setas**.
//! - **Um `Change` de valor idêntico é engolido.** A referência dispara `onValueChange` mesmo quando o
//!   número não muda, durante digitação. Aqui o eco do nosso próprio `set_value` no campo chega como
//!   `InputEvent::Change` **assíncrono** (a entrega de assinatura é no flush de efeitos, depois do
//!   render), então um flag de reentrância não o alcança — o guarda é por identidade de TEXTO. A
//!   consequência é que redigitar exatamente o valor já vigente não emite `Change`. Nada observável
//!   depende de um evento que carrega o valor que o assinante já tem.
//! - **`Enter` e `Escape` não são tratados** — e isso é fiel: as duas estão no `NAVIGATE_KEYS` do
//!   `NumberFieldInput` e o handler devolve cedo. O commit é no **blur**, e o valor já subiu ao vivo
//!   por `Change` a cada tecla parseável (é o `onChange` do primitivo). ⚠️ Diferente do
//!   [`crate::ScrubInput`], que commita no `Enter` e **cancela** no `Escape` — lá o valor NÃO é ao
//!   vivo, então precisa dos dois gestos; aqui eles não teriam o que fazer.
//! - **Nenhum modo "misto"** (o `"--"` da multi-seleção do [`crate::ScrubInput`]). Não existe na
//!   referência, e o `None` daqui já é o estado vazio — dois vazios diferentes num campo só seriam
//!   um a mais do que o usuário consegue distinguir.
//! - **No limite, o botão para de agir mas não muda de aparência.** É o que o CSS faz: o
//!   `NumberFieldDecrement` do coss não declara NENHUM `disabled:`, e `:hover` continua casando num
//!   `<button disabled>` (o `pointer-events-none` só entra com o campo inteiro desabilitado). O
//!   realce fica, o clique não. Travado em `no_limite_o_botao_para_de_agir`.
//!
//! **Superset consciente**
//!
//! - **[`NumberFieldEvent`] com dois braços.** No React são duas props (`onValueChange` /
//!   `onValueCommitted`); aqui a casa emite eventos, e um enum de dois braços é o mesmo par. O
//!   `Commit` sem `Change` antes não existe: todo commit é precedido do `Change` que o produziu.
//! - **[`NumberField::value`] como leitura pública.** No React o valor vive no
//!   `NumberFieldRootContext`. Aqui é público porque quem escreve a própria leitura ("42 de 99")
//!   precisa do MESMO número que o campo usa. Mesma decisão do [`crate::Meter`].
//!
//! **Ausente**
//!
//! - **`NumberField.ScrubArea`** — o rótulo que se arrasta pra mudar o valor. **Resolvido em outro
//!   componente**: é o [`crate::ScrubInput`], onde o ícone-alça é a área de scrub, com o clamp, o
//!   snap e o `CursorStyle::ResizeLeftRight` já no lugar. Duas gramáticas de arraste numérico na
//!   mesma app divergiriam, e a que ninguém olha é a que fica pra trás.
//! - **A seleção total ao focar** do [`crate::ScrubInput`] (`selecionar_ao_focar`) — de propósito, e
//!   é o oposto: o `onFocus` do `NumberFieldInput` põe o caret no **fim** na primeira focagem
//!   (`target.setSelectionRange(length, length)`), sem selecionar nada. E o caret no fim também não é
//!   reproduzível: o `InputState` (fork incluído) expõe `select_all_now` e nenhum setter de caret. O
//!   que vale é o do GPUI — o caret vai onde o clique caiu.
//! - **`allowWheelScrub`.** Opt-in na referência (`false` por default) e nunca ligado pelo
//!   `number-field.tsx`. Entra quando houver caso de uso; o gesto de "arrastar pra mudar" já existe
//!   nesta casa no [`crate::ScrubInput`].
//! - **`press-and-hold`** (o `usePressAndHold`: segurar o botão repete o passo). Um clique dá um
//!   passo. A repetição por TECLA existe de graça (o repeat do sistema reemite `up`/`down`), que é o
//!   caminho que o primitivo declara como o preferido pra teclado.
//! - **`format` (`Intl.NumberFormatOptions`) e `locale`.** O texto é
//!   [`crate::scrub_input::format_value`]: casas decimais derivadas do passo, ponto decimal en-US.
//!   Mesma decisão, e mesma razão, do [`crate::meter`] e do [`crate::slider`] — formatação é produto.
//!   Sem `format` também caem `hasNumberFormatRoundingOptions` e o ramo de arredondamento por
//!   formatador do `removeFloatingPointErrors`.
//! - **`name` / `form` / `required` / `readOnly`** e a integração com `Field`/`Form` do Base UI: não
//!   há formulário neste crate. `readOnly` sem isso difere de `disabled` só em "ainda foca e não
//!   esmaece", uma distinção sem consumidor.
//! - **O rótulo e a linha de apoio.** O `NumberField` raiz é um `flex flex-col items-start gap-2` que
//!   existe pra empilhar um `Label`/`Field.Label` — componentes que este crate não tem. Pôr um rótulo
//!   aqui com a escala auxiliar do [`crate::Input`] criaria uma TERCEIRA gramática de rótulo. O
//!   rótulo é de quem posiciona, no padrão de inspector que o [`crate::ScrubInput`] já usa.
//! - **`inputRef`**, **`render`/`className` como função**, e o re-export do `NumberFieldPrimitive`:
//!   escapatórias de React.
//! - **`NumberFieldContext`**: existe no original só pra o `ScrubArea` achar o `id` do input e
//!   escrever `htmlFor`. Sem DOM e sem `ScrubArea`, não há o que carregar.
//!
//! # Sem cobertura de teste — declarado
//!
//! - **Tinta, cursor e hover.** Nem a cor do glifo, nem o `CursorStyle::PointingHand`, nem o
//!   `hover:bg-accent` são legíveis de um teste de janela (o `debug_bounds` dá caixa, não tinta). O
//!   que É verificável está travado: a paleta contra literais, o par de glifos existir no bundle e
//!   ter o mesmo peso de traço, e a altura medida em janela.
//! - **O `tabular-nums` chegar ao miolo do núcleo.** O caminho é herança por `window.text_style()`, e
//!   o GPUI não deixa um teste ler o estilo de texto composto nem a cena pintada. O que se garante é
//!   que a feature PEDIDA é `("tnum", 1)` (teste no [`crate::meter`]) — o erro plausível é o nome da
//!   tag, que falha em silêncio.
//! - **A precedência dos bindings de [`init`]**. O raciocínio está no doc e vem da fonte do GPUI
//!   0.2.2, mas afirmá-lo num teste exigiria montar um `Keymap` com o `gpui_component::init` inteiro
//!   e inspecionar `bindings_for_input` — API `pub` do GPUI, mas o teste travaria a ordem de
//!   inicialização de um crate de terceiro. O que se testa é o EFEITO, em janela
//!   (`a_seta_para_cima_soma_um_passo`).
//! - **`shift`+`↑`/`↓` e `alt`+`↑`/`↓` em janela.** O `VisualTestContext` simula tecla por
//!   `simulate_keystrokes`, que aceita a sintaxe com modificador; o que não dá pra afirmar é que o
//!   `alt-up` chegue como AÇÃO num ambiente headless onde o mapa de teclas do sistema não participa.
//!   A conta dos três tamanhos de passo está travada na função pura
//!   ([`Rules::step_amount`]), que é onde a decisão mora.
//! - **`text-center`**: não há o que testar, é ausência declarada acima.
//! - **A ponte "o núcleo entrega o `InputEvent::Blur`".** No `TestAppContext` ela **não acontece**:
//!   medido, depois de um clique no vazio e dois `run_until_parked`, o `is_focused` do campo já é
//!   `false` e o `texto_sujo` do componente ainda é `true`. É a mesma limitação que o
//!   [`crate::scrub_input`] documenta pro `InputEvent::Focus` — o observador de foco do GPUI compara o
//!   CAMINHO de foco do quadro desenhado, e naquele ambiente ele não fecha no campo. A LÓGICA de
//!   commit é testada de verdade (`digitar_muda_ao_vivo_e_o_blur_confirma` chama o handler à mão, com
//!   o campo de fato desfocado); o que fica sem cobertura é a entrega do evento, e ela está em
//!   produção pelo mesmo caminho no [`crate::ScrubInput`].
//! - **A metade "o botão do outro lado continua agindo" quando o passo é NEGATIVO.** O clamp esconde a
//!   diferença; a afirmação vive em `clicar_nos_botoes_anda_um_passo`, com passo positivo, onde ela é
//!   observável. Declarado dentro do teste de limite.

use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, svg, App, AppContext as _, ClickEvent, Context, CursorStyle, Entity, EventEmitter,
    FocusHandle, Focusable, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, Render, SharedString, StatefulInteractiveElement as _, Styled, Subscription,
    Window,
};
use gpui_component::input::{InputEvent, InputState};

use crate::color::Rgba8;
use crate::input::{self, single_line, Input, InputSize};
use crate::theme;

// =================================================================================================
// Ações e teclado
// =================================================================================================

gpui::actions!(
    empire_ui_number_field,
    [
        /// `↑` — soma um [`PASSO`].
        StepUp,
        /// `↓` — subtrai um [`PASSO`].
        StepDown,
        /// `alt`+`↑` — soma um [`PASSO_PEQUENO`].
        StepUpSmall,
        /// `alt`+`↓` — subtrai um [`PASSO_PEQUENO`].
        StepDownSmall,
        /// `shift`+`↑` — soma um [`PASSO_GRANDE`].
        StepUpLarge,
        /// `shift`+`↓` — subtrai um [`PASSO_GRANDE`].
        StepDownLarge,
        /// `Home` — vai pro mínimo, se ele existir.
        JumpToMin,
        /// `End` — vai pro máximo, se ele existir.
        JumpToMax,
    ]
);

/// O contexto de teclas que o wrapper deste componente empilha.
pub const KEY_CONTEXT: &str = "NumberField";

/// O predicado dos bindings de [`init`] — **e o `> Input` não é decoração**.
///
/// Um predicado `"NumberField"` casaria no nó do WRAPPER, que é mais raso que o nó do campo; como o
/// `gpui::Keymap` ordena os bindings casados por profundidade decrescente antes de olhar a ordem de
/// registro, o `up`→`MoveUp` do núcleo (predicado `"Input"`, que casa no nó do campo) venceria e as
/// setas voltariam a mover o cursor. Com `"NumberField > Input"` o predicado casa no MESMO nó — a
/// profundidade empata, o desempate é a ordem de registro, e [`init`] roda depois do
/// `gpui_component::init`. Ver a seção de teclado no doc do módulo.
pub const PREDICADO: &str = "NumberField > Input";

/// Vincula as teclas de passo. Chamado por [`crate::input::init`], então quem já inicializava o
/// crate ganha isto sem mudar nada — o mesmo caminho do [`crate::focus_ring::init`].
///
/// ⚠️ Precisa rodar **depois** do `gpui_component::init`, que é o que o doc de
/// [`crate::input::init`] já manda fazer. A ordem é o que decide a precedência (ver [`PREDICADO`]).
pub fn init(cx: &mut App) {
    use gpui::KeyBinding;
    cx.bind_keys([
        KeyBinding::new("up", StepUp, Some(PREDICADO)),
        KeyBinding::new("down", StepDown, Some(PREDICADO)),
        KeyBinding::new("alt-up", StepUpSmall, Some(PREDICADO)),
        KeyBinding::new("alt-down", StepDownSmall, Some(PREDICADO)),
        KeyBinding::new("shift-up", StepUpLarge, Some(PREDICADO)),
        KeyBinding::new("shift-down", StepDownLarge, Some(PREDICADO)),
        KeyBinding::new("home", JumpToMin, Some(PREDICADO)),
        KeyBinding::new("end", JumpToMax, Some(PREDICADO)),
    ]);
}

// =================================================================================================
// Paleta — UMA cor, e o porquê
// =================================================================================================
//
// A superfície inteira é do `crate::input` (ver "Por que isto NÃO é um InputGroup" no doc do módulo),
// e a tinta dos glifos é o `--foreground` do próprio campo. Sobra **uma** cor que só existe neste
// componente: o `hover:bg-accent` dos botões de passo. Ela fica numa paleta por tema, como as outras
// 17 do crate, porque a paleta de componente é o registro do que a referência pediu — não um cache
// do tema.

/// A única cor própria deste componente.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NumberFieldPalette {
    /// `--accent` — o fundo do botão de passo sob o mouse (`hover:bg-accent`).
    ///
    /// No coss `--accent` e `--secondary` resolvem pro MESMO valor (preto/branco a 4%), que é o que o
    /// [`crate::button`] já registra no campo `secondary` dele. Aqui está escrito de novo, e não lido
    /// de lá, porque o campo do botão é privado e porque é o token `--accent` que a referência cita
    /// neste componente: registrar o token pedido é o que permite descobrir, mais tarde, que os dois
    /// desencostaram no design system.
    accent: Rgba8,
}

/// Tema **claro** — `--accent` = preto a 4%.
const NF_CLARO: NumberFieldPalette = NumberFieldPalette {
    accent: Rgba8(0x0000000a),
};

/// Tema **escuro** — `--accent` = branco a 4%.
const NF_ESCURO: NumberFieldPalette = NumberFieldPalette {
    accent: Rgba8(0xffffff0a),
};

/// A paleta no tema corrente.
fn palette() -> &'static NumberFieldPalette {
    match theme::theme_mode() {
        theme::ThemeMode::Dark => &NF_ESCURO,
        theme::ThemeMode::Light => &NF_CLARO,
    }
}

// =================================================================================================
// Geometria (utilitários do coss resolvidos)
// =================================================================================================

/// Lado do glifo de passo — `sm:[&_svg]:size-4`. A classe base `size-4.5` (18) é o ramo `max-sm:`.
const LADO_DO_GLIFO: f32 = 16.0;

/// O `-1px` do `rounded-s-[calc(var(--radius-lg)-1px)]`: o raio do botão é o da moldura menos a
/// borda, pra a curva dele nascer exatamente na curva INTERNA dela.
const RAIO_DESCONTO: f32 = 1.0;

/// `sm:text-sm` — o corpo do texto do grupo (a classe base `text-base`, 16, é o ramo `max-sm:`).
const CORPO_DO_TEXTO: f32 = 14.0;

/// A entrelinha que **acompanha** `text-sm` no Tailwind: 14/20.
///
/// ⚠️ Tem que ser declarada. O default do GPUI é `relative(1.618_034)`, que num corpo de 14 dá 22,65.
const ENTRELINHA: f32 = 20.0;

/// `data-disabled:opacity-64` — o coss esmaece o CONJUNTO em vez de trocar cor por cor.
const OPACIDADE_DESABILITADO: f32 = 0.64;

/// Glifo do botão de menos. Iconoir, traço 1,5 num `viewBox` 24 → **1,0px** de tinta a 16px.
const GLIFO_MENOS: &str = "iconoir/regular/minus.svg";

/// Glifo do botão de mais — o par do [`GLIFO_MENOS`], mesmo `viewBox` e mesmo traço.
const GLIFO_MAIS: &str = "iconoir/regular/plus.svg";

/// A tinta dos glifos de passo: o `--foreground` do PRÓPRIO campo (`text-foreground` no grupo).
///
/// É função, e não `input::field().text` escrito dentro do render, pelo mesmo motivo do
/// [`crate::input::ring_visible`]: é uma DECISÃO — "o `+` lê igual ao número ao lado" — e decisão
/// desta base se testa sem GPU. Inline no render, trocá-la pelo véu de hover (o erro plausível, que
/// deixaria o glifo invisível sobre o realce) não quebraria nada que compile.
fn tinta_do_glifo() -> Rgba8 {
    input::field().text
}

// =================================================================================================
// A matemática — pura, livre de `Window`/`Context`, transcrita do primitivo
// =================================================================================================

/// O `step = 1` default do `NumberField.Root`.
pub const PASSO: f32 = 1.0;

/// O `smallStep = 0.1` default (o passo do `alt`).
pub const PASSO_PEQUENO: f32 = 0.1;

/// O `largeStep = 10` default (o passo do `shift`).
pub const PASSO_GRANDE: f32 = 10.0;

/// O `Number.MAX_SAFE_INTEGER` do JS (2⁵³ − 1) — o "sem limite" do primitivo, que usa
/// `min ?? Number.MIN_SAFE_INTEGER` e `max ?? Number.MAX_SAFE_INTEGER`.
///
/// Em `f32` o valor arredonda pro par seguinte (2⁵³). Irrelevante: é um limite que existe pra não
/// existir, e nenhuma conta deste componente depende do último bit dele.
pub const SAFE_INTEGER: f32 = 9_007_199_254_740_991.0;

/// A tolerância do snap, **em ULPs de `f32`** — o `STEP_EPSILON_FACTOR` do primitivo reescalado.
///
/// Lá é `1e-10`, uma folga relativa ao passo que impede o `floor` de cair um passo atrás quando
/// `valor − base` já está um pouco abaixo de um múltiplo exato. Transcrito ao pé da letra, aqui ele
/// seria **constante morta**: `1.0f32 + 1e-10 == 1.0f32`, porque um ULP de `f32` em torno de 1,0 é
/// 1,19e-7 — ~800× MAIOR (travado em `a_folga_do_primitivo_e_morta_em_f32`).
///
/// O papel dele, porém, não é morto: em `f32` o erro que ele existe pra absorver é maior, não menor.
/// Então o que se reescala é a MAGNITUDE, escrita na unidade que `f32` tem: **8 ULPs**, folga
/// suficiente pra somar e dividir alguns passos e ainda 4 ordens de grandeza menor que o próprio
/// passo (8 × 1,19e-7 ≈ 9,5e-7).
const TOLERANCIA_DO_SNAP: f32 = 8.0 * f32::EPSILON;

/// As regras numéricas do campo — as props do `NumberField.Root` que entram na conta.
///
/// `min`/`max` são `Option` de propósito, e não um número com sentinela: o primitivo distingue "sem
/// mínimo" de "mínimo = MIN_SAFE_INTEGER" em DOIS lugares — a base do snap
/// (`minWithDefault === Number.MIN_SAFE_INTEGER ? minWithZeroDefault : minWithDefault`) e o
/// `Home`/`End`, que só pulam quando o limite foi declarado. Com sentinela, as duas leituras seriam
/// comparações contra uma constante mágica.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rules {
    min: Option<f32>,
    max: Option<f32>,
    step: f32,
    small_step: f32,
    large_step: f32,
    snap_on_step: bool,
    allow_out_of_range: bool,
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            min: None,
            max: None,
            step: PASSO,
            small_step: PASSO_PEQUENO,
            large_step: PASSO_GRANDE,
            snap_on_step: false,
            allow_out_of_range: false,
        }
    }
}

impl Rules {
    /// `min ?? Number.MIN_SAFE_INTEGER`.
    fn min_with_default(&self) -> f32 {
        self.min.unwrap_or(-SAFE_INTEGER)
    }

    /// `max ?? Number.MAX_SAFE_INTEGER`.
    fn max_with_default(&self) -> f32 {
        self.max.unwrap_or(SAFE_INTEGER)
    }

    /// `min ?? 0` — a base do snap quando não há mínimo declarado (ou quando o passo é o do `alt`).
    fn min_with_zero_default(&self) -> f32 {
        self.min.unwrap_or(0.0)
    }

    /// O mínimo declarado, ou `None` quando não há — e é o `None` que faz o `Home` não pular.
    pub fn min(&self) -> Option<f32> {
        self.min
    }

    /// O máximo declarado, ou `None`. Idem pro `End`.
    pub fn max(&self) -> Option<f32> {
        self.max
    }

    /// O passo das setas sem modificador.
    pub fn step(&self) -> f32 {
        self.step
    }

    /// O `getStepAmount(event)` do primitivo: `alt` vence `shift`, e sem modificador é o passo.
    ///
    /// A ordem é a de lá (`if (event?.altKey) … if (event?.shiftKey) … return step`) e importa:
    /// `alt+shift+↑` anda o passo PEQUENO.
    pub fn step_amount(&self, small: bool, large: bool) -> f32 {
        if small {
            self.small_step
        } else if large {
            self.large_step
        } else {
            self.step
        }
    }
}

/// O `snapToStep` do primitivo: alinha `value` à grade de `step` ancorada em `base`.
///
/// `nearest` (o ramo do `alt`) arredonda pro múltiplo mais PRÓXIMO; sem ele o snap é **direcional** —
/// desce (`floor`) quando o passo é positivo e sobe (`ceil`) quando é negativo — pra que um `↑` nunca
/// volte pra trás só por causa do alinhamento.
///
/// Só é chamado com `step != 0` (é o guarda do `to_validated`), o que é o que torna o `signum`
/// seguro aqui.
fn snap_to_step(value: f32, base: f32, step: f32, nearest: bool) -> f32 {
    let step_size = step.abs();
    let direction = step.signum();
    let tolerance = step_size * TOLERANCIA_DO_SNAP * direction;
    let raw_steps = value - base + tolerance;

    if nearest {
        return base + (raw_steps / step).round() * step;
    }

    let snapped_steps = if direction > 0.0 {
        (raw_steps / step_size).floor()
    } else {
        (raw_steps / step_size).ceil()
    };
    base + snapped_steps * step_size
}

/// O `toValidatedNumber` do primitivo: snap (se pedido) e clamp (se a interação clampa).
///
/// `step` é `Some` só quando a mudança tem DIREÇÃO (um passo); é ele que o snap usa. Digitar, colar e
/// sincronizar de fora passam `None` — sem direção não há snap direcional a aplicar, que é
/// exatamente o que o primitivo faz ao chamar `setValue` sem `direction`.
///
/// O `removeFloatingPointErrors` e o segundo clamp da referência **não** estão aqui: em `f32` o
/// primeiro é a identidade (ver "Não reproduzível" no doc do módulo) e, sendo identidade, o segundo
/// clamp reclampa um número que já está dentro da faixa.
///
/// ⚠️ O guarda de finitude é NOSSO e é obrigatório: `f32::min` devolve o outro operando quando um é
/// `NaN`, então um `NaN` sairia do [`crate::meter::clamp_js`] como `max` — silencioso e errado. Ver
/// o aviso no doc do módulo.
fn to_validated(
    value: Option<f32>,
    step: Option<f32>,
    r: &Rules,
    small: bool,
    should_clamp: bool,
) -> Option<f32> {
    let v = value?;
    if !v.is_finite() {
        return None;
    }
    let mut next = v;

    if let Some(st) = step {
        if r.snap_on_step && st != 0.0 {
            // ⚠️ **A base é `min ?? 0`, e o `small` do primitivo não muda isso.**
            //
            // Lá a condição é
            // `small || minWithDefault === MIN_SAFE_INTEGER ? minWithZeroDefault : minWithDefault`.
            // Ela colapsa: se `min` foi declarado, `minWithZeroDefault` **é** `minWithDefault` e os
            // dois ramos dão o mesmo número; se não foi, o segundo termo do `||` já é verdadeiro
            // sozinho. Ou seja o `small ||` é um operando inerte — o resultado é sempre
            // `min ?? 0`. Escrever o `if` aqui seria um ramo morto (travado em
            // `a_base_do_snap_e_sempre_o_minimo_ou_zero`).
            //
            // O que `small` de fato decide é o `nearest` abaixo: o passo do `alt` alinha pro
            // múltiplo mais PRÓXIMO, o normal alinha direcionalmente.
            next = snap_to_step(next, r.min_with_zero_default(), st, small);
        }
    }

    if should_clamp {
        next = crate::meter::clamp_js(next, r.min_with_default(), r.max_with_default());
    }
    Some(next)
}

/// O `incrementValue` do primitivo: um passo a partir do valor corrente.
///
/// Campo vazio semeia **0** — sem direção, portanto sem snap direcional, e clampado (num campo
/// `[-50, -10]` o zero vira `-10`, "o valor em faixa mais próximo de zero"). É o comentário literal
/// de lá.
fn stepped(current: Option<f32>, amount: f32, direction: f32, r: &Rules, small: bool) -> Option<f32> {
    match current {
        None => to_validated(Some(0.0), None, r, false, true),
        Some(v) => to_validated(
            Some(v + amount * direction),
            Some(amount * direction),
            r,
            small,
            // Passo SEMPRE clampa, mesmo com `allow_out_of_range`: lá o
            // `shouldClampValue = !allowOutOfRange || !isInputReason` e um passo não é `input-*`.
            true,
        ),
    }
}

/// O `isAtBoundary` do `useNumberFieldStepperButton`: se o botão daquele lado não tem mais o que
/// fazer. Campo vazio nunca está no limite — há sempre a semeadura do zero.
fn at_boundary(value: Option<f32>, r: &Rules, is_increment: bool) -> bool {
    match value {
        None => false,
        Some(v) => {
            if is_increment {
                v >= r.max_with_default()
            } else {
                v <= r.min_with_default()
            }
        }
    }
}

/// O que uma leitura do texto do campo produz.
///
/// É um enum, e não `Option<Option<f32>>`, porque os três casos levam a caminhos diferentes e um
/// aninhamento de `Option` não diz qual é qual: vazio **zera** o valor, ilegível o **preserva**.
#[derive(Clone, Copy, Debug, PartialEq)]
enum TextRead {
    /// Campo vazio (ou só espaço) → valor `None`.
    Empty,
    /// Um número, já validado pelas regras.
    Number(f32),
    /// Texto que não é número → o valor não se mexe.
    Unreadable,
}

/// Interpreta o texto do campo pelas regras dadas.
///
/// O clamp aqui é o de **entrada de texto**, então é ele — e só ele — que o
/// [`Rules::allow_out_of_range`] desliga.
fn read_text(text: &str, r: &Rules) -> TextRead {
    if text.trim().is_empty() {
        return TextRead::Empty;
    }
    match crate::scrub_input::parse_finite(text) {
        None => TextRead::Unreadable,
        Some(p) => match to_validated(Some(p), None, r, false, !r.allow_out_of_range) {
            Some(v) => TextRead::Number(v),
            // Inalcançável por este caminho (o `parse_finite` já recusou o não-finito), e o `match`
            // existe pra não haver `expect` num caminho de UI.
            None => TextRead::Unreadable,
        },
    }
}

/// O texto que o campo mostra: vazio quando o valor é `None`, senão o valor formatado pelo passo.
///
/// A formatação é a do [`crate::scrub_input::format_value`] — casas decimais derivadas do passo. Não
/// há uma segunda gramática de número nesta casa (ver "Ausente" pro `Intl.NumberFormat`).
fn display_text(value: Option<f32>, step: f32) -> String {
    match value {
        None => String::new(),
        Some(v) => crate::scrub_input::format_value(v, step),
    }
}

// =================================================================================================
// O componente
// =================================================================================================

/// O que o [`NumberField`] emite. Os dois braços são as duas props da referência.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NumberFieldEvent {
    /// O valor mudou — a cada tecla parseável e a cada passo. É o `onValueChange`.
    Change(Option<f32>),
    /// O valor foi **confirmado**: no blur e no fim de um passo. É o `onValueCommitted`.
    ///
    /// Todo `Commit` é precedido do `Change` que o produziu.
    Commit(Option<f32>),
}

/// Campo numérico com botões de passo. Ver o doc do módulo.
///
/// É uma **entidade** (não um elemento de render), como o [`crate::ScrubInput`]: o valor e o estado
/// do texto vivem aqui, e o componente **emite** [`NumberFieldEvent`] em vez de receber callback de
/// escrita.
pub struct NumberField {
    /// O estado do campo de texto — o "digitar" de verdade (IME, seleção, undo).
    input: Entity<InputState>,
    /// O valor corrente. `None` = campo vazio, que é um estado da referência (`value: number | null`).
    value: Option<f32>,
    rules: Rules,
    size: InputSize,
    disabled: bool,
    invalid: bool,
    /// Espelho do `allowInputSyncRef` do primitivo: `true` enquanto há digitação **não normalizada**.
    ///
    /// Enquanto ele está ligado, [`Self::sincronizar_texto`] não reescreve o texto — senão o campo
    /// reformataria embaixo do cursor a cada tecla (digitar `0.` viraria `0` e o ponto sumiria).
    texto_sujo: bool,
    _subscriptions: Vec<Subscription>,
}

impl NumberField {
    /// Cria o campo. `value` é o valor inicial (`None` = vazio) e é validado pelas regras default;
    /// os builders que mudam as regras revalidam.
    pub fn new(value: Option<f32>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let rules = Rules::default();
        let value = to_validated(value, None, &rules, false, true);
        let input =
            cx.new(|cx| single_line(window, cx).default_value(display_text(value, rules.step)));

        let sub = cx.subscribe(&input, |this, _state, event: &InputEvent, cx| match event {
            InputEvent::Change => this.ao_digitar(cx),
            InputEvent::Blur => this.ao_desfocar(cx),
            // Ganhou/perdeu foco → re-renderiza pro anel acender e apagar.
            InputEvent::Focus => cx.notify(),
            // `PressEnter` de propósito não é tratado — ver "Desvio consciente" no doc do módulo.
            _ => {}
        });

        Self {
            input,
            value,
            rules,
            size: InputSize::default(),
            disabled: false,
            invalid: false,
            texto_sujo: false,
            _subscriptions: vec![sub],
        }
    }

    // --- Builders (as props do `Root`) ----------------------------------------------------------

    /// Tamanho do controle — o `size` do `NumberField` raiz (`"sm" | "default" | "lg"`).
    pub fn size(mut self, size: InputSize) -> Self {
        self.size = size;
        self
    }

    /// Mínimo. Sem ele o campo não tem piso **e** o `Home` deixa de pular (é o
    /// `min != null` do primitivo).
    pub fn min(mut self, min: f32) -> Self {
        self.rules.min = Some(min);
        self.revalidar();
        self
    }

    /// Máximo. Sem ele o campo não tem teto **e** o `End` deixa de pular.
    pub fn max(mut self, max: f32) -> Self {
        self.rules.max = Some(max);
        self.revalidar();
        self
    }

    /// Passo das setas sem modificador (default [`PASSO`]).
    ///
    /// O `step: 'any'` da referência (que ela mesma resolve pra `1`) não tem representação aqui —
    /// passe `1.0`.
    pub fn step(mut self, step: f32) -> Self {
        self.rules.step = step;
        self.revalidar();
        self
    }

    /// Passo do `alt` (default [`PASSO_PEQUENO`]).
    pub fn small_step(mut self, step: f32) -> Self {
        self.rules.small_step = step;
        self
    }

    /// Passo do `shift` (default [`PASSO_GRANDE`]).
    pub fn large_step(mut self, step: f32) -> Self {
        self.rules.large_step = step;
        self
    }

    /// Liga o `snapOnStep`: um passo **alinha** o valor à grade do passo antes de clampar.
    /// Desligado por default, como na referência.
    pub fn snap_on_step(mut self) -> Self {
        self.rules.snap_on_step = true;
        self.revalidar();
        self
    }

    /// Liga o `allowOutOfRange`: **digitação** pode sair da faixa (passo e botão continuam
    /// clampando).
    pub fn allow_out_of_range(mut self) -> Self {
        self.rules.allow_out_of_range = true;
        self
    }

    /// Desabilita o conjunto: o campo não edita, os botões não agem e a moldura toda esmaece.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Marca a moldura como **inválida** (`aria-invalid`): a borda vira `--destructive` e o anel vem
    /// na cor de erro.
    ///
    /// É um `bool` e não um [`crate::Validity`] pelo mesmo motivo do
    /// [`crate::input_group::InputGroup::invalid`]: este componente é a superfície, e a MENSAGEM de
    /// erro pertence a uma moldura de formulário que fica fora dele.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    // --- Leitura e escrita --------------------------------------------------------------------

    /// O valor corrente, já validado. `None` = campo vazio.
    pub fn value(&self) -> Option<f32> {
        self.value
    }

    /// As regras em vigor — a leitura que quem escreve a própria legenda ("42 de 99") precisa, com o
    /// MESMO número que o campo usa. Mesma decisão do [`crate::Meter`].
    pub fn rules(&self) -> Rules {
        self.rules
    }

    /// Define o valor de fora: valida, e o texto acompanha no render seguinte.
    ///
    /// **Não** emite evento — é sincronização externa, não edição do usuário (evita loop com quem
    /// assina). Mesma decisão do [`crate::ScrubInput::set_value`].
    pub fn set_value(&mut self, value: Option<f32>, cx: &mut Context<Self>) {
        let v = to_validated(value, None, &self.rules, false, !self.rules.allow_out_of_range);
        if v == self.value {
            return;
        }
        self.value = v;
        // Um valor vindo de fora vence a digitação em curso: é o
        // `syncFormattedInputValueOnValueChange` da referência, que reescreve o texto quando o
        // `value` muda por fora.
        self.texto_sujo = false;
        cx.notify();
    }

    /// Marca desabilitado por render (o par mutável do builder [`Self::disabled`]).
    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        if self.disabled == disabled {
            return;
        }
        self.disabled = disabled;
        cx.notify();
    }

    /// Marca inválido por render (o par mutável do builder [`Self::invalid`]).
    pub fn set_invalid(&mut self, invalid: bool, cx: &mut Context<Self>) {
        if self.invalid == invalid {
            return;
        }
        self.invalid = invalid;
        cx.notify();
    }

    // --- Mecânica ------------------------------------------------------------------------------

    /// Reaplica as regras ao valor corrente. Chamado pelos builders que mudam limites ou snap: sem
    /// isto, `NumberField::new(Some(500.0), …).max(100.0)` guardaria 500.
    fn revalidar(&mut self) {
        self.value = to_validated(
            self.value,
            None,
            &self.rules,
            false,
            !self.rules.allow_out_of_range,
        );
    }

    /// O `syncFormattedInputValueOnValueChange` do primitivo: reescreve o texto quando o VALOR e o
    /// texto discordam, e **nunca** enquanto há digitação não normalizada.
    ///
    /// # Por que no render, e não no `set_value`
    ///
    /// Porque a ordem dos builders não pode importar: `new(Some(500.0), …).max(100.0)` precisa
    /// mostrar `100`, e no `new` o `max` ainda não existia. Adiar a escrita pro render resolve isso
    /// pelo mesmo mecanismo que a referência usa (lá é um `useIsoLayoutEffect`), e de graça também
    /// tira o `&mut Window` da assinatura de [`Self::set_value`].
    ///
    /// É o mesmo padrão (e o mesmo porquê de estar no render) do
    /// `ScrubInput::selecionar_ao_focar`.
    fn sincronizar_texto(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.texto_sujo {
            return;
        }
        let alvo = display_text(self.value, self.rules.step);
        if self.input.read(cx).value().as_ref() == alvo.as_str() {
            return;
        }
        self.input.update(cx, |st, cx| {
            st.set_value(alvo, window, cx);
        });
    }

    /// O `onChange` do `NumberFieldInput`: cada tecla parseável já muda o valor.
    fn ao_digitar(&mut self, cx: &mut Context<Self>) {
        let texto = self.input.read(cx).value();
        // **Eco do nosso próprio `set_value`.** O núcleo emite `Change` também quando o texto é
        // escrito programaticamente, e a entrega de assinatura é assíncrona (flush de efeitos,
        // depois do render) — um flag de reentrância não a alcança. O guarda é por identidade de
        // texto; a consequência declarada está em "Desvio consciente" no doc do módulo.
        if texto.as_ref() == display_text(self.value, self.rules.step) {
            self.texto_sujo = false;
            return;
        }
        self.texto_sujo = true;
        let novo = match read_text(&texto, &self.rules) {
            TextRead::Empty => None,
            TextRead::Number(v) => Some(v),
            // O primitivo devolve sem tocar no valor; o texto continua como o usuário digitou.
            TextRead::Unreadable => return,
        };
        self.value = novo;
        cx.emit(NumberFieldEvent::Change(novo));
        cx.notify();
    }

    /// O `onBlur` do `NumberFieldInput`: normaliza o texto e **confirma**.
    fn ao_desfocar(&mut self, cx: &mut Context<Self>) {
        let houve_digitacao = self.texto_sujo;
        // Limpar aqui é o que libera [`Self::sincronizar_texto`] a reescrever o texto canônico no
        // render seguinte — inclusive quando o texto é ilegível, que é como o valor antigo volta.
        self.texto_sujo = false;
        let texto = self.input.read(cx).value();
        match read_text(&texto, &self.rules) {
            TextRead::Empty => {
                let mudou = self.value.is_some();
                self.value = None;
                if mudou || houve_digitacao {
                    cx.emit(NumberFieldEvent::Commit(None));
                }
            }
            TextRead::Number(v) => {
                let mudou = self.value != Some(v);
                self.value = Some(v);
                if mudou || houve_digitacao {
                    cx.emit(NumberFieldEvent::Commit(Some(v)));
                }
            }
            TextRead::Unreadable => {}
        }
        cx.notify();
    }

    /// Um passo. `direcao` é `+1`/`-1`; `small`/`large` são os modificadores (ver
    /// [`Rules::step_amount`]).
    ///
    /// Emite `Change` **e** `Commit`: na referência um passo de teclado/roda commita na hora
    /// (`if (changed) onValueCommitted(...)`), e o clique no botão commita no `onStop`/`onClick`.
    fn passo(&mut self, direcao: f32, small: bool, large: bool, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let amount = self.rules.step_amount(small, large);
        let novo = stepped(self.value, amount, direcao, &self.rules, small);
        // Um passo normaliza o texto (é o `allowInputSyncRef.current = true` do primitivo).
        self.texto_sujo = false;
        if novo == self.value {
            // Nada mudou (limite, ou passo zero): a referência não emite nem commita.
            cx.notify();
            return;
        }
        self.value = novo;
        cx.emit(NumberFieldEvent::Change(novo));
        cx.emit(NumberFieldEvent::Commit(novo));
        cx.notify();
    }

    /// `Home`/`End`: vai direto pro limite. Só é chamado quando o limite existe.
    fn ir_para(&mut self, alvo: f32, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let novo = to_validated(Some(alvo), None, &self.rules, false, true);
        self.texto_sujo = false;
        if novo == self.value {
            cx.notify();
            return;
        }
        self.value = novo;
        cx.emit(NumberFieldEvent::Change(novo));
        cx.emit(NumberFieldEvent::Commit(novo));
        cx.notify();
    }

    /// Um botão de passo: um segmento de borda a borda, com o respiro do campo, os DOIS cantos de
    /// fora arredondados e o realce de hover.
    ///
    /// Não é um [`crate::Button`], e nem poderia ser: o botão da casa traz respiro, raio nos quatro
    /// cantos, anel de foco e uma parada de `Tab` próprios — e aqui os botões são `tabIndex={-1}`,
    /// têm a altura do MIOLO (26/30/34, que não é nenhum [`crate::ButtonSize`]) e arredondam só de um
    /// lado. É a mesma razão pela qual o olho do campo de senha não é um `Button` (ver
    /// [`crate::Input::icon_button`]).
    fn stepper(&self, is_increment: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let s = self.size;
        let no_limite = at_boundary(self.value, &self.rules, is_increment);
        let raio = px(input::FIELD_RADIUS - RAIO_DESCONTO);
        let accent = palette().accent;
        let glifo: SharedString = if is_increment { GLIFO_MAIS } else { GLIFO_MENOS }.into();

        let mut d = div()
            .id(if is_increment { "increment" } else { "decrement" })
            // Sem `occlude` o miolo de texto embaixo também responderia ao mouse.
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .flex_none()
            // ⚠️ **Sem `h_full()` aqui, e a ausência é o conserto.** A referência escreve `h-full` no
            // stepper, e no navegador funciona porque o pai tem altura definida. Aqui a moldura NÃO
            // declara altura (a dela vem do miolo hospedado), então `height: 100%` cai pra `auto` — o
            // botão saía do tamanho do glifo, com folga visível em cima e embaixo no realce de hover.
            //
            // Quem estica é o `align_items: Stretch` da moldura. E `stretch` só age quando o tamanho
            // transversal é `auto`: um `h_full()` aqui, mesmo "certo" na leitura da referência, VENCERIA
            // o esticamento e traria o defeito de volta. Medido: com `h_full` o botão respondia a
            // clique de y=1 a 17 numa moldura de 32; sem ele, de 1 a 31.
            .px(px(s.pad_x()))
            // `cursor-pointer` é incondicional na referência, e o realce também: no limite o botão
            // para de AGIR, não de parecer clicável. Ver "Desvio consciente" no doc do módulo.
            .cursor(CursorStyle::PointingHand)
            .hover(move |h| h.bg(accent.hsla()))
            .child(
                // O `gpui::svg()` pinta com a `text_color` do PRÓPRIO elemento e não herda a do pai
                // (se for `None`, não desenha nada e não loga erro) — a armadilha documentada no
                // `scrub_input`. Por isso a cor vai direto no `svg()`, com `.flex_none()`+`.size()`.
                svg()
                    .path(glifo)
                    .size(px(LADO_DO_GLIFO))
                    .flex_none()
                    .text_color(tinta_do_glifo().hsla()),
            );

        // `rounded-s-*` no botão de menos, `rounded-e-*` no de mais: só os cantos que encostam na
        // moldura.
        d = if is_increment {
            d.rounded_tr(raio).rounded_br(raio)
        } else {
            d.rounded_tl(raio).rounded_bl(raio)
        };

        d.on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _e: &MouseDownEvent, window, cx| {
                // `stop_propagation` pra apertar o botão não posicionar o cursor de texto embaixo
                // dele; o foco vai pro campo à mão, que é o
                // `inputRef.current?.focus()` do `useNumberFieldStepperButton` ("so the user can
                // continue with keyboard interactions").
                cx.stop_propagation();
                gpui::Focusable::focus_handle(this.input.read(cx), cx).focus(window);
            }),
        )
        .when(!self.disabled && !no_limite, |d| {
            d.on_click(cx.listener(move |this, e: &ClickEvent, _window, cx| {
                let m = e.modifiers();
                this.passo(
                    if is_increment { 1.0 } else { -1.0 },
                    m.alt,
                    m.shift,
                    cx,
                );
            }))
        })
    }
}

impl Focusable for NumberField {
    /// O handle é o do CAMPO — os botões não recebem foco, e o controle é uma parada de `Tab`.
    ///
    /// ⚠️ `gpui::Focusable::focus_handle(self.input.read(cx), cx)` e não `self.input.focus_handle(cx)`:
    /// a segunda forma compila e devolve outro handle. É a armadilha documentada no
    /// [`crate::scrub_input`].
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        gpui::Focusable::focus_handle(self.input.read(cx), cx)
    }
}

impl EventEmitter<NumberFieldEvent> for NumberField {}

impl Render for NumberField {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // O texto acompanha o valor. No render de propósito — ver o doc do método.
        self.sincronizar_texto(window, cx);

        let s = self.size;
        let handle = gpui::Focusable::focus_handle(self.input.read(cx), cx);
        let focado = handle.is_focused(window);
        let fp = input::field();

        // --- A moldura: TODA de `crate::input` ------------------------------------------------
        //
        // Nem uma cor nem um raio próprios aqui (ver "Por que isto NÃO é um InputGroup"). Note duas
        // ausências, as duas fiéis:
        //
        // - **sem `overflow_hidden`**: a referência não o tem, e o do GPUI é ContentMask RETANGULAR —
        //   não recortaria pelo raio de todo jeito. Quem encaixa o realce do botão na curva é o raio
        //   do próprio botão (`FIELD_RADIUS - RAIO_DESCONTO`);
        // - **sem altura declarada**: o `NumberFieldGroup` do coss também não declara `h-*`. A altura
        //   vem do campo hospedado (o `sm:h-*` do `<input>`, que é o `InputSize::content_height`) mais
        //   as duas bordas de 1px daqui — 28/32/36. Um `.h(s.height())` aqui era o que este render
        //   tinha, e era **estilo morto**: a campanha de mutação mostrou que removê-lo não mudava um
        //   pixel medido, porque a altura já vinha do conteúdo. Quem mede os três tamanhos é
        //   `tests_de_janela`.
        let mut frame = div()
            .flex()
            // ⚠️ **`items_stretch`, e NÃO `items_center`.** Os botões de passo pedem `h_full`, e a
            // moldura não declara altura (ver o parágrafo acima — a altura vem do miolo hospedado).
            // Com `items_center` o filho não estica, e `height: 100%` contra altura INDEFINIDA cai pra
            // `auto`: o botão ficava do tamanho do glifo, com folga visível em cima e embaixo. Esticando,
            // ele ocupa a altura inteira do miolo, que é o que o `h-full` do `NumberFieldStepper` da
            // referência faz. É o mesmo mecanismo do `w_full` que quebrou a largura da [`crate::table`].
            //
            // O glifo continua centrado porque quem centra é o PRÓPRIO botão
            // (`items_center().justify_center()` em [`Self::stepper`]), não a moldura.
            //
            // ⚠️ **E o esticamento tem que ser PEDIDO, não herdado.** No CSS `align-items` já é
            // `stretch` por padrão, e é por isso que a referência não escreve nada; no GPUI o padrão é
            // `FlexStart`, e o `Styled` só expõe `items_start/end/center/baseline` — não há
            // `items_stretch()`. Medido: sem pedir, o botão respondia de y=1 a 17 numa moldura de 32,
            // ancorado no topo. Daí o `align_items` na mão, logo abaixo.
            .w_full()
            .bg(fp.bg.hsla())
            .border_1()
            .border_color(input::border_color_for(self.invalid, focado).hsla())
            .rounded(px(input::FIELD_RADIUS))
            .text_size(px(CORPO_DO_TEXTO))
            // ⚠️ Sem isto o GPUI usaria `relative(1.618_034)` — ver [`ENTRELINHA`].
            .line_height(px(ENTRELINHA));

        // `tabular-nums` na MOLDURA: o miolo do núcleo pinta o texto com `window.text_style()`, ou
        // seja com o estilo HERDADO da árvore — um `.text_color()`/`.font()` aplicado ao elemento do
        // campo não o alcançaria. Reusa a feature do `crate::meter` (mesma classe na referência).
        frame
            .text_style()
            .get_or_insert_with(Default::default)
            .font_features = Some(crate::meter::tabular_nums());

        // O esticamento pedido à mão — ver o ⚠️ acima. É o que faz o `h_full` dos botões de passo
        // valer contra a altura do miolo em vez de virar `auto`.
        frame.style().align_items = Some(gpui::AlignItems::Stretch);

        let frame = frame
            .child(self.stepper(false, cx))
            .child(
                // O campo hospedado, sem moldura: a superfície é desta função. O respiro (9/11) e a
                // altura do miolo (26/30/34) são os do `InputSize`, que SÃO os do `number-field.tsx`
                // — nenhuma sobrescrita de métrica.
                div().flex_1().min_w(px(0.0)).h_full().child(
                    Input::new(&self.input)
                        .unstyled()
                        .size(s)
                        .disabled(self.disabled),
                ),
            )
            .child(self.stepper(true, cx))
            .shadow(input::shadow_stack_for(
                self.disabled,
                self.invalid,
                0.0,
                if focado { 0.0 } else { 1.0 },
            ));

        // --- Bisel e anel, IRMÃOS da moldura ---------------------------------------------------
        //
        // Irmãos e não filhos pelo mesmo motivo do campo solto e do grupo: o bisel cai SOBRE a borda
        // e o anel cresce 3px pra fora, e um filho de moldura arredondada perde os dois nas quinas.
        let transicao = input::focus_transition(self.input.entity_id(), focado);
        let mostra_anel = input::ring_visible(self.disabled, focado, transicao == Some(false));

        let mut wrap = div()
            // O contexto de teclas deste controle. É ele que o predicado dos bindings de [`init`]
            // procura — sem ele as setas voltam a mover o cursor.
            .key_context(KEY_CONTEXT)
            .relative()
            .w_full()
            // `data-disabled:opacity-64`. O campo hospedado não esmaece a si mesmo (é o que
            // `Input::unstyled` desliga), então não há multiplicação de opacidade.
            .when(self.disabled, |d| d.opacity(OPACIDADE_DESABILITADO))
            // Clicar fora tira o foco. O campo hospedado não faz isso (é `unstyled`), porque "fora
            // do campo" não é "fora do controle" — o clique pode cair num botão de passo.
            .on_mouse_down_out(input::blur_on_outside_click(handle.clone()))
            .on_action(cx.listener(|this, _: &StepUp, _w, cx| this.passo(1.0, false, false, cx)))
            .on_action(cx.listener(|this, _: &StepDown, _w, cx| this.passo(-1.0, false, false, cx)))
            .on_action(cx.listener(|this, _: &StepUpSmall, _w, cx| this.passo(1.0, true, false, cx)))
            .on_action(cx.listener(|this, _: &StepDownSmall, _w, cx| {
                this.passo(-1.0, true, false, cx)
            }))
            .on_action(cx.listener(|this, _: &StepUpLarge, _w, cx| this.passo(1.0, false, true, cx)))
            .on_action(cx.listener(|this, _: &StepDownLarge, _w, cx| {
                this.passo(-1.0, false, true, cx)
            }))
            .on_action(cx.listener(|this, _: &JumpToMin, _w, cx| match this.rules.min {
                Some(min) => this.ir_para(min, cx),
                // Sem limite declarado a tecla é do CAMPO: `propagate` devolve o `home` ao laço de
                // bindings do GPUI, e o `MoveHome` do núcleo move o cursor. É o "let the browser
                // handle it" da referência.
                None => cx.propagate(),
            }))
            .on_action(cx.listener(|this, _: &JumpToMax, _w, cx| match this.rules.max {
                Some(max) => this.ir_para(max, cx),
                None => cx.propagate(),
            }))
            .child(frame);

        if let Some(bisel) = input::bevel_for(
            self.disabled,
            focado,
            self.invalid,
            crate::group::Join::NONE,
        ) {
            wrap = wrap.child(bisel);
        }
        if mostra_anel {
            let anel = input::ring_overlay(self.invalid, crate::group::Join::NONE);
            wrap = match transicao {
                // Estável: anel cheio, sem pedir frames ao compositor.
                None => wrap.child(anel),
                // `transition-shadow`: entra e sai desvanecendo. O id inclui o SENTIDO, pra entrar e
                // sair reiniciarem a animação em vez de uma continuar da outra.
                Some(entrando) => wrap.child(gpui::AnimationExt::with_animation(
                    anel,
                    gpui::ElementId::NamedInteger(
                        if entrando {
                            "number-field-ring-in".into()
                        } else {
                            "number-field-ring-out".into()
                        },
                        self.input.entity_id().as_u64(),
                    ),
                    gpui::Animation::new(input::FOCUS_TRANSITION).with_easing(gpui::ease_in_out),
                    move |el, delta| el.opacity(if entrando { delta } else { 1.0 - delta }),
                )),
            };
        }
        wrap
    }
}

#[cfg(test)]
// O clippy trata `assert!` sobre constante como erro provável, presumindo que quem escreveu quis
// testar algo variável. Aqui é o contrário: travar o número que veio da referência É o propósito
// destes testes. Mesma decisão em todos os módulos portados do coss (ver `input.rs`, `meter.rs`).
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    /// Regras com faixa e passo — a base de quase todo teste de conta abaixo.
    fn faixa(min: f32, max: f32, step: f32) -> Rules {
        Rules {
            min: Some(min),
            max: Some(max),
            step,
            ..Rules::default()
        }
    }

    // --- Os defaults da referência --------------------------------------------------------------

    /// **Os três passos são 1 / 0,1 / 10.** Literais de propósito: comparar com a própria constante
    /// não guarda nada.
    #[test]
    fn os_tres_passos_sao_os_da_referencia() {
        assert_eq!(PASSO, 1.0);
        assert_eq!(PASSO_PEQUENO, 0.1);
        assert_eq!(PASSO_GRANDE, 10.0);
        let r = Rules::default();
        assert_eq!(r.step, 1.0);
        assert_eq!(r.small_step, 0.1);
        assert_eq!(r.large_step, 10.0);
        // E os dois interruptores nascem DESLIGADOS, como lá.
        assert!(!r.snap_on_step, "`snapOnStep` default é false");
        assert!(!r.allow_out_of_range, "`allowOutOfRange` default é false");
        // Sem limite declarado — é o que faz `Home`/`End` não pularem.
        assert!(r.min.is_none() && r.max.is_none());
    }

    /// **`alt` vence `shift`.** A ordem é a do `getStepAmount`, e `alt+shift+↑` anda o passo pequeno.
    #[test]
    fn o_modificador_escolhe_o_passo_e_alt_vence_shift() {
        let r = Rules::default();
        assert_eq!(r.step_amount(false, false), 1.0);
        assert_eq!(r.step_amount(true, false), 0.1);
        assert_eq!(r.step_amount(false, true), 10.0);
        assert_eq!(r.step_amount(true, true), 0.1, "alt vence shift");
    }

    /// Sem limite declarado, o piso e o teto são `∓ SAFE_INTEGER` — o `?? MIN/MAX_SAFE_INTEGER`.
    #[test]
    fn sem_limite_o_piso_e_o_teto_sao_o_safe_integer() {
        let r = Rules::default();
        assert_eq!(r.min_with_default(), -9_007_199_254_740_991.0);
        assert_eq!(r.max_with_default(), 9_007_199_254_740_991.0);
        // E a base zero do snap é 0, não o piso.
        assert_eq!(r.min_with_zero_default(), 0.0);
        // Com mínimo declarado, as duas leituras convergem.
        let r = faixa(5.0, 10.0, 1.0);
        assert_eq!(r.min_with_default(), 5.0);
        assert_eq!(r.min_with_zero_default(), 5.0);
    }

    /// **Os builders escrevem nas regras que [`NumberField::rules`] devolve.** É a única cobertura da
    /// ponte entre os builders e a conta: sem ela, um builder que escrevesse no campo errado
    /// (`min` no lugar de `max`, `small_step` no lugar de `large_step`) não quebraria nada.
    ///
    /// Roda sem janela porque os builders não precisam de uma — só o `new` precisa, e é por isso que
    /// as `Rules` são construídas à mão aqui.
    #[test]
    fn os_builders_escrevem_nas_regras_certas() {
        let r = Rules {
            min: Some(-5.0),
            max: Some(7.0),
            step: 2.0,
            small_step: 0.25,
            large_step: 20.0,
            snap_on_step: true,
            allow_out_of_range: true,
        };
        assert_eq!(r.min(), Some(-5.0));
        assert_eq!(r.max(), Some(7.0));
        assert_eq!(r.step(), 2.0);
        // Os dois passos de modificador chegam pela `step_amount`, cada um no seu.
        assert_eq!(r.step_amount(true, false), 0.25, "o do `alt`");
        assert_eq!(r.step_amount(false, true), 20.0, "o do `shift`");
        assert_eq!(r.step_amount(false, false), 2.0, "e o sem modificador");
        // E os dois interruptores chegam na conta: com `snap_on_step` o passo alinha, e com
        // `allow_out_of_range` a digitação sai da faixa.
        assert_eq!(stepped(Some(1.0), 2.0, 1.0, &r, false), Some(3.0));
        assert_eq!(read_text("100", &r), TextRead::Number(100.0));
    }

    // --- Clamp e casos degenerados --------------------------------------------------------------

    /// O clamp básico, nas duas pontas, e o valor em faixa passando intacto.
    #[test]
    fn o_valor_e_clampado_nas_duas_pontas() {
        let r = faixa(0.0, 100.0, 1.0);
        assert_eq!(to_validated(Some(500.0), None, &r, false, true), Some(100.0));
        assert_eq!(to_validated(Some(-7.0), None, &r, false, true), Some(0.0));
        assert_eq!(to_validated(Some(42.0), None, &r, false, true), Some(42.0));
        // Os limites são alcançáveis.
        assert_eq!(to_validated(Some(0.0), None, &r, false, true), Some(0.0));
        assert_eq!(to_validated(Some(100.0), None, &r, false, true), Some(100.0));
        // `None` atravessa como `None` — campo vazio não é zero.
        assert_eq!(to_validated(None, None, &r, false, true), None);
    }

    /// **Faixa invertida (`min > max`) devolve sempre `min`** — e é por isso que o clamp é o
    /// `clamp_js` e não o [`f32::clamp`], que entraria em PÂNICO aqui.
    ///
    /// O `max(min, min(v, max))` desce o valor pra `max` e depois o sobe pra `min`: o piso vence.
    #[test]
    fn faixa_invertida_devolve_o_minimo_em_vez_de_entrar_em_panico() {
        let r = faixa(100.0, 0.0, 1.0);
        for v in [-50.0, 0.0, 50.0, 100.0, 500.0] {
            assert_eq!(
                to_validated(Some(v), None, &r, false, true),
                Some(100.0),
                "com min=100 e max=0, {v} tinha que colapsar no piso"
            );
        }
        // `min == max` é o caso vizinho e não degenera: devolve esse valor.
        let r = faixa(7.0, 7.0, 1.0);
        assert_eq!(to_validated(Some(3.0), None, &r, false, true), Some(7.0));
        assert_eq!(to_validated(Some(9.0), None, &r, false, true), Some(7.0));
    }

    /// **`NaN` não atravessa o clamp.**
    ///
    /// Em Rust `f32::min` devolve *o outro operando* quando um é `NaN`, então um `NaN` sairia do
    /// `clamp_js` como `max` — plausível, silencioso e errado. O guarda de finitude do
    /// [`to_validated`] o transforma em campo vazio.
    ///
    /// O teste também afirma a armadilha em si, pra o próximo leitor não ter que descobrir de novo
    /// por que o guarda existe.
    #[test]
    fn nan_nao_atravessa_o_clamp() {
        let r = faixa(0.0, 100.0, 1.0);
        assert_eq!(to_validated(Some(f32::NAN), None, &r, false, true), None);
        assert_eq!(to_validated(Some(f32::INFINITY), None, &r, false, true), None);
        assert_eq!(
            to_validated(Some(f32::NEG_INFINITY), None, &r, false, true),
            None
        );

        // A armadilha, escrita: sem o guarda, o clamp devolveria 100 pra `NaN`.
        let vazado = crate::meter::clamp_js(f32::NAN, 0.0, 100.0);
        assert_eq!(
            vazado, 100.0,
            "se isto virar NaN, o `f32::min` mudou de contrato e o guarda pode sair"
        );
    }

    /// `allow_out_of_range` desliga o clamp **da digitação** e só dele.
    #[test]
    fn fora_da_faixa_e_permitido_apenas_na_digitacao() {
        let r = Rules {
            allow_out_of_range: true,
            ..faixa(0.0, 100.0, 1.0)
        };
        // Digitar (o `should_clamp` vem de `!allow_out_of_range`): passa.
        assert_eq!(read_text("500", &r), TextRead::Number(500.0));
        assert_eq!(read_text("-7", &r), TextRead::Number(-7.0));
        // Um PASSO, no mesmo campo, continua clampando — o `should_clamp` do `stepped` é fixo.
        assert_eq!(stepped(Some(100.0), 1.0, 1.0, &r, false), Some(100.0));
        assert_eq!(stepped(Some(500.0), 1.0, 1.0, &r, false), Some(100.0));
        // E sem a permissão, digitar clampa.
        let fechado = faixa(0.0, 100.0, 1.0);
        assert_eq!(read_text("500", &fechado), TextRead::Number(100.0));
    }

    // --- O passo ---------------------------------------------------------------------------------

    /// O passo soma e subtrai, e para nos limites.
    #[test]
    fn o_passo_anda_e_para_nos_limites() {
        let r = faixa(0.0, 10.0, 1.0);
        assert_eq!(stepped(Some(5.0), 1.0, 1.0, &r, false), Some(6.0));
        assert_eq!(stepped(Some(5.0), 1.0, -1.0, &r, false), Some(4.0));
        assert_eq!(stepped(Some(10.0), 1.0, 1.0, &r, false), Some(10.0), "teto");
        assert_eq!(stepped(Some(0.0), 1.0, -1.0, &r, false), Some(0.0), "piso");
        // O passo grande também é clampado, não recusado.
        assert_eq!(stepped(Some(5.0), 10.0, 1.0, &r, false), Some(10.0));
    }

    /// **Campo vazio + ↑ semeia zero, clampado.** É o `incrementValue` do primitivo: "seed an empty
    /// field with 0; `setValue` clamps it to the in-range value nearest 0".
    #[test]
    fn campo_vazio_semeia_zero_clampado() {
        // Faixa que contém o zero: vira 0, independentemente da direção e do tamanho do passo.
        let r = faixa(-10.0, 10.0, 1.0);
        assert_eq!(stepped(None, 1.0, 1.0, &r, false), Some(0.0));
        assert_eq!(stepped(None, 1.0, -1.0, &r, false), Some(0.0));
        assert_eq!(stepped(None, 10.0, 1.0, &r, false), Some(0.0));
        // Faixa que NÃO contém o zero: o zero é clampado pro limite mais próximo dele.
        assert_eq!(stepped(None, 1.0, 1.0, &faixa(5.0, 9.0, 1.0), false), Some(5.0));
        assert_eq!(
            stepped(None, 1.0, 1.0, &faixa(-50.0, -10.0, 1.0), false),
            Some(-10.0)
        );
    }

    /// **Passo zero não move nada** — e não trava, não estoura, não zera o valor.
    #[test]
    fn passo_zero_nao_move_nada() {
        let r = faixa(0.0, 100.0, 0.0);
        assert_eq!(stepped(Some(42.0), r.step, 1.0, &r, false), Some(42.0));
        assert_eq!(stepped(Some(42.0), r.step, -1.0, &r, false), Some(42.0));
        // Com `snap_on_step` ligado, o passo zero DESLIGA o snap (o `step !== 0` do primitivo) em vez
        // de dividir por zero.
        let r = Rules {
            snap_on_step: true,
            ..faixa(0.0, 100.0, 0.0)
        };
        let v = stepped(Some(42.3), r.step, 1.0, &r, false).expect("valor");
        assert!(v.is_finite(), "o snap com passo zero produziu {v}");
        assert!((v - 42.3).abs() < 1e-4, "veio {v}");
    }

    /// **Passo negativo inverte as setas** — o sinal entra na conta, e a referência não o proíbe.
    #[test]
    fn passo_negativo_inverte_as_setas() {
        let r = faixa(0.0, 100.0, -1.0);
        assert_eq!(
            stepped(Some(50.0), r.step, 1.0, &r, false),
            Some(49.0),
            "com passo -1, ↑ DIMINUI"
        );
        assert_eq!(stepped(Some(50.0), r.step, -1.0, &r, false), Some(51.0));
        // E continua clampando nas duas pontas.
        assert_eq!(stepped(Some(0.0), r.step, 1.0, &r, false), Some(0.0));
    }

    /// O botão daquele lado sabe quando não tem mais o que fazer — e campo vazio nunca está no
    /// limite, porque sempre há a semeadura do zero.
    #[test]
    fn no_limite_o_botao_para_de_agir() {
        let r = faixa(0.0, 10.0, 1.0);
        assert!(at_boundary(Some(10.0), &r, true), "no teto, o + para");
        assert!(!at_boundary(Some(10.0), &r, false), "mas o − continua");
        assert!(at_boundary(Some(0.0), &r, false), "no piso, o − para");
        assert!(!at_boundary(Some(0.0), &r, true));
        assert!(!at_boundary(Some(5.0), &r, true));
        assert!(!at_boundary(None, &r, true), "campo vazio: sempre há o zero");
        assert!(!at_boundary(None, &r, false));
        // Sem limite declarado, nunca há limite alcançável.
        assert!(!at_boundary(Some(1e6), &Rules::default(), true));
    }

    // --- O snap ----------------------------------------------------------------------------------

    /// **`snap_on_step` desligado é o default, e nada é alinhado.** Este é o teste que separa este
    /// componente do [`crate::ScrubInput`], que snapa SEMPRE.
    #[test]
    fn sem_snap_on_step_o_valor_nao_e_alinhado() {
        let r = faixa(0.0, 10.0, 1.0);
        let v = stepped(Some(2.3), 1.0, 1.0, &r, false).expect("valor");
        assert!((v - 3.3).abs() < 1e-5, "sem snap, 2,3 + 1 = 3,3; veio {v}");
        // E o `ScrubInput`, no mesmo pedido, alinha — as duas contas existem de propósito.
        assert_eq!(crate::scrub_input::format_value(v, 1.0), "3");
    }

    /// **Com `snap_on_step`, um passo a partir de um valor desalinhado pousa no ponto de grade do
    /// lado em que a seta aponta** — e não a um passo inteiro de distância.
    ///
    /// É o efeito do snap ser DIRECIONAL, e o mais fácil de errar: de 2,3, o `↑` dá 3 e o `↓` dá
    /// **2**, não 1. A conta encadeia — `↓` produz `2,3 − 1 = 1,3` com passo `−1`, e o `ceil` do ramo
    /// negativo puxa de volta pra 2, que é justamente o ponto de grade abaixo do valor original.
    #[test]
    fn com_snap_on_step_o_passo_pousa_no_ponto_de_grade_vizinho() {
        let r = Rules {
            snap_on_step: true,
            ..faixa(0.0, 10.0, 1.0)
        };
        let acima = stepped(Some(2.3), 1.0, 1.0, &r, false).expect("valor");
        assert!((acima - 3.0).abs() < 1e-5, "↑ de 2,3 vai a 3; veio {acima}");
        let abaixo = stepped(Some(2.3), 1.0, -1.0, &r, false).expect("valor");
        assert!(
            (abaixo - 2.0).abs() < 1e-5,
            "↓ de 2,3 vai a 2 (o ponto de grade ABAIXO), não a 1; veio {abaixo}"
        );
        // Já alinhado, o passo é um passo inteiro nos dois sentidos — o snap não trava o valor.
        assert_eq!(stepped(Some(3.0), 1.0, 1.0, &r, false), Some(4.0));
        assert_eq!(stepped(Some(3.0), 1.0, -1.0, &r, false), Some(2.0));
    }

    /// O snap é **direcional** com passo positivo (`floor`) e do lado oposto com passo negativo
    /// (`ceil`); com `nearest` (o ramo do `alt`) ele arredonda pro mais próximo.
    #[test]
    fn o_snap_e_direcional_e_o_do_alt_e_o_mais_proximo() {
        // Direcional: 2,9 cai em 2, não em 3.
        let v = snap_to_step(2.9, 0.0, 1.0, false);
        assert!((v - 2.0).abs() < 1e-5, "floor: veio {v}");
        // Passo negativo sobe.
        let v = snap_to_step(2.1, 0.0, -1.0, false);
        assert!((v - 3.0).abs() < 1e-5, "ceil: veio {v}");
        // `nearest`: 2,9 vira 3.
        let v = snap_to_step(2.9, 0.0, 1.0, true);
        assert!((v - 3.0).abs() < 1e-5, "nearest: veio {v}");
        // Âncora: com base 0,05 e passo 0,1, a grade é 0,05 / 0,15 / … — o mínimo é alcançável.
        let v = snap_to_step(0.12, 0.05, 0.1, true);
        assert!((v - 0.15).abs() < 1e-4, "veio {v}");
    }

    /// **A base do snap é sempre `min ?? 0` — e o `small ||` do primitivo é um operando INERTE.**
    ///
    /// A condição de lá é
    /// `small || minWithDefault === MIN_SAFE_INTEGER ? minWithZeroDefault : minWithDefault`. Este
    /// teste prova a álgebra que a colapsa, nos dois casos possíveis, pra que ninguém reintroduza o
    /// `if` achando que falta um ramo:
    ///
    /// 1. com `min` declarado, `minWithZeroDefault` **é** `minWithDefault` — os dois ramos coincidem
    ///    e o `small` não tem o que trocar;
    /// 2. sem `min`, o segundo termo do `||` já é verdadeiro sozinho.
    ///
    /// O que o `small` de fato decide é o `nearest`, e isso está travado em
    /// `o_snap_e_direcional_e_o_do_alt_e_o_mais_proximo`.
    #[test]
    fn a_base_do_snap_e_sempre_o_minimo_ou_zero() {
        // Caso 1: com mínimo declarado, as duas leituras são o MESMO número.
        let com_minimo = faixa(0.05, 1.05, 0.1);
        assert_eq!(
            com_minimo.min_with_zero_default(),
            com_minimo.min_with_default(),
            "com min declarado os dois ramos do primitivo coincidem"
        );
        // Caso 2: sem mínimo, a base é zero e o piso é o SAFE_INTEGER — ramos bem diferentes, e é
        // por isso que o primitivo precisa do segundo termo do `||`.
        let sem_minimo = Rules::default();
        assert_eq!(sem_minimo.min_with_zero_default(), 0.0);
        assert_ne!(sem_minimo.min_with_zero_default(), sem_minimo.min_with_default());

        // E o efeito observável: `small` não muda a GRADE, só o arredondamento. Com passo já
        // alinhado à grade do mínimo, os dois modos pousam no mesmo lugar.
        let r = Rules {
            snap_on_step: true,
            small_step: 0.1,
            ..com_minimo
        };
        let normal = stepped(Some(0.05), 0.1, 1.0, &r, false).expect("valor");
        let alt = stepped(Some(0.05), 0.1, 1.0, &r, true).expect("valor");
        assert!((normal - 0.15).abs() < 1e-4, "grade do mínimo: veio {normal}");
        assert!(
            (alt - normal).abs() < 1e-4,
            "o `alt` usa a MESMA grade; veio {alt} contra {normal}"
        );

        // ⚠️ **O caso que dá dente ao teste**: numa faixa SEM mínimo as duas leituras divergem por
        // 9e15, e usar o piso como base é catastrófico — `2,3 + 9e15` perde os dígitos do valor em
        // `f32` e o snap devolve zero. A versão anterior deste teste só exercitava a faixa COM
        // mínimo, onde os dois ramos coincidem, e por isso **sobrevivia** à troca.
        let sem_limite = Rules {
            snap_on_step: true,
            ..Rules::default()
        };
        let v = stepped(Some(2.3), 1.0, 1.0, &sem_limite, false).expect("valor");
        assert!(
            (v - 3.0).abs() < 1e-4,
            "sem mínimo a base do snap é ZERO, então ↑ de 2,3 pousa em 3; veio {v}"
        );
    }

    /// **A folga do primitivo (`1e-10`) é MORTA em `f32`** — é o que justifica reescalá-la.
    #[test]
    fn a_folga_do_primitivo_e_morta_em_f32() {
        const FOLGA_DO_PRIMITIVO: f32 = 1e-10;
        assert_eq!(
            1.0_f32 + FOLGA_DO_PRIMITIVO,
            1.0_f32,
            "somar 1e-10 a um f32 em torno de 1 não muda um bit"
        );
        // Um ULP de f32 em torno de 1,0 é ~1,19e-7 — três ordens de grandeza acima da folga de lá.
        assert!(f32::EPSILON > FOLGA_DO_PRIMITIVO * 100.0);
        // E a nossa folga é grande o bastante pra somar, e pequena o bastante pra não virar meio
        // passo: 8 ULPs ≈ 9,5e-7.
        assert!(1.0_f32 + TOLERANCIA_DO_SNAP > 1.0_f32, "a folga tem que somar");
        assert!(TOLERANCIA_DO_SNAP < 1e-4, "e tem que ser muito menor que um passo");
    }

    /// **A limpeza de ruído da referência é a identidade em `f32`** — o que a torna código morto e é
    /// por isso que ela não foi transcrita.
    ///
    /// `removeFloatingPointErrors` faz `parseFloat(v.toPrecision(15))`. Um `f32` impresso com 15
    /// dígitos significativos e relido volta o mesmo `f32`, sempre — `f32` tem ~7,2.
    #[test]
    fn a_limpeza_de_ruido_da_referencia_e_identidade_em_f32() {
        for v in [0.1_f32, 0.7 + 0.1, 1.0 / 3.0, 42.42, 1e-5, 1234.5678] {
            let quinze_digitos = format!("{v:.14e}");
            let relido: f32 = quinze_digitos.parse().expect("um f32 relê o próprio texto");
            assert_eq!(relido, v, "{v} não voltou de {quinze_digitos}");
        }
    }

    // --- O texto ---------------------------------------------------------------------------------

    /// Os três destinos de uma leitura de texto: vazio zera, número vale, lixo preserva.
    #[test]
    fn o_texto_tem_tres_destinos() {
        let r = faixa(0.0, 100.0, 1.0);
        assert_eq!(read_text("", &r), TextRead::Empty);
        assert_eq!(read_text("   ", &r), TextRead::Empty, "só espaço é vazio");
        assert_eq!(read_text("42", &r), TextRead::Number(42.0));
        assert_eq!(read_text(" 42 ", &r), TextRead::Number(42.0), "espaço em volta");
        for lixo in ["abc", "1.2.3", "12px", "--", "1,5", "+", "-"] {
            assert_eq!(read_text(lixo, &r), TextRead::Unreadable, "aceitou {lixo:?}");
        }
        // `inf`/`NaN` parseiam como f32 em Rust e são recusados aqui.
        for t in ["inf", "-inf", "NaN", "infinity"] {
            assert_eq!(read_text(t, &r), TextRead::Unreadable, "aceitou {t:?}");
        }
    }

    /// **Campo vazio mostra texto vazio, e não `"0"`.** É o `value: number | null` da referência: se
    /// o `None` virasse `"0"`, esvaziar o campo escreveria um zero que o usuário não digitou.
    #[test]
    fn campo_vazio_mostra_texto_vazio() {
        assert_eq!(display_text(None, 1.0), "");
        assert_eq!(display_text(Some(0.0), 1.0), "0");
        // As casas decimais vêm do PASSO — a formatação é a do `scrub_input`, não uma segunda.
        assert_eq!(display_text(Some(0.5), 0.01), "0.50");
        assert_eq!(display_text(Some(12.0), 1.0), "12");
        assert_eq!(display_text(Some(-3.0), 1.0), "-3");
    }

    // --- Geometria e tinta ----------------------------------------------------------------------

    /// **A altura é a do campo da casa, nos três tamanhos** — porque os `sm:h-*` do
    /// `number-field.tsx` SÃO os `InputSize::content_height`.
    ///
    /// Literais de propósito: escrever `s.height()` dos dois lados deixaria o teste tautológico.
    #[test]
    fn a_altura_e_a_do_campo_da_casa() {
        for (s, miolo, externa) in [
            (InputSize::Sm, 26.0, 28.0),
            (InputSize::Md, 30.0, 32.0),
            (InputSize::Lg, 34.0, 36.0),
        ] {
            assert_eq!(s.content_height(), miolo, "o `sm:h-*` do input");
            assert_eq!(s.height(), externa, "miolo + as duas bordas de 1px");
        }
    }

    /// **O respiro dos botões é o do campo: 9 no `Sm`, 11 nos outros.** É o mesmo
    /// `calc(--spacing(3)-1px)` da referência, então vem do `InputSize` e não de constante nova.
    #[test]
    fn o_respiro_dos_botoes_e_o_do_campo() {
        assert_eq!(InputSize::Sm.pad_x(), 9.0);
        assert_eq!(InputSize::Md.pad_x(), 11.0);
        assert_eq!(InputSize::Lg.pad_x(), 11.0);
        // O que se vê na tela: borda + respiro = 12px da borda externa até o glifo.
        assert_eq!(1.0 + InputSize::Md.pad_x(), 12.0, "1 + 11");
        assert_eq!(1.0 + InputSize::Sm.pad_x(), 10.0, "1 + 9");
    }

    /// **O raio do botão é o da moldura menos a borda: 9.** É o
    /// `rounded-s-[calc(var(--radius-lg)-1px)]`, e o 10 vem do campo.
    #[test]
    fn o_raio_do_botao_desconta_a_borda() {
        assert_eq!(RAIO_DESCONTO, 1.0);
        assert_eq!(input::FIELD_RADIUS, 10.0, "`rounded-lg`");
        assert_eq!(input::FIELD_RADIUS - RAIO_DESCONTO, 9.0);
        // E o botão nunca lê como pílula: o raio é menor que meia altura no menor tamanho.
        assert!(input::FIELD_RADIUS - RAIO_DESCONTO < InputSize::Sm.content_height() / 2.0);
    }

    /// O glifo é 16px e o corpo do texto 14/20 — as variantes `sm:` do Tailwind.
    #[test]
    fn o_glifo_e_o_corpo_sao_as_variantes_de_desktop() {
        assert_eq!(LADO_DO_GLIFO, 16.0, "`sm:size-4`, não o `size-4.5` de 18");
        assert_eq!(CORPO_DO_TEXTO, 14.0, "`sm:text-sm`, não o `text-base` de 16");
        assert_eq!(ENTRELINHA, 20.0, "o par do Tailwind pro text-sm");
        // O default do GPUI, que é o que sairia sem declarar a entrelinha.
        let sem_declarar = CORPO_DO_TEXTO * 1.618_034;
        assert!(
            (sem_declarar - 22.65).abs() < 0.01,
            "sem fixar a entrelinha cada linha sairia {sem_declarar:.2}px"
        );
        // E o glifo cabe no miolo do menor tamanho.
        assert!(LADO_DO_GLIFO < InputSize::Sm.content_height());
    }

    /// **O menos e o mais rendem 1,0px de tinta cada** — lido dos SVGs de verdade, não de literais.
    ///
    /// A tinta renderizada é `traço × (lado / viewBox)`, e os dois números saem do arquivo que o
    /// [`GLIFO_MENOS`]/[`GLIFO_MAIS`] aponta. É o que dá dente ao teste: a primeira versão dele
    /// comparava dois literais escritos à mão e **sobrevivia** a trocar os caminhos pelo par
    /// `icons/minus.svg` + `icons/plus.svg`, que é exatamente a regressão que ele existe pra pegar
    /// (1,5px contra 1,33px — dois pesos num controle só).
    #[test]
    fn o_menos_e_o_mais_rendem_um_pixel_de_tinta() {
        /// Lê `stroke-width` e a largura do `viewBox` do SVG embutido e devolve a tinta em px no
        /// tamanho em que este componente o desenha.
        fn tinta(path: &str) -> f32 {
            let bytes = crate::assets::lookup(path).expect("o glifo está embutido");
            let svg = std::str::from_utf8(bytes).expect("SVG é texto");
            /// Extrai o valor de um atributo `nome="…"`.
            fn atributo<'a>(svg: &'a str, nome: &str) -> &'a str {
                let inicio = svg
                    .find(&format!("{nome}=\""))
                    .map(|i| i + nome.len() + 2)
                    .unwrap_or_else(|| panic!("{nome} não está no SVG"));
                let resto = &svg[inicio..];
                &resto[..resto.find('"').expect("atributo fechado")]
            }
            let traco: f32 = atributo(svg, "stroke-width").parse().expect("traço numérico");
            // `viewBox="0 0 W H"` — a largura é o terceiro campo.
            let view_box = atributo(svg, "viewBox");
            let largura: f32 = view_box
                .split_whitespace()
                .nth(2)
                .expect("viewBox com 4 campos")
                .parse()
                .expect("largura numérica");
            traco * LADO_DO_GLIFO / largura
        }

        let menos = tinta(GLIFO_MENOS);
        let mais = tinta(GLIFO_MAIS);
        assert!(
            (menos - 1.0).abs() < 1e-4,
            "o menos rende {menos:.2}px; a regra da casa é 1,0px de tinta"
        );
        assert!((mais - 1.0).abs() < 1e-4, "o mais rende {mais:.2}px");
        assert!(
            (menos - mais).abs() < 1e-4,
            "os dois glifos de UM controle têm que ter o mesmo peso: {menos:.2} contra {mais:.2}"
        );
        // O par que NÃO foi escolhido, medido do mesmo jeito, e a diferença que ele produziria.
        let da_casa_menos = tinta("icons/minus.svg");
        let da_casa_mais = tinta("icons/plus.svg");
        assert!(
            (da_casa_menos - da_casa_mais).abs() > 0.1,
            "o par `icons/` difere em {:.2}px — é a regressão que este teste guarda",
            (da_casa_menos - da_casa_mais).abs()
        );
    }

    /// Os dois glifos estão **embutidos**. Um caminho errado não dá erro: a `AssetSource` devolve
    /// `None`, o `svg` não desenha nada e ninguém é avisado.
    #[test]
    fn os_dois_glifos_estao_no_bundle() {
        for p in [GLIFO_MENOS, GLIFO_MAIS] {
            assert!(crate::iconoir::has(p), "{p} não está embutido");
            assert!(crate::assets::lookup(p).is_some(), "{p} não é servido");
        }
        assert_ne!(GLIFO_MENOS, GLIFO_MAIS, "dois glifos, não um repetido");
    }

    /// **A paleta tem UMA cor, e ela é translúcida nos dois temas** (o `--accent` do coss é
    /// preto/branco a 4%). Decodificada de verdade, não comparada com a própria constante: um valor
    /// de 6 dígitos esquecido aqui viraria uma cor completamente diferente sem erro de compilação.
    #[test]
    fn o_accent_e_um_veu_de_quatro_por_cento_nos_dois_temas() {
        // 4% de 255 = 10,2 → 0x0a.
        const ALFA_DE_REFERENCIA: f32 = 10.0 / 255.0;
        for (nome, p) in [("claro", NF_CLARO), ("escuro", NF_ESCURO)] {
            let c: gpui::Rgba = p.accent.hsla().into();
            assert!(
                (c.a - ALFA_DE_REFERENCIA).abs() < 1e-3,
                "{nome}: o `--accent` é 4%, veio {}",
                c.a
            );
            assert!(
                (c.r - c.g).abs() < 0.01 && (c.g - c.b).abs() < 0.01,
                "{nome}: o véu é NEUTRO — se r≠g≠b, o valor foi lido deslocado"
            );
        }
        // Preto no claro, branco no escuro.
        let claro: gpui::Rgba = NF_CLARO.accent.hsla().into();
        let escuro: gpui::Rgba = NF_ESCURO.accent.hsla().into();
        assert_eq!(claro.r, 0.0, "claro: véu preto");
        assert_eq!(escuro.r, 1.0, "escuro: véu branco");
        assert_ne!(NF_CLARO, NF_ESCURO);
    }

    /// A paleta do tema corrente é a do tema corrente — e o `palette()` não devolve sempre a mesma.
    #[test]
    fn a_paleta_segue_o_tema() {
        theme::set_theme(theme::ThemeMode::Light);
        assert_eq!(palette().accent, NF_CLARO.accent);
        theme::set_theme(theme::ThemeMode::Dark);
        assert_eq!(palette().accent, NF_ESCURO.accent);
    }

    /// **A tinta do glifo é o `--foreground` do próprio campo**, nos dois temas — não uma cópia, e
    /// não o véu de hover.
    ///
    /// A primeira versão deste teste afirmava só propriedades do token (`alpha == 1`, `!= accent`) e
    /// **sobrevivia** a trocar a cor no render: por isso a decisão saiu do render pra
    /// [`tinta_do_glifo`], que é o que este teste agora aponta.
    #[test]
    fn a_tinta_do_glifo_e_o_foreground_do_campo() {
        for modo in [theme::ThemeMode::Dark, theme::ThemeMode::Light] {
            theme::set_theme(modo);
            assert_eq!(
                tinta_do_glifo(),
                input::field().text,
                "no tema {modo:?} o glifo tem que ler igual ao número ao lado"
            );
            assert_eq!(tinta_do_glifo().alpha(), 1.0, "a tinta do glifo é opaca");
            // O véu de hover NÃO pode ser a tinta do glifo: um botão realçado ficaria invisível.
            assert_ne!(tinta_do_glifo(), palette().accent);
        }
        theme::set_theme(theme::ThemeMode::Dark);
    }

    /// O esmaecimento de desabilitado é o `opacity-64` do coss, e **só o do wrapper**: o campo
    /// hospedado é `unstyled`, então ele não esmaece a si mesmo e não há multiplicação.
    #[test]
    fn o_esmaecimento_e_um_so() {
        assert_eq!(OPACIDADE_DESABILITADO, 0.64);
        // O gate do campo: sem moldura própria, ele não esmaece.
        assert!(
            !input::dim_visible(true, true),
            "o campo hospedado (unstyled) não pode esmaecer também"
        );
        assert!(input::dim_visible(true, false), "solto, ele esmaece");
        // A composição que o gate evita.
        let dobro = OPACIDADE_DESABILITADO * OPACIDADE_DESABILITADO;
        assert!(dobro < 0.45, "0,64² = {dobro:.2}");
    }

    // --- Teclado --------------------------------------------------------------------------------

    /// **O predicado dos bindings tem que ser `"NumberField > Input"`.**
    ///
    /// Com só `"NumberField"` os bindings casariam no nó do WRAPPER, mais raso que o do campo, e o
    /// `up`→`MoveUp` do núcleo (predicado `"Input"`) venceria por profundidade — as setas voltariam
    /// a mover o cursor. O teste trava o predicado E a relação entre as duas pontas dele.
    #[test]
    fn o_predicado_qualifica_o_contexto_do_campo() {
        assert_eq!(KEY_CONTEXT, "NumberField");
        assert_eq!(PREDICADO, "NumberField > Input");
        assert!(
            PREDICADO.starts_with(KEY_CONTEXT),
            "o predicado começa no contexto que o wrapper empilha"
        );
        assert!(
            PREDICADO.ends_with("> Input"),
            "e termina no contexto do NÚCLEO, senão a profundidade não empata"
        );
        // E ele é um predicado composto: `"NumberField"` sozinho perderia.
        assert_ne!(PREDICADO, KEY_CONTEXT);
    }
}

/// Testes de **janela**: a geometria como ela sai no layout, e o teclado como ele chega de verdade.
#[cfg(test)]
mod tests_de_janela {
    use super::*;
    use gpui::{canvas, Bounds, Pixels, TestAppContext, VisualTestContext};
    use std::cell::Cell;
    use std::rc::Rc;

    /// Largura da janela do harness — folgada, pra os dois botões e o campo nunca disputarem espaço.
    const LARGURA: f32 = 240.0;

    /// O medidor: o campo e, LOGO ABAIXO dele numa coluna sem `gap`, uma sonda de altura zero. A
    /// origem `y` da sonda é, portanto, a altura total que o campo ocupou.
    struct Medidor {
        campo: Entity<NumberField>,
        sonda: Rc<Cell<Option<Bounds<Pixels>>>>,
    }

    impl Render for Medidor {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            theme::set_theme(theme::ThemeMode::Dark);
            let sonda = self.sonda.clone();
            div()
                .flex()
                .flex_col()
                .w(px(LARGURA))
                .child(self.campo.clone())
                .child(
                    canvas(move |b, _w, _cx| sonda.set(Some(b)), |_, _, _, _| {})
                        .w_full()
                        .h(px(0.0)),
                )
        }
    }

    /// A sonda de altura do [`Medidor`] — o `Bounds` que o canvas de altura zero mediu.
    type Sonda = Rc<Cell<Option<Bounds<Pixels>>>>;

    /// O que [`abrir`] devolve: a entidade, o contexto visual e a sonda.
    type Harness = (Entity<NumberField>, VisualTestContext, Sonda);

    /// Abre uma janela com um `NumberField` e devolve `(entidade, contexto, sonda)`.
    ///
    /// ⚠️ Envolto em `gpui_component::Root`: qualquer clique atravessa o núcleo do `gpui-component`,
    /// que faz `unwrap` nas camadas de popover/modal dele (`root.rs:268`). Sem o `Root` o teste
    /// estoura ali, e o pânico não diz que o que falta é isto.
    fn abrir(cx: &mut TestAppContext, size: InputSize) -> Harness {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::input::init(cx);
        });
        let campo: Rc<Cell<Option<Entity<NumberField>>>> = Rc::new(Cell::new(None));
        let c = campo.clone();
        let sonda: Rc<Cell<Option<Bounds<Pixels>>>> = Rc::new(Cell::new(None));
        let s = sonda.clone();
        let window = cx.add_window(move |window, cx| {
            let e = cx.new(|cx| {
                NumberField::new(Some(5.0), window, cx)
                    .size(size)
                    .min(0.0)
                    .max(10.0)
                    .step(1.0)
            });
            c.set(Some(e.clone()));
            gpui_component::Root::new(cx.new(|_| Medidor { campo: e, sonda: s }), window, cx)
        });
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        (campo.take().expect("o campo foi construído"), vcx, sonda)
    }

    /// **O campo mede 32px de altura no `Md`** — o miolo de 30 do `InputSize::Md` mais as duas
    /// bordas de 1px da moldura.
    ///
    /// O 32 está **literal** de propósito. Comparar o medido com `InputSize::Md.height()` é
    /// tautológico: trocar o tamanho move os dois lados juntos e o teste passa calado. Com o número
    /// escrito, mexer na densidade obriga a mexer nesta linha.
    #[gpui::test]
    fn a_altura_medida_do_campo_e_de_32px(cx: &mut TestAppContext) {
        const ALTURA: f32 = 32.0;
        let (_campo, _vcx, sonda) = abrir(cx, InputSize::Md);
        let medida = f32::from(sonda.get().expect("a sonda mediu no prepaint").origin.y);
        assert!(
            (medida - ALTURA).abs() < 0.5,
            "o campo mediu {medida}px, esperado {ALTURA}px"
        );
    }

    /// **O `Sm` mede 28 e o `Lg` 36** — os outros dois tamanhos, também literais.
    #[gpui::test]
    fn os_outros_dois_tamanhos_medem_28_e_36(cx: &mut TestAppContext) {
        for (size, esperado) in [(InputSize::Sm, 28.0_f32), (InputSize::Lg, 36.0)] {
            let (_campo, _vcx, sonda) = abrir(cx, size);
            let medida = f32::from(sonda.get().expect("a sonda mediu").origin.y);
            assert!(
                (medida - esperado).abs() < 0.5,
                "{size:?} mediu {medida}px, esperado {esperado}px"
            );
        }
    }

    /// **A seta pra cima soma um passo — de verdade, com o campo focado numa janela.**
    ///
    /// É o teste que prova a precedência dos bindings de [`init`] sobre o `up`→`MoveUp` do núcleo: se
    /// o predicado ou a ordem de registro estiverem errados, a tecla move o cursor e o valor não
    /// muda.
    ///
    /// **Clica** pra focar, em vez de chamar `focus()`: o caminho de despacho é reconstruído no
    /// desenho, e um `focus()` deixa `is_focused` verdadeiro sem produzir o caminho que a ação
    /// precisa percorrer.
    #[gpui::test]
    fn a_seta_para_cima_soma_um_passo(cx: &mut TestAppContext) {
        let (campo, mut vcx, sonda) = abrir(cx, InputSize::Md);
        assert_eq!(vcx.update(|_w, cx| campo.read(cx).value()), Some(5.0));

        // Clica no MEIO da caixa. ⚠️ A sonda é um canvas de altura ZERO colocado DEPOIS do campo: o
        // `origin.y` dela é a ALTURA do campo, não a posição dele.
        let altura = f32::from(sonda.get().expect("a sonda mediu").origin.y);
        vcx.simulate_click(
            gpui::point(px(LARGURA / 2.0), px(altura / 2.0)),
            gpui::Modifiers::default(),
        );
        vcx.run_until_parked();

        vcx.simulate_keystrokes("up");
        vcx.run_until_parked();
        assert_eq!(
            vcx.update(|_w, cx| campo.read(cx).value()),
            Some(6.0),
            "`up` tinha que somar um passo; se veio 5, a tecla foi pro `MoveUp` do núcleo"
        );

        vcx.simulate_keystrokes("down down");
        vcx.run_until_parked();
        assert_eq!(vcx.update(|_w, cx| campo.read(cx).value()), Some(4.0));

        // `Home`/`End` com os limites declarados pulam pras pontas.
        vcx.simulate_keystrokes("end");
        vcx.run_until_parked();
        assert_eq!(vcx.update(|_w, cx| campo.read(cx).value()), Some(10.0));
        vcx.simulate_keystrokes("home");
        vcx.run_until_parked();
        assert_eq!(vcx.update(|_w, cx| campo.read(cx).value()), Some(0.0));
    }

    /// **Clicar no `+` soma um passo, e no `−` subtrai** — o gesto principal do componente.
    ///
    /// ⚠️ Este teste existe por causa de um risco ESTRUTURAL, não por completude: o botão tem
    /// `on_mouse_down` com `cx.stop_propagation()` **no mesmo elemento** que o `on_click` (o
    /// `stop` é o que impede o clique de posicionar o cursor de texto embaixo do botão). Se o GPUI
    /// interrompesse os ouvintes do próprio nó junto com os dos ancestrais, o `stop` mataria o
    /// clique do próprio botão e o componente ficaria sem o gesto principal — sem nenhum erro de
    /// compilação. É a diferença entre um teste de cobertura e um teste que responde a uma pergunta.
    ///
    /// As coordenadas saem da geometria declarada: o `−` ocupa `1 .. 1 + 11 + 16 + 11` = 39px à
    /// esquerda, e o `+` os mesmos 38px à direita da moldura de [`LARGURA`].
    #[gpui::test]
    /// **O botão de passo preenche a ALTURA do campo.**
    ///
    /// Escrito porque o usuário viu o realce de hover mais curto que a caixa, com folga em cima e
    /// embaixo. A causa: o botão pede `h_full`, mas a moldura **não declara altura** (decisão fiel — a
    /// altura vem do miolo hospedado), e `height: 100%` contra altura indefinida cai pra `auto`. O
    /// botão ficava do tamanho do glifo. É o mesmo mecanismo do `w_full` que quebrou a largura da
    /// [`crate::table`].
    ///
    /// ⚠️ A afirmação é COMPORTAMENTAL de propósito: nada aqui lê pixel (o `debug_bounds` do GPUI é
    /// `pub(crate)`), então o que se mede é a consequência — um clique a 2px da borda de cima, na
    /// coluna do `+`, tem que somar. Com o botão curto e centralizado, esse ponto cai no campo de
    /// texto e o valor não muda.
    #[gpui::test]
    fn o_botao_de_passo_preenche_a_altura_do_campo(cx: &mut TestAppContext) {
        let (campo, mut vcx, sonda) = abrir(cx, InputSize::Md);
        // A sonda mede a altura EXTERNA do campo: o canvas de altura zero vem logo abaixo dele, numa
        // coluna sem `gap`, então a origem `y` dele é a altura que o campo ocupou.
        let altura = f32::from(sonda.get().expect("a sonda mediu").origin.y);
        assert_eq!(altura, 32.0, "Md é 30 de miolo + 2 bordas — se mudou, o resto do teste não vale");

        // Os dois extremos do MESMO botão (o `+`), 2px dentro de cada borda. Usar um botão só isola a
        // pergunta em uma variável: a extensão VERTICAL dele. Se o botão não esticasse, um dos dois
        // cliques cairia no campo de texto e o valor pararia em 6.
        let x = px(LARGURA - 19.0);
        vcx.simulate_click(gpui::point(x, px(2.0)), gpui::Modifiers::default());
        vcx.run_until_parked();
        assert_eq!(
            vcx.update(|_w, cx| campo.read(cx).value()),
            Some(6.0),
            "clique a 2px da borda de CIMA não somou: o botão não preenche a altura"
        );

        vcx.simulate_click(gpui::point(x, px(altura - 2.0)), gpui::Modifiers::default());
        vcx.run_until_parked();
        assert_eq!(
            vcx.update(|_w, cx| campo.read(cx).value()),
            Some(7.0),
            "clique a 2px da borda de BAIXO não somou: o botão não preenche a altura"
        );
    }

    fn clicar_nos_botoes_anda_um_passo(cx: &mut TestAppContext) {
        let (campo, mut vcx, sonda) = abrir(cx, InputSize::Md);
        let y = px(f32::from(sonda.get().expect("a sonda mediu").origin.y) / 2.0);
        assert_eq!(vcx.update(|_w, cx| campo.read(cx).value()), Some(5.0));

        // O meio do botão de MAIS: 19px da borda direita.
        let mais = gpui::point(px(LARGURA - 19.0), y);
        vcx.simulate_click(mais, gpui::Modifiers::default());
        vcx.run_until_parked();
        assert_eq!(
            vcx.update(|_w, cx| campo.read(cx).value()),
            Some(6.0),
            "o clique no + não somou: o `stop_propagation` do mouse-down comeu o `on_click`?"
        );

        // O meio do botão de MENOS: 19px da borda esquerda.
        let menos = gpui::point(px(19.0), y);
        vcx.simulate_click(menos, gpui::Modifiers::default());
        vcx.simulate_click(menos, gpui::Modifiers::default());
        vcx.run_until_parked();
        assert_eq!(vcx.update(|_w, cx| campo.read(cx).value()), Some(4.0));

        // E o clique no botão **foca o campo**, pra o usuário poder continuar pelas setas (é o
        // `inputRef.current?.focus()` do `useNumberFieldStepperButton`).
        let focado = vcx.update(|window, cx| {
            gpui::Focusable::focus_handle(campo.read(cx).input.read(cx), cx).is_focused(window)
        });
        assert!(focado, "o clique no botão tinha que focar o campo");

        // **O gate de limite é por LADO, não pelo controle.** Com o valor no TETO, o `−` continua
        // agindo. É aqui que essa metade da afirmação tem dente: no
        // `no_teto_o_mais_esta_desligado_mesmo_com_passo_negativo` o passo é negativo, e lá o clamp
        // esconderia a diferença.
        vcx.update(|_w, cx| campo.update(cx, |c, cx| c.set_value(Some(10.0), cx)));
        vcx.run_until_parked();
        vcx.simulate_click(menos, gpui::Modifiers::default());
        vcx.run_until_parked();
        assert_eq!(
            vcx.update(|_w, cx| campo.read(cx).value()),
            Some(9.0),
            "no teto o − ainda tem o que fazer; se veio 10, o `at_boundary` desligou os DOIS lados"
        );
    }

    /// **No teto, o botão de `+` está desligado — mesmo quando apertá-lo BAIXARIA o valor.**
    ///
    /// É o `isAtBoundary` do `useNumberFieldStepperButton` ao pé da letra: ele compara
    /// `value >= max` e **não olha o sinal do passo**. Com `step = -1` o `+` diminui (ver
    /// `passo_negativo_inverte_as_setas`), então no teto ele teria o que fazer — e continua
    /// desligado. Fiel, surpreendente, e a ÚNICA configuração em que o gate é observável: com passo
    /// positivo o clamp já torna o passo um no-op, e tirar o gate não muda um número.
    ///
    /// A primeira versão deste teste usava passo positivo e **sobrevivia** à remoção do gate.
    #[gpui::test]
    fn no_teto_o_mais_esta_desligado_mesmo_com_passo_negativo(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::input::init(cx);
        });
        let campo: Rc<Cell<Option<Entity<NumberField>>>> = Rc::new(Cell::new(None));
        let c = campo.clone();
        let sonda: Sonda = Rc::new(Cell::new(None));
        let s = sonda.clone();
        let window = cx.add_window(move |window, cx| {
            let e = cx.new(|cx| {
                NumberField::new(Some(10.0), window, cx)
                    .min(0.0)
                    .max(10.0)
                    .step(-1.0)
            });
            c.set(Some(e.clone()));
            gpui_component::Root::new(cx.new(|_| Medidor { campo: e, sonda: s }), window, cx)
        });
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        let campo = campo.take().expect("o campo foi construído");
        assert_eq!(vcx.update(|_w, cx| campo.read(cx).value()), Some(10.0));

        let y = px(f32::from(sonda.get().expect("a sonda mediu").origin.y) / 2.0);
        vcx.simulate_click(
            gpui::point(px(LARGURA - 19.0), y),
            gpui::Modifiers::default(),
        );
        vcx.run_until_parked();
        assert_eq!(
            vcx.update(|_w, cx| campo.read(cx).value()),
            Some(10.0),
            "no teto o + está desligado — se veio 9, o gate de `at_boundary` saiu da árvore"
        );

        // ⚠️ **Aqui NÃO se afirma que o `−` do outro lado age.** Ele age (o `10 <= 0` do
        // `at_boundary` é falso), mas com passo negativo o `−` SOBE e o clamp o segura no teto — o
        // resultado é o mesmo 10 com ou sem o gate, e uma asserção nesse ponto sobreviveria a
        // desligar os dois lados. Essa metade da afirmação vive em
        // `clicar_nos_botoes_anda_um_passo`, com passo positivo, onde ela é observável.
    }

    /// **Desfoca o campo e roda o caminho de blur do componente.**
    ///
    /// O clique no vazio é o gesto real (o `on_mouse_down_out` +
    /// [`crate::input::blur_on_outside_click`], e o foco de fato vai embora), mas o
    /// `InputEvent::Blur` do núcleo **não é entregue no `TestAppContext`** — medido: depois de dois
    /// `run_until_parked`, `is_focused` já é `false` e `texto_sujo` ainda é `true`. É a mesma
    /// limitação que o [`crate::scrub_input`] documenta pro `InputEvent::Focus`: o observador de foco
    /// do GPUI compara o CAMINHO de foco do quadro desenhado, e naquele ambiente ele não fecha no
    /// campo.
    ///
    /// Então o handler é chamado à mão. O que fica sem cobertura é só a ponte "o núcleo entrega o
    /// `Blur`" — declarada no doc do módulo —, e não a lógica de commit, que é o que interessa aqui e
    /// roda de verdade.
    fn desfocar(campo: &Entity<NumberField>, vcx: &mut VisualTestContext) {
        vcx.simulate_click(
            gpui::point(px(LARGURA / 2.0), px(200.0)),
            gpui::Modifiers::default(),
        );
        vcx.run_until_parked();
        vcx.update(|_w, cx| campo.update(cx, |c, cx| c.ao_desfocar(cx)));
        vcx.run_until_parked();
    }

    /// Um assinante do componente, em miniatura: ele existe só pra manter a [`Subscription`] viva
    /// (o vetor de eventos é do teste, capturado pela closure).
    struct Ouvinte {
        campo: Entity<NumberField>,
        _sub: Subscription,
    }

    impl Render for Ouvinte {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            theme::set_theme(theme::ThemeMode::Dark);
            div().w(px(LARGURA)).child(self.campo.clone())
        }
    }

    /// O registro de eventos que [`abrir_ouvindo`] devolve.
    type Registro = Rc<std::cell::RefCell<Vec<NumberFieldEvent>>>;

    /// Abre uma janela com um `NumberField` `[0, 10]` de passo 1 em `5`, com um assinante ligado.
    fn abrir_ouvindo(cx: &mut TestAppContext) -> (Entity<NumberField>, VisualTestContext, Registro) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::input::init(cx);
        });
        let eventos: Registro = Rc::new(std::cell::RefCell::new(Vec::new()));
        let campo: Rc<Cell<Option<Entity<NumberField>>>> = Rc::new(Cell::new(None));
        let c = campo.clone();
        let ev = eventos.clone();
        let window = cx.add_window(move |window, cx| {
            let e = cx.new(|cx| {
                NumberField::new(Some(5.0), window, cx)
                    .min(0.0)
                    .max(10.0)
                    .step(1.0)
            });
            c.set(Some(e.clone()));
            let sub = cx.subscribe(&e, move |_this, _campo, evento: &NumberFieldEvent, _cx| {
                ev.borrow_mut().push(*evento);
            });
            gpui_component::Root::new(cx.new(|_| Ouvinte { campo: e, _sub: sub }), window, cx)
        });
        let vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();
        (
            campo.take().expect("o campo foi construído"),
            vcx,
            eventos,
        )
    }

    /// **O componente EMITE, e emite o par certo.** É o contrato público inteiro: quem usa não lê o
    /// valor por polling, ele assina.
    ///
    /// As três afirmações que importam, e que a referência separa em duas props:
    ///
    /// - um passo (botão ou tecla) dispara `Change` **e** `Commit`, nessa ordem — na referência o
    ///   passo de teclado commita na hora e o clique commita no `onClick`/`onStop`;
    /// - `set_value` (sincronização externa) **não** dispara nada, senão quem assina realimentaria o
    ///   próprio `set` — a mesma decisão do [`crate::ScrubInput::set_value`];
    /// - um passo que **não muda nada** (o limite) não dispara nada: lá é `if (changed)`.
    #[gpui::test]
    fn o_passo_emite_change_e_commit_e_o_set_value_nao_emite(cx: &mut TestAppContext) {
        let (campo, mut vcx, eventos) = abrir_ouvindo(cx);
        assert!(
            eventos.borrow().is_empty(),
            "abrir a janela não pode emitir nada; veio {:?}",
            eventos.borrow()
        );

        // Um passo: `Change` e depois `Commit`, nessa ordem, com o mesmo valor.
        vcx.update(|_w, cx| campo.update(cx, |c, cx| c.passo(1.0, false, false, cx)));
        vcx.run_until_parked();
        assert_eq!(
            *eventos.borrow(),
            vec![
                NumberFieldEvent::Change(Some(6.0)),
                NumberFieldEvent::Commit(Some(6.0)),
            ],
            "o passo tem que emitir Change e então Commit"
        );

        // `set_value` é sincronização: silencioso.
        eventos.borrow_mut().clear();
        vcx.update(|_w, cx| campo.update(cx, |c, cx| c.set_value(Some(2.0), cx)));
        vcx.run_until_parked();
        assert!(
            eventos.borrow().is_empty(),
            "`set_value` não pode emitir; veio {:?}",
            eventos.borrow()
        );

        // E um passo no LIMITE não emite: a referência só commita `if (changed)`.
        vcx.update(|_w, cx| campo.update(cx, |c, cx| c.set_value(Some(10.0), cx)));
        vcx.run_until_parked();
        eventos.borrow_mut().clear();
        vcx.update(|_w, cx| campo.update(cx, |c, cx| c.passo(1.0, false, false, cx)));
        vcx.run_until_parked();
        assert!(
            eventos.borrow().is_empty(),
            "no teto o passo não muda nada, logo não emite; veio {:?}",
            eventos.borrow()
        );
    }

    /// **Digitar muda o valor AO VIVO; o blur normaliza o texto e confirma.**
    ///
    /// É a divisão de trabalho da referência entre `onChange` (cada tecla parseável já chama
    /// `setValue`) e `onValueCommitted` (só no blur), e o que faz `Enter`/`Escape` não terem o que
    /// fazer aqui — diferente do [`crate::ScrubInput`], onde o valor só sobe no commit.
    ///
    /// Também é o único lugar que prova o par `texto_sujo` / [`NumberField::sincronizar_texto`]: o
    /// texto **não** pode ser reformatado embaixo do cursor enquanto se digita, e **tem** que ser
    /// normalizado no blur.
    #[gpui::test]
    fn digitar_muda_ao_vivo_e_o_blur_confirma(cx: &mut TestAppContext) {
        let (campo, mut vcx, eventos) = abrir_ouvindo(cx);
        let estado = vcx.update(|_w, cx| campo.read(cx).input.clone());

        // Clica pra focar (no meio, entre os dois botões). O caret cai no FIM do texto, não numa
        // seleção total — é a ausência declarada do `selecionar_ao_focar` do [`crate::ScrubInput`],
        // e é o que faz um `7` digitado aqui virar `57` (e não `7`).
        vcx.simulate_click(gpui::point(px(LARGURA / 2.0), px(16.0)), gpui::Modifiers::default());
        vcx.run_until_parked();
        eventos.borrow_mut().clear();
        vcx.simulate_input("7");
        vcx.run_until_parked();
        assert_eq!(
            vcx.update(|_w, cx| campo.read(cx).value()),
            Some(10.0),
            "`5` + `7` = `57`, clampado no teto — digitar NÃO substitui a seleção aqui"
        );

        // Esvaziar o campo é um estado: valor `None`, e `Change(None)` ao vivo.
        eventos.borrow_mut().clear();
        vcx.simulate_keystrokes("backspace backspace");
        vcx.run_until_parked();
        assert_eq!(
            vcx.update(|_w, cx| campo.read(cx).value()),
            None,
            "campo vazio é `None`, não zero"
        );
        assert_eq!(
            eventos.borrow().last(),
            Some(&NumberFieldEvent::Change(None)),
            "esvaziar emite `Change(None)`"
        );

        // E digitar num campo vazio muda o valor AO VIVO.
        eventos.borrow_mut().clear();
        vcx.simulate_input("7");
        vcx.run_until_parked();
        assert_eq!(
            vcx.update(|_w, cx| campo.read(cx).value()),
            Some(7.0),
            "digitar tinha que mudar o valor AO VIVO"
        );
        assert_eq!(
            *eventos.borrow(),
            vec![NumberFieldEvent::Change(Some(7.0))],
            "ao vivo é `Change`, e ainda NÃO `Commit`"
        );

        // **Texto ilegível**: o valor não se move, nada é emitido, e o texto GUARDA o que se
        // digitou — é o `texto_sujo` impedindo o campo de reformatar embaixo do cursor.
        //
        // ⚠️ `"7."` não serve de exemplo: o parser de `f32` do Rust o aceita como 7. O `-`
        // pendurado, sim.
        eventos.borrow_mut().clear();
        vcx.simulate_input("-");
        vcx.run_until_parked();
        assert_eq!(vcx.update(|_w, cx| campo.read(cx).value()), Some(7.0));
        assert!(
            eventos.borrow().is_empty(),
            "texto ilegível não emite; veio {:?}",
            eventos.borrow()
        );
        assert_eq!(
            vcx.update(|_w, cx| estado.read(cx).value().to_string()),
            "7-",
            "o texto não pode ser reformatado embaixo do cursor"
        );

        // **Blur com texto ilegível**: não confirma (o primitivo devolve antes do commit) e o texto
        // volta ao valor formatado, porque o `texto_sujo` foi limpo.
        eventos.borrow_mut().clear();
        desfocar(&campo, &mut vcx);
        assert!(
            eventos.borrow().is_empty(),
            "blur com texto ilegível não confirma nada; veio {:?}",
            eventos.borrow()
        );
        assert_eq!(
            vcx.update(|_w, cx| estado.read(cx).value().to_string()),
            "7",
            "e o texto volta ao valor: o `-` pendurado sai"
        );

        // **Blur com texto legível CONFIRMA.** Segunda rodada: foca, apaga, digita, desfoca.
        vcx.simulate_click(gpui::point(px(LARGURA / 2.0), px(16.0)), gpui::Modifiers::default());
        vcx.run_until_parked();
        vcx.simulate_keystrokes("backspace");
        vcx.simulate_input("3");
        vcx.run_until_parked();
        eventos.borrow_mut().clear();
        desfocar(&campo, &mut vcx);
        assert_eq!(
            *eventos.borrow(),
            vec![NumberFieldEvent::Commit(Some(3.0))],
            "o blur confirma o que foi digitado"
        );
        assert_eq!(vcx.update(|_w, cx| estado.read(cx).value().to_string()), "3");
    }

    /// **O texto acompanha o valor**, e um valor posto de fora reescreve o campo sem evento.
    #[gpui::test]
    fn o_texto_acompanha_o_valor(cx: &mut TestAppContext) {
        let (campo, mut vcx, _sonda) = abrir(cx, InputSize::Md);
        let estado = vcx.update(|_w, cx| campo.read(cx).input.clone());
        assert_eq!(vcx.update(|_w, cx| estado.read(cx).value().to_string()), "5");

        vcx.update(|_w, cx| campo.update(cx, |c, cx| c.set_value(Some(9.0), cx)));
        vcx.run_until_parked();
        assert_eq!(vcx.update(|_w, cx| estado.read(cx).value().to_string()), "9");

        // Fora da faixa: o valor é clampado ANTES de virar texto.
        vcx.update(|_w, cx| campo.update(cx, |c, cx| c.set_value(Some(999.0), cx)));
        vcx.run_until_parked();
        assert_eq!(vcx.update(|_w, cx| estado.read(cx).value().to_string()), "10");
        assert_eq!(vcx.update(|_w, cx| campo.read(cx).value()), Some(10.0));

        // E `None` esvazia o campo, em vez de escrever `0`.
        vcx.update(|_w, cx| campo.update(cx, |c, cx| c.set_value(None, cx)));
        vcx.run_until_parked();
        assert_eq!(vcx.update(|_w, cx| estado.read(cx).value().to_string()), "");
    }

    /// **A ordem dos builders não importa**: `new(Some(500.0), …).max(100.0)` mostra `100`.
    ///
    /// É o que o [`NumberField::sincronizar_texto`] no render existe pra garantir — no `new` o `max`
    /// ainda não existia, e o texto inicial foi escrito com as regras default.
    #[gpui::test]
    fn a_ordem_dos_builders_nao_deixa_o_texto_atras(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::input::init(cx);
        });
        let campo: Rc<Cell<Option<Entity<NumberField>>>> = Rc::new(Cell::new(None));
        let c = campo.clone();
        let sonda: Rc<Cell<Option<Bounds<Pixels>>>> = Rc::new(Cell::new(None));
        let window = cx.add_window(move |window, cx| {
            let e = cx.new(|cx| NumberField::new(Some(500.0), window, cx).min(0.0).max(100.0));
            c.set(Some(e.clone()));
            Medidor { campo: e, sonda }
        });
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.run_until_parked();

        let campo = campo.take().expect("o campo foi construído");
        assert_eq!(vcx.update(|_w, cx| campo.read(cx).value()), Some(100.0));
        let estado = vcx.update(|_w, cx| campo.read(cx).input.clone());
        assert_eq!(
            vcx.update(|_w, cx| estado.read(cx).value().to_string()),
            "100",
            "o texto ficou no valor de antes do `.max()`"
        );
    }
}

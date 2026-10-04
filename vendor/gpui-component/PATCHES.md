# Modificações locais no `gpui-component`

> **Aviso de licença (Apache-2.0 §4b).** Este diretório é uma cópia do crate
> [`gpui-component`](https://github.com/longbridge/gpui-component) **v0.5.1** do crates.io,
> Copyright 2024–2025 Longbridge, licenciado sob Apache-2.0 (ver `LICENSE-APACHE`). Os arquivos
> **foram modificados** por este projeto; as mudanças estão listadas abaixo e marcadas no código
> com o comentário `FORK (Fennel):`.

## Por que existe um fork

A seleção por mouse do campo de texto precisava bater com a do navegador (Chrome). O
comportamento em falta — granularidade de palavra/parágrafo que **sobrevive** ao clique e passa a
valer no arraste e no shift+clique — só é implementável **de dentro** do `InputState`:

- `selected_range`, `selection_reversed` e `selected_word_range` são privados;
- não existe setter público de range de seleção;
- o `element.rs`, que **pinta** a seleção, lê esses campos direto.

Ou seja, nenhuma composição por fora resolve. A alternativa era arrancar o núcleo de edição
(`state.rs` + `element.rs` + rope/wrapping ≈ 5–6k LOC, mais `ropey`/`smallvec`/
`unicode-segmentation`) para dentro do nosso crate e herdar a manutenção de IME, grafemas e
quebra de linha. O fork com patch pequeno é bem mais barato e reversível.

Não é possível resolver atualizando a dependência: **o HEAD do upstream tem as mesmas lacunas**
(mesmo mecanismo de clamp em `state.rs`), e além disso depende do `gpui` via git da Zed em vez do
`gpui 0.2.2` do nosso workspace.

## Como está ligado

`Cargo.toml` da raiz:

```toml
[patch.crates-io]
gpui-component = { path = "vendor/gpui-component" }
```

## O que foi mudado

Referência de comportamento: o `SelectionController` do Blink (Chromium) — usado como
**especificação**, não como código.

### `src/input/cursor.rs`
- **Novo** `enum SelectionGranularity { Character, Word, Paragraph }` — a unidade em que a
  seleção cresce, no espírito do `TextGranularity` do Blink.

### `src/input/selection.rs`
- **Novo** `word_range_at(&Rope, offset)` — reexpõe o `TextSelector::word_range`, que já existia
  mas era privado do módulo, para o `state.rs` poder arredondar pela palavra.

### `src/input/state.rs`
1. **Novos campos** `selection_granularity` e `pending_caret`.
2. **`on_mouse_down`** — três regras:
   - *Clique simples dentro de uma seleção não a colapsa.* Guarda o offset em `pending_caret` e
     retorna; a seleção sobrevive para o gesto ainda poder virar arraste — ou o 2º clique de um
     duplo clique. (No Blink: `mouse_down_was_single_click_in_selection_`.)
   - *Granularidade pelo número de cliques*: 1 → `Character`, 2 → `Word`, 3+ → `Paragraph`. Um
     clique com `shift` **não** redefine a granularidade — estende com a que já vale.
   - *Triplo clique* → `select_paragraph` (trecho entre `\n`; num campo de uma linha, o conteúdo
     todo). Antes o 3º clique caía no `move_to` e **colapsava** a seleção.
3. **`select_to`** — passa a chamar `snap_selection_to_granularity`, que expande as **duas** pontas
   até a fronteira da unidade corrente. O código original só impedia a seleção de encolher abaixo
   da palavra âncora (clamp), sem arredondar a ponta que se move — daí arrastar até o meio de uma
   palavra selecionar meia palavra. Expande, nunca encolhe.
4. **`on_mouse_up`** — deixou de limpar `selected_word_range` (era isso que matava a granularidade
   no fim do duplo clique, fazendo o shift+clique seguinte voltar a andar por caractere). A limpeza
   passou para o `on_mouse_down` de um clique simples, que é quando o usuário de fato começa uma
   seleção nova. Também aplica o `pending_caret`, se não houve arraste.
5. **`handle_mouse_move`** — se havia `pending_caret` e o gesto virou arraste, colapsa no ponto
   clicado e segue estendendo (não implementamos arrastar-e-soltar de texto).
6. **Novos** `select_paragraph`, `paragraph_range` e `snap_selection_to_granularity`.

### `src/input/state.rs` — seleção total chamável de fora

`select_all` é handler de ação (`pub(super)`, assinatura com `&SelectAll`), então de fora do crate
só se alcança despachando a ação — e despachar não serve pra selecionar em resposta a foco: no
instante em que o evento é entregue, a árvore de despacho ainda não reflete o foco novo e a ação não
chega em ninguém (medido: o campo ficava com `100%5` em vez de `5`). `selected_range` é privado e não
há setter público de range, a mesma lacuna que motivou este fork.

Acrescentado `pub fn select_all_now(&mut self, cx)`, e o handler da ação passou a chamá-lo. É
adição, não mudança de comportamento: nada que já funcionava muda de resultado.

Quem usa: `empire_ui::ColorPicker`, onde focar qualquer campo seleciona o conteúdo (os campos
mostram valores derivados da cor, e quem clica neles quer substituir o número, não editar um dígito
no meio de `#FF8000`).

### `src/input/state.rs` — máscara de senha
7. **Novo** `pub fn is_masked()`. O crate tinha `masked()`/`set_masked()` e nenhum getter, então um
   botão de mostrar/esconder construído FORA do crate não tinha como saber o que alternar — só dava
   pra fazer press-and-hold (que é o que o `render_toggle_mask_button` do crate faz). Com o getter,
   a apresentação implementa um toggle de verdade.

### `src/input/input.rs` — métricas de tamanho
8. **Novo** `bare_metrics()` + campo `bare_metrics`. Quando ligado, o campo **não** aplica
   `input_px`/`input_py`/`input_h`/`input_text_size` do `Size` dele. Sem isso, quem desenha a
   própria moldura em volta briga com a geometria do crate: no `Large` ele impõe 44px de altura e
   10px de padding vertical, e o texto sai do centro vertical de uma moldura de 36px.
9. **`height` passou a valer em modo de UMA LINHA.** Era honrado só em multi-linha
   (`when(is_multi_line, |this| this.when_some(self.height, ...))`), então um `h_full()` de quem
   chama era **ignorado em silêncio** num campo de uma linha — o `input_h` vencia. Agora o
   `when_some(self.height, ...)` está fora daquele `when`.

### `src/input/element.rs` — altura do caret
10. **A altura do caret passou a vir da FONTE, não da altura de linha.** O original usava uma
    fração da `line_height` (0.85 no `Medium`), o que só funciona quando o próprio campo dita a
    altura de linha. Quem desenha a moldura por fora costuma mandar uma `line_height` grande — no
    nosso caso igual à altura do miolo, que é como o design system centraliza o texto — e aí o
    caret crescia junto até ocupar o campo inteiro. Navegador e campo nativo dimensionam o caret
    pelas métricas da fonte; agora é `1.2 × corpo`, limitado pela altura de linha. A função já
    recebia o `Window` e o ignorava (`_: &mut Window`).

### `src/resizable/mod.rs` — tamanho de painel em pixel inteiro

11. **`resize_panel` arredonda o tamanho pedido pra pixel inteiro.** É o que o Zed faz no próprio
    código de painéis (`pane_group.rs` usa `.map(|d| d.round())` no resize; `dock.rs`, um
    `size.round()`), e a razão é desempenho: o mouse entrega posição fracionária, então sem
    arredondar cada movimento subpixel produzia um tamanho novo — e portanto um relayout do taffy
    inteiro, que é o caminho mais caro de um frame nesta base (medido com `/usr/bin/sample`: 2800 de
    4332 amostras da main thread, 64,6%, dentro de `taffy::compute_layout`). Meio pixel de painel
    não é exibível, então não se perde gesto nenhum.

    ⚠️ **Não fecha o ciclo, e isso está declarado no código.** O atalho `move_changed == px(0.)`
    compara com os tamanhos GRAVADOS, e o `sync_real_panel_sizes` os relê dos bounds reais do taffy,
    que são fracionários. O arredondamento garante o lado do PEDIDO; garantir o no-op em todo frame
    pediria arredondar também os bounds lidos, o que mudaria a geometria e não só o desempenho —
    fora do escopo de um patch de performance.

    Este é o único patch do fork que **não** é sobre o campo de texto.

### `src/resizable/resize_handle.rs` + `src/resizable/panel.rs` — a barra da alça vira opcional

12. **`ResizeHandle` ganhou `.bar(bool)`, e o `resizable/panel.rs` passou a chamar `.bar(false)`.** A
    alça pintava um `div().bg(theme().border)` de 1px por conta própria, SEMPRE. O `resizable` do
    `empire-ui` desenha a divisa dele no mesmo pixel — com cor, realce de foco e realce de arraste
    vindos dos tokens da casa — então eram duas linhas empilhadas, e a de baixo continuava visível
    mesmo quando a de cima era escondida de propósito (`Resizable::separator_hidden`). Não havia como
    calá-la de fora do crate: `resize_handle`, `HANDLE_SIZE` e `HANDLE_PADDING` são `pub(crate)`.

    Medido antes e depois na mesma coluna de pixels do shell do storybook: a divisa marcava 34 com a
    barra ligada e 31 com ela desligada, sendo **31 exatamente o valor do layout sem o componente** —
    a faixa ficou pixel a pixel idêntica (diferença máxima 0 numa tira de 30×1570).

    O padrão continua `true`, então o **`dock` deste crate — o outro chamador de `resize_handle` — não
    muda em nada.**

## Nota: o que o fork NÃO precisou resolver

Duas lacunas visuais do campo foram fechadas **sem** tocar no fork, e vale registrar pra ninguém
inflar este patch por engano:

- **`transition-shadow`** (o fade do halo de foco) é feito no `empire-ui` com o `with_animation`
  do próprio GPUI — a engine tem animação por frame, só não tem transição declarativa de estilo.
- **O fio de bisel** é um filho absoluto recuado em 1px, também no `empire-ui`.

## Nota sobre a leitura da fonte do Blink

O `HandleDoubleClick` tem um retorno-cedo quando a seleção já é um range, e numa primeira leitura
isso parece significar "duplo clique sobre texto selecionado não muda a seleção". **Não é isso**:
o mouse-up do 1º clique já colapsou a seleção, então o 2º clique encontra seleção vazia e
seleciona a palavra — que é o comportamento observável do Chrome. Chegamos a implementar a leitura
errada e o teste a derrubou. Ver
`crates/empire-ui/tests/selection_parity.rs::duplo_clique_sobre_texto_selecionado_seleciona_a_palavra`.

## Rede de proteção

`crates/empire-ui/tests/selection_parity.rs` abre uma janela headless, simula cliques/arrastes com
`click_count` exato e verifica **cada** regra acima. É o que impede o patch de regredir num
upgrade.

## Ao atualizar o `gpui-component`

1. Substitua este diretório pela nova versão do crates.io.
2. Reaplique as mudanças acima (procure por `FORK (Fennel):` no diretório antigo — `git log` deste
   caminho também mostra o diff).
3. Rode `cargo test -p empire-ui`. Se as regras já vierem do upstream, **remova o patch e o
   `[patch.crates-io]`** e mantenha só os testes.

## Interview Companion: intrinsic Markdown layout (2026-10-05)

This app-local copy originates from `promo-video-editor/vendor/gpui-component`.
The Fennel checkout is unchanged.

- `src/text/text_view.rs`: use intrinsic height in the non-virtualized text view
  when an external ScrollArea owns scrolling; retain full height for internal
  virtualized scrolling.
- `src/text/node.rs`: list rows fill the available width, and the text cell uses
  the remaining width with `flex_1` / `min_w_0`. This prevents text from collapsing
  to zero width inside an intrinsically sized Markdown document.

### Interview Companion: editor nativo de blocos

- `InputState::block_mode`, eventos de Enter, Backspace/Delete nos limites, navegação e desfazer/refazer permitem ao documento controlar operações entre blocos. IME em composição continua no campo nativo. Shift Enter mantém a quebra de linha.
- `selection_range` / `set_byte_selection` expõem seleção em bytes UTF-8 com limites Unicode válidos.
- `set_inline_highlights` e `TextElement::highlight_lines` aplicam estilos de texto rico também fora do modo de código. Os intervalos cobrem o conteúdo e são recortados à área visível; estilos desatualizados voltam ao desenho normal.
- O comportamento novo é opt-in. Os inputs comuns do Fennel mantêm seu comportamento.

- `TextWrapper::wrap_rich_text`: mede quebras com os trechos de fonte reais (negrito/itálico), via `TextSystem::shape_text`; evita ultrapassar a largura disponível quando os estilos mudam a largura dos glifos.

- `TextElement::paint` só pede outro frame quando a geometria muda; edição, seleção e cursor já notificam suas próprias mudanças. Evita repintura contínua de todos os blocos quando o documento está parado.

### Companion: seleção e exclusão

- `InputEvent::SelectionChanged` notifica a mudança de intervalo após a pintura do input, para mostrar as ferramentas somente quando houver seleção de texto.
- Backspace/Delete com cursor usam intervalos explícitos de caracteres, sem reutilizar `select_to`, que expande seleções conforme a granularidade do mouse. Evita apagar um título inteiro após seleção por palavra/parágrafo.
- `set_byte_selection` limpa âncoras e granularidade antigas; `delete_backward` expõe o mesmo handler nativo para operações e regressões sem eventos de teclado do sistema. `select_paragraph` expõe a seleção nativa de parágrafo e define sua granularidade, também usada na regressão de exclusão após clique triplo e digitação.

### Companion: posição da toolbar de seleção

- `InputState::selection_first_line_bounds` expõe coordenadas da primeira linha visual selecionada, medidas pelo layout de texto já moldado. Considera fontes ricas, quebras de linha e a posição do input na janela, permitindo ancorar a toolbar diretamente acima do trecho selecionado.

### Companion: seleção contínua entre blocos

- `InputState::byte_offset_for_point` expõe o hit test nativo em bytes UTF-8, respeitando o texto moldado e a quebra de linhas.
- `cancel_mouse_selection` transfere um gesto de seleção para o documento quando ele atravessa inputs, sem deixar o campo inicial reescrever o intervalo global. O comportamento normal dos inputs permanece igual.

- `InputEvent::BlockSelectionStart` informa a posição UTF-8 exata do clique no campo nativo para fixar a âncora do documento, inclusive no meio de palavras e de linhas quebradas. A seleção passa a ser controlada pelo documento já durante o movimento dentro do bloco inicial, antes de atravessar uma borda.

- O início de seleção informa também o número de cliques, para armar o gesto em cliques simples, duplos e triplos. `selection_unit_for_offset` reutiliza os limites nativos de palavra/parágrafo; o documento mantém essa unidade ao atravessar blocos e inverter o sentido do arraste.
- O teste nativo assíncrono devolve o controle ao event loop entre mouse-down, movimentos e mouse-up, verificando a entrega real dos eventos de foco/seleção nos seis casos (três tipos de clique × dois sentidos), sem publicar eventos de mouse no sistema.

# empire-ui

Biblioteca de **componentes de UI para apps desktop em [GPUI]**, extraída do editor de vídeo
Fennel (`pve-ui`) — onde estes controles nasceram e continuam em uso.

O crate é **agnóstico de domínio**: depende só de `gpui` e `gpui-component` (mais `serde_json`,
usado só pra persistir as cores recentes do color picker). Nenhum crate `pve-*`.

[GPUI]: https://docs.rs/gpui

## Componentes

| Componente | O que é |
|---|---|
| `ScrubInput` | Controle numérico estilo Figma: digita o valor **ou** arrasta pelo ícone (scrub) pra mudá-lo ao vivo. Faixa/passo configuráveis, modo keyframe-able. |
| `ColorPicker` | Painel de cor completo: área SV, matiz, alpha, hex, paleta e recentes persistidas. |
| `Select` | Dropdown 1-de-N com menu flutuante. Variantes: padrão, com ícone, borderless. |
| `Checkbox` | Booleano com rótulo e área de clique generosa. |
| `Switch` | Toggle em pílula (trilho + bolinha deslizante). |

Todos são `Entity` (views) próprios que mantêm o próprio estado e **emitem eventos**
(`CheckboxEvent::Toggle`, `SelectEvent::Change`, `ScrubInputEvent::Changed`, …). Quem usa
assina com `cx.subscribe` e grava onde quiser — sem callback acoplado.

## Infra

- **`theme`** — design tokens claro/escuro. Chame `theme::set_theme(mode)` **uma vez por frame**,
  no topo do `render` do seu shell; todos os tokens resolvem nesse modo.
- **`assets`** — os ícones SVG embutidos, servidos como `icons/<nome>.svg` por uma
  `gpui::AssetSource`.
- **`iconoir`** — o [Iconoir] **completo** embutido (v7.11.1, MIT), servido pela mesma
  `AssetSource`. Veja abaixo.
- **`PaintLatencyMeter`** — instrumentação de latência de frame.

[Iconoir]: https://iconoir.com

## Ícones

Dois conjuntos, em namespaces separados:

| Namespace | Quantos | O que é |
|---|---|---|
| `icons/<nome>.svg` | 89 | Ícones nativos, desenhados pro editor. Usados pelos componentes deste crate. |
| `iconoir/regular/<nome>.svg` | 1383 | Iconoir traço (`stroke="currentColor"`). |
| `iconoir/solid/<nome>.svg` | 288 | Iconoir preenchido (`fill="currentColor"`). |

```rust
svg().path("iconoir/regular/heart.svg").size(px(18.)).text_color(rgb(0xffffff))
```

Os 288 nomes de `solid` existem **todos** também em `regular` — por isso a variante entra no
caminho. Não é preciso resolver o `currentColor`: o GPUI rasteriza o SVG e usa o resultado como
**máscara de alfa**, pintada com a `text_color` do elemento.

Pra descobrir/validar nomes em runtime (ex.: quando o ícone vem de config, não do código):

```rust
empire_ui::iconoir::regular_names()      // iterador dos 1383 nomes, sem prefixo nem .svg
empire_ui::iconoir::has("iconoir/solid/heart.svg")   // valida antes de renderizar
```

> Um caminho errado **não** gera erro no GPUI — o ícone simplesmente não aparece. Daí o `has`.

Pra navegar os 1671 visualmente, com busca: `cargo run -p empire-ui-storybook -- iconoir`.

### Licença do Iconoir

Iconoir é MIT, © 2021 Luca Burgio. A licença está vendorizada em `assets/iconoir/LICENSE` e
**precisa acompanhar qualquer redistribuição** deste crate.

### Atualizar o Iconoir

Os SVGs ficam em `assets/iconoir/{regular,solid}/` e as tabelas em `src/iconoir.rs` são
**geradas** a partir deles, ordenadas por byte (`LC_ALL=C sort`) — a ordenação é o que autoriza a
busca binária do `lookup`. Troque os SVGs, regenere as tabelas na mesma ordem e rode os testes: o
`nenhum_arquivo_do_disco_ficou_de_fora` compara tabela × diretório, então um ícone esquecido
quebra o teste em vez de sumir calado da UI.

## Uso

```toml
[dependencies]
# O crate é membro do workspace do repo — dá pra depender direto do git, sem publicar:
empire-ui = { git = "https://github.com/<você>/promo-video-editor" }
```

```rust
use gpui::{Application, WindowOptions};

Application::new()
    // ⚠️ Sem `with_assets`, o `gpui::svg()` não resolve `icons/*.svg` e os ícones somem
    // SILENCIOSAMENTE (o gpui devolve `Ok(None)` e não loga nada).
    .with_assets(empire_ui::assets::Assets)
    .run(|cx| {
        gpui_component::init(cx); // os `Input` internos do ScrubInput/ColorPicker dependem disto
        cx.open_window(WindowOptions::default(), |window, cx| {
            let view = cx.new(|cx| MinhaView::new(window, cx));
            cx.new(|cx| gpui_component::Root::new(view, window, cx)) // camadas de popover
        })
        .unwrap();
    });
```

No `render` da sua view:

```rust
theme::set_theme(self.mode); // antes de montar a UI
div().bg(gpui::rgb(theme::BG_PANEL())).child(self.meu_select.clone())
```

### Assets próprios junto com os da lib

O GPUI aceita **uma** `AssetSource` por aplicação. Se a sua app tem assets próprios, implemente
a sua e delegue o resto pra `empire_ui::assets::lookup` / `list_prefix` — é o que o `pve-ui` faz
(veja `crates/pve-ui/src/assets.rs`):

```rust
impl AssetSource for MeusAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        let found = MINHAS_TABELAS.iter().flat_map(|t| t.iter())
            .find(|(p, _)| *p == path).map(|(_, b)| *b)
            .or_else(|| empire_ui::assets::lookup(path));
        Ok(found.map(Cow::Borrowed))
    }
    // `list` idem, concatenando com `empire_ui::assets::list_prefix(path)`.
}
```

### Tokens próprios no mesmo tema

Não infle a `Palette` da lib com tokens da sua app. Declare o seu struct e resolva por
`theme::theme_mode()` — assim um único `set_theme` acerta os dois níveis:

```rust
struct MinhaPaleta { linha: u32 }
const MINHA_DARK: MinhaPaleta = MinhaPaleta { linha: 0x8a8a8a };
const MINHA_LIGHT: MinhaPaleta = MinhaPaleta { linha: 0x52525b };

fn minha() -> &'static MinhaPaleta {
    match empire_ui::theme::theme_mode() {
        empire_ui::theme::ThemeMode::Dark => &MINHA_DARK,
        empire_ui::theme::ThemeMode::Light => &MINHA_LIGHT,
    }
}
```

O `pve-ui` é o exemplo real disso: `crates/pve-ui/src/theme.rs` re-exporta os tokens genéricos
e adiciona os do editor (`TL_*`, `CLIP_*`) num `EditorPalette` separado.

## Storybook

Pra inspecionar os componentes isolados — todos os estados, os dois temas, e um log dos eventos
que cada um emite:

```
cargo run -p empire-ui-storybook                    # abre no Overview
cargo run -p empire-ui-storybook -- iconoir         # abre direto numa seção
cargo run -p empire-ui-storybook -- iconoir solid   # ...e numa variante do Iconoir
```

Seções: `overview`, `scrubinput`, `colorpicker`, `select`, `checkbox`, `switch`, `tokens`,
`icons`, `iconoir`.

## Convenção de cores

- **Opacas** são `u32` (`0xRRGGBB`), usadas com `gpui::rgb(...)`.
- **Com alpha** vêm de helpers que já devolvem `Rgba` (`theme::border_subtle()`), usadas direto.

## Licença

AGPL-3.0-or-later (igual ao resto do workspace).

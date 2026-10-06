//! `empire-ui` — biblioteca de **componentes de UI** para apps desktop em [GPUI].
//!
//! Extraída do editor de vídeo Fennel (`pve-ui`), onde estes controles nasceram e continuam
//! sendo usados em produção. O crate é **agnóstico de domínio**: depende só de `gpui` e
//! `gpui-component`, então serve qualquer app GPUI.
//!
//! [GPUI]: https://docs.rs/gpui
//!
//! # Componentes
//!
//! - [`ScrubInput`] — controle numérico estilo Figma: digitar o valor **e** arrastar pelo
//!   ícone (scrub) pra mudá-lo ao vivo. O cavalo de batalha de um inspector de propriedades.
//! - [`ColorPicker`] — seletor de cor completo: área SV, matiz, alpha, campo hex e
//!   conta-gotas, em popover.
//! - [`Button`] — o botão: 7 variantes (`Default`, `Secondary`, `Outline`, `Ghost`, `Link`,
//!   `Destructive`, `DestructiveOutline`) × 10 tamanhos, incluindo os quadrados de ícone.
//! - [`Select`] — dropdown (escolha 1-de-N): campo clicável + menu flutuante (popover via
//!   `deferred`/`anchored`), emitindo [`SelectEvent::Change`].
//! - [`Checkbox`] — controle booleano (quadrado + rótulo + check), emitindo
//!   [`CheckboxEvent::Toggle`].
//! - [`RadioGroup`] — escolha 1-de-N em círculos: exclusividade pelo tipo, navegação por setas e
//!   UMA parada de `Tab` pro grupo inteiro (roving tabindex), emitindo [`RadioGroupEvent::Change`].
//! - [`Switch`] — toggle em pílula (trilho + bolinha que desliza), emitindo
//!   [`SwitchEvent::Toggle`].
//! - [`Toggle`] — botão de dois estados (o negrito de uma barra de ferramentas), controlado.
//! - [`ToggleGroup`] — botão segmentado: vários [`Toggle`] numa peça só, com seleção única ou
//!   múltipla, emitindo [`ToggleGroupEvent::Change`].
//! - [`InputGroup`] — a superfície do campo virada wrapper: addons nas quatro posições
//!   (esquerda, direita, e linhas de largura cheia acima/abaixo) dentro de UMA moldura, com um anel
//!   de foco só. Não confundir com [`Group`], que costura VÁRIAS superfícies.
//! - [`Table`] — tabela de dados em duas variantes (`Default` e `Card`), com colunas de largura
//!   declarada, seleção de linha vinda de fora e rolagem horizontal.
//! - [`Tooltip`] — etiqueta de ajuda ancorada no GATILHO (não no cursor, como a `.tooltip()` do
//!   GPUI), com 600ms de espera e fechamento imediato.
//! - [`Popover`] — a camada flutuante de CONTEÚDO LIVRE, aberta por clique num gatilho: a mesma
//!   superfície do menu (`bg-popover`, raio 10, sombra, bisel), com `w-72` de largura, `p-4` de
//!   respiro e as animações de entrada/saída por lado.
//!
//! # Infra
//!
//! - [`theme`] — os design tokens (claro/escuro) que todos os componentes leem. Chame
//!   [`theme::set_theme`] uma vez por frame, no topo do `render` do seu shell.
//! - [`assets`] — os ícones SVG embutidos, servidos via [`gpui::AssetSource`]. Registre
//!   [`assets::Assets`] no bootstrap da app.
//! - [`iconoir`] — o [Iconoir](https://iconoir.com) completo embutido (1383 regular + 288
//!   solid, MIT), servido pela mesma `AssetSource` em `iconoir/<variante>/<nome>.svg`.
//! - [`PaintLatencyMeter`] — instrumentação de latência de frame (diagnóstico de jank).
//!
//! # Bootstrap mínimo
//!
//! ```ignore
//! use gpui::{Application, WindowOptions};
//!
//! Application::new()
//!     .with_assets(empire_ui::assets::Assets) // sem isto, os ícones somem SILENCIOSAMENTE
//!     .run(|cx| {
//!         cx.open_window(WindowOptions::default(), |_, cx| {
//!             empire_ui::theme::set_theme(empire_ui::theme::ThemeMode::Dark);
//!             cx.new(|cx| MinhaView::new(cx))
//!         })
//!         .unwrap();
//!     });
//! ```
//!
//! Pra ver todos os componentes rodando, com os estados e variantes lado a lado, rode o
//! **storybook** deste repo: `cargo run -p empire-ui-storybook`.

pub mod assets;
pub(crate) mod color;

pub mod autocomplete;
pub mod avatar;
pub mod badge;
pub mod button;
pub mod calendar;
pub mod calendar_selection;
pub mod card;
pub mod checkbox;
pub mod color_picker;
pub mod combobox;
pub mod command;
pub mod context_menu;
pub mod date;
pub mod dialog;
pub mod empty;
pub mod eyedropper;
pub mod flow;
pub mod flow_layout;
pub mod focus_ring;
pub mod frame;
pub mod group;
pub mod iconoir;
pub mod input;
pub mod input_group;
pub mod kbd;
pub mod menu;
pub mod meter;
pub mod number_field;
pub mod otp_field;
pub mod paint_latency;
pub mod popover;
pub mod radio_group;
pub mod resizable;
pub mod scroll_area;
pub mod scrub_input;
pub mod select;
pub mod sheet;
pub mod slider;
pub mod switch;
pub mod table;
pub mod tabs;
pub mod theme;
pub mod toast;
pub mod toggle;
pub mod toggle_group;
pub mod tooltip;

pub use autocomplete::{Autocomplete, AutocompleteEvent, AutocompleteRow};
pub use avatar::{Avatar, AvatarFit, AvatarSize};
pub use button::{Button, ButtonSize, ButtonVariant};
pub use calendar::{Calendar, CalendarEvent};
pub use calendar_selection::{CalendarMode, Selection};
pub use card::{Card, CardFrame, CardHeader};
pub use checkbox::{Checkbox, CheckboxEvent};
pub use color_picker::{ColorPicker, ColorPickerEvent};
pub use combobox::{Combobox, ComboboxEvent, ComboboxItem, ComboboxMode};
pub use command::{command_dialog, Command, CommandEvent, CommandRow};
pub use flow::{AnchorKind, Flow, FlowList, FlowNode, FlowParallel};
pub use frame::{Frame, FrameFooter, FrameHeader, FramePanel};
pub use group::{Group, GroupChild, GroupText, Join, Orientation, SeparatorTone};
pub use input::{Input, InputSize, Validity};
pub use input_group::{AddonAlign, AddonItemKind, InputGroup, InputGroupAddon};
pub use kbd::{Kbd, KbdGroup};
pub use meter::{meter_label, meter_value, Meter};
pub use number_field::{NumberField, NumberFieldEvent};
pub use paint_latency::{LatencySummary, PaintLatencyMeter};
pub use popover::{Popover, PopoverDescription, PopoverEvent, PopoverTitle};
pub use radio_group::{RadioGroup, RadioGroupEvent, RadioGroupItem};
pub use resizable::{Resizable, ResizablePanel};
pub use scroll_area::{ScrollArea, ScrollAxis};
pub use scrub_input::{ScrubIcon, ScrubInput, ScrubInputEvent};
pub use select::{Select, SelectEvent, SelectSize};
pub use slider::{Slider, SliderEvent, SliderOrientation};
pub use switch::{Switch, SwitchEvent};
pub use table::{Table, TableAlign, TableColumn, TableColumnWidth, TableRow, TableVariant};
pub use theme::{set_theme, theme_mode, Palette, ThemeMode};
pub use toggle::{Toggle, ToggleSize, ToggleVariant};
pub use toggle_group::{ToggleGroup, ToggleGroupEvent, ToggleGroupItem, ToggleGroupMode};
pub use tooltip::Tooltip;
pub mod motion;

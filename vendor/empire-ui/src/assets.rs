//! Fonte de assets embutida do `empire-ui` — os **ícones SVG** da biblioteca.
//!
//! São dois conjuntos, em namespaces separados e servidos pela mesma [`Assets`]:
//!
//! - **`icons/<nome>.svg`** — os ícones NATIVOS ([`ICONS`]), desenhados pro editor Fennel e
//!   usados pelos componentes deste crate (check do `Checkbox`, alça do `ScrubInput`, …).
//! - **`iconoir/<variante>/<nome>.svg`** — o [Iconoir][crate::iconoir] completo (1671 ícones,
//!   MIT), pra você ter um conjunto amplo sem desenhar nada.
//!
//! O GPUI renderiza SVG via [`gpui::svg()`], que carrega os bytes do SVG por **caminho**
//! a partir de uma [`gpui::AssetSource`] registrada na aplicação. O crate `gpui-component`
//! **não** empacota ícones nem registra uma fonte, então a app precisa registrar uma. Este
//! módulo embute (em tempo de compilação, via [`include_bytes!`]) os SVGs usados pelos
//! componentes do `empire-ui` e os serve pelos caminhos `icons/<nome>.svg`.
//!
//! Os SVGs são desenhados como silhuetas: o renderer do GPUI usa o SVG como **máscara de
//! alfa** e pinta com a `text_color` do elemento — por isso a cor interna do SVG é
//! irrelevante (vale só o formato/contorno).
//!
//! # Uso
//!
//! No caso simples, registre a [`Assets`] direto no bootstrap:
//!
//! ```ignore
//! gpui::Application::new().with_assets(empire_ui::assets::Assets)
//! ```
//!
//! Se a sua app tem assets PRÓPRIOS (thumbnails, imagens de domínio), não fork esta tabela:
//! implemente a sua `AssetSource` e delegue o que não for seu pra [`lookup`]/[`ICONS`]:
//!
//! ```ignore
//! const MEUS: &[(&str, &[u8])] = &[("thumbs/a.png", include_bytes!("../assets/thumbs/a.png"))];
//!
//! impl AssetSource for MeusAssets {
//!     fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
//!         Ok(empire_ui::assets::lookup(path)
//!             .or_else(|| MEUS.iter().find(|(p, _)| *p == path).map(|(_, b)| *b))
//!             .map(Cow::Borrowed))
//!     }
//!     // `list` idem, concatenando as duas tabelas.
//! }
//! ```

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

/// Tabela `(caminho, bytes)` de TODOS os ícones embutidos, servidos como `icons/<nome>.svg`.
///
/// Pública porque uma app que implementa a própria [`AssetSource`] precisa concatenar esta
/// tabela com a sua (ver o doc do módulo). Ordenada alfabeticamente pelo nome do arquivo.
pub const ICONS: &[(&str, &[u8])] = &[
    ("icons/align_bottom.svg", include_bytes!("../assets/icons/align_bottom.svg")),
    ("icons/align_center_h.svg", include_bytes!("../assets/icons/align_center_h.svg")),
    ("icons/align_center_v.svg", include_bytes!("../assets/icons/align_center_v.svg")),
    ("icons/align_justify.svg", include_bytes!("../assets/icons/align_justify.svg")),
    ("icons/align_left.svg", include_bytes!("../assets/icons/align_left.svg")),
    ("icons/align_right.svg", include_bytes!("../assets/icons/align_right.svg")),
    ("icons/align_top.svg", include_bytes!("../assets/icons/align_top.svg")),
    ("icons/blend.svg", include_bytes!("../assets/icons/blend.svg")),
    ("icons/border_sides.svg", include_bytes!("../assets/icons/border_sides.svg")),
    ("icons/border_width.svg", include_bytes!("../assets/icons/border_width.svg")),
    ("icons/box.svg", include_bytes!("../assets/icons/box.svg")),
    ("icons/box_frame.svg", include_bytes!("../assets/icons/box_frame.svg")),
    ("icons/check.svg", include_bytes!("../assets/icons/check.svg")),
    ("icons/chevron_down.svg", include_bytes!("../assets/icons/chevron_down.svg")),
    ("icons/chevron_left.svg", include_bytes!("../assets/icons/chevron_left.svg")),
    ("icons/chevron_right.svg", include_bytes!("../assets/icons/chevron_right.svg")),
    ("icons/chevron_up.svg", include_bytes!("../assets/icons/chevron_up.svg")),
    ("icons/chromatic.svg", include_bytes!("../assets/icons/chromatic.svg")),
    ("icons/circle_dot.svg", include_bytes!("../assets/icons/circle_dot.svg")),
    ("icons/circle_help.svg", include_bytes!("../assets/icons/circle_help.svg")),
    ("icons/clip_handle.svg", include_bytes!("../assets/icons/clip_handle.svg")),
    ("icons/copy.svg", include_bytes!("../assets/icons/copy.svg")),
    ("icons/corner_bl.svg", include_bytes!("../assets/icons/corner_bl.svg")),
    ("icons/corner_br.svg", include_bytes!("../assets/icons/corner_br.svg")),
    ("icons/corner_radius.svg", include_bytes!("../assets/icons/corner_radius.svg")),
    ("icons/corner_tl.svg", include_bytes!("../assets/icons/corner_tl.svg")),
    ("icons/corner_tr.svg", include_bytes!("../assets/icons/corner_tr.svg")),
    ("icons/droplet.svg", include_bytes!("../assets/icons/droplet.svg")),
    ("icons/ease_curve.svg", include_bytes!("../assets/icons/ease_curve.svg")),
    ("icons/eye.svg", include_bytes!("../assets/icons/eye.svg")),
    ("icons/eye_off.svg", include_bytes!("../assets/icons/eye_off.svg")),
    ("icons/file.svg", include_bytes!("../assets/icons/file.svg")),
    ("icons/film.svg", include_bytes!("../assets/icons/film.svg")),
    ("icons/folder.svg", include_bytes!("../assets/icons/folder.svg")),
    ("icons/folder_open.svg", include_bytes!("../assets/icons/folder_open.svg")),
    ("icons/folder_plus.svg", include_bytes!("../assets/icons/folder_plus.svg")),
    ("icons/globe.svg", include_bytes!("../assets/icons/globe.svg")),
    ("icons/glyph_x.svg", include_bytes!("../assets/icons/glyph_x.svg")),
    ("icons/glyph_y.svg", include_bytes!("../assets/icons/glyph_y.svg")),
    ("icons/glyph_z.svg", include_bytes!("../assets/icons/glyph_z.svg")),
    ("icons/graph_curve.svg", include_bytes!("../assets/icons/graph_curve.svg")),
    ("icons/grip_horizontal.svg", include_bytes!("../assets/icons/grip_horizontal.svg")),
    ("icons/grip_vertical.svg", include_bytes!("../assets/icons/grip_vertical.svg")),
    ("icons/hexagon.svg", include_bytes!("../assets/icons/hexagon.svg")),
    ("icons/home.svg", include_bytes!("../assets/icons/home.svg")),
    ("icons/image.svg", include_bytes!("../assets/icons/image.svg")),
    ("icons/kf_add.svg", include_bytes!("../assets/icons/kf_add.svg")),
    ("icons/kf_diamond.svg", include_bytes!("../assets/icons/kf_diamond.svg")),
    ("icons/kf_diamond_filled.svg", include_bytes!("../assets/icons/kf_diamond_filled.svg")),
    ("icons/layout_grid.svg", include_bytes!("../assets/icons/layout_grid.svg")),
    ("icons/lens_dispersion.svg", include_bytes!("../assets/icons/lens_dispersion.svg")),
    ("icons/lens_distort.svg", include_bytes!("../assets/icons/lens_distort.svg")),
    ("icons/lock.svg", include_bytes!("../assets/icons/lock.svg")),
    ("icons/maximize.svg", include_bytes!("../assets/icons/maximize.svg")),
    ("icons/minus.svg", include_bytes!("../assets/icons/minus.svg")),
    ("icons/moon.svg", include_bytes!("../assets/icons/moon.svg")),
    ("icons/music.svg", include_bytes!("../assets/icons/music.svg")),
    ("icons/opacity.svg", include_bytes!("../assets/icons/opacity.svg")),
    ("icons/outline_offset.svg", include_bytes!("../assets/icons/outline_offset.svg")),
    ("icons/pause.svg", include_bytes!("../assets/icons/pause.svg")),
    ("icons/pen_tool.svg", include_bytes!("../assets/icons/pen_tool.svg")),
    ("icons/percent.svg", include_bytes!("../assets/icons/percent.svg")),
    ("icons/pipette.svg", include_bytes!("../assets/icons/pipette.svg")),
    ("icons/play.svg", include_bytes!("../assets/icons/play.svg")),
    ("icons/play_fill.svg", include_bytes!("../assets/icons/play_fill.svg")),
    ("icons/plus.svg", include_bytes!("../assets/icons/plus.svg")),
    ("icons/plus_thin.svg", include_bytes!("../assets/icons/plus_thin.svg")),
    ("icons/radius.svg", include_bytes!("../assets/icons/radius.svg")),
    ("icons/repeat.svg", include_bytes!("../assets/icons/repeat.svg")),
    ("icons/resize_corner.svg", include_bytes!("../assets/icons/resize_corner.svg")),
    ("icons/save.svg", include_bytes!("../assets/icons/save.svg")),
    ("icons/scan.svg", include_bytes!("../assets/icons/scan.svg")),
    ("icons/scissors.svg", include_bytes!("../assets/icons/scissors.svg")),
    ("icons/scrub.svg", include_bytes!("../assets/icons/scrub.svg")),
    ("icons/search.svg", include_bytes!("../assets/icons/search.svg")),
    ("icons/skip_back.svg", include_bytes!("../assets/icons/skip_back.svg")),
    ("icons/skip_forward.svg", include_bytes!("../assets/icons/skip_forward.svg")),
    ("icons/square.svg", include_bytes!("../assets/icons/square.svg")),
    ("icons/square_dot.svg", include_bytes!("../assets/icons/square_dot.svg")),
    ("icons/strength.svg", include_bytes!("../assets/icons/strength.svg")),
    ("icons/sun.svg", include_bytes!("../assets/icons/sun.svg")),
    ("icons/threshold.svg", include_bytes!("../assets/icons/threshold.svg")),
    ("icons/trash.svg", include_bytes!("../assets/icons/trash.svg")),
    ("icons/type.svg", include_bytes!("../assets/icons/type.svg")),
    ("icons/unlock.svg", include_bytes!("../assets/icons/unlock.svg")),
    ("icons/upload.svg", include_bytes!("../assets/icons/upload.svg")),
    ("icons/video_file.svg", include_bytes!("../assets/icons/video_file.svg")),
    ("icons/volume.svg", include_bytes!("../assets/icons/volume.svg")),
    ("icons/volume_x.svg", include_bytes!("../assets/icons/volume_x.svg")),
    ("icons/x.svg", include_bytes!("../assets/icons/x.svg")),
    ("icons/zoom_in.svg", include_bytes!("../assets/icons/zoom_in.svg")),
];

/// Resolve um caminho de asset: primeiro os ícones nativos ([`ICONS`]), depois o
/// [Iconoir][crate::iconoir] (`iconoir/<variante>/<nome>.svg`). `None` se não existir.
///
/// É o gancho pra uma app **estender** o conjunto de assets sem duplicar as tabelas: a
/// `AssetSource` da app tenta a sua tabela e cai aqui no fallback (ou vice-versa).
///
/// Busca **binária** nas duas tabelas (ambas ordenadas por caminho, com teste que garante).
/// A linear era aceitável com 89 ícones; com os 1671 do Iconoir não é — o `gpui::svg()`
/// resolve o asset a cada paint, então uma tela cheia de ícones multiplicaria o custo.
pub fn lookup(path: &str) -> Option<&'static [u8]> {
    bsearch(ICONS, path).or_else(|| crate::iconoir::lookup(path))
}

/// Busca binária num par `(caminho, bytes)` ordenado por caminho.
pub(crate) fn bsearch(
    table: &'static [(&'static str, &'static [u8])],
    path: &str,
) -> Option<&'static [u8]> {
    table
        .binary_search_by(|(p, _)| (*p).cmp(path))
        .ok()
        .map(|i| table[i].1)
}

/// Lista os caminhos servidos por este módulo que começam com `prefix` — ícones nativos
/// **e** Iconoir.
pub fn list_prefix(prefix: &str) -> Vec<SharedString> {
    ICONS
        .iter()
        .chain(crate::iconoir::REGULAR)
        .chain(crate::iconoir::SOLID)
        .filter(|(p, _)| p.starts_with(prefix))
        .map(|(p, _)| SharedString::from(*p))
        .collect()
}

/// Fonte de assets do `empire-ui` — serve os SVGs de [`ICONS`] e do
/// [Iconoir][crate::iconoir] por caminho.
///
/// Registre no bootstrap com `Application::new().with_assets(Assets)`. Se a sua app tem
/// assets próprios além dos ícones, veja o doc do módulo.
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(lookup(path).map(Cow::Borrowed))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(list_prefix(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// O `gpui::svg()` carrega os bytes do SVG chamando `AssetSource::load(path)` com
    /// EXATAMENTE o caminho passado em `svg().path(...)` (sem normalizar barra inicial nem
    /// prefixo). Se o casamento falhar, `load` devolve `Ok(None)` e o gpui **não renderiza
    /// nada e não loga erro** — o ícone some silenciosamente. Este teste garante que TODA
    /// entrada da tabela resolve pra bytes não-vazios (pega `include_bytes!` de arquivo vazio
    /// e chave com typo/extensão errada).
    #[test]
    fn toda_entrada_da_tabela_resolve_bytes_nao_vazios() {
        for (path, _) in ICONS {
            let bytes = Assets
                .load(path)
                .expect("load não deve falhar")
                .unwrap_or_else(|| panic!("asset ausente para o path {path:?}"));
            assert!(!bytes.is_empty(), "asset vazio para o path {path:?}");
        }
    }

    /// Toda chave é `icons/<nome>.svg`. Protege contra a regeneração da tabela esquecer a
    /// extensão (foi um erro real ao extrair o crate): sem o `.svg` o `svg().path()` da UI
    /// não casa e o ícone desaparece calado.
    #[test]
    fn toda_chave_tem_prefixo_icons_e_extensao_svg() {
        for (path, _) in ICONS {
            assert!(
                path.starts_with("icons/") && path.ends_with(".svg"),
                "chave fora do padrão `icons/<nome>.svg`: {path:?}"
            );
        }
    }

    /// A [`lookup`] usa **busca binária** em [`ICONS`] — o que só é correto se a tabela estiver
    /// ordenada por caminho. Fora de ordem, ícones somem de forma intermitente (a busca erra
    /// dependendo de onde o item caiu), que é péssimo de diagnosticar olhando a tela.
    #[test]
    fn tabela_de_icones_ordenada_por_caminho() {
        let paths: Vec<&str> = ICONS.iter().map(|(p, _)| *p).collect();
        let mut ordenado = paths.clone();
        ordenado.sort_unstable();
        assert_eq!(paths, ordenado, "ICONS fora de ordem — a busca binária vai errar");
    }

    /// Os ícones do Iconoir são alcançáveis pela `AssetSource` registrada na app (o `lookup`
    /// cai neles quando o caminho não é de um ícone nativo).
    #[test]
    fn assets_serve_tambem_o_iconoir() {
        for path in [
            "iconoir/regular/heart.svg",
            "iconoir/solid/heart.svg",
            "iconoir/regular/accessibility-sign.svg",
        ] {
            let bytes = Assets
                .load(path)
                .expect("load não deve falhar")
                .unwrap_or_else(|| panic!("iconoir não alcançável via Assets: {path}"));
            assert!(!bytes.is_empty(), "SVG vazio: {path}");
        }
    }

    /// Os dois namespaces não se cruzam: `icons/` (nativos) e `iconoir/` são disjuntos, então
    /// nenhum `list`/`lookup` de um pega o outro por engano. (`"iconoir/"` não começa com
    /// `"icons/"` — diferem no 5º byte — mas convém amarrar, já que os nomes são parecidos.)
    #[test]
    fn namespaces_nativo_e_iconoir_sao_disjuntos() {
        assert!(ICONS.iter().all(|(p, _)| !p.starts_with("iconoir/")));
        assert!(crate::iconoir::REGULAR
            .iter()
            .chain(crate::iconoir::SOLID)
            .all(|(p, _)| !p.starts_with("icons/")));
        // `list("icons/")` continua trazendo só os nativos, apesar dos 1671 do Iconoir.
        assert_eq!(Assets.list("icons/").unwrap().len(), ICONS.len());
    }

    /// Nenhum caminho duplicado — duplicata significa que a segunda entrada é morta (a busca
    /// devolve só uma) e mascara um ícone que se pensava ter trocado.
    #[test]
    fn nenhum_caminho_duplicado() {
        let mut paths: Vec<&str> = ICONS.iter().map(|(p, _)| *p).collect();
        let total = paths.len();
        paths.sort_unstable();
        paths.dedup();
        assert_eq!(total, paths.len(), "há caminhos duplicados em ICONS");
    }

    /// Alguns ícones são referenciados por NOME por componentes deste crate. Se um deles
    /// sair da tabela, o componente perde o ícone silenciosamente — então amarramos aqui.
    #[test]
    fn icones_exigidos_pelos_componentes_existem() {
        for path in [
            "icons/scrub.svg",        // ScrubInput (alça de arraste)
            "icons/check.svg",        // Checkbox
            "icons/chevron_down.svg", // Select (seta do dropdown)
            "icons/chevron_up.svg",
            "icons/pipette.svg", // ColorPicker (conta-gotas)
            "icons/copy.svg",    // ColorPicker (hex -> clipboard)
            "icons/percent.svg", // ColorPicker (sufixo do field de opacidade)
        ] {
            assert!(lookup(path).is_some(), "ícone exigido ausente: {path}");
        }
    }

    /// Caminho inexistente devolve `Ok(None)` (não erro).
    #[test]
    fn load_path_inexistente_e_none() {
        assert!(Assets.load("icons/nope.svg").unwrap().is_none());
    }

    /// `list` filtra por prefixo — é como a app descobre o conjunto disponível.
    #[test]
    fn list_filtra_por_prefixo() {
        assert_eq!(Assets.list("icons/").unwrap().len(), ICONS.len());
        assert!(Assets.list("nada/").unwrap().is_empty());
    }
}

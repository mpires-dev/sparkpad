#![allow(dead_code, unused_imports)]
//! Native font shaping and input regressions; all events stay inside an invisible window.
#[cfg(target_os = "macos")]
#[path = "../src/app_theme.rs"]
mod app_theme;
#[cfg(target_os = "macos")]
#[path = "../src/assets.rs"]
mod assets;
#[cfg(target_os = "macos")]
#[path = "../src/block_editor.rs"]
mod block_editor;
#[cfg(target_os = "macos")]
#[path = "../src/blocks.rs"]
mod blocks;
#[cfg(target_os = "macos")]
#[path = "../src/covers.rs"]
mod covers;
#[cfg(target_os = "macos")]
#[path = "../src/macos.rs"]
mod macos;
#[cfg(target_os = "macos")]
#[path = "../src/store.rs"]
mod store;
#[cfg(target_os = "macos")]
#[path = "../src/note_tree.rs"]
mod note_tree;
#[cfg(target_os = "macos")]
#[path = "../src/mcp_presence.rs"]
mod mcp_presence;
#[cfg(target_os = "macos")]
#[path = "../src/icon_picker.rs"]
mod icon_picker;
#[path = "../src/icon_catalog.rs"]
mod icon_catalog;
#[path = "../src/preferences.rs"]
mod preferences;
#[cfg(target_os = "macos")]
#[path = "../src/ui.rs"]
mod ui;
#[cfg(target_os = "macos")]
fn main() {
    use gpui::{font, px, rgb, Application, FontStyle, FontWeight, TextRun, WindowTextSystem};
    let app = Application::new().with_assets(assets::Assets);
    assets::register_fonts(&app.text_system());
    let system = WindowTextSystem::new(app.text_system());
    let mut family_ids=Vec::new();
    for (_,family,_) in assets::CONTENT_FONTS {
        let mut fonts=Vec::new();
        for (weight,style) in [(FontWeight::NORMAL,FontStyle::Normal),(FontWeight::BOLD,FontStyle::Normal),(FontWeight::NORMAL,FontStyle::Italic),(FontWeight::BOLD,FontStyle::Italic)] {
            let mut f=font(family);f.weight=weight;f.style=style;fonts.push(f);
        }
        let runs=fonts.into_iter().map(|font| TextRun {len:2,font,color:rgb(0x262626).into(),background_color:None,underline:None,strikethrough:None}).collect::<Vec<_>>();
        let shaped=system.layout_line("aaBBiiBI",px(22.),&runs,None);
        let ids=shaped.runs.iter().map(|r| r.font_id).collect::<std::collections::HashSet<_>>();
        assert_eq!(ids.len(),4,"{family} must supply four distinct embedded faces");
        family_ids.push(shaped.runs[0].font_id);
    }
    assert_eq!(family_ids.iter().collect::<std::collections::HashSet<_>>().len(),3,"all content fonts must shape with distinct faces");
    println!("Embedded content fonts verified: sans, Libron and JetBrains Mono with regular/bold/italic/bold italic faces.");
    let normal = font(assets::FONT_FAMILY);
    let mut bold = normal.clone();
    bold.weight = FontWeight::BOLD;
    let mut italic = normal.clone();
    italic.style = FontStyle::Italic;
    let runs: Vec<_> = [normal.clone(), bold, italic, normal]
        .into_iter()
        .map(|font| TextRun {
            len: 2,
            font,
            color: rgb(0xebf1f4).into(),
            background_color: None,
            underline: None,
            strikethrough: None,
        })
        .collect();
    let line = system.layout_line("aaBBiiNN", px(22.), &runs, None);
    assert_eq!(
        line.runs.len(),
        4,
        "normal/bold/italic/normal must keep separate font runs"
    );
    assert_ne!(
        line.runs[0].font_id, line.runs[1].font_id,
        "bold must use a different font"
    );
    assert_ne!(
        line.runs[0].font_id, line.runs[2].font_id,
        "italic must use a different font"
    );
    assert_eq!(
        line.runs[0].font_id, line.runs[3].font_id,
        "normal style must be restored"
    );
    println!("Native rich text verified: normal → bold → italic → normal, equal foreground color.");
    drop(system);
    // Exercise the actual title input's deletion handler in an invisible, unfocused window.
    // No system mouse/keyboard events are posted and no application window is activated.
    app.run(|cx| {
        use gpui::{AppContext, EntityInputHandler, WindowOptions};
        use gpui_component::input::InputState;
        gpui_component::init(cx);
        gpui_component::Theme::global_mut(cx).font_family = assets::FONT_FAMILY.into();
        let window = cx.open_window(WindowOptions { show: false, focus: false, ..Default::default() }, |window, cx| {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            use objc::{msg_send, sel, sel_impl};
            let RawWindowHandle::AppKit(handle) = window.window_handle().unwrap().as_raw() else { unreachable!() };
            let native_view = handle.ns_view.as_ptr() as cocoa::base::id;
            let native_window: cocoa::base::id = unsafe { msg_send![native_view, window] };
            macos::configure(window, 1., true);
            for pinned in [true, false, true, false] {
                macos::set_pinned(window, pinned);
                let level: i64 = unsafe { msg_send![native_window, level] };
                let behavior: u64 = unsafe { msg_send![native_window, collectionBehavior] };
                assert_eq!(level, if pinned { 3 } else { 0 });
                assert_eq!(behavior, if pinned { 257 } else { 0 });
            }
            println!("Native pin verified: floating/normal stacking and Space behavior toggle on the same window.");
            cx.new(|cx| {
                let mut input = InputState::new(window, cx);
                input.set_value("Título ação 💚", window, cx);
                let end = input.value().len();
                input.set_byte_selection(end..end, cx);
                input.delete_backward(window, cx);
                assert_eq!(input.value().as_ref(), "Título ação ");
                input.delete_backward(window, cx);
                assert_eq!(input.value().as_ref(), "Título ação");
                input.set_byte_selection("Título aç".len().."Título aç".len(), cx);
                input.delete_backward(window, cx);
                assert_eq!(input.value().as_ref(), "Título aão");
                input.set_byte_selection(0.."Título ".len(), cx);
                input.delete_backward(window, cx);
                assert_eq!(input.value().as_ref(), "aão");
                input.set_value("Título original", window, cx);
                // The same paragraph selection routine used by a triple click.
                input.select_paragraph(0, cx);
                assert_eq!(input.selection_range(), 0.."Título original".len());
                input.replace_text_in_range(None, "Novo título", window, cx);
                // Typing collapses the range, but retains the mouse's paragraph granularity.
                // The old Backspace expanded to the entire paragraph here.
                input.delete_backward(window, cx);
                assert_eq!(input.value().as_ref(), "Novo títul");
                input.delete_backward(window, cx);
                assert_eq!(input.value().as_ref(), "Novo títu");
                input
            })
        }).expect("hidden native input regression window");
        window.update(cx, |_, window, _| window.remove_window()).unwrap();
        println!("Title Backspace verified: single characters, Unicode, middle caret and explicit selection.");
        ui::verify_native_tree(cx);
        block_editor::verify_native_editor(cx);
        ui::verify_native_hover_async(cx);
    });
}
#[cfg(not(target_os = "macos"))]
fn main() {}

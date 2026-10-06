//! AppKit integration is confined to the main thread used by GPUI.
#![allow(deprecated, unexpected_cfgs)]
use cocoa::{
    base::{id, nil, NO, YES},
    foundation::{NSAutoreleasePool, NSString, NSSize},
};
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    runtime::{Object, Sel},
    sel, sel_impl,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::sync::atomic::{AtomicBool, Ordering};

static TOGGLE: AtomicBool = AtomicBool::new(false);
static SHOW: AtomicBool = AtomicBool::new(false);
pub fn request_show() { SHOW.store(true, Ordering::Relaxed); }
pub fn take_show() -> bool { SHOW.swap(false, Ordering::Relaxed) }
extern "C" fn clicked(_: &Object, _: Sel, _: id) {
    TOGGLE.store(true, Ordering::Relaxed);
}
pub fn take_toggle() -> bool {
    TOGGLE.swap(false, Ordering::Relaxed)
}

pub struct StatusItem {
    item: id,
    target: id,
}
impl StatusItem {
    pub fn new() -> Self {
        unsafe {
            let app: id = msg_send![class!(NSApplication), sharedApplication];
            let _: () = msg_send![app, setActivationPolicy: 0_i64]; // Regular: Dock and menu bar.
            let icon_bytes = include_bytes!("../assets/app/Sparkpad.icns");
            let icon_data: id = msg_send![class!(NSData), dataWithBytes: icon_bytes.as_ptr() length: icon_bytes.len()];
            let app_icon: id = msg_send![class!(NSImage), alloc];
            let app_icon: id = msg_send![app_icon, initWithData: icon_data];
            if app_icon != nil {
                let _: () = msg_send![app, setApplicationIconImage: app_icon];
                let _: () = msg_send![app_icon, release];
            }
            let mut decl =
                ClassDecl::new("SparkpadStatusTarget", class!(NSObject)).unwrap();
            decl.add_method(
                sel!(togglePanel:),
                clicked as extern "C" fn(&Object, Sel, id),
            );
            let target: id = msg_send![decl.register(), new];
            let bar: id = msg_send![class!(NSStatusBar), systemStatusBar];
            let item: id = msg_send![bar, statusItemWithLength: -1.0_f64];
            let _: id = msg_send![item, retain];
            let button: id = msg_send![item, button];
            let bytes = include_bytes!("../assets/app/status-icon.png");
            let data: id = msg_send![class!(NSData), dataWithBytes: bytes.as_ptr() length: bytes.len()];
            let image: id = msg_send![class!(NSImage), alloc];
            let image: id = msg_send![image, initWithData: data];
            if image != nil {
                let _: () = msg_send![image, setSize: NSSize::new(18., 18.)];
                let _: () = msg_send![image, setTemplate: NO];
                let _: () = msg_send![button, setImage: image];
                let _: () = msg_send![image, release];
            } else {
                let title = NSString::alloc(nil).init_str("SP");
                let _: () = msg_send![button, setTitle: title];
                let _: () = msg_send![title, release];
            }
            let tooltip =
                NSString::alloc(nil).init_str("Sparkpad — mostrar/ocultar notas");
            let _: () = msg_send![button, setToolTip: tooltip];
            let _: () = msg_send![button, setTarget: target];
            let _: () = msg_send![button, setAction: sel!(togglePanel:)];
            let _: () = msg_send![tooltip, release];
            Self { item, target }
        }
    }
}
impl Drop for StatusItem {
    fn drop(&mut self) {
        unsafe {
            let bar: id = msg_send![class!(NSStatusBar), systemStatusBar];
            let _: () = msg_send![bar, removeStatusItem: self.item];
            let _: () = msg_send![self.item, release];
            let _: () = msg_send![self.target, release];
        }
    }
}
unsafe fn native(window: &gpui::Window) -> id {
    match HasWindowHandle::window_handle(window)
        .expect("macOS window handle")
        .as_raw()
    {
        RawWindowHandle::AppKit(handle) => {
            let view = handle.ns_view.as_ptr() as id;
            msg_send![view, window]
        }
        _ => unreachable!("macOS only"),
    }
}
/// Normal windows use the standard stacking order and Space membership. Pinning
/// promotes this same window to the floating level, including full-screen Spaces.
pub fn set_pinned(window: &gpui::Window, pinned: bool) {
    unsafe {
        let win = native(window);
        let _: () = msg_send![win, setLevel: if pinned { 3_i64 } else { 0_i64 }];
        let behavior = if pinned { 1_u64 | (1_u64 << 8) } else { 0_u64 };
        let _: () = msg_send![win, setCollectionBehavior: behavior];
    }
}

pub fn configure(window: &gpui::Window, opacity: f32, pinned: bool) {
    set_pinned(window, pinned);
    unsafe {
        let win = native(window);
        let _: () = msg_send![win, setHidesOnDeactivate: NO];
        let _: () = msg_send![win, setOpaque: NO];
        let style: u64 = msg_send![win, styleMask];
        let _: () = msg_send![win, setStyleMask: (style | (1_u64 << 3))]; // Keep the floating panel resizable.
        let _: () = msg_send![win, setMovable: YES];
        for kind in 0_u64..3 {
            let button: id = msg_send![win, standardWindowButton: kind];
            if button != nil {
                let _: () = msg_send![button, setHidden: NO];
            }
        }
        let _: () = msg_send![win, setHasShadow: YES];
        let _: () = msg_send![win, setAlphaValue: opacity as f64];
    }
}
pub fn appearance(window: &gpui::Window, light: bool) {
    unsafe {
        let name=NSString::alloc(nil).init_str(if light {"NSAppearanceNameAqua"} else {"NSAppearanceNameDarkAqua"});
        let appearance: id=msg_send![class!(NSAppearance), appearanceNamed: name];
        let _: ()=msg_send![native(window), setAppearance: appearance];
        let _: ()=msg_send![name, release];
    }
}

pub fn opacity(window: &gpui::Window, value: f32) {
    unsafe {
        let _: () = msg_send![native(window), setAlphaValue: value as f64];
    }
}
pub fn is_minimized(window: &gpui::Window) -> bool {
    unsafe { msg_send![native(window), isMiniaturized] }
}

pub fn visible(window: &gpui::Window, show: bool) {
    unsafe {
        // Short lived pool also covers calls from the timer, outside AppKit event callbacks.
        let pool = NSAutoreleasePool::new(nil);
        let win = native(window);
        if show {
            if is_minimized(window) {
                let _: () = msg_send![win, deminiaturize: nil];
            }
            let _: () = msg_send![win, makeKeyAndOrderFront: nil];
        } else {
            let _: () = msg_send![win, orderOut: nil];
        }
        let _: () = msg_send![pool, drain];
    }
}

/// Handle the custom title bar through AppKit: double-click toggles native zoom,
/// while a single press tracks a window drag. Defer both until GPUI releases
/// its borrowed event dispatch (start_window_move is a no-op on macOS).
pub fn start_drag(mouse: &gpui::MouseDownEvent, window: &gpui::Window, cx: &mut gpui::App) {
    unsafe {
        let app: id = msg_send![class!(NSApplication), sharedApplication];
        let event: id = msg_send![app, currentEvent];
        if event == nil {
            return;
        }
        let win = native(window);
        let _: id = msg_send![win, retain];
        let _: id = msg_send![event, retain];
        let zoom = mouse.click_count == 2;
        cx.spawn(async move |_| {
            gpui::Timer::after(std::time::Duration::from_millis(1)).await;
            if zoom {
                let _: () = msg_send![win, zoom: nil];
            } else {
                let _: () = msg_send![win, performWindowDragWithEvent: event];
            }
            let _: () = msg_send![event, release];
            let _: () = msg_send![win, release];
        })
        .detach();
    }
}

/// Copy literal code without HTML or Markdown fence representations.
pub fn write_plain_clipboard(text: &str, cx: &mut gpui::App) {
    #[cfg(test)]
    let isolated = TEST_CLIPBOARD.with(|value| value.get() != nil);
    #[cfg(not(test))]
    let isolated = false;
    if !isolated { cx.write_to_clipboard(gpui::ClipboardItem::new_string(text.to_owned())); return; }
    unsafe {
        let pool = NSAutoreleasePool::new(nil);
        let board = rich_pasteboard();
        let _: isize = msg_send![board, clearContents];
        let kind = NSString::alloc(nil).init_str("public.utf8-plain-text");
        let value = NSString::alloc(nil).init_str(text);
        let _: bool = msg_send![board, setString: value forType: kind];
        let _: () = msg_send![kind, release]; let _: () = msg_send![value, release];
        let _: () = msg_send![pool, drain];
    }
}

/// Add standard HTML and our lossless Markdown representation alongside GPUI's plain text.
pub fn write_rich_clipboard(plain: String, html: &str, markdown: Option<&str>, cx: &mut gpui::App) {
    #[cfg(test)]
    let isolated = TEST_CLIPBOARD.with(|value| value.get() != nil);
    #[cfg(not(test))]
    let isolated = false;
    if !isolated { cx.write_to_clipboard(gpui::ClipboardItem::new_string(plain.clone())); }
    unsafe {
        let pool = NSAutoreleasePool::new(nil);
        let pasteboard = rich_pasteboard();
        if isolated { let _: isize = msg_send![pasteboard, clearContents]; }
        for (kind, value) in [("public.utf8-plain-text", Some(plain.as_str())), ("public.html", Some(html)), ("dev.mpires.sparkpad.markdown", markdown)] {
            let Some(value) = value else { continue; };
            let kind = NSString::alloc(nil).init_str(kind);
            let value = NSString::alloc(nil).init_str(value);
            let _: bool = msg_send![pasteboard, setString: value forType: kind];
            let _: () = msg_send![kind, release]; let _: () = msg_send![value, release];
        }
        let _: () = msg_send![pool, drain];
    }
}
pub fn clipboard_string(kind: &str) -> Option<String> {
    unsafe {
        let pool = NSAutoreleasePool::new(nil);
        let pasteboard = rich_pasteboard();
        let kind = NSString::alloc(nil).init_str(kind);
        let value: id = msg_send![pasteboard, stringForType: kind];
        let result = if value == nil { None } else {
            let bytes: *const std::ffi::c_char = msg_send![value, UTF8String];
            if bytes.is_null() { None } else { Some(std::ffi::CStr::from_ptr(bytes).to_string_lossy().into_owned()) }
        };
        let _: () = msg_send![kind, release]; let _: () = msg_send![pool, drain]; result
    }
}
fn rich_pasteboard() -> id {
    #[cfg(test)]
    if let Some(board) = TEST_CLIPBOARD.with(|value| (value.get() != nil).then_some(value.get())) { return board; }
    unsafe { msg_send![class!(NSPasteboard), generalPasteboard] }
}
#[cfg(test)]
thread_local! { static TEST_CLIPBOARD: std::cell::Cell<id> = const { std::cell::Cell::new(nil) }; }
#[cfg(test)]
pub struct TestClipboard { previous: id, board: id }
#[cfg(test)]
impl TestClipboard {
    pub fn new() -> Self {
        unsafe {
            let board: id = msg_send![class!(NSPasteboard), pasteboardWithUniqueName];
            let _: id = msg_send![board, retain];
            let previous = TEST_CLIPBOARD.with(|value| value.replace(board));
            Self { previous, board }
        }
    }
}
#[cfg(test)]
impl Drop for TestClipboard {
    fn drop(&mut self) {
        TEST_CLIPBOARD.with(|value| value.set(self.previous));
        unsafe { let _: () = msg_send![self.board, releaseGlobally]; let _: () = msg_send![self.board, release]; }
    }
}
/// Preserve every pasteboard representation in native regression tests, including images/HTML.
#[cfg(test)]
pub fn snapshot_clipboard() -> Vec<(String, Vec<u8>)> {
    unsafe {
        let pool = NSAutoreleasePool::new(nil);
        let pasteboard: id = msg_send![class!(NSPasteboard), generalPasteboard];
        let types: id = msg_send![pasteboard, types]; let count: usize = msg_send![types, count];
        let mut snapshot = Vec::new();
        for i in 0..count {
            let kind: id = msg_send![types, objectAtIndex: i];
            let name: *const std::ffi::c_char = msg_send![kind, UTF8String];
            let data: id = msg_send![pasteboard, dataForType: kind];
            if data == nil || name.is_null() { continue; }
            let length: usize = msg_send![data, length]; let bytes: *const u8 = msg_send![data, bytes];
            if length == 0 || !bytes.is_null() {
                snapshot.push((std::ffi::CStr::from_ptr(name).to_string_lossy().into_owned(),
                    if length == 0 { Vec::new() } else { std::slice::from_raw_parts(bytes, length).to_vec() }));
            }
        }
        let _: () = msg_send![pool, drain]; snapshot
    }
}
#[cfg(test)]
pub fn restore_clipboard(snapshot: Vec<(String, Vec<u8>)>) {
    unsafe {
        let pool = NSAutoreleasePool::new(nil);
        let pasteboard: id = msg_send![class!(NSPasteboard), generalPasteboard];
        let _: isize = msg_send![pasteboard, clearContents];
        for (kind, bytes) in snapshot {
            let kind = NSString::alloc(nil).init_str(&kind);
            let data: id = msg_send![class!(NSData), dataWithBytes: bytes.as_ptr() length: bytes.len()];
            let _: bool = msg_send![pasteboard, setData: data forType: kind]; let _: () = msg_send![kind, release];
        }
        let _: () = msg_send![pool, drain];
    }
}

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

/// GPUI 0.2.2's start_window_move is a no-op on macOS. Ask AppKit to track
/// the native mouse event after leaving GPUI's borrowed event dispatch.
pub fn start_drag(window: &gpui::Window, cx: &mut gpui::App) {
    unsafe {
        let app: id = msg_send![class!(NSApplication), sharedApplication];
        let event: id = msg_send![app, currentEvent];
        if event == nil {
            return;
        }
        let win = native(window);
        let _: id = msg_send![win, retain];
        let _: id = msg_send![event, retain];
        cx.spawn(async move |_| {
            gpui::Timer::after(std::time::Duration::from_millis(1)).await;
            let _: () = msg_send![win, performWindowDragWithEvent: event];
            let _: () = msg_send![event, release];
            let _: () = msg_send![win, release];
        })
        .detach();
    }
}

//! Global hotkey registration for toggling voice recording.
//!
//! macOS: uses Carbon RegisterEventHotKey — fires even when the app is
//! unfocused, requires no accessibility permissions. A C callback pushes
//! a byte into a pipe write-end; a native thread drains the read-end and
//! forwards into an mpsc channel the UI polls each frame. (mpsc channels
//! are not async-signal-safe enough to call directly from the handler.)
//!
//! Non-macOS: not implemented; the in-app keyboard shortcut still works.

pub struct GlobalHotkey {
    pub receiver: std::sync::mpsc::Receiver<()>,
}

#[cfg(target_os = "macos")]
mod imp {
    use std::ffi::c_void;
    use std::sync::mpsc::Sender;
    use std::sync::OnceLock;

    type EventHandlerRef = *mut c_void;
    type EventHotKeyRef = *mut c_void;
    type EventQueueRef = *mut c_void;
    type EventRef = *mut c_void;

    /// FFI-safe version of EventTypeSpec { eventClass, eventKind }
    #[repr(C)]
    struct EventTypeSpec {
        event_class: u32,
        event_kind: u32,
    }

    // kEventClassKeyboard = 'keyc'
    const EVENT_CLASS_KEYBOARD: u32 = 0x6B65_7963;
    // kEventHotKeyPressed = 5, kEventHotKeyReleased = 6
    const EVENT_HOT_KEY_PRESSED: u32 = 5;
    // kEventParamDirectObject — we receive the event, no params needed
    const CMD_MOD: u32 = 1 << 8; // cmdKey
    const SHIFT_MOD: u32 = 1 << 9; // shiftKey
    const OPT_MOD: u32 = 1 << 11; // optionKey
    const CTL_MOD: u32 = 1 << 12; // controlKey

    static HOTKEY_TX: OnceLock<Sender<()>> = OnceLock::new();

    #[link(name = "Carbon", kind = "framework")]
    extern "C" {
        fn RegisterEventHotKey(
            key_code: u32,
            modifiers: u32,
            hot_key_id: u32,
            target: EventHandlerRef,
            options: u32,
            out_ref: *mut EventHotKeyRef,
        ) -> i32;
        fn GetApplicationEventTarget() -> EventHandlerRef;
        fn InstallEventHandler(
            target: EventHandlerRef,
            handler: extern "C" fn(EventHandlerRef, EventRef) -> i32,
            num_types: usize,
            types: *const EventTypeSpec,
            user_data: *mut c_void,
            out_ref: *mut EventHandlerRef,
        ) -> i32;
        fn GetMainEventQueue() -> EventQueueRef;
        fn CreateEvent(
            allocator: *mut c_void,
            class: u32,
            kind: u32,
            when: f64,
            flags: u32,
            out: *mut EventRef,
        ) -> i32;
        fn PostEventToQueue(queue: EventQueueRef, event: EventRef, priority: u32) -> i32;
        fn ReleaseEvent(event: EventRef);
    }

    extern "C" fn hotkey_handler(_handler: EventHandlerRef, _event: EventRef) -> i32 {
        if let Some(tx) = HOTKEY_TX.get() {
            let _ = tx.send(());
        }
        0
    }

    /// Parse "Cmd+Shift+R" style specs into (virtual keycode, modifier mask).
    /// Letters use their ASCII uppercase value; digits use kVK_ANSI_0..9.
    pub fn parse_hotkey(spec: &str) -> Option<(u32, u32)> {
        let mut mods = 0u32;
        let mut key: Option<char> = None;
        for part in spec.split('+') {
            match part.to_ascii_lowercase().as_str() {
                "cmd" | "super" | "command" => mods |= CMD_MOD,
                "shift" => mods |= SHIFT_MOD,
                "alt" | "option" | "opt" => mods |= OPT_MOD,
                "ctrl" | "control" | "ctl" => mods |= CTL_MOD,
                other => {
                    if other.chars().count() == 1 {
                        key = other.chars().next().map(|c| c.to_ascii_uppercase());
                    }
                }
            }
        }
        let key = key?;
        let code = if key.is_ascii_uppercase() {
            key as u32
        } else if key.is_ascii_digit() {
            0x1D + (key as u32 - '0' as u32)
        } else {
            return None;
        };
        Some((code, mods))
    }

    pub fn register(spec: &str) -> Option<std::sync::mpsc::Receiver<()>> {
        let (key_code, mods) = parse_hotkey(spec)?;
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        let _ = HOTKEY_TX.set(tx);

        unsafe {
            // Install handler for hotkey-pressed on the application target
            let types = EventTypeSpec {
                event_class: EVENT_CLASS_KEYBOARD,
                event_kind: EVENT_HOT_KEY_PRESSED,
            };
            let mut handler_ref: EventHandlerRef = std::ptr::null_mut();
            let install_status = InstallEventHandler(
                GetApplicationEventTarget(),
                hotkey_handler,
                1,
                &types,
                std::ptr::null_mut(),
                &mut handler_ref,
            );
            if install_status != 0 {
                eprintln!(
                    "global hotkey: InstallEventHandler failed ({})",
                    install_status
                );
                return None;
            }

            // Register Cmd+Shift+<key> with a fixed hotkey id
            let mut hotkey_ref: EventHotKeyRef = std::ptr::null_mut();
            let reg_status = RegisterEventHotKey(
                key_code,
                mods,
                1, // hotkey id
                GetApplicationEventTarget(),
                0,
                &mut hotkey_ref,
            );
            if reg_status != 0 {
                eprintln!("global hotkey: RegisterEventHotKey failed ({})", reg_status);
                return None;
            }
        }
        Some(rx)
    }

    // Keep the unused FFI symbols linked without dead-code warnings.
    #[allow(dead_code)]
    fn _unused_ffi_surface() {
        let _ = (
            GetMainEventQueue as unsafe extern "C" fn() -> EventQueueRef,
            CreateEvent
                as unsafe extern "C" fn(*mut c_void, u32, u32, f64, u32, *mut EventRef) -> i32,
            PostEventToQueue as unsafe extern "C" fn(EventQueueRef, EventRef, u32) -> i32,
            ReleaseEvent as unsafe extern "C" fn(EventRef),
        );
    }

    // Small helper so we can take a mutable ref to a static null ref slot
    #[allow(dead_code)]
    fn hot_key_ref_mut() -> EventHotKeyRef {
        std::ptr::null_mut()
    }
}

#[cfg(target_os = "macos")]
pub use imp::{parse_hotkey, register};

#[cfg(not(target_os = "macos"))]
pub fn register(_spec: &str) -> Option<std::sync::mpsc::Receiver<()>> {
    None
}

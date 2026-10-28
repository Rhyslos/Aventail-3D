//! Small fixes for one operating system at a time, kept apart so the rest
//! of the app stays the same everywhere.

/// Makes sure the window gets the keyboard while it is the active window.
///
/// On Windows, the window library opens the window hidden and shows it
/// once the first picture is drawn (to avoid a white flash). Windows
/// makes it the active window while it is still hidden, and a hidden
/// window can't take the keyboard, so after it appears it can be active
/// with *nothing* holding the keyboard. Windows then sends every key as a
/// "system" key: the app still sees it, but Windows plays its error sound,
/// finding no menu for it. Switching to another app and back fixed it,
/// because coming back hands the window the keyboard properly. This does
/// that hand-over whenever the window is active and nothing holds the
/// keyboard ([decision 0026](../../../docs/decisions/0026-windows-keyboard-focus.md)).
///
/// Elsewhere it does nothing.
pub fn keep_keyboard_focus(frame: &eframe::Frame) {
    #[cfg(windows)]
    windows::keep_keyboard_focus(frame);
    #[cfg(not(windows))]
    let _ = frame;
}

/// Lets `--version`, `--help` and error messages reach the terminal the
/// app was started from, if any ([decision 0170](../../../docs/decisions/0170-windows-install-folder-and-no-console.md)).
///
/// On Windows the app is a window program, not a console program, so it
/// opens no console window of its own; started from a terminal, it
/// borrows that terminal for what it prints. Elsewhere programs always
/// print to the terminal they were started from, so it does nothing.
pub fn use_parent_terminal() {
    #[cfg(windows)]
    windows::use_parent_terminal();
}

#[cfg(windows)]
mod windows {
    // The one place the app talks to Windows directly. Allowed by
    // decision 0026; elsewhere only the tablet's Wintab calls
    // (`tablet::wintab`, decision 0184) may use `unsafe`.
    #![allow(unsafe_code)]

    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};

    pub(super) fn use_parent_terminal() {
        // SAFETY: attaches this process to its parent's console, if it has
        // one; it reads and writes no memory of ours, and failing (no
        // parent console, double-clicked from Explorer) changes nothing.
        unsafe {
            AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetActiveWindow, GetFocus, SetFocus};

    pub(super) fn keep_keyboard_focus(frame: &eframe::Frame) {
        let Ok(handle) = frame.window_handle() else {
            return;
        };
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return;
        };
        let window = handle.hwnd.get() as HWND;
        // SAFETY: these three calls only ask which window is active and
        // which holds the keyboard, and hand the keyboard to our own
        // window. `window` is the live window eframe gives us, and this
        // runs on the thread that owns it (eframe draws frames there), as
        // the calls require. None of them touch memory we own.
        unsafe {
            if GetActiveWindow() == window && GetFocus().is_null() {
                SetFocus(window);
            }
        }
    }
}

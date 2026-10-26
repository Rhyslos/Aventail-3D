//! Gives the Windows program file its icon (decision 0171): the app's logo,
//! `assets/icon.ico`, shown in Explorer, the Start menu, the taskbar and
//! Windows' list of installed apps. Elsewhere it does nothing (Linux and
//! macOS take a program's icon from elsewhere).

fn main() {
    println!("cargo:rerun-if-changed=../../assets/icon.ico");
    let for_windows = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    #[cfg(windows)]
    if for_windows {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("../../assets/icon.ico");
        if let Err(e) = resource.compile() {
            // A build without the icon still works; say why it is missing.
            println!("cargo:warning=the program's icon couldn't be added: {e}");
        }
    }
    #[cfg(not(windows))]
    let _ = for_windows;
}

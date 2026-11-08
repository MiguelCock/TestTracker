#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![deny(unsafe_code)]

#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
fn apply_linux_webkit_workarounds() {
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        // SAFETY: this runs before Tauri/GTK starts any threads.
        unsafe {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    apply_linux_webkit_workarounds();

    testtracker_lib::run()
}

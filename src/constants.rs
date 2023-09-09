#[cfg(target_os = "macos")]
pub static FILE_MARKER: &str = "/";
#[cfg(target_os = "linux")]
pub static FILE_MARKER: &str = "/";
#[cfg(target_os = "windows")]
pub static FILE_MARKER: &str = "\\";

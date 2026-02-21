use std::ffi::c_char;
use std::fs;
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

use crate::error::AppError;

pub type PluginProcessFn = unsafe extern "C" fn(u32, u32, *mut u8, *const c_char) -> i32;

pub struct LoadedPlugin {
    _library: Library,
    pub process: PluginProcessFn,
}

pub fn load_plugin(plugin_path: &Path, plugin_name: &str) -> Result<LoadedPlugin, AppError> {
    let library_path = resolve_library_path(plugin_path, plugin_name);
    if !library_path.exists() {
        return Err(AppError::PluginLibraryNotFound(library_path));
    }
    if !fs::metadata(&library_path)?.is_file() {
        return Err(AppError::PluginPathNotFile(library_path));
    }

    let library = unsafe { Library::new(&library_path)? };

    let process = {
        let symbol: Symbol<'_, PluginProcessFn> = unsafe { library.get(b"process_image\0")? };
        *symbol
    };

    Ok(LoadedPlugin {
        _library: library,
        process,
    })
}

fn resolve_library_path(plugin_path: &Path, plugin_name: &str) -> PathBuf {
    plugin_path.join(library_file_name(plugin_name))
}

fn library_file_name(plugin_name: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        format!("{plugin_name}.dll")
    }

    #[cfg(target_os = "macos")]
    {
        format!("lib{plugin_name}.dylib")
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        format!("lib{plugin_name}.so")
    }
}

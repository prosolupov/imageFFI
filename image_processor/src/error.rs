use std::ffi::NulError;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{label} file not found: {path}")]
    MissingFile { label: &'static str, path: PathBuf },
    #[error("{label} path is not a file: {path}")]
    NotAFile { label: &'static str, path: PathBuf },
    #[error("plugin library not found: {0}")]
    PluginLibraryNotFound(PathBuf),
    #[error("plugin path is not a file: {0}")]
    PluginPathNotFile(PathBuf),
    #[error("input file is not PNG: {0}")]
    NotPng(PathBuf),
    #[error("failed to decode PNG {path}: {source}")]
    DecodePng {
        path: PathBuf,
        #[source]
        source: image::ImageError,
    },
    #[error(
        "invalid RGBA buffer size for image {width}x{height} when saving {path}"
    )]
    InvalidSaveBuffer {
        path: PathBuf,
        width: u32,
        height: u32,
    },
    #[error("invalid RGBA buffer length: expected {expected}, got {actual}")]
    InvalidRgbaLen { expected: usize, actual: usize },
    #[error("plugin params contain interior NUL byte")]
    InvalidParamsNul(#[from] NulError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Image(#[from] image::ImageError),
    #[error(transparent)]
    Library(#[from] libloading::Error),
}

use std::ffi::CString;
use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;
use env_logger::Env;
use image::ImageFormat;
use image::ImageReader;
use image::RgbaImage;
use log::{error, info};

mod error;
mod plugin_loader;

use error::AppError;
use plugin_loader::load_plugin;

#[derive(Debug, Parser)]
#[command(author, version, name = "image_processor")]
struct CliArgs {
    input: PathBuf,
    output: PathBuf,
    plugin: String,
    params: PathBuf,
    #[arg(long, default_value = "target/debug")]
    plugin_path: PathBuf,
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    if let Err(error) = run() {
        error!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), AppError> {
    let args = CliArgs::parse();
    info!("starting image processing");
    ensure_existing_file(&args.input, "input")?;
    ensure_existing_file(&args.params, "params")?;

    info!("loading input image: {}", args.input.display());
    let (width, height, mut pixels_rgba) = load_png_rgba(&args.input)?;
    ensure_rgba_buffer_len(width, height, pixels_rgba.len())?;

    info!("loading params: {}", args.params.display());
    let params_text = fs::read_to_string(&args.params)?;
    let params_cstr = CString::new(params_text)?;
    info!("loading plugin '{}' from {}", args.plugin, args.plugin_path.display());
    let loaded_plugin = load_plugin(&args.plugin_path, &args.plugin)?;
    unsafe {
        (loaded_plugin.process)(width, height, pixels_rgba.as_mut_ptr(), params_cstr.as_ptr())
    };

    info!("saving output image: {}", args.output.display());
    save_png_rgba(&args.output, width, height, pixels_rgba)?;
    info!("done");
    Ok(())
}

fn save_png_rgba(path: &Path, width: u32, height: u32, pixels: Vec<u8>) -> Result<(), AppError> {
    let image = RgbaImage::from_raw(width, height, pixels).ok_or_else(|| AppError::InvalidSaveBuffer {
        path: path.to_path_buf(),
        width,
        height,
    })?;
    image.save_with_format(path, ImageFormat::Png)?;
    Ok(())
}

fn ensure_existing_file(path: &Path, label: &'static str) -> Result<(), AppError> {
    if !path.exists() {
        return Err(AppError::MissingFile {
            label,
            path: path.to_path_buf(),
        });
    }
    if !fs::metadata(path)?.is_file() {
        return Err(AppError::NotAFile {
            label,
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

fn load_png_rgba(path: &Path) -> Result<(u32, u32, Vec<u8>), AppError> {
    let reader = ImageReader::open(path)?.with_guessed_format()?;
    if reader.format() != Some(ImageFormat::Png) {
        return Err(AppError::NotPng(path.to_path_buf()));
    }

    let image = reader.decode().map_err(|source| AppError::DecodePng {
        path: path.to_path_buf(),
        source,
    })?;
    let rgba8 = image.to_rgba8();
    let (width, height) = rgba8.dimensions();
    let pixels = rgba8.into_raw();

    Ok((width, height, pixels))
}

fn ensure_rgba_buffer_len(width: u32, height: u32, actual_len: usize) -> Result<(), AppError> {
    let expected_len = width as usize * height as usize * 4;
    if actual_len != expected_len {
        return Err(AppError::InvalidRgbaLen {
            expected: expected_len,
            actual: actual_len,
        });
    }
    Ok(())
}

use std::ffi::CStr;
use std::os::raw::c_char;
use std::slice;

#[derive(Clone, Copy)]
struct BlurParams {
    radius: usize,
    iterations: usize,
}

impl Default for BlurParams {
    fn default() -> Self {
        Self {
            radius: 1,
            iterations: 1,
        }
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `rgba_data` must point to a valid writable buffer of `width * height * 4` bytes.
/// `params` must be either null or a valid NUL-terminated UTF-8 C string.
pub unsafe extern "C" fn process_image(width: u32, height: u32, rgba_data: *mut u8, params: *const c_char) {
    unsafe { process_image_impl(width, height, rgba_data, params) };
}

unsafe fn process_image_impl(width: u32, height: u32, rgba_data: *mut u8, params: *const c_char) {
    if rgba_data.is_null() {
        return;
    }

    let w = width as usize;
    let h = height as usize;
    let pixel_count = match w.checked_mul(h) {
        Some(v) => v,
        None => return,
    };
    let len = match pixel_count.checked_mul(4) {
        Some(v) => v,
        None => return,
    };

    let cfg = parse_params(params);
    if cfg.radius == 0 || cfg.iterations == 0 || len == 0 {
        return;
    }

    let data = unsafe { slice::from_raw_parts_mut(rgba_data, len) };
    apply_weighted_blur(data, w, h, cfg.radius, cfg.iterations);
}

fn apply_weighted_blur(data: &mut [u8], width: usize, height: usize, radius: usize, iterations: usize) {
    for _ in 0..iterations {
        let src = data.to_vec();
        let mut dst = vec![0u8; src.len()];

        for y in 0..height {
            for x in 0..width {
                let mut sum = [0.0_f32; 4];
                let mut weight_sum = 0.0_f32;

                let y_start = y.saturating_sub(radius);
                let y_end = (y + radius).min(height - 1);
                let x_start = x.saturating_sub(radius);
                let x_end = (x + radius).min(width - 1);

                for ny in y_start..=y_end {
                    for nx in x_start..=x_end {
                        let dx = nx.abs_diff(x) as f32;
                        let dy = ny.abs_diff(y) as f32;
                        let dist = (dx * dx + dy * dy).sqrt();
                        let weight = 1.0 / (1.0 + dist);

                        let idx = (ny * width + nx) * 4;
                        for c in 0..4 {
                            sum[c] += src[idx + c] as f32 * weight;
                        }
                        weight_sum += weight;
                    }
                }

                let out_idx = (y * width + x) * 4;
                for c in 0..4 {
                    dst[out_idx + c] = (sum[c] / weight_sum).round().clamp(0.0, 255.0) as u8;
                }
            }
        }

        data.copy_from_slice(&dst);
    }
}

fn parse_params(params: *const c_char) -> BlurParams {
    let text = if params.is_null() {
        ""
    } else {
        unsafe { CStr::from_ptr(params) }
            .to_str()
            .unwrap_or_default()
    };

    let mut out = BlurParams::default();
    for (key, value) in parse_pairs(text) {
        match key.as_str() {
            "radius" => {
                if let Ok(v) = value.parse::<usize>() {
                    out.radius = v.max(1);
                }
            }
            "iterations" => {
                if let Ok(v) = value.parse::<usize>() {
                    out.iterations = v.max(1);
                }
            }
            _ => {}
        }
    }
    out
}

fn parse_pairs(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for raw in text.split([',', '\n', ';']) {
        let cleaned = raw
            .trim()
            .trim_matches('{')
            .trim_matches('}')
            .trim()
            .replace('"', "");
        if cleaned.is_empty() {
            continue;
        }
        let pair = cleaned
            .split_once('=')
            .or_else(|| cleaned.split_once(':'));
        if let Some((k, v)) = pair {
            out.push((k.trim().to_ascii_lowercase(), v.trim().to_ascii_lowercase()));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn parses_blur_params() {
        let params = CString::new("radius=3;iterations=2").expect("valid params");
        let parsed = parse_params(params.as_ptr());
        assert_eq!(parsed.radius, 3);
        assert_eq!(parsed.iterations, 2);
    }

    #[test]
    fn keeps_single_pixel_stable() {
        let mut data = vec![42, 84, 126, 255];
        apply_weighted_blur(&mut data, 1, 1, 3, 2);
        assert_eq!(data, vec![42, 84, 126, 255]);
    }

    #[test]
    fn blur_changes_center_pixel() {
        let mut data = vec![
            0, 0, 0, 255,
            0, 0, 0, 255,
            0, 0, 0, 255,
            0, 0, 0, 255,
            255, 255, 255, 255,
            0, 0, 0, 255,
            0, 0, 0, 255,
            0, 0, 0, 255,
            0, 0, 0, 255,
        ];
        apply_weighted_blur(&mut data, 3, 3, 1, 1);
        let center = &data[(4 * 4)..(4 * 4 + 4)];
        assert!(center[0] < 255);
        assert!(center[0] > 0);
    }
}

use std::ffi::CStr;
use std::os::raw::c_char;
use std::slice;

const PROCESS_OK: i32 = 0;
const PROCESS_INVALID_PARAMS: i32 = 1;
const PROCESS_INVALID_INPUT: i32 = 2;

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
pub unsafe extern "C" fn process_image(
    width: u32,
    height: u32,
    rgba_data: *mut u8,
    params: *const c_char,
) -> i32 {
    unsafe { process_image_impl(width, height, rgba_data, params) }
}

unsafe fn process_image_impl(
    width: u32,
    height: u32,
    rgba_data: *mut u8,
    params: *const c_char,
) -> i32 {
    if rgba_data.is_null() {
        return PROCESS_INVALID_INPUT;
    }

    let w = width as usize;
    let h = height as usize;
    let pixel_count = match w.checked_mul(h) {
        Some(v) => v,
        None => return PROCESS_INVALID_INPUT,
    };
    let len = match pixel_count.checked_mul(4) {
        Some(v) => v,
        None => return PROCESS_INVALID_INPUT,
    };

    let cfg = match parse_params(params) {
        Ok(cfg) => cfg,
        Err(_) => return PROCESS_INVALID_PARAMS,
    };
    if cfg.radius == 0 || cfg.iterations == 0 || len == 0 {
        return PROCESS_INVALID_INPUT;
    }

    let data = unsafe { slice::from_raw_parts_mut(rgba_data, len) };
    apply_weighted_blur(data, w, h, cfg.radius, cfg.iterations);
    PROCESS_OK
}

fn apply_weighted_blur(
    data: &mut [u8],
    width: usize,
    height: usize,
    radius: usize,
    iterations: usize,
) {
    let mut tmp = vec![0u8; data.len()];
    let mut dst = vec![0u8; data.len()];

    for _ in 0..iterations {
        horizontal_box_pass(data, &mut tmp, width, height, radius);
        vertical_box_pass(&tmp, &mut dst, width, height, radius);
        data.copy_from_slice(&dst);
    }
}

fn horizontal_box_pass(src: &[u8], dst: &mut [u8], width: usize, height: usize, radius: usize) {
    for y in 0..height {
        for c in 0..4 {
            let mut start = 0usize;
            let mut end = radius.min(width - 1);
            let mut sum: u64 = 0;

            for nx in start..=end {
                sum += src[(y * width + nx) * 4 + c] as u64;
            }

            for x in 0..width {
                let count = (end - start + 1) as u64;
                dst[(y * width + x) * 4 + c] = (sum / count) as u8;

                if x + 1 == width {
                    continue;
                }

                let next_start = (x + 1).saturating_sub(radius);
                let next_end = ((x + 1) + radius).min(width - 1);

                if next_start > start {
                    sum -= src[(y * width + start) * 4 + c] as u64;
                }
                if next_end > end {
                    sum += src[(y * width + next_end) * 4 + c] as u64;
                }

                start = next_start;
                end = next_end;
            }
        }
    }
}

fn vertical_box_pass(src: &[u8], dst: &mut [u8], width: usize, height: usize, radius: usize) {
    for x in 0..width {
        for c in 0..4 {
            let mut start = 0usize;
            let mut end = radius.min(height - 1);
            let mut sum: u64 = 0;

            for ny in start..=end {
                sum += src[(ny * width + x) * 4 + c] as u64;
            }

            for y in 0..height {
                let count = (end - start + 1) as u64;
                dst[(y * width + x) * 4 + c] = (sum / count) as u8;

                if y + 1 == height {
                    continue;
                }

                let next_start = (y + 1).saturating_sub(radius);
                let next_end = ((y + 1) + radius).min(height - 1);

                if next_start > start {
                    sum -= src[(start * width + x) * 4 + c] as u64;
                }
                if next_end > end {
                    sum += src[(next_end * width + x) * 4 + c] as u64;
                }

                start = next_start;
                end = next_end;
            }
        }
    }
}

fn parse_params(params: *const c_char) -> Result<BlurParams, ()> {
    let text = if params.is_null() {
        ""
    } else {
        match unsafe { CStr::from_ptr(params) }.to_str() {
            Ok(text) => text,
            Err(_) => return Err(()),
        }
    };

    let mut out = BlurParams::default();
    for (key, value) in parse_pairs(text)? {
        match key.as_str() {
            "radius" => {
                let v = value.parse::<usize>().map_err(|_| ())?;
                if v == 0 {
                    return Err(());
                }
                out.radius = v;
            }
            "iterations" => {
                let v = value.parse::<usize>().map_err(|_| ())?;
                if v == 0 {
                    return Err(());
                }
                out.iterations = v;
            }
            _ => return Err(()),
        }
    }
    Ok(out)
}

fn parse_pairs(text: &str) -> Result<Vec<(String, String)>, ()> {
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
        let (k, v) = cleaned
            .split_once('=')
            .or_else(|| cleaned.split_once(':'))
            .ok_or(())?;
        out.push((k.trim().to_ascii_lowercase(), v.trim().to_ascii_lowercase()));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn parses_blur_params() {
        let params = CString::new("radius=3;iterations=2").expect("valid params");
        let parsed = parse_params(params.as_ptr()).expect("valid blur params");
        assert_eq!(parsed.radius, 3);
        assert_eq!(parsed.iterations, 2);
    }

    #[test]
    fn rejects_unknown_key() {
        let params = CString::new("horizontal=true").expect("valid params");
        let parsed = parse_params(params.as_ptr());
        assert!(parsed.is_err());
    }

    #[test]
    fn rejects_invalid_numeric_value() {
        let params = CString::new("radius=0").expect("valid params");
        let parsed = parse_params(params.as_ptr());
        assert!(parsed.is_err());
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
            0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0,
            255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255,
        ];
        apply_weighted_blur(&mut data, 3, 3, 1, 1);
        let center = &data[(4 * 4)..(4 * 4 + 4)];
        assert!(center[0] < 255);
        assert!(center[0] > 0);
    }
}

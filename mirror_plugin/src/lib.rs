use std::ffi::CStr;
use std::os::raw::c_char;
use std::slice;

#[derive(Clone, Copy, Default)]
struct MirrorParams {
    horizontal: bool,
    vertical: bool,
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
    let pixel_count = match (width as usize).checked_mul(height as usize) {
        Some(v) => v,
        None => return,
    };
    let len = match pixel_count.checked_mul(4) {
        Some(v) => v,
        None => return,
    };
    let mut cfg = parse_params(params);

    if !cfg.horizontal && !cfg.vertical {
        cfg.horizontal = true;
    }

    let data = unsafe { slice::from_raw_parts_mut(rgba_data, len) };
    if cfg.horizontal {
        mirror_horizontal(data, width as usize, height as usize);
    }
    if cfg.vertical {
        mirror_vertical(data, width as usize, height as usize);
    }
}

fn mirror_horizontal(data: &mut [u8], width: usize, height: usize) {
    for y in 0..height {
        for x in 0..(width / 2) {
            let left = (y * width + x) * 4;
            let right = (y * width + (width - 1 - x)) * 4;
            for c in 0..4 {
                data.swap(left + c, right + c);
            }
        }
    }
}

fn mirror_vertical(data: &mut [u8], width: usize, height: usize) {
    for y in 0..(height / 2) {
        for x in 0..width {
            let top = (y * width + x) * 4;
            let bottom = ((height - 1 - y) * width + x) * 4;
            for c in 0..4 {
                data.swap(top + c, bottom + c);
            }
        }
    }
}

fn parse_params(params: *const c_char) -> MirrorParams {
    let text = if params.is_null() {
        ""
    } else {
        unsafe { CStr::from_ptr(params) }
            .to_str()
            .unwrap_or_default()
    };

    let mut out = MirrorParams::default();
    for (key, value) in parse_pairs(text) {
        match key.as_str() {
            "horizontal" => {
                if let Some(v) = parse_bool(&value) {
                    out.horizontal = v;
                }
            }
            "vertical" => {
                if let Some(v) = parse_bool(&value) {
                    out.vertical = v;
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

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn parses_bool_params() {
        let params = CString::new("horizontal=true,vertical=0").expect("valid params");
        let parsed = parse_params(params.as_ptr());
        assert!(parsed.horizontal);
        assert!(!parsed.vertical);
    }

    #[test]
    fn mirrors_horizontally() {
        let mut data = vec![
            255, 0, 0, 255, // left pixel
            0, 255, 0, 255, // right pixel
        ];
        mirror_horizontal(&mut data, 2, 1);
        assert_eq!(
            data,
            vec![
                0, 255, 0, 255, // becomes right
                255, 0, 0, 255, // becomes left
            ]
        );
    }

    #[test]
    fn mirrors_vertically() {
        let mut data = vec![
            10, 0, 0, 255, // top
            20, 0, 0, 255, // bottom
        ];
        mirror_vertical(&mut data, 1, 2);
        assert_eq!(
            data,
            vec![
                20, 0, 0, 255, // top <- bottom
                10, 0, 0, 255, // bottom <- top
            ]
        );
    }
}

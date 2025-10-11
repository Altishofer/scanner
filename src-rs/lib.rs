#![no_std]
#[macro_use]
extern crate alloc;

use wasm_bindgen::{prelude::*, Clamped};
use web_sys::ImageData;

mod image;
use image::{Quad, RGBAImage};

#[cfg(not(target_arch = "wasm32"))]
compile_error!("Only compilable to WASM");

fn sum_sides(quad: Quad) -> (f32, f32) {
    let Quad { a, b, c, d } = quad;
    // With normalized ordering: a=bottom-left, b=top-left, c=top-right, d=bottom-right
    // Calculate width (top and bottom sides) and height (left and right sides)
    let top_width = (c.x - b.x).hypot(c.y - b.y);    // top-left to top-right
    let bottom_width = (d.x - a.x).hypot(d.y - a.y); // bottom-left to bottom-right
    let left_height = (b.x - a.x).hypot(b.y - a.y);  // bottom-left to top-left
    let right_height = (c.x - d.x).hypot(c.y - d.y); // bottom-right to top-right
    
    let avg_width = (top_width + bottom_width) / 2.0;
    let avg_height = (left_height + right_height) / 2.0;
    
    // Return (width, height) instead of (side, top) for clarity
    (avg_width, avg_height)
}

// Removed sort_quad function - using normalize_quad_ordering in perspective.rs instead

impl From<ImageData> for RGBAImage {
    fn from(data: ImageData) -> Self {
        let width = data.width() as usize;
        let height = data.height() as usize;
        let data = data.data().0;
        RGBAImage {
            data,
            width,
            height,
        }
    }
}

// use js_sys::Array;
// #[wasm_bindgen]
// pub fn find_edges(data: ImageData, threshold: f32) -> Array {
//     console_error_panic_hook::set_once();
//     let rgba: RGBAImage = data.into();
//     let mut by = (rgba.width.min(rgba.height) as f32) / 360.0;
//     if by < 2.0 {
//         by = 1.0
//     }
//     let mut src = rgba.to_grayscale();
//     if by != 1.0 {
//         src = src.downscale(by);
//     }
//     src.gaussian().edges(threshold).into_iter().map(JsValue::from).collect()
// }

#[macro_export]
macro_rules! perf {
    ($b:expr) => {{
        use js_sys::{global, Reflect};
        use wasm_bindgen::{prelude::*, JsCast};
        use web_sys::Performance;

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = console)]
            fn log(a: &str, b: &str, c: &str, d: f64);
        }
        let performance = Reflect::get(&global(), &JsValue::from_str("performance"))
            .unwrap()
            .unchecked_into::<Performance>();
        let ts = performance.now();
        let ret = $b;
        log("time", stringify!($b), "=", performance.now() - ts);
        ret
    }};
}

#[wasm_bindgen]
pub fn find_document(data: ImageData) -> Option<Quad> {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();
    let rgba: RGBAImage = data.into();
    let mut by = (rgba.width.min(rgba.height) as f32) / 360.0;
    if by < 2.0 {
        by = 1.0
    }
    let mut src = rgba.to_grayscale();
    if by != 1.0 {
        src = src.downscale(by);
    }
    src.gaussian().document().map(|doc| {
        let mut doc = doc.quad; // Remove sort_quad call - normalization handled in perspective function
        doc.a.x *= by;
        doc.a.y *= by;
        doc.b.x *= by;
        doc.b.y *= by;
        doc.c.x *= by;
        doc.c.y *= by;
        doc.d.x *= by;
        doc.d.y *= by;
        doc
    })
}

#[wasm_bindgen]
pub fn extract_document(
    data: ImageData,
    region: Quad,
    target_width: usize,
    target_height: Option<usize>,
) -> ImageData {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();
    let rgba: RGBAImage = data.into();
    let target_height = if let Some(height) = target_height {
        height
    } else {
        let (width, height) = sum_sides(region);
        (height / width * (target_width as f32)) as usize
    };
    ImageData::new_with_u8_clamped_array_and_sh(
        Clamped(&rgba.perspective(region, target_width, target_height).data),
        target_width as u32,
        target_height as u32,
    )
    .unwrap()
}

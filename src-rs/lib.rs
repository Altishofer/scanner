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
    // Calculate width (horizontal sides) and height (vertical sides)
    let top_width = (c.x - b.x).hypot(c.y - b.y);    // top-left to top-right
    let bottom_width = (d.x - a.x).hypot(d.y - a.y); // bottom-left to bottom-right
    let left_height = (b.x - a.x).hypot(b.y - a.y);  // bottom-left to top-left
    let right_height = (c.x - d.x).hypot(c.y - d.y); // bottom-right to top-right
    
    let avg_width = (top_width + bottom_width) / 2.0;
    let avg_height = (left_height + right_height) / 2.0;
    
    // Return (width, height) - this should match the actual document dimensions
    (avg_width, avg_height)
}

fn normalize_quad_for_document_orientation(quad: Quad) -> Quad {
    let Quad { a, b, c, d } = quad;
    let points = [a, b, c, d];
    
    // Find which point is closest to each corner of the bounding rectangle
    let min_x = points.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
    let max_x = points.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
    let min_y = points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
    let max_y = points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
    
    // Find closest point to each corner
    let mut top_left = points[0];
    let mut top_right = points[0];
    let mut bottom_left = points[0];
    let mut bottom_right = points[0];
    
    let mut min_dist_tl = f32::INFINITY;
    let mut min_dist_tr = f32::INFINITY;
    let mut min_dist_bl = f32::INFINITY;
    let mut min_dist_br = f32::INFINITY;
    
    for &point in &points {
        // Distance to top-left corner (min_x, min_y)
        let dist_tl = (point.x - min_x).hypot(point.y - min_y);
        if dist_tl < min_dist_tl {
            min_dist_tl = dist_tl;
            top_left = point;
        }
        
        // Distance to top-right corner (max_x, min_y)
        let dist_tr = (point.x - max_x).hypot(point.y - min_y);
        if dist_tr < min_dist_tr {
            min_dist_tr = dist_tr;
            top_right = point;
        }
        
        // Distance to bottom-left corner (min_x, max_y)
        let dist_bl = (point.x - min_x).hypot(point.y - max_y);
        if dist_bl < min_dist_bl {
            min_dist_bl = dist_bl;
            bottom_left = point;
        }
        
        // Distance to bottom-right corner (max_x, max_y)
        let dist_br = (point.x - max_x).hypot(point.y - max_y);
        if dist_br < min_dist_br {
            min_dist_br = dist_br;
            bottom_right = point;
        }
    }
    
    // Return in the standard order: a=bottom-left, b=top-left, c=top-right, d=bottom-right
    Quad {
        a: bottom_left,
        b: top_left,
        c: top_right,
        d: bottom_right,
    }
}

// Remove the complex sort_quad function as it causes unwanted rotations

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
        let mut doc = normalize_quad_for_document_orientation(doc.quad);
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
    
    // Normalize the quad to ensure consistent orientation
    let normalized_quad = normalize_quad_for_document_orientation(region);
    
    let target_height = if let Some(height) = target_height {
        height
    } else {
        let (width, height) = sum_sides(normalized_quad);
        (height / width * (target_width as f32)) as usize
    };
    ImageData::new_with_u8_clamped_array_and_sh(
        Clamped(&rgba.perspective(normalized_quad, target_width, target_height).data),
        target_width as u32,
        target_height as u32,
    )
    .unwrap()
}

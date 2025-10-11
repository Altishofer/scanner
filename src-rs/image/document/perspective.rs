use super::{super::RGBAImage, Point, Quad};
use alloc::vec::Vec;

type Vec3 = [f32; 3];
type Mat3 = [f32; 9];

fn normalize_quad_ordering(quad: Quad) -> Quad {
    let Quad { a, b, c, d } = quad;
    let points = [a, b, c, d];
    
    // Calculate the center of the quad
    let center_x = (a.x + b.x + c.x + d.x) / 4.0;
    let center_y = (a.y + b.y + c.y + d.y) / 4.0;
    
    // Sort points by angle from center to ensure consistent ordering
    let mut indexed_points: Vec<(Point, usize)> = points.iter().enumerate()
        .map(|(i, &p)| (p, i))
        .collect();
    
    indexed_points.sort_by(|a, b| {
        let angle_a = (a.0.y - center_y).atan2(a.0.x - center_x);
        let angle_b = (b.0.y - center_y).atan2(b.0.x - center_x);
        angle_a.partial_cmp(&angle_b).unwrap()
    });
    
    // Now we have points in clockwise order starting from the rightmost point
    // We need to identify which point should be which corner
    // Calculate the bounding box
    let min_x = points.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
    let max_x = points.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
    let min_y = points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
    let max_y = points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
    
    // Find the closest point to each corner of the bounding rectangle
    let mut corners = [Point { x: 0.0, y: 0.0 }; 4];
    
    // Find top-left (min_x, min_y)
    let mut min_dist = f32::INFINITY;
    for &point in &points {
        let dist = (point.x - min_x).hypot(point.y - min_y);
        if dist < min_dist {
            min_dist = dist;
            corners[1] = point; // b: top-left
        }
    }
    
    // Find top-right (max_x, min_y)
    min_dist = f32::INFINITY;
    for &point in &points {
        let dist = (point.x - max_x).hypot(point.y - min_y);
        if dist < min_dist {
            min_dist = dist;
            corners[2] = point; // c: top-right
        }
    }
    
    // Find bottom-left (min_x, max_y)
    min_dist = f32::INFINITY;
    for &point in &points {
        let dist = (point.x - min_x).hypot(point.y - max_y);
        if dist < min_dist {
            min_dist = dist;
            corners[0] = point; // a: bottom-left
        }
    }
    
    // Find bottom-right (max_x, max_y)
    min_dist = f32::INFINITY;
    for &point in &points {
        let dist = (point.x - max_x).hypot(point.y - max_y);
        if dist < min_dist {
            min_dist = dist;
            corners[3] = point; // d: bottom-right
        }
    }
    
    // Return in the order expected by the perspective function:
    // a: bottom-left, b: top-left, c: top-right, d: bottom-right
    Quad {
        a: corners[0], // bottom-left
        b: corners[1], // top-left
        c: corners[2], // top-right
        d: corners[3], // bottom-right
    }
}

fn adj(src: Mat3) -> Mat3 {
    [
        src[4] * src[8] - src[5] * src[7],
        src[2] * src[7] - src[1] * src[8],
        src[1] * src[5] - src[2] * src[4],
        src[5] * src[6] - src[3] * src[8],
        src[0] * src[8] - src[2] * src[6],
        src[2] * src[3] - src[0] * src[5],
        src[3] * src[7] - src[4] * src[6],
        src[1] * src[6] - src[0] * src[7],
        src[0] * src[4] - src[1] * src[3],
    ]
}

fn mul(a: Mat3, b: Mat3) -> Mat3 {
    [
        a[0] * b[0] + a[1] * b[3] + a[2] * b[6],
        a[0] * b[1] + a[1] * b[4] + a[2] * b[7],
        a[0] * b[2] + a[1] * b[5] + a[2] * b[8],
        a[3] * b[0] + a[4] * b[3] + a[5] * b[6],
        a[3] * b[1] + a[4] * b[4] + a[5] * b[7],
        a[3] * b[2] + a[4] * b[5] + a[5] * b[8],
        a[6] * b[0] + a[7] * b[3] + a[8] * b[6],
        a[6] * b[1] + a[7] * b[4] + a[8] * b[7],
        a[6] * b[2] + a[7] * b[5] + a[8] * b[8],
    ]
}

fn mulv(a: Mat3, b: Vec3) -> Vec3 {
    [
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2],
        a[3] * b[0] + a[4] * b[1] + a[5] * b[2],
        a[6] * b[0] + a[7] * b[1] + a[8] * b[2],
    ]
}

fn basis_to_points(src: Quad) -> Mat3 {
    let Quad { a, b, c, d } = src;
    let m = [a.x, b.x, c.x, a.y, b.y, c.y, 1.0, 1.0, 1.0];
    let coeffs = mulv(adj(m), [d.x, d.y, 1.0]);
    mul(
        m,
        [
            coeffs[0], 0.0, 0.0, 0.0, coeffs[1], 0.0, 0.0, 0.0, coeffs[2],
        ],
    )
}

fn create_projector(from: Quad, to: Quad) -> impl Fn(Point) -> Point {
    let src_basis = basis_to_points(from);
    let dst_basis = basis_to_points(to);
    let proj = mul(dst_basis, adj(src_basis));
    move |pt: Point| {
        let projected = mulv(proj, [pt.x, pt.y, 1.0]);
        Point {
            x: projected[0] / projected[2],
            y: projected[1] / projected[2],
        }
    }
}

pub fn perspective(source: &RGBAImage, quad: Quad, width: usize, height: usize) -> RGBAImage {
    // Normalize the quad ordering to prevent unwanted flipping/rotation
    let normalized_quad = normalize_quad_ordering(quad);
    
    // Calculate the dimensions of the normalized quad to understand its orientation
    let top_width = (normalized_quad.c.x - normalized_quad.b.x).hypot(normalized_quad.c.y - normalized_quad.b.y);
    let bottom_width = (normalized_quad.d.x - normalized_quad.a.x).hypot(normalized_quad.d.y - normalized_quad.a.y);
    let left_height = (normalized_quad.b.x - normalized_quad.a.x).hypot(normalized_quad.b.y - normalized_quad.a.y);
    let right_height = (normalized_quad.c.x - normalized_quad.d.x).hypot(normalized_quad.c.y - normalized_quad.d.y);
    
    let avg_quad_width = (top_width + bottom_width) / 2.0;
    let avg_quad_height = (left_height + right_height) / 2.0;
    
    // Check if the quad orientation matches the target dimensions
    let quad_is_portrait = avg_quad_height > avg_quad_width;
    let target_is_portrait = height > width;
    
    let mut data = vec![0u8; (width * height) << 2];
    let wf = width as f32;
    let hf = height as f32;
    
    // Create target rectangle - adjust if orientations don't match
    let target_quad = if quad_is_portrait == target_is_portrait {
        // Orientations match - use standard mapping
        Quad {
            a: Point { x: 0.0, y: hf },
            b: Point { x: 0.0, y: 0.0 },
            c: Point { x: wf, y: 0.0 },
            d: Point { x: wf, y: hf },
        }
    } else {
        // Orientations don't match - need to rotate the target rectangle 90 degrees
        Quad {
            a: Point { x: 0.0, y: 0.0 },     // bottom-left -> top-left
            b: Point { x: wf, y: 0.0 },      // top-left -> top-right  
            c: Point { x: wf, y: hf },       // top-right -> bottom-right
            d: Point { x: 0.0, y: hf },      // bottom-right -> bottom-left
        }
    };
    
    let projector = create_projector(target_quad, normalized_quad);
    let off_sw = source.width << 2;
    let off_se = off_sw + 4;
    for y in 0..height {
        let ib = y * width;
        for x in 0..width {
            let pt = projector(Point {
                x: x as f32,
                y: y as f32,
            });
            let xf = pt.x as usize;
            let yf = pt.y as usize;
            let dest_base = (ib + x) << 2;
            data[dest_base + 3] = 255;
            if xf + 1 < source.width && yf + 1 < source.height {
                let xt = pt.x.fract();
                let xtr = 1.0 - xt;
                let yt = pt.y.fract();
                let ytr = 1.0 - yt;
                let raw_base = (yf * source.width + xf) << 2;
                for i in 0..3 {
                    let base = raw_base + i;
                    let a = (source.data[base] as f32) * xtr + (source.data[base + 4] as f32) * xt;
                    let b = (source.data[base + off_sw] as f32) * xtr
                        + (source.data[base + off_se] as f32) * xt;
                    data[dest_base + i] = (a * ytr + b * yt) as u8;
                }
            } else {
                data[dest_base] = 255;
                data[dest_base + 1] = 255;
                data[dest_base + 2] = 255;
            }
        }
    }
    RGBAImage {
        data,
        width,
        height,
    }
}

use super::{super::RGBAImage, Point, Quad};

type Vec3 = [f32; 3];
type Mat3 = [f32; 9];

// Normalize quad points to ensure consistent ordering
// This prevents unwanted flipping/rotation during perspective correction
fn normalize_quad_ordering(quad: Quad) -> Quad {
    let mut points = [quad.a, quad.b, quad.c, quad.d];
    
    // Find the top-left point (smallest x + y)
    let mut top_left_idx = 0;
    let mut min_sum = points[0].x + points[0].y;
    for (i, point) in points.iter().enumerate().skip(1) {
        let sum = point.x + point.y;
        if sum < min_sum {
            min_sum = sum;
            top_left_idx = i;
        }
    }
    
    // Rotate the array so top-left is first
    points.rotate_left(top_left_idx);
    
    // Check if we need to reverse the order (if points are in clockwise order)
    // We want counter-clockwise order: top-left, top-right, bottom-right, bottom-left
    let cross_product = (points[1].x - points[0].x) * (points[3].y - points[0].y) 
                      - (points[3].x - points[0].x) * (points[1].y - points[0].y);
    
    if cross_product < 0.0 {
        // Points are in clockwise order, reverse to get counter-clockwise
        points.swap(1, 3);
    }
    
    // Now we have: [top-left, top-right, bottom-right, bottom-left]
    // But we need: [bottom-left, top-left, top-right, bottom-right] for the perspective function
    Quad {
        a: points[3], // bottom-left
        b: points[0], // top-left
        c: points[1], // top-right
        d: points[2], // bottom-right
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
    
    let mut data = vec![0u8; (width * height) << 2];
    let wf = width as f32;
    let hf = height as f32;
    let projector = create_projector(
        Quad {
            a: Point { x: 0.0, y: hf },
            b: Point { x: 0.0, y: 0.0 },
            c: Point { x: wf, y: 0.0 },
            d: Point { x: wf, y: hf },
        },
        normalized_quad,
    );
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

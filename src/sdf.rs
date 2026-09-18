/**
 * This implementation is ported from https://github.com/mapbox/tiny-sdf by Mapbox, which is licensed under the BSD 2-Clause license.
 * It's based directly on the algorithm published in the Felzenszwalb/Huttenlocher paper,
 * and is not a port of the existing C++ implementation provided by the paper's authors.
 */

const INF: f64 = 1e20;

pub fn calculate_sdf(
    bitmap: &[u8],
    glyph_width: u32,
    glyph_height: u32,
    buffer: u32,
    radius: f64,
    cutoff: f64,
) -> (Vec<u8>, u32, u32) {
    let width = glyph_width + 2 * buffer;
    let height = glyph_height + 2 * buffer;
    let grid_length = (width * height) as usize;
    let mut grid_outer = vec![INF; grid_length];
    let mut grid_inner = vec![0.; grid_length];
    let working_length = width.max(height) as usize;
    let mut f = vec![0.; working_length];
    let mut z = vec![0.; working_length + 1];
    let mut v = vec![0; working_length];

    for y in 0..glyph_height {
        for x in 0..glyph_width {
            let a = bitmap[(y * glyph_width + x) as usize]; // alpha value
            if a == 0 {
                // empty pixels
                continue;
            }

            let j = ((y + buffer) * width + x + buffer) as usize;

            if a == 255 {
                // fully drawn pixels
                grid_outer[j] = 0.;
                grid_inner[j] = INF;
            } else {
                // aliased pixels
                let d = 0.5 - a as f64 / 255.;
                grid_outer[j] = if d > 0. { d * d } else { 0. };
                grid_inner[j] = if d < 0. { d * d } else { 0. };
            }
        }
    }

    edt(
        &mut grid_outer,
        0,
        0,
        width,
        height,
        width,
        &mut f,
        &mut v,
        &mut z,
    );
    edt(
        &mut grid_inner,
        buffer,
        buffer,
        glyph_width,
        glyph_height,
        width,
        &mut f,
        &mut v,
        &mut z,
    );

    // Prevent INF zone from rupturing in interpolation
    for val in grid_outer.iter_mut() {
        if *val == INF {
            *val = radius * radius;
        }
    }

    let len = (width * height) as usize;

    let mut data = vec![0; len];

    for i in 0..len {
        let d = grid_outer[i].sqrt() - grid_inner[i].sqrt();
        data[i] = (255. - 255. * (d / radius + cutoff))
            .round()
            .clamp(0., 255.) as u8;
    }

    (data, width, height)
}

// 2D Euclidean squared distance transform by Felzenszwalb & Huttenlocher https://cs.brown.edu/~pff/papers/dt-final.pdf
pub fn edt(
    data: &mut [f64],
    x0: u32,
    y0: u32,
    width: u32,
    height: u32,
    grid_size: u32,
    f: &mut [f64],
    v: &mut Vec<u16>,
    z: &mut [f64],
) {
    for x in x0..(x0 + width) {
        edt1d(
            data,
            (y0 * grid_size + x) as usize,
            grid_size as usize,
            height as usize,
            f,
            v,
            z,
        );
    }

    for y in y0..(y0 + height) {
        edt1d(
            data,
            (y * grid_size + x0) as usize,
            1,
            width as usize,
            f,
            v,
            z,
        );
    }
}

// 1D squared distance transform
pub fn edt1d(
    grid: &mut [f64],
    offset: usize,
    stride: usize,
    length: usize,
    f: &mut [f64],
    v: &mut Vec<u16>,
    z: &mut [f64],
) {
    v[0] = 0;
    z[0] = -INF;
    z[1] = INF;
    f[0] = grid[offset];

    let mut k = 0_i32;
    let mut s;
    for q in 1..length {
        f[q] = grid[offset + q * stride];

        let q2 = (q * q) as f64;

        loop {
            let r = v[k as usize] as usize;
            s = (f[q] - f[r] + q2 - (r * r) as f64) / (q - r) as f64 / 2.;
            if s <= z[k as usize] {
                k -= 1;
                if k > -1 {
                    continue;
                }
            }
            break;
        }

        k += 1;

        v[k as usize] = q as u16;
        z[k as usize] = s;
        z[k as usize + 1] = INF;
    }

    let mut k = 0;
    for q in 0..length {
        loop {
            if z[k + 1] < q as f64 {
                k += 1;
                continue;
            }
            break;
        }

        let r = v[k];
        let qr = q as i16 - r as i16;

        grid[offset + q * stride] = f[r as usize] + (qr * qr) as f64;
    }
}

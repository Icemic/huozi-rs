/**
 * This implementation is ported from https://github.com/mapbox/tiny-sdf by Mapbox, which is licensed under the BSD 2-Clause license.
 * It's based directly on the algorithm published in the Felzenszwalb/Huttenlocher paper,
 * and is not a port of the existing C++ implementation provided by the paper's authors.
 */

const INF: f64 = 1e20;

pub struct TinySDF {
    grid_outer: Vec<f64>,
    grid_inner: Vec<f64>,
    f: Vec<f64>,
    z: Vec<f64>,
    v: Vec<u16>,
    grid_size: u32,
    buffer: u32,
    radius: f64,
    cutoff: f64,
}

impl TinySDF {
    pub fn new(grid_size: u32, buffer: u32, radius: f64, cutoff: f64) -> Self {
        let grid_outer = vec![0.; (grid_size * grid_size) as usize];
        let grid_inner = vec![0.; (grid_size * grid_size) as usize];
        let f = vec![0.; grid_size as usize];
        let z = vec![0.; grid_size as usize + 1];
        let v = vec![0; grid_size as usize];
        Self {
            grid_outer,
            grid_inner,
            f,
            z,
            v,
            grid_size,
            buffer,
            radius,
            cutoff,
        }
    }
    pub fn calculate(
        &mut self,
        bitmap: &Vec<u8>,
        glyph_width: u32,
        glyph_height: u32,
    ) -> (Vec<u8>, u32, u32) {
        // Initialize grids outside the glyph range to alpha 0
        self.grid_outer.fill(INF);
        self.grid_inner.fill(0.);

        let width = (glyph_width + 2 * self.buffer).min(self.grid_size);
        let height = (glyph_height + 2 * self.buffer).min(self.grid_size);

        for y in 0..glyph_height {
            for x in 0..glyph_width {
                let a = bitmap[(y * glyph_width + x) as usize]; // alpha value
                if a == 0 {
                    // empty pixels
                    continue;
                }

                let j = ((y + self.buffer) * width + x + self.buffer) as usize;

                if a == 1 {
                    // fully drawn pixels
                    self.grid_outer[j] = 0.;
                    self.grid_inner[j] = INF;
                } else {
                    // aliased pixels
                    let d = 0.5 - a as f64 / 255.;
                    self.grid_outer[j] = if d > 0. { d * d } else { 0. };
                    self.grid_inner[j] = if d < 0. { d * d } else { 0. };
                }
            }
        }

        edt(
            &mut self.grid_outer,
            0,
            0,
            width,
            height,
            width,
            &mut self.f,
            &mut self.v,
            &mut self.z,
        );
        edt(
            &mut self.grid_inner,
            self.buffer,
            self.buffer,
            glyph_width,
            glyph_height,
            width,
            &mut self.f,
            &mut self.v,
            &mut self.z,
        );

        let len = (width * height) as usize;
        let mut data = vec![0; len];

        for i in 0..len {
            let d = self.grid_outer[i].sqrt() - self.grid_inner[i].sqrt();
            data[i] = (255. - 255. * (d / self.radius + self.cutoff))
                .round()
                .clamp(0., 255.) as u8;
        }

        (data, width, height)
    }
}

// 2D Euclidean squared distance transform by Felzenszwalb & Huttenlocher https://cs.brown.edu/~pff/papers/dt-final.pdf
pub fn edt(
    data: &mut Vec<f64>,
    x0: u32,
    y0: u32,
    width: u32,
    height: u32,
    grid_size: u32,
    f: &mut Vec<f64>,
    v: &mut Vec<u16>,
    z: &mut Vec<f64>,
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
    grid: &mut Vec<f64>,
    offset: usize,
    stride: usize,
    length: usize,
    f: &mut Vec<f64>,
    v: &mut Vec<u16>,
    z: &mut Vec<f64>,
) {
    v[0] = 0;
    z[0] = -INF;
    z[1] = INF;
    f[0] = grid[offset];

    let mut k = 0_i32;
    let mut s = 0.;
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

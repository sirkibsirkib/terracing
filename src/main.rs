use noise::{NoiseFn, Perlin, Seedable};

use std::io::BufWriter;
use std::io::Write;
use std::{fs::File, path::Path};

const DIMS: [usize; 2] = [512; 2];
const CELLS: usize = DIMS[0] * DIMS[1];

fn noise_pt([xi, yi]: [usize; 2]) -> [f64; 2] {
    [
        //
        xi as f64 / DIMS[0] as f64,
        yi as f64 / DIMS[1] as f64,
    ]
}
fn index([xi, yi]: [usize; 2]) -> usize {
    (yi * DIMS[1]) + xi
}

struct ImgWriter {
    w: png::StreamWriter<'static, std::io::BufWriter<std::fs::File>>,
}
impl ImgWriter {
    fn new(path: &str) -> Self {
        let path = Path::new(path);
        let file = File::create(path).unwrap();
        let w = BufWriter::new(file);

        let mut encoder = png::Encoder::new(w, DIMS[0] as u32, DIMS[1] as u32);
        encoder.set_color(png::ColorType::RGBA);
        encoder.set_depth(png::BitDepth::Eight);
        let w = encoder.write_header().unwrap().into_stream_writer();
        Self { w }
    }
    fn pixel(&mut self, pixel: &[u8; 4]) -> std::io::Result<()> {
        self.w.write(pixel).map(drop)
    }
}
// impl Drop for ImgWriter {
//     fn drop(&mut self) {
//         let _ = self.w.flush();
//     }
// }

fn png_dump(path: &str, bytes: impl Iterator<Item = [u8; 3]>) {
    let path = Path::new(path);
    let file = File::create(path).unwrap();
    let w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, DIMS[0] as u32, DIMS[1] as u32);
    encoder.set_color(png::ColorType::RGBA);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap().into_stream_writer();
    for [r, g, b] in bytes {
        writer.write(&[r, g, b, 0xff]).unwrap();
    }
    writer.flush().unwrap();
}
const WRITE_RAW: bool = true;
const WRITE_TER: bool = true;
const WRITE_BRI: bool = true;
const WRITE_WAT: bool = true;

fn new_vec() -> Vec<u8> {
    let mut v = Vec::with_capacity(CELLS);
    unsafe {
        v.set_len(CELLS);
    }
    v
}

// #[inline(always)]
fn frac_to_byte(x: f64) -> u8 {
    if x >= 1. {
        return 255;
    }
    (x * 256.) as u8
}

fn main() {
    const P_GROUPS: usize = 13;
    const SCALAR_C: f64 = 2.;

    let exp: Vec<f64> = (0..P_GROUPS).map(|x| 1.3f64.powf(x as f64)).collect(); // 2^n

    let sum_inv_exp: f64 = exp.iter().copied().map(|x| x.recip()).sum();
    let p: Vec<Perlin> = (1..)
        .take(P_GROUPS)
        .map(|seed| Perlin::new().set_seed(seed))
        .collect();

    use rayon::prelude::*;
    (0..128).into_par_iter().for_each(|var| {
        let mut iw_ground = ImgWriter::new(&format!("images/image_ground_{}.png", var));
        let mut iw_terraced = ImgWriter::new(&format!("images/image_terrraced_{}.png", var));
        let mut iw_water = ImgWriter::new(&format!("images/image_water_{}.png", var));
        let z = 0.;
        for xi in 0..DIMS[0] {
            for yi in 0..DIMS[1] {
                let [x, y] = noise_pt([xi, yi]);
                let ground = {
                    let mut value = 0.;
                    for (i, perlin) in p.iter().enumerate() {
                        let scalar = SCALAR_C * exp[i];
                        let v = perlin.get([x * scalar, y * scalar, z]) * 0.5 + 0.5;
                        assert!(0. <= v && v <= 1.);
                        value += v / exp[i];
                    }
                    value / sum_inv_exp
                };
                let ground_byte = frac_to_byte(ground);

                const MASK: u8 = 0b11111000;
                let terraced_byte = ground_byte & MASK;

                let water = { p[0].get([x * -0.7, y * -0.7, var as f64 * 0.01]) * 0.3 + 0.5 };
                let [water_rg_byte, water_b_byte] = {
                    if ground < water {
                        let depth = water - ground;
                        let b_darkness = (depth * 6.).min(1.);
                        let b = 1. - b_darkness;
                        // dbg!(depth, b_darkness, b);
                        [0, frac_to_byte(b)]
                    } else {
                        [terraced_byte, terraced_byte]
                    }
                };

                iw_ground
                    .pixel(&[ground_byte, ground_byte, ground_byte, 0xff])
                    .unwrap();
                iw_terraced
                    .pixel(&[terraced_byte, terraced_byte, terraced_byte, 0xff])
                    .unwrap();
                iw_water
                    .pixel(&[water_rg_byte, water_rg_byte, water_b_byte, 0xff])
                    .unwrap();
            }
        }
    });
}

fn foo() {
    const P_GROUPS: usize = 13;
    const SCALAR_C: f64 = 2.;

    let exp: Vec<f64> = (0..P_GROUPS).map(|x| 1.3f64.powf(x as f64)).collect(); // 2^n
                                                                                // let squaring: Vec<f64> = (0..P_GROUPS).map(|x| (x * x) as f64).yilect(); // n^2

    let sum_inv_exp: f64 = exp.iter().copied().map(|x| x.recip()).sum();

    struct Vecs {
        ground: Vec<u8>,
        terraced: Vec<u8>,
        bridges: Vec<u8>,
        water: Vec<u8>,
    }

    let mut vecs = Vecs {
        ground: new_vec(),
        terraced: new_vec(),
        bridges: new_vec(),
        water: new_vec(),
    };

    let p: Vec<Perlin> = (1..)
        .take(P_GROUPS)
        .map(|seed| Perlin::new().set_seed(seed))
        .collect();
    for _ in 0..1 {
        let z = 0.;
        let ground_val_fn = |[xi, yi]: [usize; 2]| {
            let [x, y] = noise_pt([xi, yi]);
            let mut value = 0.0;
            for (i, perlin) in p.iter().enumerate() {
                let scalar = SCALAR_C * exp[i];
                let v = perlin.get([x * scalar, y * scalar, z]) * 0.5 + 0.5;
                assert!(0. <= v && v <= 1.);
                value += v / exp[i];
            }
            (value * 255. / sum_inv_exp) as u8
        };

        for xi in 0..DIMS[0] {
            for yi in 0..DIMS[1] {
                vecs.ground[index([xi, yi])] = ground_val_fn([xi, yi]);
            }
        }
        if WRITE_RAW {
            png_dump(
                &format!("images/seed_{:.5}_0_ground.png", z),
                vecs.ground.iter().map(|&b| [b; 3]),
            );
        }

        const MASK: u8 = 0b11111000;
        const STEP: u8 = 0b00001000;
        const HALF: u8 = 0b00000100;

        for (&ground, terraced) in vecs.ground.iter().zip(vecs.terraced.iter_mut()) {
            *terraced = ground & MASK;
        }
        if WRITE_TER {
            png_dump(
                &format!("images/seed_{:.5}_1_terraced.png", z),
                vecs.terraced.iter().map(|&b| [b; 3]),
            );
        }

        vecs.bridges.clear();
        vecs.bridges.extend(vecs.terraced.iter().copied());

        let p0 = &p[0];
        for xi in 1..DIMS[0] {
            for yi in 1..DIMS[1] {
                let left = vecs.terraced[index([xi - 1, yi])];
                let up = vecs.terraced[index([xi, yi - 1])];
                let me = vecs.terraced[index([xi, yi])];
                fn dist(a: u8, b: u8) -> u8 {
                    if a < b {
                        b - a
                    } else {
                        a - b
                    }
                }
                let avg = left / 2 + up / 2;
                if dist(me, left) > STEP || dist(me, up) > STEP {
                    continue;
                }
                let check = || {
                    let [x, y] = noise_pt([xi, yi]);
                    p0.get([x * 45. * SCALAR_C, y * 45. * SCALAR_C, z]) > 0.76
                };
                if avg < me && check() {
                    vecs.bridges[index([xi, yi])] = 255;
                } else if avg > me && check() {
                    vecs.bridges[index([xi, yi])] = 255;
                }
            }
        }
        if WRITE_BRI {
            png_dump(
                &format!("images/seed_{:.5}_2_bridges.png", z),
                vecs.bridges.iter().map(|&b| [b; 3]),
            );
        }
        for xi in 1..DIMS[0] {
            for yi in 1..DIMS[1] {
                let idx = index([xi, yi]);
                // let [x, y] = noise_pt([xi, yi]);
                // let mut s = 0.0;
                // s += p[0].get([x + z.sin(), y, 0.]) * 0.82;
                // s += p[1].get([(x + z.sin()) * 25.0, y * 25.0, z]) * 0.18;
                let s = 0.0;
                let wat_byte = ((s * 0.5 + 0.5) * 256.) as u8;
                let nb = &mut vecs.water[idx];
                let r = vecs.ground[idx];
                let n = &mut vecs.bridges[idx];
                if wat_byte > r {
                    *n = 255 - 8u8.checked_mul(wat_byte - *n).unwrap_or(255);
                    *nb = 0;
                } else {
                    *nb = vecs.bridges[idx];
                };
            }
        }
        if WRITE_WAT {
            png_dump(
                &format!("images/seed_{:.5}_3_water.png", z),
                vecs.bridges
                    .iter()
                    .zip(vecs.water.iter())
                    .map(|(&b, &nb)| [nb, nb, b]),
            );
        }
    }
}

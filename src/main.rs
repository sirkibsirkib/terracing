const DIMS: [usize; 2] = [512; 2];
const CELLS: usize = DIMS[0] * DIMS[1];

use core::cmp::Ordering::Equal;
use noise::{NoiseFn, Perlin, Seedable};
use rand::Rng;
use rand::SeedableRng;
use std::io::BufWriter;
use std::io::Write;
use std::{fs::File, path::Path};

fn normalized(n: f64) -> f64 {
    n * 0.5 + 0.5
}
fn scaled(i: [f64; 2], scalar: f64) -> [f64; 3] {
    [i[0] * scalar, i[1] * scalar, 0.]
}

fn index([row, col]: [usize; 2]) -> usize {
    (row * DIMS[1]) + col
}

fn png_dump(path: &str, bytes: &[u8; CELLS]) {
    let path = Path::new(path);
    let file = File::create(path).unwrap();
    let w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, DIMS[0] as u32, DIMS[1] as u32);
    encoder.set_color(png::ColorType::RGBA);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap().into_stream_writer();
    for &byte in bytes.iter() {
        writer.write(&[byte, byte, byte, 0xff]).unwrap();
    }
    writer.flush().unwrap();
}

fn foo() {
    let mut arr = [0u8; CELLS];
    let mut n = noise::BasicMulti::new().set_seed(0);

    n.octaves = 6;
    n.frequency = 2.;
    n.lacunarity = std::f64::consts::PI * 2.0 / 3.0;
    n.persistence = 0.5;

    for row in 0..DIMS[0] {
        for col in 0..DIMS[1] {
            let xy = [row as f64 / DIMS[0] as f64, col as f64 / DIMS[0] as f64];
            let value = n.get(xy);
            arr[index([row, col])] = ((value * 0.5 + 0.5) * 256.0) as u8;
        }
    }
    png_dump("multi.png", &arr);
}

fn main() {
    foo();
    return;
    const SCALES: [f64; 10] = [0.125, 0.25, 0.5, 1., 2., 4., 8., 16., 32., 64.];
    let offset = 0;

    let p = [
        Perlin::new().set_seed(offset + 0),
        Perlin::new().set_seed(offset + 1),
        Perlin::new().set_seed(offset + 2),
        Perlin::new().set_seed(offset + 3),
        Perlin::new().set_seed(offset + 4),
        Perlin::new().set_seed(offset + 5),
        Perlin::new().set_seed(offset + 6),
        Perlin::new().set_seed(offset + 7),
        Perlin::new().set_seed(offset + 8),
        Perlin::new().set_seed(offset + 9),
        Perlin::new().set_seed(offset + 10),
        Perlin::new().set_seed(offset + 11),
    ];

    let raw_val_fn = move |[row, col]: [usize; 2]| {
        let xy = [row as f64 / DIMS[0] as f64, col as f64 / DIMS[0] as f64];

        let mut raw = 0.0;
        let threshscale = p[10].get(scaled(xy, 8.0)) + 1.0;
        let thresh = p[11].get(scaled(xy, threshscale)) * 0.4 + 0.5;
        for i in 0..10 {
            let sample = p[i].get(scaled(xy, SCALES[i])) * 0.5 + 0.5;
            if sample > thresh {
                let sample = (sample - thresh) * (1. - thresh).min(2.0);
                raw += sample / 10.0;
            }
        }
        (raw * 256.0) as u8
    };

    let mut arr = [0u8; CELLS];

    for row in 0..DIMS[0] {
        for col in 0..DIMS[1] {
            let value = raw_val_fn([row, col]);
            arr[index([row, col])] = value;
        }
    }
    png_dump(&format!("seed_{}_0raw.png", offset), &arr);

    const MASK: u8 = 0b11110000;
    const STEP: u8 = 0b00010000;
    const HALF: u8 = 0b00001000;

    for cell in arr.iter_mut() {
        *cell = *cell & MASK;
    }
    png_dump(&format!("seed_{}_1terraced.png", offset), &arr);

    let mut arr2 = arr;

    let mut rng = rand::rngs::SmallRng::from_seed([4; 16]);
    for row in 1..DIMS[0] {
        for col in 1..DIMS[1] {
            let left = arr[index([row - 1, col])];
            let up = arr[index([row, col - 1])];
            let me = arr[index([row, col])];
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
            if avg < me && rng.gen_bool(0.03) {
                arr2[index([row, col])] = 255;
            } else if avg > me && rng.gen_bool(0.03) {
                arr2[index([row, col])] = 255;
            }
        }
    }
    png_dump(&format!("seed_{}_2bridges.png", offset), &arr2);
}

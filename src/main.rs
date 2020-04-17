use noise::{NoiseFn, Perlin, Seedable};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

const DIMS: [usize; 2] = [512; 2];

fn noise_pt([xi, yi]: [usize; 2]) -> [f64; 2] {
    [xi as f64 / DIMS[0] as f64, yi as f64 / DIMS[1] as f64]
}

struct ImgWriter {
    w: png::StreamWriter<'static, BufWriter<File>>,
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

fn frac_to_byte(x: f64) -> u8 {
    if x >= 1. {
        return 255;
    }
    (x * 256.) as u8
}

fn main() {
    const P_GROUPS: usize = 13;
    const SCALAR_C: f64 = 2.7;
    const SEED_OFFSET: u32 = 4;

    let exp: Vec<f64> = (0..P_GROUPS).map(|x| 1.3f64.powf(x as f64)).collect(); // 2^n

    let sum_inv_exp: f64 = exp.iter().copied().map(|x| x.recip()).sum();
    let p: Vec<Perlin> = (SEED_OFFSET..)
        .take(P_GROUPS)
        .map(|seed| Perlin::new().set_seed(seed))
        .collect();

    use rayon::iter::{IntoParallelIterator as _, ParallelIterator as _};
    (0..32).into_par_iter().for_each(|var| {
        let mut iw_ground = ImgWriter::new(&format!("images/image_a_ground_{}.png", var));
        let mut iw_terrace = ImgWriter::new(&format!("images/image_b_terraced_{}.png", var));
        let mut iw_water = ImgWriter::new(&format!("images/image_c_water_{}.png", var));
        let mut iw_ramps = ImgWriter::new(&format!("images/image_d_ramps_{}.png", var));

        let z = var as f64 * 0.0001;
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

                const TERRACES: f64 = 30.;
                const TERRACE_STEP: f64 = 1. / TERRACES;
                let terrace = (ground * TERRACES).round() / TERRACES;
                let terrace_byte = frac_to_byte(terrace);

                let water = {
                    //
                    let pt = [x * -0.7, y * -0.7, var as f64 * 0.01];
                    p[0].get(pt) * 0.1 + 0.5
                };
                let [water_rg_byte, water_b_byte, ramp_rg_byte, ramp_b_byte] = {
                    if ground < water {
                        let depth = water - ground;
                        let b_darkness = (depth * 8.).min(0.6);
                        let b_byte = frac_to_byte(1. - b_darkness * b_darkness);
                        [0, b_byte, 0, b_byte]
                    } else {
                        let ramp_byte = {
                            const BRIDGE_PROP: f64 = 0.1;
                            let terrace_dist = (ground - terrace).abs();
                            const HALFWAY_DIST: f64 = TERRACE_STEP / 2.;
                            if (terrace_dist - HALFWAY_DIST).abs() <= HALFWAY_DIST * BRIDGE_PROP {
                                // this is a ramp!
                                ground_byte / 2
                            } else {
                                // not a ramp!
                                terrace_byte
                            }
                        };
                        [terrace_byte, terrace_byte, ramp_byte, ramp_byte]
                    }
                };

                iw_ground
                    .pixel(&[ground_byte, ground_byte, ground_byte, 0xff])
                    .unwrap();
                iw_terrace
                    .pixel(&[terrace_byte, terrace_byte, terrace_byte, 0xff])
                    .unwrap();
                iw_water
                    .pixel(&[water_rg_byte, water_rg_byte, water_b_byte, 0xff])
                    .unwrap();
                iw_ramps
                    .pixel(&[ramp_rg_byte, ramp_rg_byte, ramp_b_byte, 0xff])
                    .unwrap();
            }
        }
    });
}

use noise::{NoiseFn, Perlin, Seedable};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

mod rivers;

const DIMS: [usize; 2] = [256; 2];

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

fn exp_sample<'a>(
    perlins: impl Iterator<Item = &'a Perlin> + 'a,
    [x, y, z]: [f64; 3],
    mut scalar: f64, // doubles
) -> f64 {
    const FACTOR: f64 = 2.;
    // invariant: scalar.recip() == scalar_recip
    let mut scalar_recip = scalar.recip(); // halves.
    let mut sample = 0.;
    // invariant: -1 <= (sample / sample_unnorm) <= 1
    let mut sample_unnorm = 0.;
    for perlin in perlins {
        let v = perlin.get([x * scalar, y * scalar, z]);

        sample += v * scalar_recip;
        sample_unnorm += scalar_recip;

        scalar *= FACTOR;
        scalar_recip /= FACTOR;
    }
    sample /= sample_unnorm;
    assert!(-1. <= sample && sample <= 1.);
    sample
}

fn main() {
    rivers::rivers();
    return;
    const PERLINS: usize = 7;
    const SCALAR_C: f64 = 2.7;
    const SEED_OFFSET: u32 = 5;
    const TERRACES: f64 = 25.;
    const RAMP_PROP: f64 = 0.14;

    let p: Vec<Perlin> = (SEED_OFFSET..)
        .take(PERLINS)
        .map(|seed| Perlin::new().set_seed(seed))
        .collect();

    use rayon::iter::{IntoParallelIterator as _, ParallelIterator as _};
    (0..1).into_par_iter().for_each(|var| {
        let mut iw_ground = ImgWriter::new(&format!("images/image_a_ground_{}.png", var));
        let mut iw_terrace = ImgWriter::new(&format!("images/image_b_terraced_{}.png", var));
        let mut iw_water = ImgWriter::new(&format!("images/image_c_water_{}.png", var));
        let mut iw_ramps = ImgWriter::new(&format!("images/image_d_ramps_{}.png", var));

        let ground_z = 0.;
        for yi in 0..DIMS[1] {
            for xi in 0..DIMS[0] {
                let [x, y] = noise_pt([xi, yi]);
                let ground = exp_sample(p.iter(), [x, y, ground_z], SCALAR_C) * 0.5 + 0.5;
                let ground_byte = frac_to_byte(ground);

                let (terrace, is_ramp) = {
                    let approx_level = ground * TERRACES;
                    let mut level = approx_level.trunc();
                    let mut is_ramp = {
                        // this pixel is a ramp if it was CLOSE to being rounded differently
                        let diff = approx_level - level;
                        // is_ramp if we were close to being rounded higher
                        diff > (1. - RAMP_PROP)
                    };
                    let level_is_even = level as u32 % 2 == 0;
                    {
                        let mut sample =
                            exp_sample(p.iter().rev(), [x, y, ground_z], -SCALAR_C * 2.);
                        const INC_WHEN_OVER: f64 = 0.2;
                        const CLOSE_WHEN_OVER: f64 = INC_WHEN_OVER * (1. - RAMP_PROP * 0.5);
                        if level_is_even {
                            sample = -sample;
                        }
                        if sample > CLOSE_WHEN_OVER {
                            if sample > INC_WHEN_OVER {
                                level += 1.;
                                is_ramp = false;
                            } else {
                                // we were close to being higher
                                is_ramp = true;
                            }
                        }
                    }
                    (level / TERRACES, is_ramp)
                };
                let terrace_byte = frac_to_byte(terrace);

                let water_z = var as f64 * 0.01;
                let water = {
                    //
                    let pt = [x * -0.7, y * -0.7, water_z];
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
                            if is_ramp {
                                // this is a cliff
                                let pt = [x * 20. * SCALAR_C, y * 20. * SCALAR_C, ground_z];
                                if p[1].get(pt) > 0.55 {
                                    // ramp
                                    ground_byte
                                } else {
                                    // cliff
                                    ground_byte / 2
                                }
                            } else {
                                // flat terrace
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

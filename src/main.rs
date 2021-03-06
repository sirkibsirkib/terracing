use noise::{NoiseFn, OpenSimplex as Field, Seedable};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

const DIMS: [usize; 2] = [2048; 2];

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

#[inline]
fn normalize(x: f64) -> f64 {
    (x * 0.5 * 1.9) + 0.5
}

static mut MAX: f64 = -0.999;
fn exp_sample<'a>(
    fields: impl Iterator<Item = &'a Field> + 'a,
    [x, y]: [f64; 2],
    mut scalar: f64, // doubles
) -> f64 {
    const FACTOR: f64 = 2.1;
    // invariant: scalar.recip() == scalar_recip
    let mut scalar_recip = scalar.recip(); // halves.
    let mut sample = 0.;
    // invariant: -1 <= (sample / mass) <= 1
    let mut mass = 0.;
    for field in fields {
        let v = field.get([x * scalar, y * scalar]);

        sample += v * scalar_recip * 1.;
        mass += scalar_recip;

        scalar *= FACTOR;
        scalar_recip /= FACTOR;
    }
    sample / mass

    // sample *= 1.9 / mass;

    // // more_asserts::assert_le!(-1., sample);
    // // more_asserts::assert_le!(sample, 1.);
    // unsafe { MAX = MAX.max(sample) }

    // sample
}

fn _field_test_max(field: &Field) -> f64 {
    let mut m: f64 = -99999999.;
    let mut x = 0.;
    for _ in 0usize..1_000_000 {
        let sample = field.get([x, 0.]);
        m = m.max(sample);
        x += 0.0001;
    }
    m
}

fn main() {
    const TOT_OCTAVES: usize = 32;
    const GROUND_OCTAVES: usize = 6;
    const WATER_OCTAVES: usize = 2;
    const TERRACE_OCTAVES: usize = 4;

    const GROUND_OCTAVE_OFFSET: usize = 0;
    const TERRACE_OCTAVE_OFFSET: usize = 2;
    const WATER_OCTAVE_OFFSET: usize = 4;
    const RAMP_OCTAVE_OFFSET: usize = 6;

    const SCALAR_C: f64 = 4.;
    const TERRACES: f64 = 21.;
    const RAMP_PROP: f64 = 0.15;

    let fields: Vec<Field> = (0..)
        .take(TOT_OCTAVES)
        .map(|seed| Field::new().set_seed(seed))
        .collect();

    use rayon::iter::{IntoParallelIterator as _, ParallelIterator as _};
    (8usize..16).into_par_iter().for_each(|var| {
        let mut iw_ground = ImgWriter::new(&format!("images/image_{}a_ground.png", var));
        let mut iw_terrace = ImgWriter::new(&format!("images/image_{}b_terraced.png", var));
        let mut iw_water = ImgWriter::new(&format!("images/image_{}c_water.png", var));
        let mut iw_ramps = ImgWriter::new(&format!("images/image_{}d_ramps.png", var));

        // let ground_z = 0.;
        for yi in 0..DIMS[1] {
            for xi in 0..DIMS[0] {
                let p = fields.iter().cycle().skip(var);

                let [x, y] = noise_pt([xi, yi]);
                let ground = {
                    let raw = normalize(exp_sample(
                        p.clone().skip(GROUND_OCTAVE_OFFSET).take(GROUND_OCTAVES),
                        [x, y],
                        SCALAR_C,
                    ));
                    let sample = |offset, scalar| {
                        normalize(
                            p.clone()
                                .nth(GROUND_OCTAVE_OFFSET + GROUND_OCTAVES + offset)
                                .unwrap()
                                .get([y * scalar, x * scalar]),
                        )
                    };
                    let mut samples = [
                        sample(07, SCALAR_C * 00.4),
                        sample(08, SCALAR_C * 00.7),
                        sample(09, SCALAR_C * 01.09),
                        sample(10, SCALAR_C * 01.80),
                        sample(11, SCALAR_C * 01.99),
                        sample(11, SCALAR_C * 2.045),
                        sample(12, SCALAR_C * raw),
                    ];
                    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    let (lower_idx, upper_contrib) = match raw {
                        x if x < 0.2 => (0, (x - 0.0) / 0.2),
                        x if x < 0.4 => (1, (x - 0.2) / 0.2),
                        x if x < 0.6 => (2, (x - 0.4) / 0.2),
                        x if x < 0.8 => (3, (x - 0.6) / 0.2),
                        x => (4, (x - 0.8) / 0.2),
                    };
                    assert!(0. <= upper_contrib && upper_contrib <= 1.);
                    samples[lower_idx] * (1. - upper_contrib)
                        + samples[lower_idx + 1] * upper_contrib
                };
                let ground_byte = frac_to_byte(ground);

                let (rawterrace, terrace, is_ramp) = {
                    let approx_level = ground * TERRACES;
                    let mut level = approx_level.trunc();
                    let rawterrace = level / TERRACES;
                    let mut is_ramp = {
                        // this pixel is a ramp if it was CLOSE to being rounded differently
                        let diff = approx_level - level;
                        // is_ramp if we were close to being rounded higher
                        diff > (1. - RAMP_PROP)
                    };
                    let level_is_even = level as u32 % 2 == 0;
                    {
                        let mut sample = normalize(exp_sample(
                            p.clone().skip(TERRACE_OCTAVE_OFFSET).take(TERRACE_OCTAVES),
                            [x, y],
                            -SCALAR_C * 2.,
                        ));
                        const INC_WHEN_OVER: f64 = 0.6;
                        const CLOSE_WHEN_OVER: f64 = 0.582;
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
                    (rawterrace, level / TERRACES, is_ramp)
                };
                let terrace_byte = frac_to_byte(terrace);
                let rawterrace_byte = frac_to_byte(rawterrace);

                // let water_z = 0.8;
                let water = {
                    normalize(exp_sample(
                        p.clone().skip(WATER_OCTAVE_OFFSET).take(WATER_OCTAVES),
                        [x, y],
                        SCALAR_C * 0.25,
                    ))
                };
                let [water_rg_byte, water_b_byte, ramp_rg_byte, ramp_b_byte] = {
                    if ground < water {
                        let depth = water - ground;
                        let b_darkness = (depth * 6.).min(0.6);
                        let b = 0.4 * (1. - b_darkness * b_darkness);
                        let rg = b * 0.4;
                        let b_byte = frac_to_byte(b);
                        let rg_byte = frac_to_byte(rg);
                        [rg_byte, b_byte, rg_byte, b_byte]
                    } else {
                        let ramp_byte = {
                            if is_ramp {
                                // this is a cliff
                                let pt = [x * 20. * SCALAR_C, y * 20. * SCALAR_C];
                                if normalize(p.clone().nth(RAMP_OCTAVE_OFFSET).unwrap().get(pt))
                                    > 0.65
                                {
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
                    .pixel(&[rawterrace_byte, terrace_byte, terrace_byte, 0xff])
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
    // unsafe {
    //     dbg!(MAX);
    // }
}

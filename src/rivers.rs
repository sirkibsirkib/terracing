use super::*;
use rand::Rng;
use rand::SeedableRng;
use std::collections::HashMap;

type RiverIndex = usize;
struct RiverNode {
    parent: Option<RiverIndex>,
    coord: [f32; 3],
}
struct LandNode {
    coord: [f32; 3],
    river_supports: [RiverIndex; 3],
}

pub fn rivers() {
    fn discrete_coord([x, y]: [f32; 2]) -> [usize; 2] {
        [(x * DIMS[0] as f32) as usize, (y * DIMS[1] as f32) as usize]
    }

    let mut iw_rivers = ImgWriter::new("images/rivers.png");
    let mut rng = rand::rngs::SmallRng::from_seed([1; 16]);
    let mut river_nodes = vec![];
    for _ in 0..9 {
        let coord = [
            rng.gen_range(0., 1.),
            rng.gen_range(0., 1.),
            rng.gen_range(0., 1.),
        ];
        let n = RiverNode {
            parent: None,
            coord,
        };
        river_nodes.push(n);
    }

    let closest_3_river_indices = |coord: [f32; 2]| {
        let distances = (0..river_nodes.len())
            .map(|index| {
                let river_coord = river_nodes[index].coord;
                let dx = river_coord[0] - coord[0];
                let dy = river_coord[1] - coord[1];
                (dx * dx + dy * dy).sqrt()
            })
            .collect::<Vec<_>>();
        let mut indices = (0..river_nodes.len()).collect::<Vec<_>>();
        indices.sort_by(|&a, &b| distances[a].partial_cmp(&distances[b]).unwrap());
        let mut answer = [indices[0], indices[1], indices[2]];
        answer.sort();
        answer
    };

    let mut pixel_map: HashMap<[usize; 2], [u8; 3]> = Default::default();

    let vor = voronoi::voronoi(
        river_nodes
            .iter()
            .map(|r| voronoi::Point {
                x: ordered_float::OrderedFloat(r.coord[0].into()),
                y: ordered_float::OrderedFloat(r.coord[1].into()),
            })
            .collect(),
        1.,
    );
    for vert in vor.vertices.iter() {
        println!("{:?}", vert.coordinates);
        let key = discrete_coord([vert.coordinates.x.0 as f32, vert.coordinates.y.0 as f32]);
        pixel_map.insert(key, [255, 100, 100]);
    }
    for r in river_nodes.iter() {
        let key = discrete_coord([r.coord[0], r.coord[1]]);
        pixel_map.insert(key, [100, 100, 255]);
    }
    // use rayon::iter::{IntoParallelIterator as _, ParallelIterator as _};
    for yi in 0..DIMS[1] {
        for xi in 0..DIMS[0] {
            let pixel = if let Some([r, g, b]) = pixel_map.get(&[xi, yi]).copied() {
                [r, g, b, 255]
            } else {
                [0, 0, 0, 255]
            };
            iw_rivers.pixel(&pixel).unwrap();
        }
    }
}

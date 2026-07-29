use crate::algorithm::data::planet::Planet;
use crate::algorithm::data::planet_algorithms::{create_and_prepare_algo, PlanetAlgorithm};
use crate::algorithm::data::planet_grid::{
    get_planet_grid, position_hash, PlanetGrid, DATA_LENGTH, PRECISION, STRIDE,
};

use super::vector_f3::VectorF3;
use std::cell::RefCell;
use std::f64::consts::PI;

const VALID_WORDS: usize = DATA_LENGTH.div_ceil(u64::BITS as usize);

#[derive(Default)]
struct TerrainCache {
    heights: Vec<u16>,
    valid: Vec<u64>,
    touched: Vec<usize>,
}

impl TerrainCache {
    fn new() -> Self {
        Self {
            heights: vec![0; DATA_LENGTH],
            valid: vec![0; VALID_WORDS],
            touched: Vec::new(),
        }
    }

    fn clear(&mut self) {
        for index in self.touched.drain(..) {
            self.valid[index / u64::BITS as usize] &= !(1 << (index % u64::BITS as usize));
        }
    }
}

thread_local! {
    static TERRAIN_CACHE_POOL: RefCell<Vec<TerrainCache>> = const { RefCell::new(Vec::new()) };
}

pub struct PlanetRawData {
    grid: &'static PlanetGrid,
    algo: Box<dyn PlanetAlgorithm>,
    cache: TerrainCache,
}

impl PlanetRawData {
    pub fn new(planet: &Planet) -> Self {
        let cache = TERRAIN_CACHE_POOL
            .with(|pool| pool.borrow_mut().pop())
            .unwrap_or_else(TerrainCache::new);
        Self {
            grid: get_planet_grid(),
            algo: create_and_prepare_algo(planet),
            cache,
        }
    }

    #[inline]
    fn get_height(&mut self, index: usize) -> f32 {
        let word = index / u64::BITS as usize;
        let mask = 1 << (index % u64::BITS as usize);
        if self.cache.valid[word] & mask == 0 {
            self.cache.heights[index] = (self.algo.get_height(index) * 100.0) as u16;
            self.cache.valid[word] |= mask;
            self.cache.touched.push(index);
        }
        self.cache.heights[index] as f32
    }

    pub fn query_height_normalized(&mut self, vpos_normalized: &VectorF3) -> f32 {
        let index1 = self.grid.index_map[position_hash(vpos_normalized, 0)];

        let num1: f64 = (PI / (PRECISION as f64 * 2.0)) * 1.2_f64;
        let num2: f64 = num1 * num1;

        let mut num3: f32 = 0.0f32;
        let mut num4: f32 = 0.0f32;

        for i3 in -1..=3 {
            let i4 = index1 + i3 * STRIDE;
            for i2 in -1_i32..=3 {
                let idx4 = (i4 + i2) as usize;
                if idx4 < DATA_LENGTH {
                    let sqr_mag = self.grid.vertices[idx4].distance_sq_from(vpos_normalized);
                    if (sqr_mag as f64) <= num2 {
                        let num5 = 1.0f32 - (sqr_mag.sqrt() / num1 as f32);
                        let num6 = self.get_height(idx4);
                        num3 += num5;
                        num4 += num6 * num5;
                    }
                }
            }
        }

        if num3 != 0.0f32 {
            num4 / num3 * 0.01
        } else {
            self.get_height(0) * 0.01
        }
    }

    #[inline]
    pub fn query_height(&mut self, vpos: &VectorF3) -> f32 {
        let mut vpos = *vpos;
        vpos.normalize();
        self.query_height_normalized(&vpos)
    }
}

impl Drop for PlanetRawData {
    fn drop(&mut self) {
        self.cache.clear();
        let cache = std::mem::take(&mut self.cache);
        TERRAIN_CACHE_POOL.with(|pool| pool.borrow_mut().push(cache));
    }
}

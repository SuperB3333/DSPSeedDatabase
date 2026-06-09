mod data;
mod worldgen;

use postgres::{Client, NoTls};
use crossbeam_channel::{Sender, Receiver, unbounded};
use std::time::{Duration, Instant};
use std::io::Write;
use std::ops::Range;
use std::thread;
use crate::data::enums::{PlanetType, ORES};
use crate::data::game_desc::GameDesc;
use crate::worldgen::galaxy_gen::create_galaxy;

const START_SEED: i32 = 0;
const END_SEED: i32 = 30;
const STAR_COUNT: usize = 64;
const REC_MULTIPLIER: f32 = 1.0;
const THREAD_COUNT: usize = 1;
const COMMIT_THREADS: usize = 1;
const COMMIT_ENTRIES: usize = 10;
const DB_STR: &str = "postgres://postgres:rootpassword@localhost:5432/dsp?sslmode=disable";



fn gen_formatted(seed: i32, star_count: usize, resource_multiplier: f32) -> Result<(String, String), Box<dyn std::error::Error>> {
    let mut game_desc: GameDesc = GameDesc::default();
    game_desc.seed = seed;
    game_desc.star_count = star_count;
    game_desc.resource_multiplier = resource_multiplier;
    let galaxy = create_galaxy(&game_desc);

    let mut stars: String = String::with_capacity(star_count * 128);
    let mut planets: String = String::with_capacity(star_count * 256 * 5);

    for solar_system in galaxy.stars {
        let star = solar_system.star.clone();
        let star_id = star.index as i32 + seed * 100;

        stars.push_str(format!("{},{},{},{},{},{},{},{},",
               star_id,
               seed,
               star.position.magnitude(),
               star.index,
               star.get_luminosity(),
               star.get_dyson_radius(),
               star.star_type as i32 + 1,
               *star.get_spectr() as i32
        ).as_str());

        for (index, ore) in ORES[1..15].iter().enumerate() {
            stars.push_str(format!("{}{}", solar_system.get_avg_vein(ore) as i32, if index == 13 {"\n"} else {","}).as_str());
        }

        for planet in solar_system.get_planets() {
            let satellite_count = -1; //todo implement

            let mut gas_h = &0.0;
            let mut gas_d = &0.0;
            let mut gas_i = &0.0;
            if planet.get_type() == &PlanetType::Gas {
                for (gas, rate) in planet.get_gases() {
                    match gas {
                        1120 => gas_h = rate,
                        1121 => gas_d = rate,
                        1011 => gas_i = rate,
                        _ => panic!("Illegal state: gas was not 1120, 1121 or 1011! This should never happen.")
                    }
                }
            }

            planets.push_str(format!("{},{},{},{},{},{},{},{},{},{},{},{},{},",
                   star_id,
                   planet.index,
                   planet.get_theme().water_item_id,
                   planet.get_type() == &PlanetType::Gas,
                   planet.get_orbital_radius(),
                   planet.get_orbital_radius() * 40000.0 < star.get_dyson_radius() as f32,
                   satellite_count,
                   planet.get_theme().temperature,
                   planet.get_theme().id,
                   gas_h, gas_d, gas_i,
                   planet.get_rotation_period() == planet.get_orbital_period()
            ).as_str());

            let veins = planet.get_veins();
            // OPTIMIZATION: Use fixed-size array indexed by VeinType discriminant instead of HashMap.
            // VeinType is #[repr(i32)] with variants None=0..Max=15, so a 16-element array gives
            // O(1) lookup with zero heap allocation, vs HashMap which allocates per-planet.
            // Impact: Eliminates ~256K HashMap allocations per 1000 seeds in the hot loop.
            let mut vein_map: [Option<&crate::data::vein::Vein>; 16] = [None; 16];
            for v in veins.iter() {
                vein_map[v.vein_type as usize] = Some(v);
            }

            if planet.get_type() == &PlanetType::Gas {
                for _ in 0..41 {
                    planets.push_str("-1,");
                }
                planets.push_str("-1\n");
            } else {
                for (index, ore) in ORES[1..15].iter().enumerate() {
                    if let Some(vein) = vein_map[*ore as usize] {
                        planets.push_str(format!("{},{},{}{}",
                            vein.min(),
                            vein.max(),
                            vein.estimate(),
                            if index == 13 { "\n" } else { "," }
                        ).as_str());
                    } else {
                        planets.push_str(format!("-1,-1,-1{}", if index == 13 { "\n" } else { "," }).as_str());
                    }
                }
            }
        }
    }
    Ok((stars, planets))
}

const COPY_PLANET: &str = "COPY planets(star_id, index, water_item, gas_giant, sun_distance, inside_ds, satellites, temperature, theme_id, gas_h, gas_d, gas_i, tidal_lock, min_iron, max_iron, estimate_iron, min_copper, max_copper, estimate_copper, min_silicium, max_silicium, estimate_silicium, min_titanium, max_titanium, estimate_titanium, min_stone, max_stone, estimate_stone, min_coal, max_coal, estimate_coal, min_oil, max_oil, estimate_oil, min_fireice, max_fireice, estimate_fireice, min_diamond, max_diamond, estimate_diamond, min_fractal, max_fractal, estimate_fractal, min_crysrub, max_crysrub, estimate_crysrub, min_grat, max_grat, estimate_grat, min_bamboo, max_bamboo, estimate_bamboo, min_mag, max_mag, estimate_mag) FROM STDIN WITH (FORMAT CSV)";
const COPY_STAR: &str = "COPY stars(id, seed, start_dist, star_index, luminosity, dyson_radius, type, spectr, ore_iron, ore_copper, ore_silicium, ore_titanium, ore_stone, ore_coal, ore_oil, ore_fireice, ore_diamond, ore_fractal, ore_crysrub, ore_grat, ore_bamboo, ore_mag) FROM STDIN WITH (FORMAT CSV)";


fn worker_per_thread(seeds: Range<i32>, send: Sender<(String, String)>) {
    for seed in seeds {
        let entry = gen_formatted(seed, STAR_COUNT, REC_MULTIPLIER).expect("gen_formatted failed");
        send.send(entry).unwrap();

    }
}
fn commit_thread(rec: Receiver<(String, String)>) {
    let mut star_client = Client::connect(DB_STR, NoTls).unwrap();
    let mut planet_client = Client::connect(DB_STR, NoTls).unwrap();

    let mut scpy = star_client.copy_in(COPY_STAR).unwrap();
    let mut pcpy = planet_client.copy_in(COPY_PLANET).unwrap();
    let mut i = 0;
    loop {
        i += 1;
        let (star, planet) = match rec.recv_timeout(Duration::new(1,0)) {
            Ok(msg) => msg,
            Err(_) => break,
        };
        scpy.write_all(star.as_bytes()).expect("writing to scpy failed");
        pcpy.write_all(planet.as_bytes()).expect("writing to pcpy failed");
        if i % COMMIT_ENTRIES == 0 {
            scpy.finish().unwrap();
            pcpy.finish().unwrap();
            scpy = star_client.copy_in(COPY_STAR).unwrap();
            pcpy = planet_client.copy_in(COPY_PLANET).unwrap();
        }

    }
    scpy.finish().unwrap();
    pcpy.finish().unwrap();
    star_client.close().unwrap();
    planet_client.close().unwrap();
}
fn split_chunks(r: Range<i32>, chunks: usize) -> Vec<Range<i32>> {
    let total = (r.end - r.start) as usize;
    let base = total / chunks;
    let mut extra = total % chunks;
    let mut cur = r.start;
    let mut out = Vec::with_capacity(chunks);
    for _ in 0..chunks {
        let add = if extra > 0 { extra -= 1; base + 1 } else { base };
        out.push(cur..(cur + add as i32));
        cur += add as i32;
    }
    out
}
fn main() {
    assert!(START_SEED < END_SEED);
    assert!(THREAD_COUNT < END_SEED as usize);
    let start = Instant::now();

    let all_seeds = START_SEED..END_SEED;
    let workloads = split_chunks(all_seeds, THREAD_COUNT);
    let (entry_sender, entry_reciever): (Sender<(String, String)>, Receiver<(String, String)>) = unbounded();

    let mut work_handles = vec![];
    let mut commit_handles = vec![];
    for work in workloads {
        let thread_sender = entry_sender.clone();
        work_handles.push(thread::spawn(move || {
            worker_per_thread(work, thread_sender);
        }))
    }
    for _ in 0..COMMIT_THREADS {
        let thread_receiver = entry_reciever.clone();
        commit_handles.push(thread::spawn(move || {
            commit_thread(thread_receiver);
        }))
    }
    for handle in work_handles {
        handle.join().unwrap(); // wait for all workers to finish
    }
    drop(entry_sender); // close the channel so recv() returns Err instead of blocking
    for handle in commit_handles {
        handle.join().unwrap(); // wait for senders to finish
    }

    let elapsed = start.elapsed();
    let per_second = (END_SEED - START_SEED) as f32 / elapsed.as_secs() as f32;
    println!("seeds/sec: {:?}", per_second);
}
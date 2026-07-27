#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use crate::algorithm::data::game_desc::GameDesc;
    use crate::algorithm::generate_stars;

    #[test]
    fn test_worldgen() {
        let game = GameDesc {
            star_count: 64,
            resource_multiplier: 1.0,
        };
        let habitable_count = Cell::new(0_i32);
        let galaxy = generate_stars(1, &game, &habitable_count);
        let _result = galaxy
            .first()
            .unwrap()
            .get_planets()
            .get(3)
            .unwrap()
            .get_actual_veins();
    }
}

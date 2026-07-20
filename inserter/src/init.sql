CREATE TABLE themes (
    id SMALLINT PRIMARY KEY,
    name TEXT,
    temperature REAL,
    ocean_type SMALLINT
);
INSERT INTO themes (id, name, temperature, ocean_type) VALUES
(1, 'Ocean 1', 0.0, 1000),
(2, 'Gas 1', 2.0, 0),
(3, 'Gas 2', 1.0, 0),
(4, 'Gas 3', -1.0, 0),
(5, 'Gas 4', -2.0, 0),
(6, 'Desert 1', 2.0, 0),
(7, 'Desert 2', -1.0, 0),
(8, 'Ocean 2', 0.0, 1000),
(9, 'Lava 1', 5.0, -1),
(10, 'Ice 1', -5.0, 1000),
(11, 'Desert 3', -2.0, 0),
(12, 'Desert 4', 1.0, 0),
(13, 'Volcanic 1', 4.0, 1116),
(14, 'Ocean 3', 0.0, 1000),
(15, 'Ocean 4', 0.0, 1000),
(16, 'Ocean 5', 0.0, 1000),
(17, 'Desert 5', 1.0, 0),
(18, 'Ocean 6', 0.0, 1000),
(19, 'Desert 6', 1.0, 0),
(20, 'Desert 7', -2.0, -2),
(21, 'Gas 5', 1.0, 0),
(22, 'Desert 8', 0.0, 1000),
(23, 'Desert 9', 0.08, 0),
(24, 'Desert 10', -4.0, 0),
(25, 'Desert 11', 0.0, 1000);

CREATE UNLOGGED TABLE stars (
    id INT PRIMARY KEY,
    seed INT,
    dyson_radius INT,

    ore_iron INT,
    ore_copper INT,
    ore_silicium INT,
    ore_titanium INT,
    ore_stone INT,
    ore_coal INT,
    ore_oil INT,
    ore_fireice INT,
    ore_diamond INT,
    ore_fractal INT,
    ore_crysrub INT,
    ore_grat INT,
    ore_bamboo INT,
    ore_mag INT,

    start_dist REAL,
    luminosity REAL,
    star_index SMALLINT,
    type SMALLINT,
    spectr SMALLINT
);
CREATE UNLOGGED TABLE planets (
    star_id INT,

    ore_iron INT,
    ore_copper INT,
    ore_silicium INT,
    ore_titanium INT,
    ore_stone INT,
    ore_coal INT,
    ore_oil INT,
    ore_fireice INT,
    ore_diamond INT,
    ore_fractal INT,
    ore_crysrub INT,
    ore_grat INT,
    ore_bamboo INT,
    ore_mag INT,

    sun_distance REAL,
    gas_h REAL,
    gas_d REAL,
    gas_i REAL,
    index SMALLINT,
    orbiting SMALLINT,
    satellites SMALLINT,
    theme_id SMALLINT,
    gas_giant BOOL,
    inside_ds BOOL,
    tidal_lock BOOL,
    UNIQUE(star_id, index)
);

-- Create user for RESTful API
CREATE USER api_user WITH PASSWORD 'api_password';

GRANT USAGE ON SCHEMA public TO api_user;
GRANT SELECT ON stars TO api_user;
GRANT SELECT ON planets TO api_user;
GRANT SELECT ON themes TO api_user;
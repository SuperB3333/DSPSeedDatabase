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
    temperature REAL,
    gas_h REAL,
    gas_d REAL,
    gas_i REAL,
    index SMALLINT,
    orbiting SMALLINT,
    water_item SMALLINT,
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
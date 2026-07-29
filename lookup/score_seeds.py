"""Score-based seed searching for the DSP seed database.

Strictly read-only DB consumer: opens a single connection with
``conn.set_session(readonly=True)`` and issues a single ``SELECT`` (via a CTE).
It ranks seeds by a user-weighted sum of per-seed metrics.

Sentinel rule (critical for score correctness):
  * gas giants carry ``-1`` in all 42 vein columns;
  * rocky planets without a given vein carry ``-1`` in that ore's
    min/max/estimate columns.
Therefore every ore aggregate uses ``GREATEST(p.estimate_<vein>, 0)`` so a
sentinel can never subtract from a score.

Config mirrors ``src/misc.rs`` env-var names/defaults, and the ore metrics are
generated from ``misc.py``'s ``veins`` list.

Note: ``psycopg2`` is imported lazily so that ``--explain`` (which prints the
generated SQL and exits) works even when psycopg2 is not installed and no
Postgres is reachable. Only the DB-executing path requires psycopg2.
"""

import argparse
import csv
import os
import sys
import time

from server.misc import veins


# --- Config: env-var names/defaults mirror src/misc.rs (get_db_str) ---------
DB_DEFAULTS = {
    "netloc": ("PG_NETLOC", "localhost"),
    "port": ("PG_PORT", "5432"),
    "user": ("PG_USER", "postgres"),
    "pass": ("PG_PASS", "rootpassword"),
    "dbname": ("PG_DBNAME", "dsp"),
}


def _env_default(key: str) -> str:
    env_name, fallback = DB_DEFAULTS[key]
    return os.environ.get(env_name, fallback)


# --- Metric registry --------------------------------------------------------
# name -> (sql_expr, description). Each expression is a single aggregate that
# is valid inside the per_star CTE (references s.* and p.*). Values come only
# from this trusted registry; they are never derived from user input.
def _build_metrics():
    metrics = {}
    for vein in veins:
        metrics[f"ore_{vein}"] = (
            f"SUM(COALESCE(p.estimate_{vein}))",
            f"Total estimated {vein} ore across the seed's planets "
            "(sentinel-guarded).",
        )
    metrics["luminosity"] = (
        "SUM(s.luminosity)",
        "Sum of star luminosity over the seed's stars.",
    )
    metrics["max_luminosity"] = (
        "MAX(s.luminosity)",
        "Maximum star luminosity in the seed.",
    )
    metrics["dyson_radius"] = (
        "MAX(s.dyson_radius)",
        "Maximum Dyson sphere radius in the seed.",
    )
    metrics["gas_giants"] = (
        "COUNT(*) FILTER (WHERE p.gas_giant)",
        "Number of gas giant planets in the seed.",
    )
    metrics["tidal_locked"] = (
        "COUNT(*) FILTER (WHERE p.tidal_lock)",
        "Number of tidally locked planets in the seed.",
    )
    metrics["planets_inside_ds"] = (
        "COUNT(*) FILTER (WHERE p.inside_ds)",
        "Number of planets that lie inside the Dyson sphere.",
    )
    metrics["oceans"] = (
        "COUNT(*) FILTER (WHERE (SELECT t.ocean_type FROM themes t WHERE t.id = p.theme_id) = 1000)",
        "Number of ocean (water) worlds in the seed.",
    )
    return metrics


METRICS = _build_metrics()


class WeightAction(argparse.Action):
    """Collect repeatable --weight NAME=FLOAT flags into an ordered dict."""

    def __call__(self, parser, namespace, values, option_string=None):
        weights = getattr(namespace, self.dest, None)
        if weights is None:
            weights = {}
            setattr(namespace, self.dest, weights)
        if "=" not in values:
            parser.error(
                f"--weight expects NAME=FLOAT, got {values!r}"
            )
        name, _, raw = values.partition("=")
        name = name.strip()
        try:
            weight = float(raw)
        except ValueError:
            parser.error(
                f"--weight {name}: {raw!r} is not a valid float"
            )
        weights[name] = weight


def parse_args(argv=None):
    parser = argparse.ArgumentParser(
        description="Rank DSP seeds by a user-weighted sum of metrics "
        "(strictly read-only).",
    )
    parser.add_argument("--host", default=_env_default("netloc"),
                        help="DB host (env PG_NETLOC, default localhost).")
    parser.add_argument("--port", default=_env_default("port"),
                        help="DB port (env PG_PORT, default 5432).")
    parser.add_argument("--user", default=_env_default("user"),
                        help="DB user (env PG_USER, default postgres).")
    parser.add_argument("--pass", dest="password",
                        default=_env_default("pass"),
                        help="DB password (env PG_PASS, default rootpassword).")
    parser.add_argument("--dbname", default=_env_default("dbname"),
                        help="DB name (env PG_DBNAME, default dsp).")

    parser.add_argument("--top", type=int, default=25,
                        help="Number of top seeds to return (default 25).")
    parser.add_argument("--seed-range", nargs=2, type=int,
                        metavar=("LO", "HI"), default=None,
                        help="Optional pre-filter: s.seed >= LO AND s.seed < HI.")
    parser.add_argument("--csv", dest="csv_path", default=None,
                        help="Optional output CSV file path.")
    parser.add_argument("--explain", action="store_true",
                        help="Print the generated SQL and exit (no DB needed).")
    parser.add_argument("--weight", action=WeightAction, dest="weights",
                        metavar="NAME=FLOAT", default=None,
                        help="Repeatable metric weight, e.g. --weight ore_oil=2.")
    return parser.parse_args(argv)


def resolve_weights(weights, parser_error):
    """Validate weights against METRICS; apply default; return ordered dict."""
    if not weights:
        print(
            "No --weight given; defaulting to --weight ore_iron=1. "
            "Pass one or more --weight NAME=FLOAT to customize scoring.",
            file=sys.stderr,
        )
        return {"ore_iron": 1.0}

    unknown = [name for name in weights if name not in METRICS]
    if unknown:
        valid = "\n".join(
            f"  {name:<20} {METRICS[name][1]}" for name in sorted(METRICS)
        )
        parser_error(
            "unknown metric name(s): {}\nValid metrics:\n{}".format(
                ", ".join(unknown), valid
            )
        )
    return dict(weights)


def build_sql(weights, seed_range):
    """Build (sql, params) for the ranking query.

    Only numeric weights, seed bounds and the LIMIT are parameters (%s). All
    SQL fragments come from the trusted METRICS registry.
    """
    requested = list(weights.keys())
    params = []

    # per_star CTE: aggregate planet columns per star first so star-level
    # columns are not double-counted across the planet join, then aggregate
    # per seed in the outer query.
    star_agg_lines = []
    for name in requested:
        expr, _ = METRICS[name]
        star_agg_lines.append(f"           {expr} AS {name}")
    star_agg_sql = ",\n".join(star_agg_lines)

    where_sql = ""
    if seed_range is not None:
        lo, hi = seed_range
        where_sql = "\n     WHERE s.seed >= %s AND s.seed < %s"
        params.extend([lo, hi])

    per_star = (
        "  per_star AS (\n"
        "    SELECT s.seed,\n"
        f"{star_agg_sql}\n"
        "    FROM stars s JOIN planets p ON p.star_id = s.id"
        f"{where_sql}\n"
        "    GROUP BY s.id, s.seed\n"
        "  )"
    )

    # Outer query: re-aggregate the per-star metrics up to the seed level.
    # For SUM-based metrics this sums across stars; for MAX/COUNT metrics we
    # combine them with the same aggregate family so seed-level values are
    # correct regardless of how many stars a seed has.
    seed_agg = {}
    for name in requested:
        expr, _ = METRICS[name]
        upper = expr.upper()
        if upper.startswith("MAX("):
            seed_agg[name] = f"MAX({name})"
        else:
            # SUM(...) and COUNT(...) FILTER(...) roll up additively.
            seed_agg[name] = f"SUM({name})"

    # Weighted score. Weights are parameters; expressions are trusted.
    score_terms = []
    for name in requested:
        score_terms.append(f"%s * COALESCE({seed_agg[name]}, 0)")
        params.append(weights[name])
    score_sql = " + ".join(score_terms)

    metric_cols = ",\n".join(
        f"         {seed_agg[name]} AS {name}" for name in requested
    )

    params.append(_limit_placeholder_marker())  # replaced below

    sql = (
        "WITH\n"
        f"{per_star}\n"
        "SELECT seed,\n"
        f"       {score_sql} AS score,\n"
        f"{metric_cols}\n"
        "FROM per_star\n"
        "GROUP BY seed\n"
        "ORDER BY score DESC\n"
        "LIMIT %s;"
    )
    return sql, params, requested


# Sentinel object so build_sql can append the limit param in the right slot
# without importing anything DB-specific.
_LIMIT_MARKER = object()


def _limit_placeholder_marker():
    return _LIMIT_MARKER


def finalize_params(params, top):
    return [top if p is _LIMIT_MARKER else p for p in params]


def render_explain(sql, weights, seed_range, top):
    """Render the SQL plus the concrete parameter values for --explain."""
    out = [sql, "", "-- Parameters (in order):"]
    if seed_range is not None:
        out.append(f"--   seed_range LO = {seed_range[0]}")
        out.append(f"--   seed_range HI = {seed_range[1]}")
    for name, w in weights.items():
        out.append(f"--   weight {name} = {w}")
    out.append(f"--   LIMIT = {top}")
    return "\n".join(out)


def format_table(header, rows):
    """Return an aligned plain-text table."""
    cols = [header] + [[_fmt_cell(c) for c in r] for r in rows]
    widths = [0] * len(header)
    for row in cols:
        for i, cell in enumerate(row):
            widths[i] = max(widths[i], len(cell))
    lines = []
    for row in cols:
        lines.append("  ".join(
            cell.rjust(widths[i]) for i, cell in enumerate(row)
        ))
    # Insert a separator after the header.
    sep = "  ".join("-" * w for w in widths)
    lines.insert(1, sep)
    return "\n".join(lines)


def _fmt_cell(value):
    if isinstance(value, float):
        # Trim trailing zeros for readability while keeping precision.
        return f"{value:.4f}".rstrip("0").rstrip(".") or "0"
    return str(value)


def run_query(args, weights, requested, sql, params):
    """Execute the read-only query and return (header, rows)."""
    import psycopg2  # lazy import: only needed when actually hitting the DB

    conn = psycopg2.connect(
        host=args.host,
        port=args.port,
        user=args.user,
        password=args.password,
        dbname=args.dbname,
    )
    try:
        # Hard read-only guard: no writes can escape this session.
        conn.set_session(readonly=True)
        start = time.perf_counter()
        with conn.cursor() as cur:
            cur.execute(sql, params)
            rows = cur.fetchall()
        elapsed = time.perf_counter() - start
    finally:
        conn.close()

    header = ["seed", "score"] + requested
    print(
        f"{len(rows)} row(s) in {elapsed:.3f}s",
        file=sys.stderr,
    )
    return header, rows


def write_csv(path, header, rows):
    with open(path, "w", newline="", encoding="utf-8") as fh:
        writer = csv.writer(fh)
        writer.writerow(header)
        writer.writerows(rows)


def main(argv=None):
    args = parse_args(argv)

    # Build a small parser-error shim so resolve_weights can exit(2) with the
    # metric list (argparse's error() exits 2 by convention).
    def parser_error(msg):
        print(f"score_seeds.py: error: {msg}", file=sys.stderr)
        sys.exit(2)

    weights = resolve_weights(args.weights, parser_error)

    sql, params, requested = build_sql(weights, args.seed_range)
    params = finalize_params(params, args.top)

    if args.explain:
        print(render_explain(sql, weights, args.seed_range, args.top))
        return 0

    header, rows = run_query(args, weights, requested, sql, params)

    print(format_table(header, rows))
    if args.csv_path:
        write_csv(args.csv_path, header, rows)
        print(f"Wrote {len(rows)} row(s) to {args.csv_path}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())

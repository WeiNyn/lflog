# LogQL Project Plan (Updated: Profile DSL)

## Overview
**Goal**: Build a high-performance SQL query engine for application logs driven by a universal *profile DSL* (regex + lightweight metadata) instead of attempting to reverse-engineer many logging ecosystems. Profiles are simple, declarative, language-neutral artifacts (YAML/JSON) that define how to parse, type, and prefilter log lines.

**Tech Stack**: Rust (core engine) + Python (profile tooling, CLI) + PyO3 (bindings)

Why the change?
- A universal profile DSL is deterministic and fast (named-capture regex + hints).
- Easier to validate, test, share and optimize (prefilter, fast-split, multiline hints).
- Avoids brittle heuristics and the explosion of framework-specific parsers.
- Enables powerful optimizations: predicate pushdown, column pruning, memoized indexes, and parallel chunked parsing.

---

## Profile DSL (MVP)
Profiles are small YAML documents describing a line-level pattern and per-field metadata. Profiles are:
- human-editable and machine-validated
- centered on a single regex with named groups
- have optional fast prefilters and multiline rules
- include per-field types and parse hints

Minimal profile keys (recommended for MVP):
- `name`: profile name
- `version`
- `pattern`: regex with named captures (anchored by default)
- `prefilter` (optional): cheap substring or regex to quickly skip lines
- `fields`: map of `name -> FieldSpec`
- `multiline` (optional): `start` / `continuation` patterns
- `notes` (optional)

FieldSpec (recommended keys):
- `type`: `string | int | float | datetime | enum | json`
- `formats`: list of datetime formats (strftime)
- `values`: for enum allowed values
- `normalize`: e.g., `lower`, `upper`
- `optional`: bool
- `index`: bool (hint to create index later)
- `transform`: shallow helpers (e.g., `trim`, `json_path`)

Example profile (YAML):
```/dev/null/profile_example.yaml#L1-28
name: python_default
version: 1
pattern: '^(?P<ts>\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2},\d+) - (?P<level>[A-Z]+) - (?P<logger>[^ ]+) - (?P<message>.*)$'
prefilter: " - "
fields:
  ts:
    type: datetime
    formats:
      - "%Y-%m-%d %H:%M:%S,%f"
    index: true
  level:
    type: enum
    values: [DEBUG, INFO, WARNING, ERROR, CRITICAL]
    normalize: upper
  logger: { type: string }
  message: { type: string }
multiline:
  start: '^\d{4}-\d{2}-\d{2}'
  continuation: '^\s+'
notes: "Profile for default Python formatter"
```

Profile validation rules (MVP):
- `pattern` must compile and its named groups should correspond to `fields` (missing fields are allowed but warned).
- `prefilter` must be either a literal substring or a simple regex.
- `datetime` fields should have at least one format or accept lenient parsing if omitted.
- `multiline` if present must provide a `start` pattern.

Storage:
- Profiles are stored under `profiles/` (YAML). A global registry `~/.config/logql/profiles` can be added later.

---

## Project Phases & Milestones

### Phase 1: Foundation & Proof of Concept (Week 1-2)
Milestone 1.1: Profile DSL & Tools (Python)
- [ ] Draft `docs/PROFILE_SPEC.md` (YAML schema + examples)
- [ ] Implement `logql.profile` Python module:
  - `load_profile(path)` -> validated profile object
  - `validate_profile(obj)` -> diagnostics
  - `test_profile(profile, sample_lines, n=100)` -> match rate + sample extraction
  - `from_logging_config(logging_conf)` helper (migration)
- [ ] Add sample profiles in `profiles/`
- [ ] Unit tests for validation and `test_profile()`

Milestone 1.2: Basic Log Parser (Rust)
- [ ] Create Rust project skeleton: `core` crate
- [ ] Implement `Profile` and `CompiledProfile` structs (serde for YAML)
- [ ] Implement prefilter (substring/regex) and `Regex`-based parser
- [ ] Parse single-line into typed `Row` (support `string`, `int`, `float`, `datetime`, `enum`)
- [ ] Unit tests for parse correctness on sample logs

Deliverable: End-to-end example: load profile, parse 10k lines, verify typed rows.

---

### Phase 2: Query Engine Core (Week 3-4)
Milestone 2.1: SQL Parsing & Planning
- [ ] Integrate `sqlparser-rs`
- [ ] Accept basic SQL subset: `SELECT` (projections), `FROM` (file), `WHERE` (`=, !=, <, >, IN, LIKE`), `LIMIT`
- [ ] Allow caller to pass a `profile` (or default/no-profile behavior)
- [ ] Unit tests for SQL parsing + simple sanity checks

Milestone 2.2: Query Execution Engine
- [ ] Implement scan -> filter -> project pipeline
- [ ] Column pruning (parse only fields referenced by WHERE/SELECT)
- [ ] Predicate compilation & pushdown (compile `WHERE` -> closure operating on minimal parse)
- [ ] Cheap prefilters for `LIKE` -> `contains` when possible (run before regex)
- [ ] Streaming iterator model returning typed rows
- [ ] Benchmarks: baseline target: Query 1M lines in <1s (MVP hardware dependent)

Deliverable: Simple CLI `logql query` that executes queries against single files using a profile.

---

### Phase 3: Advanced Features & Optimizations (Week 5-6)
Milestone 3.1: Aggregations & Grouping
- [ ] GROUP BY support
- [ ] Aggregates: `COUNT`, `SUM`, `AVG`, `MIN`, `MAX`
- [ ] `ORDER BY`, `LIMIT`, `OFFSET`, `HAVING`
- [ ] Tests and examples

Milestone 3.2: Performance Optimizations
- [ ] Column pruning + predicate pushdown improvements
- [ ] Memory-mapped I/O (`memmap2`)
- [ ] Parallel chunked processing (`rayon`) with chunk boundary alignment (newline)
- [ ] Chunked streaming and merging semantics (ordering & LIMIT correctness)
- [ ] Indexing hints (timestamp -> offset index)
- [ ] Faster substring search (Aho-Corasick or memchr) for prefilters

Performance target: Query 100M lines in <10s on typical server-class hardware (platform-dependent).

---

### Phase 4: Python Integration & UX (Week 7)
Milestone 4.1: PyO3 Bindings
- [ ] Expose core functions: `query(file, sql, profile=None, profile_path=None, as_dataframe=False)`
- [ ] Expose `load_profile`, `validate_profile`, `test_profile`
- [ ] Map errors/exceptions clearly to Python exceptions
- [ ] Return Pandas DataFrame when requested

Milestone 4.2: CLI Tool
- [ ] `logql` CLI (typer/click) with subcommands:
  - `query` (run SQL)
  - `profile test` (validate and test profiles on sample files)
  - `profile add/list/remove`
  - `interactive` (REPL)
- [ ] Output formats: table, JSON, CSV
- [ ] Profile registry support and auto-detection toggle

---

### Phase 5: Polish & Extensions (Week 8+)
Milestone 5.1: Production Readiness
- [ ] Comprehensive errors & messages
- [ ] Full documentation and README
- [ ] Examples and sample profiles
- [ ] Benchmarks and `cargo bench` harness
- [ ] CI/CD (GitHub Actions) and packaging (PyPI, Cargo)

Milestone 5.2: Optional Advanced Features
- [ ] Multi-file queries and glob support
- [ ] Rotating logs awareness (app.log, app.log.1, ...)
- [ ] JSON logs (field extraction with `json_path`)
- [ ] Custom UDFs (user-defined functions)
- [ ] Query result caching/store or materialization (Parquet/Arrow)
- [ ] Watch mode / live tail + continuous queries
- [ ] Auto-detection of profile from samples (profiling/fit command)

---

## Data Model & Implementation Notes (practical guidance)
- Represent `Profile` in Rust as a serde-deserializable struct and provide a `CompiledProfile` that caches compiled `Regex` and prefilters.
- Use zero-copy referencing (`&str` / byte slices) into the original buffer when possible to avoid allocations.
- `Row` representation: MVP can use `Vec<Option<Value>>` keyed by field index for low overhead; later optimize to columnar or specialized typed buffers.
- Predicate compilation: compile SQL `WHERE` AST to a small evaluation tree/closure that extracts necessary fields in the minimal order.
- Multiline handling: opt-in per-profile (explicit `multiline` spec). Read lines and group until the next `start` match. Avoid attempting to automatically coalesce stack traces by default.
- Fast path for delimiter-like logs: add `fast_split` hint in profile. If present, parse with `splitn` instead of regex for speed.
- Avoid expensive regex features in `pattern` for high-throughput use cases (document best practices in `PROFILE_SPEC.md`).

---

## Testing & Benchmarks
- Create synthetic log generator to produce datasets at different scales (10k, 1M, 100M).
- Unit tests:
  - Profile validation and `test_profile()`
  - Line parsing correctness (field types and edge cases)
  - SQL parser and execution correctness
- Integration tests: E2E queries with profiles and expected result sets.
- Benchmarks: measure lines/sec, memory, and end-to-end query latency. Keep benchmarks in `bench/`.

---

## Success Metrics
1. Performance: Comparable to or faster than equivalent `grep` pipelines on typical real-world queries; target 1M lines/s for trivial scans; 100M lines in <10s with optimizations.
2. Usability: Ability to register and run queries with a new profile in <5 minutes.
3. Accuracy: Correct parse & SQL semantics for supported subset (tests cover edge cases).
4. Adoption: Profiles and tooling make it easy to share and reuse parsing logic.

---

## Getting Started (Week 1 action items)
1. Create `docs/PROFILE_SPEC.md` with definitive schema, examples and migration notes.
2. Create `profiles/` and add 4–6 sample profiles (python_default, simple_delimiter, nginx_common, json_simple).
3. Implement Python profile loader & `profile.test()` CLI helper.
4. Initialize Rust `core` crate and implement `Profile`/`CompiledProfile` + basic line parser.
5. Write an E2E smoke test: use a sample profile to parse a sample log and run a simple SQL (`SELECT level, message FROM 'sample.log' WHERE level='ERROR' LIMIT 10`).
6. Add basic CI job that runs unit tests.

---

If you'd like, I can:
- Draft `docs/PROFILE_SPEC.md` (schema + examples), or
- Scaffold `profiles/` and a couple canonical sample profiles,
- Or scaffold the Python `logql.profile` helpers with tests so you can iterate on profiles quickly.

Tell me which of these you want me to draft first and I’ll prepare a concrete scaffold you can implement or adapt.
# lflog Benchmark

This directory contains scripts to benchmark `lflog` against standard Unix tools (`awk`, `gawk`, `mawk`).

## Setup

The benchmark uses a generated 100MB Apache log file (approx 1.2 million lines).

1. **Generate Data**:
   ```bash
   ./generate_data.sh
   ```
   This repeats the `Apache_2k.log` sample until it reaches ~100MB.

2. **Run Benchmark**:
   ```bash
   ./benchmark.sh
   ```

## Results Summary

Tested on a 100MB log file. `lflog` uses regex parsing with DataFusion's parallel execution engine. `awk`/`mawk` use string splitting (space delimiter).

| Tool | Task 1: Filter (Count Errors) | Task 2: Aggregate (Group By) | Notes |
|------|-------------------------------|------------------------------|-------|
| **lflog** | **~0.16s** | **~0.17s** | **Regex parsing** + SQL engine |
| mawk | ~0.20s | ~0.20s | String split, optimized bytecode |
| gawk | ~0.22s | ~0.20s | String split, standard GNU awk |
| awk | ~0.22s | ~0.20s | Standard awk |

### Analysis

- **Performance**: `lflog` is competitive with `mawk` (the fastest awk variant) and often faster on multi-core systems due to parallel processing.
- **Robustness**: `lflog` correctly handles complex log formats (quoted strings, brackets) using regex profiles, whereas `awk`'s simple whitespace splitting fails on complex formats (e.g., user agents with spaces).
- **Usability**: `lflog` allows standard SQL queries (`GROUP BY`, `ORDER BY`), which are much more verbose to implement in `awk`.

### Example Queries

**Task 1: Count Filter**
- **SQL**: `SELECT COUNT(*) FROM log WHERE level = 'error'`
- **AWK**: `awk '$6 == "[error]" {n++} END {print n}'`

**Task 2: Aggregation**
- **SQL**: `SELECT level, COUNT(*) FROM log GROUP BY level`
- **AWK**: `awk '{a[$6]++} END {for (k in a) print k, a[k]}'`

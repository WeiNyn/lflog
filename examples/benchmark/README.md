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

### Standard Benchmark (100MB)

| Tool | Task 1: Filter (Count Errors) | Task 2: Aggregate (Group By) |
|------|-------------------------------|------------------------------|
| **lflog** | **~0.16s** | **~0.17s** |
| mawk | ~0.20s | ~0.20s |
| gawk | ~0.22s | ~0.20s |
| awk | ~0.22s | ~0.20s |

### XXL Benchmark (10M lines, ~800MB)

| Tool | Task 1: Filter (Count Errors) | Task 2: Aggregate (Group By) |
|------|-------------------------------|------------------------------|
| **lflog** | **~2.30s** | **~3.00s** |
| mawk | ~2.00s | ~2.10s |
| gawk | ~1.80s | ~1.70s |
| awk | ~1.75s | ~1.60s |

### Parallel Scaling Benchmark (10M lines)

Tested with `LFLOGTHREADS` environment variable to control parallelism.

| Threads | Task 1: Filter | Task 2: Aggregate | Speedup vs 2 Threads |
|---------|----------------|-------------------|----------------------|
| 2       | ~2.42s         | ~3.33s            | 1.0x                 |
| **4**   | **~1.72s**     | **~2.24s**        | **1.4x - 1.5x**      |
| 8       | ~1.76s         | ~2.31s            | 1.4x                 |
| 16      | ~2.18s         | ~3.05s            | 1.1x                 |

*Note: Results suggest optimal performance at 4 threads on this test machine, likely matching physical core count. Overhead increases beyond that.*

### Analysis

- **Performance**: 
    - On smaller files (100MB), `lflog` overhead is negligible and parallelism wins (faster than awk).
    - On larger files (800MB), `lflog` is ~30-50% slower than `awk`. This is a trade-off: `lflog` parses with **full Regex** (handling quotes, brackets correctly) while `awk` just splits by space.
    - `lflog` achieves ~350 MB/s regex parsing throughput.

- **Robustness**: `lflog` correctly handles complex log formats (quoted strings, brackets) using regex profiles, whereas `awk`'s simple whitespace splitting fails on complex formats (e.g., user agents with spaces).
- **Usability**: `lflog` allows standard SQL queries (`GROUP BY`, `ORDER BY`), which are much more verbose to implement in `awk`.

### Example Queries

**Task 1: Count Filter**
- **SQL**: `SELECT COUNT(*) FROM log WHERE level = 'error'`
- **AWK**: `awk '$6 == "[error]" {n++} END {print n}'`

**Task 2: Aggregation**
- **SQL**: `SELECT level, COUNT(*) FROM log GROUP BY level`
- **AWK**: `awk '{a[$6]++} END {for (k in a) print k, a[k]}'`

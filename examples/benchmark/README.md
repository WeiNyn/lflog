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

**Best Configuration (4 Threads):**

| Tool | Task 1: Filter (Count Errors) | Task 2: Aggregate (Group By) |
|------|-------------------------------|------------------------------|
| **lflog** | **~1.50s** | **~1.51s** |
| mawk | ~1.55s | ~1.61s |
| gawk | ~1.67s | ~1.54s |
| awk | ~1.66s | ~1.54s |

### Analysis

- **Performance**: 
    - **lflog wins**: With optimal parallelism (4 threads), `lflog` outperforms even `mawk` (the fastest awk) on large datasets.
    - **Throughput**: Processing 800MB in 1.5s equates to **~533 MB/s** parsing throughput with full Regex.
    - **Efficiency**: Despite the heavy computational cost of Regex (vs split), `lflog`'s parallel architecture overcomes the overhead.

- **Robustness**: `lflog` correctly handles complex log formats (quoted strings, brackets) using regex profiles, whereas `awk`'s simple whitespace splitting fails on complex formats (e.g., user agents with spaces).
- **Usability**: `lflog` allows standard SQL queries (`GROUP BY`, `ORDER BY`), which are much more verbose to implement in `awk`.


### Example Queries

**Task 1: Count Filter**
- **SQL**: `SELECT COUNT(*) FROM log WHERE level = 'error'`
- **AWK**: `awk '$6 == "[error]" {n++} END {print n}'`

**Task 2: Aggregation**
- **SQL**: `SELECT level, COUNT(*) FROM log GROUP BY level`
- **AWK**: `awk '{a[$6]++} END {for (k in a) print k, a[k]}'`

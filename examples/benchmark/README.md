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
| **lflog** | **0.14s** | **0.16s** |
| mawk | 0.19s | 0.19s |
| gawk | 0.23s | 0.21s |
| awk | 0.22s | 0.20s |

### XXL Benchmark (10M lines, ~817MB)

Values are averages over 5 runs to ensure stability.

| Tool | Task 1: Filter (Count Errors) | Task 2: Aggregate (Group By) |
|------|-------------------------------|------------------------------|
| **lflog (8 threads)** | **1.07s** | **1.47s** |
| mawk | 1.71s | 1.61s |
| gawk | 1.76s | 1.59s |
| awk | 1.80s | 1.56s |

### Scaling Analysis (lflog)

Performance scales well with thread count, diminishing after 4-8 threads (on this 8-core machine).

| Threads | Filter Time | Group By Time |
|---------|-------------|---------------|
| 1 | 3.20s | 3.31s |
| 2 | 2.05s | 2.16s |
| 4 | 1.93s | **1.71s** |
| 8 | **1.88s** | 1.83s |

*Note: Filter task scales better because it's purely parallelizable. Group By requires a merge phase which adds overhead as parallelism increases.*

### Example Queries

**Task 1: Count Filter**
- **SQL**: `SELECT COUNT(*) FROM log WHERE level = 'error'`
- **AWK**: `awk '$6 == "[error]" {n++} END {print n}'`

**Task 2: Aggregation**
- **SQL**: `SELECT level, COUNT(*) FROM log GROUP BY level`
- **AWK**: `awk '{a[$6]++} END {for (k in a) print k, a[k]}'`

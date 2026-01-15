# Loghub Demos

This directory contains configuration profiles, SQL queries, and scripts to demonstrate `lflog` capabilities on the [Loghub](https://github.com/logpai/loghub) dataset collection.

## Structure

- **config/**: Contains `loghub.toml` with regex profiles for 16 different log types.
- **queries/**: Example SQL queries for each dataset.
- **scripts/**: Helper script to run the demos.

## Prerequisites

1.  Build the `lflog` binary:
    ```bash
    cargo build --release
    ```
2.  Ensure the `loghub` submodule is initialized (it should be if you are in this repo).

## Running Demos

Use the `run_demo.sh` script to run a demo for a specific dataset.

```bash
cd examples/loghub_demos/scripts
./run_demo.sh <dataset_name>
```

### Available Datasets

- `android`
- `apache`
- `bgl`
- `hadoop`
- `hdfs`
- `healthapp`
- `hpc`
- `linux`
- `mac`
- `openssh`
- `openstack`
- `proxifier`
- `spark`
- `thunderbird`
- `windows`
- `zookeeper`

### Example

```bash
./run_demo.sh apache
```

Output:
```
=== Running Demo: apache ===
Log File: .../loghub/Apache/Apache_2k.log
Profile:  apache
Query:    SELECT level, COUNT(*) as count FROM log GROUP BY level ORDER BY count DESC;
--------------------------------
+--------+-------+
| level  | count |
+--------+-------+
| notice | 1850  |
| error  | 150   |
+--------+-------+
```

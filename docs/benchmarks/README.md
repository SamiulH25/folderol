# Benchmarks

This directory records measured performance, not estimates. Every number is
reproducible with a command and a described machine.

## Method

1. Build in release mode: `cargo build --release`.
2. Close other workloads; do not benchmark on battery or a shared CI runner.
3. Run the command shown in the entry.
4. Record wall time, p50/p95 query latency, peak RSS, index size and file count.
5. Note the filesystem (NVMe, ext4/btrfs, encryption) and the CPU.
6. Repeat three times; publish the median run and keep the raw output as JSON
   when the command supports it.

Reference machine:

| | |
| --- | --- |
| CPU | 8-core x86_64, 3.2 GHz |
| RAM | 32 GB |
| Disk | NVMe, ≥ 2 GB/s sequential |
| OS | Arch Linux, current |
| Rust | stable, release profile |
| Filesystem | btrfs on LUKS (note the overhead) |

## Commands

```bash
# Synthetic index + query latency, in memory
cargo run --release -p folderol-cli -- bench --files 100000 --queries 20

# Real-tree scan (generate a corpus first; script planned)
scripts/gen-corpus.sh /tmp/folderol-corpus 100000
/usr/bin/time -v cargo run --release -p folderol-cli -- \
  --db /tmp/folderol-bench.sqlite3 scan /tmp/folderol-corpus

# Second scan (unchanged skip path)
/usr/bin/time -v cargo run --release -p folderol-cli -- \
  --db /tmp/folderol-bench.sqlite3 scan /tmp/folderol-corpus

du -h /tmp/folderol-bench.sqlite3
```

## Entry template

```markdown
## YYYY-MM-DD — vX.Y.Z / commit

- Machine: CPU, RAM, disk, filesystem
- Index: N files, DB size, embedder id
- Cold scan: X s (N files/s), peak RSS
- Second scan: X s, indexed 0
- Query: p50 X ms, p95 X ms, max X ms
- Notes: anomalies, thermal state, build flags
- Command: exact invocation
```

## Results

No released benchmark yet. The first entry lands with M1 (0.2.0).

## Rules

- A result is not comparable across machines. Compare only same-machine runs.
- Report the slow run, not the best one.
- If a release is more than 20% slower than the previous entry, the release
  notes must explain why.
- CI benchmarks use a generous threshold to avoid flakiness; they detect
  catastrophes, not micro-regressions. The release benchmark is the source of
  truth.

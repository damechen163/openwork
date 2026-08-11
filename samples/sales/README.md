# Sales Analysis Sample

This workspace demonstrates an OpenWork safe-execution task: an AI runtime
analyzes a sales dataset, and the analysis runs inside an isolated sandbox.

## Task

Read `sales.csv` (header: `date,region,product,units,revenue`, July 2026 data)
and implement a Python script `analyze.py` per the contract below.

## Task contract

1. **Interface**: `python3 analyze.py <output_dir>`; `argv[1]` is an existing,
   writable output directory; the working directory is the workspace root,
   so `sales.csv` is in the current directory.
2. **Input**: read `sales.csv` from the current directory.
3. **Outputs** (written into `argv[1]`):
   - `sales-analysis.csv` — per-region aggregate of `units` and `revenue`
     (columns: `region,units,revenue`), sorted by region.
   - `summary.md` — a short Markdown summary: total revenue, revenue share
     per region, and the top product by revenue.
4. **Constraints**:
   - Use only the Python standard library (no pandas/numpy; the sandbox image
     does not include them).
   - Make no network requests; the sandbox has no network access.
   - Produce deterministic output: no randomness, no timestamps, no absolute
     paths.
   - Do **not** execute any commands or run any scripts yourself. OpenWork
     will execute `analyze.py` inside an isolated sandbox after you finish.
   - Write only `analyze.py`; do not modify `sales.csv` or `README.md`.

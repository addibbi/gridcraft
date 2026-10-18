# Reproduce a missing January row with date text

> **Last updated:** 2026-10-18

This original, fictional workbook contains two rows, generated entirely in Rust. It does
not use an Excel-authored file, private data, or GETPIVOTDATA. The source has date text,
`DATEVALUE`, a `YEAR`/`MONTH` label, and an amount; a PivotTable sums the amount by month.

From the repository root, choose a new output path and generate the XLSX:

```sh
cargo run -p gridcraft-engine --example date_order_pivot -- /tmp/date-order-pivot.xlsx
```

| Date text | DMY date | Month | Amount |
|---|---|---|---|
| `13/01/2026` | 2026-01-13 | 2026-01 | 17 |
| `02/02/2026` | 2026-02-02 | 2026-02 | 23 |

The saved report at `F3:G6` contains **2026-01: 17, 2026-02: 23, Grand Total: 40**.
The date order is an application preference, not an XLSX property. Opening the file with
the default MDY order recalculates `DATEVALUE("13/01/2026")` to `#VALUE!`. After refreshing
the PivotTable, the January row disappears. This reproduces the previous MDY-only behavior:

```sh
cargo run -p gridcraft-cli -- run --in /tmp/date-order-pivot.xlsx \
  --cmd 'data.refreshAll' --print A1:G7
```

With this change, select DMY before the initial recalculation:

```sh
cargo run -p gridcraft-cli -- run --in /tmp/date-order-pivot.xlsx \
  --date-order dmy --cmd 'data.refreshAll' --print A1:G7
```

January is **17**, February is **23**, and the total is **40**. All source formulas and
date text are unchanged. For a native check:

```sh
cargo run -p gridcraft -- --date-order dmy /tmp/date-order-pivot.xlsx
```

Select the pivot and refresh it; the same three report values remain. To recover the
report in a running MDY session through the control channel, execute `formulas.dateOrder`
with `{"order":"dmy"}`, then `data.refreshAll`. The preference recalculates source formulas;
refreshing rebuilds the report. It does not rewrite the workbook's source formulas.

The generated regression tests cover this recovery, two XLSX save/open cycles, ambiguous
`02/01/2026` text (2 January in DMY), invalid dates, pre-open preference selection,
validation, undo/redo, dirty-state preservation, and editing an imported numeric date:

```sh
cargo test -p gridcraft-engine --test date_order
```

This is deliberately limited to explicit numeric date-text order for cell entry,
`DATEVALUE` and `TIMEVALUE`. Broader regional formula coercion remains open in #889.

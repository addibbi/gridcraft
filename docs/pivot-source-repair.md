# Repairing an invalid PivotTable source

> **Last updated:** 2026-10-21

An existing PivotTable can outlive its source's validity: an unused column header may be
deleted, or its defined source name may disappear. Refresh must reject that invalid source,
but **Change Data Source** must still accept a valid replacement. Previously, it validated
the old source first and refused the repair itself.

## Generate a minimal example

From the repository root, generate a workbook containing only fictional data:

```sh
cargo run -p gridcraft-engine --example pivot_source_repair -- /tmp/pivot-source-repair.xlsx
```

The generator creates a new file and refuses to overwrite an existing one. It constructs the
workbook through GridCraft commands; no Excel-authored or private workbook is needed. The
source is `Sheet1!$A$1:$E$3`, with `Month`, `Status`, `Amount` and `Category` in A:D and a
blank unused header in E. The existing `SourceRepair` pivot at G3 retains its rendered report.

Refreshing the invalid source reports `The PivotTable field name is not valid.` The
repair command replaces it with `Sheet1!$A$1:$D$3`. The report should contain:

| Month | Sum of Amount |
|---|---:|
| 2026-01 | 17 |
| 2026-02 | 23 |
| Grand Total | 40 |

Reproduce the failed refresh headlessly:

```sh
cargo run -p gridcraft-cli -- run --in /tmp/pivot-source-repair.xlsx \
  --cmd 'data.refreshAll={}'
```

Expect zero refreshed pivots and one invalid-field-name error in the command result.
Then run the explicit repair:

```sh
cargo run -p gridcraft-cli -- run --in /tmp/pivot-source-repair.xlsx \
  --cmd 'pivot.changeSource={"pivot":"SourceRepair","source":"Sheet1!$A$1:$D$3"}' \
  --cmd 'data.refreshAll={}' --out /tmp/pivot-source-repaired.xlsx --print G3:H6
```

Expect `dropped: []`, one refreshed pivot and no refresh errors. Before the fix,
`pivot.changeSource` fails with the same invalid-field-name error despite the valid replacement.
The original invalid source is never silently trimmed or given an invented header.
Open `/tmp/pivot-source-repaired.xlsx` in the native app to inspect the result. This
reproduction uses the existing engine command through the CLI and requires no native dialog.

## Regression coverage

```sh
cargo test -p gridcraft-engine --test pivot_source_repair
```

Five tests generate their workbooks in Rust and cover four imported reports with blank
unused headers, a deleted defined source name, invalid replacement rejection without changing
the workbook or history, protection and occupied report cells, undo/redo, and repeated XLSX
save/open/refresh. This repair is independent of date parsing and GETPIVOTDATA support.

The branch integrates the official `v0.9.0` tag without changing the repair contract.
It also uses a zero-sized non-Linux desktop-theme handle instead of `()`, allowing
strict Clippy to check the shared watcher on macOS. The Linux portal connection and
theme behavior are unchanged.

The command helpers document the source-validation order and atomic edit contract.
Rustdoc on the generator and regression helpers explains how the fictional workbook is
constructed, which failures must preserve history, and what each regression verifies.

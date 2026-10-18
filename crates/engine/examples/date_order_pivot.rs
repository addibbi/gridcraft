//! Generate the original, fictional XLSX used by docs/date-order-repro.md.
//! `cargo run -p gridcraft-engine --example date_order_pivot -- /tmp/date-order-pivot.xlsx`

use std::error::Error;
use std::io;

use gridcraft_engine::Session;
use serde_json::json;

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args_os()
        .nth(1)
        .and_then(|path| path.into_string().ok())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "supply a new UTF-8 output .xlsx path"))?;
    if std::path::Path::new(&path).exists() {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, "the output already exists; choose a new path").into());
    }
    let mut s = Session::new();
    s.execute("formulas.dateOrder", json!({"order": "dmy"}))?;
    s.new_workbook();
    s.execute(
        "range.setValues",
        json!({"range": "A1", "values": [
            ["Date text", "Date", "Month", "Amount"],
            ["'13/01/2026", "=DATEVALUE(A2)", "=YEAR(B2)&\"-\"&RIGHT(\"00\"&MONTH(B2),2)", 17],
            ["'02/02/2026", "=DATEVALUE(A3)", "=YEAR(B3)&\"-\"&RIGHT(\"00\"&MONTH(B3),2)", 23]
        ]}),
    )?;
    s.execute("home.numberFormat", json!({"range": "B2:B3", "code": "yyyy-mm-dd"}))?;
    s.execute(
        "insert.pivotTable",
        json!({"source": "Sheet1!$A$1:$D$3", "destination": "Sheet1!F3", "name": "Months", "rows": ["Month"], "columns": [], "values": ["Amount"]}),
    )?;
    s.execute("file.save", json!({"path": path}))?;
    println!("Created {path}: January 17, February 23, Grand Total 40 (DMY).");
    Ok(())
}

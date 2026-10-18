//! Date-text order is an explicit application preference, independent of formula language.

use gridcraft_engine::Session;
use gridcraft_engine::core::{CellError, CellRef, Value};
use serde_json::json;

fn value(s: &Session, cell: &str) -> Value {
    s.doc().unwrap().wb.active().unwrap().value(CellRef::parse(cell).unwrap())
}

#[test]
fn date_order_can_be_selected_before_a_workbook_is_created() {
    for (order, expected) in [("mdy", 46054.0), ("dmy", 46024.0)] {
        let mut s = Session::new();
        assert_eq!(s.execute("formulas.dateOrder", json!({})).unwrap()["order"], "mdy");
        assert_eq!(s.execute("formulas.dateOrder", json!({"order": order})).unwrap()["order"], order);
        s.new_workbook();
        s.execute("cell.set", json!({"cell": "A1", "input": "02/01/2026"})).unwrap();
        s.execute("cell.set", json!({"cell": "A2", "input": "'02/01/2026"})).unwrap();
        s.execute("cell.set", json!({"cell": "B2", "input": "=DATEVALUE(A2)"})).unwrap();
        s.execute("cell.set", json!({"cell": "A3", "input": "2026-01-02"})).unwrap();
        assert_eq!(value(&s, "A1"), Value::Number(expected), "typed date in {order}");
        assert_eq!(value(&s, "A2"), Value::text("02/01/2026"), "apostrophe keeps source text");
        assert_eq!(value(&s, "B2"), Value::Number(expected), "DATEVALUE in {order}");
        assert_eq!(value(&s, "A3"), Value::Number(46024.0), "ISO date in {order}");
    }
}

#[test]
fn invalid_date_order_preserves_the_preference_and_calculated_values() {
    let mut s = Session::new();
    s.execute("formulas.dateOrder", json!({"order": "dmy"})).unwrap();
    s.new_workbook();
    s.execute("cell.set", json!({"cell": "A1", "input": "=DATEVALUE(\"02/01/2026\")"})).unwrap();
    for invalid in [json!("ymd"), json!(""), json!(42), json!(true), json!(null)] {
        assert!(s.execute("formulas.dateOrder", json!({"order": invalid})).is_err(), "invalid order {invalid}");
        assert_eq!(s.execute("formulas.dateOrder", json!({})).unwrap()["order"], "dmy");
        assert_eq!(value(&s, "A1"), Value::Number(46024.0));
    }
}

#[test]
fn date_order_preserves_dirty_state_and_applies_to_undo_and_redo() {
    let mut source = Session::new();
    source.new_workbook();
    source.execute("cell.set", json!({"cell": "A1", "input": "=DATEVALUE(\"02/01/2026\")"})).unwrap();
    let saved = source.execute("file.saveBytes", json!({"format": "xlsx"})).unwrap();
    let mut s = Session::new();
    s.execute("file.open", json!({"name": "date-order.xlsx", "base64": saved["base64"]})).unwrap();
    assert!(!s.doc().unwrap().is_dirty());
    assert_eq!(value(&s, "A1"), Value::Number(46054.0));
    s.execute("formulas.dateOrder", json!({"order": "dmy"})).unwrap();
    assert!(!s.doc().unwrap().is_dirty(), "changing an application preference does not dirty the workbook");
    assert_eq!(value(&s, "A1"), Value::Number(46024.0));
    s.execute("cell.set", json!({"cell": "B1", "input": "5"})).unwrap();
    assert!(s.doc().unwrap().is_dirty());
    s.execute("formulas.dateOrder", json!({"order": "mdy"})).unwrap();
    assert!(s.doc().unwrap().is_dirty(), "changing the preference does not mark an edit as saved");
    s.execute("formulas.dateOrder", json!({"order": "dmy"})).unwrap();
    s.execute("edit.undo", json!({})).unwrap();
    assert_eq!(value(&s, "B1"), Value::Empty);
    assert_eq!(value(&s, "A1"), Value::Number(46024.0), "undo uses the current application preference");
    s.execute("edit.redo", json!({})).unwrap();
    assert_eq!(value(&s, "B1"), Value::Number(5.0));
    assert_eq!(value(&s, "A1"), Value::Number(46024.0), "redo uses the current application preference");
}

#[test]
fn editing_an_imported_mdy_formatted_date_in_dmy_keeps_its_numeric_value() {
    let mut source = Session::new();
    source.new_workbook();
    source.execute("cell.set", json!({"cell": "A1", "input": 46024})).unwrap();
    source.execute("home.numberFormat", json!({"range": "A1", "code": "m/d/yyyy"})).unwrap();
    let saved = source.execute("file.saveBytes", json!({"format": "xlsx"})).unwrap();
    let mut s = Session::new();
    s.execute("formulas.dateOrder", json!({"order": "dmy"})).unwrap();
    s.execute("file.open", json!({"name": "imported-date.xlsx", "base64": saved["base64"]})).unwrap();
    let editor_text = {
        let d = s.doc().unwrap();
        let sh = d.wb.active().unwrap();
        let at = CellRef::parse("A1").unwrap();
        assert_eq!(gridcraft_engine::display::cell_text(&d.wb, sh, at), "1/2/2026", "the file's existing display format is kept");
        gridcraft_engine::display::edit_text(&d.wb, sh, at)
    };
    // F2 followed by Enter must not reinterpret this January date as 1 February.
    assert_eq!(editor_text, "2026-01-02", "an unambiguous editor value is needed with the DMY preference");
    s.execute("cell.set", json!({"cell": "A1", "input": editor_text})).unwrap();
    assert_eq!(value(&s, "A1"), Value::Number(46024.0));
}

#[test]
fn date_validation_uses_the_same_date_order_as_typed_input() {
    let mut s = Session::new();
    s.execute("formulas.dateOrder", json!({"order": "dmy"})).unwrap();
    s.new_workbook();
    s.execute(
        "data.validation",
        json!({"range": "A1", "type": "date", "operator": "between", "formula1": "DATE(2026,1,1)", "formula2": "DATE(2026,1,31)"}),
    )
    .unwrap();
    for (input, expected) in [("13/01/2026", true), ("02/01/2026", true), ("01/02/2026", false), ("31/02/2026", false)] {
        let checked = s.execute("data.validate", json!({"cell": "A1", "input": input})).unwrap();
        assert_eq!(checked["ok"], expected, "January-only validation in DMY: {input}");
    }
}

/// Generated source and report: no Excel-authored workbook or binary fixture is needed.
fn datevalue_month_pivot(january_text: &str) -> Session {
    let mut s = Session::new();
    s.execute("formulas.dateOrder", json!({"order": "dmy"})).unwrap();
    s.new_workbook();
    s.execute(
        "range.setValues",
        json!({"range": "A1", "values": [
            ["Date text", "Date", "Month", "Amount"],
            [format!("'{january_text}"), "=DATEVALUE(A2)", "=YEAR(B2)&\"-\"&RIGHT(\"00\"&MONTH(B2),2)", 17],
            ["'02/02/2026", "=DATEVALUE(A3)", "=YEAR(B3)&\"-\"&RIGHT(\"00\"&MONTH(B3),2)", 23]
        ]}),
    )
    .unwrap();
    s.execute(
        "insert.pivotTable",
        json!({"source": "Sheet1!$A$1:$D$3", "destination": "Sheet1!F3", "name": "Months", "rows": ["Month"], "columns": [], "values": ["Amount"]}),
    )
    .unwrap();
    s
}

fn assert_report_value(s: &Session, label: &str, expected: f64) {
    let sh = s.doc().unwrap().wb.active().unwrap();
    let range = sh.pivots[0].last_range.unwrap();
    let found: Vec<_> = range.iter().filter(|c| sh.value(*c) == Value::text(label)).collect();
    assert_eq!(found.len(), 1, "one report row for {label}");
    let at = found[0];
    assert_eq!(sh.value(CellRef::new(at.row, at.col + 1)), Value::Number(expected), "report value for {label}");
}

fn assert_datevalue_month_report(s: &Session, january_text: &str, january_serial: f64) {
    for (cell, expected) in [
        ("A2", Value::text(january_text)),
        ("B2", Value::Number(january_serial)),
        ("C2", Value::text("2026-01")),
        ("A3", Value::text("02/02/2026")),
        ("B3", Value::Number(46055.0)),
        ("C3", Value::text("2026-02")),
        ("D2", Value::Number(17.0)),
        ("D3", Value::Number(23.0)),
    ] {
        assert_eq!(value(s, cell), expected, "{january_text}: {cell}");
    }
    assert_report_value(s, "2026-01", 17.0);
    assert_report_value(s, "2026-02", 23.0);
    assert_report_value(s, "Grand Total", 40.0);
    let sh = s.doc().unwrap().wb.active().unwrap();
    for (cell, formula) in [
        ("B2", "DATEVALUE(A2)"),
        ("C2", "YEAR(B2)&\"-\"&RIGHT(\"00\"&MONTH(B2),2)"),
        ("B3", "DATEVALUE(A3)"),
        ("C3", "YEAR(B3)&\"-\"&RIGHT(\"00\"&MONTH(B3),2)"),
    ] {
        assert_eq!(sh.cell(CellRef::parse(cell).unwrap()).unwrap().formula.as_ref().unwrap().text, formula, "{january_text}: {cell}");
    }
}

#[test]
fn dmy_datevalue_keeps_january_after_xlsx_open_and_pivot_refresh() {
    // 13/01 is impossible as MDY; 02/01 is valid in both orders and must stay January in DMY.
    for (january_text, january_serial) in [("13/01/2026", 46035.0), ("02/01/2026", 46024.0)] {
        let mut s = datevalue_month_pivot(january_text);
        assert_datevalue_month_report(&s, january_text, january_serial);
        for _ in 0..2 {
            let saved = s.execute("file.saveBytes", json!({"format": "xlsx"})).unwrap();
            let mut reopened = Session::new();
            // Date order is an application preference, not a property imported from the XLSX.
            reopened.execute("formulas.dateOrder", json!({"order": "dmy"})).unwrap();
            reopened.execute("file.open", json!({"name": "months.xlsx", "base64": saved["base64"]})).unwrap();
            assert_datevalue_month_report(&reopened, january_text, january_serial);
            let refreshed = reopened.execute("data.refreshAll", json!({})).unwrap();
            assert_eq!(refreshed, json!({"refreshed": 1, "errors": []}));
            assert_datevalue_month_report(&reopened, january_text, january_serial);
            s = reopened;
        }
    }
}

#[test]
fn changing_date_order_recovers_january_without_rewriting_formulas() {
    let mut source = datevalue_month_pivot("13/01/2026");
    let saved = source.execute("file.saveBytes", json!({"format": "xlsx"})).unwrap();
    let mut s = Session::new();
    s.execute("file.open", json!({"name": "months.xlsx", "base64": saved["base64"]})).unwrap();
    assert_eq!(value(&s, "B2"), Value::Error(CellError::Value), "the default MDY order rejects 13/01");
    s.execute("data.refreshAll", json!({})).unwrap();
    let sh = s.doc().unwrap().wb.active().unwrap();
    assert!(sh.pivots[0].last_range.unwrap().iter().all(|c| sh.value(c) != Value::text("2026-01")), "January is absent with the invalid source date");
    // Changing the preference recalculates the source; refreshing then rebuilds the report.
    s.execute("formulas.dateOrder", json!({"order": "dmy"})).unwrap();
    assert_eq!(value(&s, "B2"), Value::Number(46035.0));
    let refreshed = s.execute("data.refreshAll", json!({})).unwrap();
    assert_eq!(refreshed, json!({"refreshed": 1, "errors": []}));
    assert_datevalue_month_report(&s, "13/01/2026", 46035.0);
}

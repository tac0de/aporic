use settlement_fixture::{merchant_totals, MerchantTotal, Row, Status, SummaryError};

fn row(merchant: &str, invoice: &str, revision: u32, status: Status, cents: i64) -> Row {
    Row {
        merchant_id: merchant.into(),
        invoice_id: invoice.into(),
        revision,
        status,
        cents,
    }
}

#[test]
fn highest_revision_wins_even_when_input_order_is_reversed() {
    let rows = [
        row("m", "i", 4, Status::Paid, 900),
        row("m", "i", 2, Status::Paid, 100),
    ];
    assert_eq!(merchant_totals(&rows), Ok(vec![MerchantTotal { merchant_id: "m".into(), cents: 900 }]));
}

#[test]
fn winning_void_replaces_prior_paid_revision() {
    let rows = [
        row("m", "i", 1, Status::Paid, 500),
        row("m", "i", 2, Status::Void, 500),
        row("m", "other", 1, Status::Paid, 700),
    ];
    assert_eq!(merchant_totals(&rows), Ok(vec![MerchantTotal { merchant_id: "m".into(), cents: 700 }]));
}

#[test]
fn duplicate_highest_revision_is_invalid_even_if_identical() {
    let rows = [
        row("m", "i", 3, Status::Paid, 100),
        row("m", "i", 3, Status::Paid, 100),
    ];
    assert_eq!(merchant_totals(&rows), Err(SummaryError::DuplicateRevision));
}

#[test]
fn lower_duplicate_revision_does_not_invalidate_unique_winner() {
    let rows = [
        row("m", "i", 1, Status::Paid, 100),
        row("m", "i", 1, Status::Paid, 100),
        row("m", "i", 2, Status::Paid, 250),
    ];
    assert_eq!(merchant_totals(&rows), Ok(vec![MerchantTotal { merchant_id: "m".into(), cents: 250 }]));
}

#[test]
fn totals_are_sorted_and_zero_totals_omitted() {
    let rows = [
        row("z", "one", 1, Status::Paid, 20),
        row("a", "one", 1, Status::Paid, -20),
        row("z", "two", 1, Status::Paid, 30),
        row("a", "two", 1, Status::Paid, 20),
    ];
    assert_eq!(merchant_totals(&rows), Ok(vec![MerchantTotal { merchant_id: "z".into(), cents: 50 }]));
}

#[test]
fn checked_addition_reports_overflow() {
    let rows = [
        row("m", "one", 1, Status::Paid, i64::MAX),
        row("m", "two", 1, Status::Paid, 1),
    ];
    assert_eq!(merchant_totals(&rows), Err(SummaryError::Overflow));
}

#[test]
fn invoice_keys_are_scoped_to_merchant() {
    let rows = [
        row("b", "same", 1, Status::Paid, 3),
        row("a", "same", 1, Status::Paid, 2),
    ];
    assert_eq!(merchant_totals(&rows), Ok(vec![
        MerchantTotal { merchant_id: "a".into(), cents: 2 },
        MerchantTotal { merchant_id: "b".into(), cents: 3 },
    ]));
}

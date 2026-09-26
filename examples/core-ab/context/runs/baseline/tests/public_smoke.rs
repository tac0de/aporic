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
fn empty_input_is_empty_output() {
    assert!(merchant_totals(&[]).unwrap().is_empty());
}

#[test]
fn highest_revision_wins_even_when_rows_are_out_of_order() {
    let rows = [
        row("b", "one", 3, Status::Paid, 7),
        row("a", "one", 2, Status::Paid, 4),
        row("b", "one", 1, Status::Paid, 100),
        row("a", "one", 1, Status::Paid, 9),
    ];
    assert_eq!(
        merchant_totals(&rows).unwrap(),
        vec![
            MerchantTotal {
                merchant_id: "a".into(),
                cents: 4
            },
            MerchantTotal {
                merchant_id: "b".into(),
                cents: 7
            },
        ]
    );
}

#[test]
fn void_winner_replaces_paid_and_zero_totals_are_omitted() {
    let rows = [
        row("a", "voided", 1, Status::Paid, 8),
        row("a", "voided", 2, Status::Void, 8),
        row("a", "left", 1, Status::Paid, 5),
        row("a", "right", 1, Status::Paid, -5),
    ];
    assert!(merchant_totals(&rows).unwrap().is_empty());
}

#[test]
fn duplicate_highest_revision_is_invalid_even_if_identical() {
    let winner = row("a", "one", 2, Status::Void, 0);
    assert_eq!(
        merchant_totals(&[winner.clone(), winner]),
        Err(SummaryError::DuplicateRevision)
    );
}

#[test]
fn duplicate_lower_revision_is_superseded() {
    let rows = [
        row("a", "one", 1, Status::Paid, 9),
        row("a", "one", 1, Status::Paid, 9),
        row("a", "one", 2, Status::Paid, 3),
    ];
    assert_eq!(merchant_totals(&rows).unwrap()[0].cents, 3);
}

#[test]
fn checked_sum_rejects_overflow() {
    let rows = [
        row("a", "one", 1, Status::Paid, i64::MAX),
        row("a", "two", 1, Status::Paid, 1),
    ];
    assert_eq!(merchant_totals(&rows), Err(SummaryError::Overflow));
}

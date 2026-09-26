use settlement_fixture::merchant_totals;

#[test]
fn empty_input_is_empty_output() {
    assert!(merchant_totals(&[]).unwrap().is_empty());
}

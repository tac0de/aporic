#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Paid,
    Void,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub merchant_id: String,
    pub invoice_id: String,
    pub revision: u32,
    pub status: Status,
    pub cents: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MerchantTotal {
    pub merchant_id: String,
    pub cents: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SummaryError {
    DuplicateRevision,
    Overflow,
}

pub fn merchant_totals(rows: &[Row]) -> Result<Vec<MerchantTotal>, SummaryError> {
    use std::collections::BTreeMap;

    // Keep the winner and whether its revision occurs more than once. A later
    // revision can replace an earlier duplicate, so validate after selection.
    let mut winners: BTreeMap<(&str, &str), (&Row, bool)> = BTreeMap::new();
    for row in rows {
        let key = (row.merchant_id.as_str(), row.invoice_id.as_str());
        match winners.get_mut(&key) {
            Some((winner, duplicate)) if row.revision > winner.revision => {
                *winner = row;
                *duplicate = false;
            }
            Some((winner, duplicate)) if row.revision == winner.revision => {
                *duplicate = true;
            }
            Some(_) => {}
            None => {
                winners.insert(key, (row, false));
            }
        }
    }

    if winners.values().any(|(_, duplicate)| *duplicate) {
        return Err(SummaryError::DuplicateRevision);
    }

    let mut totals: BTreeMap<&str, i64> = BTreeMap::new();
    for ((merchant_id, _), (row, _)) in winners {
        if row.status == Status::Paid {
            let total = totals.entry(merchant_id).or_default();
            *total = total.checked_add(row.cents).ok_or(SummaryError::Overflow)?;
        }
    }

    Ok(totals
        .into_iter()
        .filter(|(_, cents)| *cents != 0)
        .map(|(merchant_id, cents)| MerchantTotal {
            merchant_id: merchant_id.to_owned(),
            cents,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn selects_latest_revision_and_omits_void_and_zero_totals() {
        let rows = vec![
            row("b", "one", 2, Status::Paid, 30),
            row("a", "one", 1, Status::Paid, 50),
            row("b", "one", 1, Status::Paid, 10),
            row("a", "one", 2, Status::Void, 50),
            row("b", "two", 1, Status::Paid, -10),
            row("b", "three", 1, Status::Paid, -20),
            row("c", "one", 1, Status::Paid, 7),
        ];
        assert_eq!(
            merchant_totals(&rows),
            Ok(vec![MerchantTotal {
                merchant_id: "c".into(),
                cents: 7
            }])
        );
        assert_eq!(merchant_totals(&[]), Ok(vec![]));
    }

    #[test]
    fn rejects_only_duplicate_winning_revision() {
        let duplicate_old = vec![
            row("a", "one", 1, Status::Paid, 1),
            row("a", "one", 1, Status::Paid, 1),
            row("a", "one", 2, Status::Paid, 2),
        ];
        assert_eq!(
            merchant_totals(&duplicate_old),
            Ok(vec![MerchantTotal {
                merchant_id: "a".into(),
                cents: 2
            }])
        );
        let duplicate_winner = vec![
            row("a", "one", 2, Status::Void, 0),
            row("a", "one", 2, Status::Void, 0),
        ];
        assert_eq!(
            merchant_totals(&duplicate_winner),
            Err(SummaryError::DuplicateRevision)
        );
    }

    #[test]
    fn detects_checked_sum_overflow() {
        let rows = vec![
            row("a", "one", 1, Status::Paid, i64::MAX),
            row("a", "two", 1, Status::Paid, 1),
        ];
        assert_eq!(merchant_totals(&rows), Err(SummaryError::Overflow));
    }

    #[test]
    fn sorts_merchant_totals() {
        let rows = vec![
            row("z", "one", 1, Status::Paid, 3),
            row("a", "one", 1, Status::Paid, 2),
        ];
        assert_eq!(
            merchant_totals(&rows),
            Ok(vec![
                MerchantTotal {
                    merchant_id: "a".into(),
                    cents: 2
                },
                MerchantTotal {
                    merchant_id: "z".into(),
                    cents: 3
                },
            ])
        );
    }
}

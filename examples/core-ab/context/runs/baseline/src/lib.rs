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

    // Track duplicates at the winning revision so duplicates at a lower revision
    // do not invalidate a later, unique winner.
    let mut winners: BTreeMap<(&str, &str), (&Row, bool)> = BTreeMap::new();
    for row in rows {
        let key = (row.merchant_id.as_str(), row.invoice_id.as_str());
        match winners.get_mut(&key) {
            None => {
                winners.insert(key, (row, false));
            }
            Some((winner, duplicate)) if row.revision > winner.revision => {
                *winner = row;
                *duplicate = false;
            }
            Some((winner, duplicate)) if row.revision == winner.revision => {
                *duplicate = true;
            }
            Some(_) => {}
        }
    }

    let mut totals: BTreeMap<&str, i64> = BTreeMap::new();
    for ((merchant_id, _), (winner, duplicate)) in winners {
        if duplicate {
            return Err(SummaryError::DuplicateRevision);
        }
        if winner.status == Status::Paid {
            let total = totals.entry(merchant_id).or_default();
            *total = total
                .checked_add(winner.cents)
                .ok_or(SummaryError::Overflow)?;
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

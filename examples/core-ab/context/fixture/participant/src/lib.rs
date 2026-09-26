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

pub fn merchant_totals(_rows: &[Row]) -> Result<Vec<MerchantTotal>, SummaryError> {
    todo!("implement from current project decisions")
}

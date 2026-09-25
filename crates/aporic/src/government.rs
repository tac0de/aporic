use crate::domain::{GovernmentDefinition, OfficeDefinition};

pub const GOVERNMENT_ID: &str = "aporic.government";
pub const PRODUCT_EXPERIMENT_OFFICE_ID: &str = "product.experiment";

pub fn definition() -> GovernmentDefinition {
    GovernmentDefinition {
        government_id: GOVERNMENT_ID.to_owned(),
        version: 1,
        title: "Aporic 정부".to_owned(),
        authority_source: "현재 인간 지시".to_owned(),
        executive_role_id: "executive.prime_minister".to_owned(),
        offices: vec![OfficeDefinition {
            office_id: PRODUCT_EXPERIMENT_OFFICE_ID.to_owned(),
            version: 1,
            title: "제품실험부".to_owned(),
            head_title: "제품실험부 장관".to_owned(),
            responsibility: "제품 의제를 검증 가능한 문제와 가설로 바꾸고, 의제별 다학제 제품반이 최소 프로토타입으로 직접 증거를 생산하도록 책임진다.".to_owned(),
            output_contract: "문제, 가설, 성공 척도, 프로토타입 증거, 계속·수정·중단 권고를 구분한 보고".to_owned(),
            head_role_id: "portfolio.steward".to_owned(),
            cell_based: true,
            advisory: true,
            grants_authority: false,
        }],
        advisory: true,
        grants_authority: false,
    }
}

pub fn office(office_id: &str, version: u32) -> Option<OfficeDefinition> {
    definition()
        .offices
        .into_iter()
        .find(|office| office.office_id == office_id && office.version == version)
}

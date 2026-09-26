use crate::domain::{GovernmentDefinition, GovernmentPositionDefinition, OfficeDefinition};

pub const GOVERNMENT_ID: &str = "aporic.government";
pub const PRODUCT_EXPERIMENT_OFFICE_ID: &str = "product.experiment";
pub const MEMORY_INFORMATION_OFFICE_ID: &str = "memory.information";
pub const EXECUTION_OPERATIONS_OFFICE_ID: &str = "execution.operations";

pub const INITIAL_CABINET: [(&str, &str); 9] = [
    ("김민준", "executive.prime_minister"),
    ("이서연", "product.experiment.minister"),
    ("박지훈", "memory.information.minister"),
    ("최수진", "execution.operations.minister"),
    ("정민지", "product.design.lead"),
    ("강도윤", "product.frontend.lead"),
    ("조현우", "product.backend.lead"),
    ("윤지우", "product.game.lead"),
    ("한예진", "oversight.inspector"),
];

fn position(
    id: &str,
    title: &str,
    office: Option<&str>,
    reports_to: Option<&str>,
    independent: bool,
) -> GovernmentPositionDefinition {
    GovernmentPositionDefinition {
        position_id: id.to_owned(),
        version: 1,
        title: title.to_owned(),
        office_id: office.map(str::to_owned),
        reports_to: reports_to.map(str::to_owned),
        independent,
    }
}

pub fn positions() -> Vec<GovernmentPositionDefinition> {
    vec![
        position("executive.prime_minister", "국무총리", None, None, false),
        position(
            "product.experiment.minister",
            "제품실험부 장관",
            Some(PRODUCT_EXPERIMENT_OFFICE_ID),
            Some("executive.prime_minister"),
            false,
        ),
        position(
            "memory.information.minister",
            "기억정보부 장관",
            Some(MEMORY_INFORMATION_OFFICE_ID),
            Some("executive.prime_minister"),
            false,
        ),
        position(
            "execution.operations.minister",
            "실행운영부 장관",
            Some(EXECUTION_OPERATIONS_OFFICE_ID),
            Some("executive.prime_minister"),
            false,
        ),
        position(
            "product.design.lead",
            "디자인 책임자",
            Some(PRODUCT_EXPERIMENT_OFFICE_ID),
            Some("product.experiment.minister"),
            false,
        ),
        position(
            "product.frontend.lead",
            "프론트엔드 책임자",
            Some(PRODUCT_EXPERIMENT_OFFICE_ID),
            Some("product.experiment.minister"),
            false,
        ),
        position(
            "product.backend.lead",
            "백엔드 책임자",
            Some(PRODUCT_EXPERIMENT_OFFICE_ID),
            Some("product.experiment.minister"),
            false,
        ),
        position(
            "product.game.lead",
            "게임 개발 책임자",
            Some(PRODUCT_EXPERIMENT_OFFICE_ID),
            Some("product.experiment.minister"),
            false,
        ),
        position("oversight.inspector", "독립 감사관", None, None, true),
    ]
}

pub fn known_position(position_id: &str) -> bool {
    positions()
        .iter()
        .any(|position| position.position_id == position_id)
}

pub fn position_definition(position_id: &str) -> Option<GovernmentPositionDefinition> {
    positions()
        .into_iter()
        .find(|position| position.position_id == position_id)
}

pub fn definition() -> GovernmentDefinition {
    GovernmentDefinition {
        government_id: GOVERNMENT_ID.to_owned(),
        version: 2,
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
        }, OfficeDefinition {
            office_id: MEMORY_INFORMATION_OFFICE_ID.to_owned(),
            version: 1,
            title: "기억정보부".to_owned(),
            head_title: "기억정보부 장관".to_owned(),
            responsibility: "프로젝트 기억과 외부 자료의 출처·시점·수명주기를 보존하고 필요한 맥락을 제공한다.".to_owned(),
            output_contract: "출처와 불확실성을 표시한 기억·자료 조회 및 인수인계".to_owned(),
            head_role_id: "portfolio.steward".to_owned(),
            cell_based: false,
            advisory: true,
            grants_authority: false,
        }, OfficeDefinition {
            office_id: EXECUTION_OPERATIONS_OFFICE_ID.to_owned(),
            version: 1,
            title: "실행운영부".to_owned(),
            head_title: "실행운영부 장관".to_owned(),
            responsibility: "과업 조정, 호스트 연동, 실행 기록 및 운영 복구를 관리한다.".to_owned(),
            output_contract: "과업 상태, 연동 관측, 검증 영수증, 복구 상태를 구분한 보고".to_owned(),
            head_role_id: "portfolio.steward".to_owned(),
            cell_based: false,
            advisory: true,
            grants_authority: false,
        }],
        positions: positions(),
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

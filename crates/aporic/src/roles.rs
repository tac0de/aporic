use crate::domain::RoleDefinition;

pub fn definitions() -> Vec<RoleDefinition> {
    [
        (
            "executive.prime_minister",
            "국무총리",
            "현재 인간 지시를 해석하고 과업을 조정해 최종 결과를 보고한다.",
            "근거, 미완료 사항, 다음 행동을 구분한 종합 보고",
            false,
        ),
        (
            "portfolio.steward",
            "담당관",
            "프로젝트 목표와 결정의 연속성 및 미해결 문제를 살핀다.",
            "맥락과 근거가 표시된 방향 제안",
            false,
        ),
        (
            "delivery.worker",
            "실무관",
            "경계가 정해진 과업의 실행 결과를 보고한다.",
            "산출물, 수행한 검증, 남은 위험을 구분한 보고",
            true,
        ),
        (
            "oversight.inspector",
            "감사관",
            "과업의 주장과 증거를 독립적으로 점검한다.",
            "발견 사항, 근거, 검증 한계를 구분한 점검 보고",
            true,
        ),
    ]
    .into_iter()
    .map(
        |(role_id, title, responsibility, output_contract, task_required)| RoleDefinition {
            role_id: role_id.to_owned(),
            version: 1,
            title: title.to_owned(),
            responsibility: responsibility.to_owned(),
            output_contract: output_contract.to_owned(),
            task_required,
            advisory: true,
        },
    )
    .collect()
}

pub fn get(role_id: &str, version: u32) -> Option<RoleDefinition> {
    definitions()
        .into_iter()
        .find(|role| role.role_id == role_id && role.version == version)
}

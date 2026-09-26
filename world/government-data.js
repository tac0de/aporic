/**
 * Fixed, renderer-independent view of the version 2 advisory government roster.
 * Refresh this file from the government roster when the roster changes.
 */
const freeze = (value) => {
  if (value && typeof value === "object" && !Object.isFrozen(value)) {
    Object.values(value).forEach(freeze);
    Object.freeze(value);
  }
  return value;
};

export const GOVERNMENT_SNAPSHOT = freeze({
  snapshot: {
    id: "aporic-government-world-v2-2026-09-26",
    capturedAt: "2026-09-26",
    charterVersion: 2,
    status: "advisory",
    authoritySource: "현재 인간 지시",
    grantsAuthority: false,
    description: "Aporic 정부는 대통령님의 현재 지시를 보조하는 자문 조직입니다.",
    sources: [
      {
        label: "Version 2 government charter",
        path: "crates/aporic/src/government.rs",
      },
      {
        label: "Initial advisory government roster",
        path: "README.md#v0.22-initial-advisory-government-roster",
      },
    ],
  },
  status: {
    capturedAt: '2026-09-26 09:59 UTC',
    activeMinistries: 3,
    activeOfficeholders: 9,
    activeProductCells: 0,
    taskRegistry: { leased: 1, queued: 3, completed: 4, cancelled: 4 },
    note: '작업 레지스트리는 자문 기록이며 실제 실행이나 최신 코드 상태를 증명하지 않습니다.',
  },

  player: {
    id: "current-human",
    name: "대통령님",
    title: "대통령",
    kind: "human",
    description: "현재 지시로 정부의 방향을 정하는 월드의 주인공입니다.",
    authorityNote: "현재 인간 지시가 모든 자문 기록보다 우선합니다.",
  },

  offices: [
    {
      id: "product.experiment",
      title: "제품실험부",
      headId: "product.experiment.minister",
      color: "#EF8D45",
      icon: "flask",
      cardDescription: "문제를 가설과 작은 프로토타입으로 바꿔 실제 증거를 만듭니다.",
      responsibility:
        "제품 의제를 검증 가능한 문제와 가설로 바꾸고, 의제별 다학제 제품반이 최소 프로토타입으로 직접 증거를 생산하도록 책임진다.",
      outputContract:
        "문제, 가설, 성공 척도, 프로토타입 증거, 계속·수정·중단 권고를 구분한 보고",
    },
    {
      id: "memory.information",
      title: "기억정보부",
      headId: "memory.information.minister",
      color: "#4D93D8",
      icon: "archive",
      cardDescription: "기억과 자료의 출처를 보존해 필요한 맥락을 제때 꺼냅니다.",
      responsibility:
        "프로젝트 기억과 외부 자료의 출처·시점·수명주기를 보존하고 필요한 맥락을 제공한다.",
      outputContract: "출처와 불확실성을 표시한 기억·자료 조회 및 인수인계",
    },
    {
      id: "execution.operations",
      title: "실행운영부",
      headId: "execution.operations.minister",
      color: "#57A773",
      icon: "compass",
      cardDescription: "과업과 연동, 기록, 복구 상태를 한 흐름으로 정리합니다.",
      responsibility: "과업 조정, 호스트 연동, 실행 기록 및 운영 복구를 관리한다.",
      outputContract: "과업 상태, 연동 관측, 검증 영수증, 복구 상태를 구분한 보고",
    },
  ],

  people: [
    {
      id: "executive.prime_minister",
      name: "김민준",
      title: "국무총리",
      kind: "prime-minister",
      officeId: null,
      reportsTo: "current-human",
      cardDescription: "각 부처의 보고를 모아 정부의 다음 판단을 정리합니다.",
      responsibility: "부처를 조정하고 그 보고를 종합한다.",
    },
    {
      id: "product.experiment.minister",
      name: "이서연",
      title: "제품실험부 장관",
      kind: "minister",
      officeId: "product.experiment",
      reportsTo: "executive.prime_minister",
      cardDescription: "문제, 가설, 프로토타입, 증거의 제품 실험을 이끕니다.",
      responsibility: "문제·가설·프로토타입·증거 작업을 이끈다.",
    },
    {
      id: "memory.information.minister",
      name: "박지훈",
      title: "기억정보부 장관",
      kind: "minister",
      officeId: "memory.information",
      reportsTo: "executive.prime_minister",
      cardDescription: "쌓인 맥락과 출처를 보존하고, 찾을 수 있게 만듭니다.",
      responsibility: "지속 맥락, 출처, 검색 품질을 관리한다.",
    },
    {
      id: "execution.operations.minister",
      name: "최수진",
      title: "실행운영부 장관",
      kind: "minister",
      officeId: "execution.operations",
      reportsTo: "executive.prime_minister",
      cardDescription: "과업, 연동, 운영 기록과 복구를 조율합니다.",
      responsibility: "과업·연동·운영 기록·복구를 조정한다.",
    },
    {
      id: "product.design.lead",
      name: "정민지",
      title: "디자인 책임자",
      kind: "lead",
      officeId: "product.experiment",
      reportsTo: "product.experiment.minister",
      cardDescription: "제품반의 UX, 시각 표현, 모션 디자인을 이끕니다.",
      responsibility: "제품반의 UX·시각·모션 디자인을 이끈다.",
    },
    {
      id: "product.frontend.lead",
      name: "강도윤",
      title: "프론트엔드 책임자",
      kind: "lead",
      officeId: "product.experiment",
      reportsTo: "product.experiment.minister",
      cardDescription: "브라우저 구현과 접근성, 화면 검토를 책임집니다.",
      responsibility: "브라우저 구현, 접근성, 렌더링 검토를 이끈다.",
    },
    {
      id: "product.backend.lead",
      name: "조현우",
      title: "백엔드 책임자",
      kind: "lead",
      officeId: "product.experiment",
      reportsTo: "product.experiment.minister",
      cardDescription: "서비스 계약, 데이터, 운영 신뢰성을 다룹니다.",
      responsibility: "서비스 계약, 데이터, 운영 신뢰성 작업을 이끈다.",
    },
    {
      id: "product.game.lead",
      name: "윤지우",
      title: "게임 개발 책임자",
      kind: "lead",
      officeId: "product.experiment",
      reportsTo: "product.experiment.minister",
      cardDescription: "게임 시스템과 레벨, 플레이 가능한 빌드의 검증을 이끕니다.",
      responsibility: "게임 시스템, 레벨, 플레이 가능한 빌드, 플레이테스트를 이끈다.",
    },
    {
      id: "oversight.inspector",
      name: "한예진",
      title: "독립 감사관",
      kind: "inspector",
      officeId: null,
      reportsTo: "current-human",
      independent: true,
      cardDescription: "완료 주장과 근거를 부처와 독립적으로 살핍니다.",
      responsibility: "증거와 완료 주장을 독립적으로 점검한다.",
    },
  ],
});

export default GOVERNMENT_SNAPSHOT;

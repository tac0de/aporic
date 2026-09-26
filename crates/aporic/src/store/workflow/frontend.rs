//! Versioned frontend procedure modules. These describe checks; the host runs the browser.

use super::{StepSpec, WorkflowProcedureDepth, WorkflowStage};

pub(super) const BROWSER_REVIEW_ID: &str = "frontend.browser_review";

// This first module stays narrow. Later additions use a new procedure template
// version so an existing task keeps the contract it originally selected.
pub(super) const BROWSER_REVIEW_STEPS: &[StepSpec] = &[
    StepSpec {
        id: "browser_scenarios",
        stage: WorkflowStage::Planning,
        description: "Define the primary browser scenario, viewports, and states to inspect",
        minimum_depth: WorkflowProcedureDepth::Standard,
        ui_only: false,
        skippable: false,
    },
    StepSpec {
        id: "rendered_browser_review",
        stage: WorkflowStage::Implementation,
        description: "Inspect the rendered frontend and exercise its primary interaction",
        minimum_depth: WorkflowProcedureDepth::Light,
        ui_only: false,
        skippable: false,
    },
    StepSpec {
        id: "responsive_accessibility_review",
        stage: WorkflowStage::Verification,
        description: "Inspect narrow layouts, keyboard operation, and accessibility structure",
        minimum_depth: WorkflowProcedureDepth::Standard,
        ui_only: false,
        skippable: false,
    },
];

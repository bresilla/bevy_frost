use egui::{Color32, Id};

use crate::{
    ribbon::{RibbonOverrideLayer, RibbonSlotDef},
    workspace::{WorkspaceBar, WorkspaceLevelState, WorkspacePolicy, WorkspaceStack},
};

/// Options for a module's inline pod representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModuleInlineOptions {
    pub allow_workspace: bool,
    pub units: usize,
}

impl Default for ModuleInlineOptions {
    fn default() -> Self {
        Self {
            allow_workspace: true,
            units: 10,
        }
    }
}

/// Context passed to `FrostModule::inline`.
pub struct ModuleInlineCtx<'a> {
    pub pod_id: Id,
    pub slot_index: usize,
    pub accent: Color32,
    pub options: ModuleInlineOptions,
    pub workspace: Option<&'a mut WorkspaceStack>,
}

impl ModuleInlineCtx<'_> {
    #[must_use]
    pub fn can_enter_workspace(&self) -> bool {
        self.options.allow_workspace
            && self.workspace.as_ref().map_or(true, |stack| {
                stack.current_policy().allow_module_workspace_push
            })
    }
}

/// Result of rendering a module inline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ModuleResponse {
    pub enter_workspace: bool,
}

impl ModuleResponse {
    #[must_use]
    pub const fn none() -> Self {
        Self {
            enter_workspace: false,
        }
    }

    #[must_use]
    pub const fn enter_workspace() -> Self {
        Self {
            enter_workspace: true,
        }
    }
}

/// Context passed to `FrostModule::workspace`.
///
/// This is intentionally small for the first implementation slice:
/// it carries workspace identity, policy, bars, accent, and stack
/// mutation. Rendering panes/containers into an L1 workspace will be
/// layered on top of this type in the next phase.
pub struct WorkspaceCtx<'a> {
    pub level: WorkspaceLevelState,
    pub policy: WorkspacePolicy,
    pub accent: Color32,
    stack: &'a mut WorkspaceStack,
    bars: Vec<WorkspaceBar>,
    ribbons: Vec<RibbonSlotDef>,
    ribbon_overrides: Vec<RibbonOverrideLayer>,
}

impl<'a> WorkspaceCtx<'a> {
    #[must_use]
    pub fn new(stack: &'a mut WorkspaceStack, accent: Color32) -> Self {
        let level = stack.current();
        let policy = stack.current_policy();
        Self {
            level,
            policy,
            accent,
            stack,
            bars: Vec::new(),
            ribbons: Vec::new(),
            ribbon_overrides: Vec::new(),
        }
    }

    pub fn add_bar(&mut self, bar: WorkspaceBar) {
        self.bars.push(bar);
    }

    #[must_use]
    pub fn bars(&self) -> &[WorkspaceBar] {
        &self.bars
    }

    pub fn add_ribbon(&mut self, ribbon: RibbonSlotDef) {
        self.ribbons.push(ribbon);
    }

    #[must_use]
    pub fn ribbons(&self) -> &[RibbonSlotDef] {
        &self.ribbons
    }

    pub fn add_ribbon_override(&mut self, layer: RibbonOverrideLayer) {
        self.ribbon_overrides.push(layer);
    }

    #[must_use]
    pub fn ribbon_overrides(&self) -> &[RibbonOverrideLayer] {
        &self.ribbon_overrides
    }

    pub fn push_module_workspace(&mut self, module_id: Id) -> WorkspaceLevelState {
        self.stack.push_module(module_id)
    }

    pub fn pop_workspace(
        &mut self,
    ) -> Result<WorkspaceLevelState, crate::workspace::WorkspaceStackError> {
        self.stack.pop()
    }
}

use crate::{
    build_enforcement_plan, ConfiguredBackend, DisplayState, EnforcementMode, EnforcementPlan,
    LayoutPolicy,
};

use super::EnforceOptions;

pub(super) struct EnforcementSession {
    backend: ConfiguredBackend,
    state: DisplayState,
    policy: LayoutPolicy,
}

impl EnforcementSession {
    pub(super) fn new(
        options: &EnforceOptions,
        build_backend: impl Fn(&str) -> Result<ConfiguredBackend, String>,
    ) -> Result<Self, String> {
        let policy = LayoutPolicy::read_from_path(&options.layout_path)
            .map_err(|error| error.to_string())?;
        let backend = build_backend(&options.backend_name)?;

        Ok(Self {
            backend,
            state: DisplayState::new(),
            policy,
        })
    }

    pub(super) fn backend(&self) -> &ConfiguredBackend {
        &self.backend
    }

    pub(super) fn build_recoverable_plan(&mut self) -> Result<EnforcementPlan, String> {
        self.build_plan(EnforcementMode::Daemon)
    }

    fn build_plan(&mut self, mode: EnforcementMode) -> Result<EnforcementPlan, String> {
        self.refresh_state()?;
        build_enforcement_plan(&self.policy, &self.state, mode).map_err(|error| error.to_string())
    }

    fn refresh_state(&mut self) -> Result<(), String> {
        let events = self
            .backend
            .snapshot_events()
            .map_err(|error| error.to_string())?;
        for event in events {
            self.state.apply(event);
        }

        Ok(())
    }
}

use crate::{backend::WindowLayoutBackend, EnforcementPlan, LayoutOperation};

pub(super) fn apply_plan(
    backend: &impl WindowLayoutBackend,
    plan: &EnforcementPlan,
) -> Result<(), String> {
    for operation in &plan.operations {
        apply_operation(backend, operation)?;
    }

    Ok(())
}

fn apply_operation(
    backend: &impl WindowLayoutBackend,
    operation: &LayoutOperation,
) -> Result<(), String> {
    match operation {
        LayoutOperation::ConfigureWindow { id, .. } => {
            let change = operation
                .geometry_change()
                .expect("configure operation should have geometry");
            backend
                .configure_window(*id, &change)
                .map_err(|error| error.to_string())
        }
        LayoutOperation::RaiseWindow { id, .. } => {
            backend.raise_window(*id).map_err(|error| error.to_string())
        }
        LayoutOperation::StackWindowAbove { id, sibling, .. } => backend
            .stack_window_above(*id, *sibling)
            .map_err(|error| error.to_string()),
    }
}

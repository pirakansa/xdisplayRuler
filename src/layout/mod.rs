mod planner;
mod policy;

pub use planner::{build_enforcement_plan, EnforcementMode, EnforcementPlan, LayoutOperation};
pub use policy::{
    LayoutError, LayoutPolicy, ManagedWindowRule, UnmanagedWindowsPolicy, WindowSelector,
};

#[cfg(test)]
mod tests;

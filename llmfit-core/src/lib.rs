pub mod electricity;
pub mod hardware;
pub mod mining;
pub mod rig_profiles;

pub use electricity::{
    ElectricityMode, ElectricityProfile, PowerContext, PowerCostEstimate, PowerPlanId,
    expand_power_context_options, fallback_power_context, resolve_electricity_profile,
    resolve_power_context,
};
pub use hardware::{GpuBackend, SystemSpecs, parse_memory_size};
pub use mining::{
    FitLevel, METHODS, MiningCoin, MiningMethod, MiningRow, MiningSnapshot, MiningStrategy,
    SnapshotCacheStatus, SnapshotLoad, SnapshotLoadMode, SortColumn, build_rankings,
    build_rankings_for_rigs, sort_rankings,
};
pub use rig_profiles::{
    AlgorithmBenchmark, AlgorithmRule, MiningRigProfile, RigKind, algorithm_rule,
    available_rig_profiles, describe_rig_scope, describe_rig_scope_summary,
    resolve_default_comparison_rigs, resolve_detected_rig_profiles, resolve_rig_profile,
};

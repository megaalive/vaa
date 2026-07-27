pub mod lint;
pub mod pipeline;
pub mod repro;

pub use lint::{lint_nasm_source, SourceLintFinding};
pub use pipeline::{
    binary_extension_for_target, default_linker_for_target, linker_args_for_target,
    nasm_format_for_target, probe_container_runtime, remap_host_args_to_container,
    resolve_output_binary_path, suggested_linux_linker_args, suggested_win64_linker_args,
    tool_digest, BuildManifest, BuildOutcome, BuildPipeline, ContainerBuildOpts, PipelineConfig,
    DEFAULT_CONTAINER_IMAGE,
};
pub use repro::{
    check_reproducible, compare_canonical, reproducible_build_check, CanonicalBuildView,
    ReproReport,
};

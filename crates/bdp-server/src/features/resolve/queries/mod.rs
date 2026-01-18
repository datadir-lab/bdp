pub mod resolve_manifest;

pub use resolve_manifest::{
    handle, DependencyInfo, ResolveManifestError, ResolveManifestQuery, ResolveManifestResponse,
    ResolvedSource, ResolvedTool, SourceSpec, ToolSpec,
};

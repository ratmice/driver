use crate::diagnostics::{Diagnostics, DiagnosticsEmitter};
use crate::source::{Session, SourceArtifact, SourceCache};

use crate::{Args, Params, Spanned};
use std::error;

/// Tool specific types and their bounds.
pub trait Tool: Args
where
    Self: Sized + Copy,
{
    /// The type of errors specific to a tool.
    type Error: SourceArtifact + error::Error + Spanned;
    /// The type of warnings specific to a tool.
    type Warning: SourceArtifact + Spanned;
    /// The type output by the tool.
    type Output: ToolInit<Self>;
    /// A tool specific kind for source text
    /// accessible from a session.
    type SourceKind;
}

/// Trait for running a tool.
pub trait ToolInit<X>
where
    X: Tool,
{
    fn tool_init<D: Diagnostics<X>>(
        config: Params<X>,
        source_cache: SourceCache<'_>,
        emitter: DiagnosticsEmitter<X, D>,
        session: &mut Session<X::SourceKind>,
    ) -> Self;
}

/// Errors have been emitted by the tool, that were observed by the driver.
#[derive(Debug)]
pub enum ToolError {
    ToolFailure,
}

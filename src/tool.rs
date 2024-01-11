use crate::diagnostics::Diagnostics;
use crate::source::SourceArtifact;
use crate::driver::ToolInitEnv;

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
        tool_env: &mut ToolInitEnv<X, D>,
    ) -> Self;
}

/// Errors have been emitted by the tool, that were observed by the driver.
#[derive(Debug)]
pub enum ToolError {
    ToolFailure,
}

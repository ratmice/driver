use crate::tool::{Tool, ToolError};

pub trait Diagnostics<X: Tool> {
    /// Indicatation that an error has occurred and the
    /// `Diagnostics` should take ownership of it.
    fn emit_error(&mut self, error: X::Error);
    /// Indicatation that a `warning` has occurred and the
    /// `Diagnostics` should take ownership of it.
    fn emit_warning(&mut self, warning: X::Warning);
    /// Called by the `DiagnosticsEmitter` drop handler.
    /// The receiver may need to propagate this call.
    ///
    /// If an implementation buffers errors emitting them here
    /// they should consider the case where the diagnostics
    /// outlives multiple `DiagnosticsEmitters`s by being passed
    /// through multiple drivers, and drain those buffers here.
    /// To avoid emitting the same errors multiple times.
    fn no_more_data(&mut self);
}

/// Sends ownership and observes emission of diagnostics from a tool.
pub struct DiagnosticsEmitter<'diag, X, D>
where
    X: Tool,
    D: Diagnostics<X>,
{
    observed_warning: bool,
    observed_error: bool,
    diagnostics: &'diag mut D,
    // This is primarily used to guide inference.
    #[allow(unused)]
    tool: X,
}

impl<'diag, X: Tool, Diag> Drop for DiagnosticsEmitter<'diag, X, Diag>
where
    Diag: Diagnostics<X>,
{
    /// Calls `Diagnostics::no_more_data()`
    fn drop(&mut self) {
        self.diagnostics.no_more_data()
    }
}

impl<'diag, X, D> DiagnosticsEmitter<'diag, X, D>
where
    X: Tool,
    D: Diagnostics<X>,
{
    pub(crate) fn new(tool: X, diagnostics: &'diag mut D) -> Self {
        Self {
            observed_error: false,
            observed_warning: false,
            diagnostics,
            tool,
        }
    }
    /// 1. Notes the indication of an error for later observation.
    /// 2. Sends the error off to be owned by `self.diagnostics`.
    /// 3. Returns a `ToolError::ToolFailure`
    pub fn emit_error(&mut self, e: X::Error) -> Result<(), ToolError> {
        self.observed_error = true;
        self.diagnostics.emit_error(e);
        Err(ToolError::ToolFailure)
    }
    /// 1. Notes the indication of an error for later observation.
    /// 2. Sends the error off to be owned by `self.diagnostics`.
    pub fn emit_non_fatal_error(&mut self, e: X::Error) {
        self.observed_error = true;
        self.diagnostics.emit_error(e);
    }
    /// 1. Notes the indication of the warning for later observation.
    /// 2. Sends the warning off to be owned by `self.diagnostics`.
    pub fn emit_warning(&mut self, w: X::Warning) {
        self.observed_warning = true;
        self.diagnostics.emit_warning(w);
    }

    /// Returns whether any errors have been observed during it's lifetime.
    pub fn observed_error(&self) -> bool {
        self.observed_error
    }
    /// Returns whether any warnings have been observed during it's lifetime.
    pub fn observed_warning(&self) -> bool {
        self.observed_warning
    }
}

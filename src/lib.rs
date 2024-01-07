/// I don't know how I feel about this, but it works.
use std::io::Read as _;
use std::sync::atomic::AtomicUsize;
use std::{collections::HashMap, sync::atomic::Ordering};
use std::{error, fmt, io, path};

#[cfg(test)]
mod test;

mod _unstable_api_ {

    /// A sealed trait.
    pub trait InternalTrait {}

    #[derive(Default)]
    pub struct InternalDefault;
}

pub trait SourceArtifact {
    fn source_id(&self) -> SourceId;
}

pub trait Tool
where
    Self: Sized + Copy,
{
    /// The type of errors specific to a tool.
    type Error: SourceArtifact + error::Error + Spanned;
    /// The type of warnings specific to a tool.
    type Warning: Spanned;
    /// The type output by the tool.
    type Output: for<'args> ToolInit<'args, Self>;
    /// A tool specific type for arguments must be given.
    type RequiredArgs<'args>;
    /// A tool specific type for arguments which derive `Default`
    type OptionalArgs: Default;
}

/// Trait for constructing tool output.
pub trait ToolInit<'args, X>
where
    X: Tool,
{
    fn tool_init<D: Diagnostics<X>>(
        config: Options<X::RequiredArgs<'args>, X::OptionalArgs>,
        source_cache: SourceCache<'_>,
        diagnostics: DiagnosticsEmitter<X, D>,
        session: Session,
    ) -> X::Output;
}

#[doc(hidden)]
pub struct DefaultDriver;

#[doc(hidden)]
pub trait DriverArgsSelection: _unstable_api_::InternalTrait {
    type RequiredArgs;
    type OptionalArgs: Default;
}

impl _unstable_api_::InternalTrait for DefaultDriver {}

impl DriverArgsSelection for DefaultDriver {
    type OptionalArgs = DriverOptionalArgs;
    type RequiredArgs = DriverArgs;
}

pub struct SourceCache<'a> {
    source_cache: &'a mut HashMap<SourceId, (std::path::PathBuf, String)>,
}

impl<'src> SourceCache<'src> {
    pub fn source_ids(&self) -> impl Iterator<Item = SourceId> + '_ {
        self.source_cache.iter().map(|(src_id, _)| *src_id)
    }

    pub fn source_for_id(&self, src_id: SourceId) -> Option<&str> {
        self.source_cache.get(&src_id).map(|(_, src)| src.as_str())
    }

    pub fn path_for_id(&self, src_id: SourceId) -> Option<&path::Path> {
        self.source_cache
            .get(&src_id)
            .map(|(path, _)| path.as_path())
    }

    /// This should allow us to populate the source cache with generated code.
    pub fn add_source(&mut self, path: path::PathBuf, src: String) -> SourceId {
        LAST_SOURCE_ID.fetch_add(1, Ordering::SeqCst);
        let source_id = SourceId(LAST_SOURCE_ID.load(Ordering::SeqCst));
        self.source_cache.insert(source_id, (path, src));
        source_id
    }
}

/// Used to configure and initialize a driver for a tool.
///
/// Contains the tool to run which must implement `Tool`,
/// `driver_options` for itself, and `options` for the tool.
///
/// Fields are public so that they are constructable by the caller.
pub struct Driver<'args, X: Tool, D: DriverArgsSelection = DefaultDriver> {
    /// This is mostly here to guide inference, and generally would be a unitary type.
    pub tool: X,
    pub driver: D,
    /// Options which are specific to the driver and kept hidden
    /// from the tool. For instance whether warnings are errors.
    /// Since `tools` route errors through the driver, tools should
    /// not concern themselves with it.
    ///
    /// Similarly if we implement `Path`/source providing in the driver.
    /// Tools should also probably not concern themselves with that.
    pub driver_options: Options<D::RequiredArgs, D::OptionalArgs>,
    pub options: Options<X::RequiredArgs<'args>, X::OptionalArgs>,
}

/// Errors occurred by the driver.
#[derive(thiserror::Error, Debug)]
pub enum DriverError {
    #[error("Io error {0} ")]
    Io(#[from] io::Error),
}

static LAST_SOURCE_ID: AtomicUsize = AtomicUsize::new(0);

pub struct Session {
    source_ids: Vec<SourceId>,
}

/// A session is created during `driver_init`, and contains
/// `SourceId`s for the documents loaded during driver init.
///
/// While `source_cache`, and `diagnostics` are allowed to
/// persist across driver runs. `Session` is ephemeral.
///
/// This can be used to obtain the subset of the files asked to
/// be loaded from the `source_cache`.
impl Session {
    pub fn source_ids(&self) -> impl Iterator<Item = SourceId> + '_ {
        self.source_ids.iter().copied()
    }
}

impl<'args, X: Tool> Driver<'args, X /* Driver = DefaultDriver */> {
    ///
    /// 1. Populates a `source_cache`
    /// 2. Constructes a `Diagnostics emitter`.
    /// 3. Passes everything above to the tool's implementation of `tool_init`.
    pub fn driver_init<D: Diagnostics<X>>(
        mut self,
        diagnostics: &mut D,
        source_cache: &mut HashMap<SourceId, (path::PathBuf, String)>,
    ) -> Result<X::Output, DriverError> {
        let mut source_ids = Vec::new();
        if let Some(source_path) = self.driver_options.optional.source_path.take() {
            let dir = cap_std::fs::Dir::open_ambient_dir(".", cap_std::ambient_authority())?;
            let mut file = dir.open(&source_path)?;
            let mut source = String::new();
            LAST_SOURCE_ID.fetch_add(1, Ordering::SeqCst);
            let source_id = SourceId(LAST_SOURCE_ID.load(Ordering::SeqCst));
            file.read_to_string(&mut source)?;
            source_cache.insert(source_id, (source_path, source));
            source_ids.push(source_id);
        }
        if let Some((string_path_name, source_string)) = self.driver_options.optional.source_string
        {
            LAST_SOURCE_ID.fetch_add(1, Ordering::SeqCst);
            let source_id = SourceId(LAST_SOURCE_ID.load(Ordering::SeqCst));
            source_cache.insert(source_id, (string_path_name, source_string));
            source_ids.push(source_id);
        }
        let source_cache = SourceCache { source_cache };

        let emitter = DiagnosticsEmitter::new(self.tool, diagnostics);
        let session = Session { source_ids };
        Ok(X::Output::tool_init(
            self.options,
            source_cache,
            emitter,
            session,
        ))
    }
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

impl<'diag, X, D> DiagnosticsEmitter<'diag, X, D>
where
    X: Tool,
    D: Diagnostics<X>,
{
    fn new(tool: X, diagnostics: &'diag mut D) -> Self {
        Self {
            observed_error: false,
            observed_warning: false,
            diagnostics,
            tool,
        }
    }
    pub fn emit_error(&mut self, e: X::Error) -> Result<(), ToolError> {
        self.observed_error = true;
        self.diagnostics.emit_error(e);
        Err(ToolError::ToolFailure)
    }
    pub fn emit_non_fatal_error(&mut self, e: X::Error) {
        self.observed_error = true;
        self.diagnostics.emit_error(e);
    }
    pub fn emit_warning(&mut self, w: X::Warning) {
        self.observed_warning = true;
        self.diagnostics.emit_warning(w);
    }
    pub fn observed_error(&self) -> bool {
        self.observed_error
    }
    pub fn observed_warning(&self) -> bool {
        self.observed_warning
    }
}

/// Required options that are common to all drivers.
pub struct DriverArgs {
    // There are not currently any required options for the tool.
}

#[derive(Default)]
/// Optional arguments common to all drivers.
pub struct DriverOptionalArgs {
    /// Reads a source at the given `path`` relative to the `current_dir()`.
    pub source_path: Option<path::PathBuf>,
    /// Uses a given name, and string.
    pub source_string: Option<(path::PathBuf, String)>,
    #[doc(hidden)]
    _non_exhaustive: _unstable_api_::InternalDefault,
}

/// A Simple implementation of a `Diagnostics` trait.
/// This was previously called a `Report`.
pub struct SimpleDiagnostics<X: Tool> {
    warnings: Vec<X::Warning>,
    errors: Vec<X::Error>,
}

impl<X: Tool> Default for SimpleDiagnostics<X> {
    fn default() -> Self {
        Self {
            warnings: vec![],
            errors: vec![],
        }
    }
}

impl<X: Tool> Diagnostics<X> for SimpleDiagnostics<X> {
    /// Indicatation that an error has occurred and the
    /// `Diagnostics` should take ownership.
    fn emit_error(&mut self, e: X::Error) {
        self.errors.push(e);
    }
    /// Indicatation that a `warning` has occurred and the
    /// `Diagnostics` should take ownership of it.
    fn emit_warning(&mut self, w: X::Warning) {
        self.warnings.push(w);
    }
    /// Called by the `DiagnosticsEmitter` drop handler.
    fn no_more_data(&mut self) {
        println!("no_more_data");
    }
}

impl<'diag, X: Tool, Diag> Drop for DiagnosticsEmitter<'diag, X, Diag>
where
    Diag: Diagnostics<X>,
{
    fn drop(&mut self) {
        self.diagnostics.no_more_data()
    }
}

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
/// opaque ID for source strings:
///
/// * A source string may have multiple SourceIDs.
/// * A SourceID refers uniquely to a single source string.
#[derive(Debug)]
pub struct SourceId(usize);

/// Errors have been emitted by the tool, that were observed by the driver.
#[derive(Debug)]
pub enum ToolError {
    ToolFailure,
}
#[derive(Debug)]
#[allow(dead_code)]
pub struct Span {
    start: usize,
    end: usize,
}

pub enum SpansKind {
    DuplicationError,
    Error,
}
pub trait Spanned: fmt::Display {
    // Required methods
    fn spans(&self) -> &[Span];
    fn spanskind(&self) -> SpansKind;
}

pub trait Diagnostics<X: Tool> {
    fn emit_error(&mut self, error: X::Error);
    fn emit_warning(&mut self, warning: X::Warning);
    /// Is called from `DiagnosticsEmitter::drop` To the top-level
    /// diagnostics value. Receivers may need to propagate it.
    fn no_more_data(&mut self);
}

/// A pair of required and optional fields.
pub struct Options<Required, Optional> {
    pub required: Required,
    pub optional: Optional,
}

impl<Required, Optional> From<(Required, Optional)> for Options<Required, Optional> {
    fn from((required, optional): (Required, Optional)) -> Self {
        Self { required, optional }
    }
}

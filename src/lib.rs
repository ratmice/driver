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
}

pub trait Args {
    /// A tool specific type for arguments that must be given.
    type RequiredArgs;
    /// A tool specific type for arguments which derive `Default`
    type OptionalArgs: Default;
}

/// Trait for constructing tool output.
pub trait ToolInit<X>
where
    X: Tool,
{
    fn tool_init<D: Diagnostics<X>>(
        config: Params<X::RequiredArgs, X::OptionalArgs>,
        source_cache: SourceCache<'_>,
        emitter: DiagnosticsEmitter<X, D>,
        session: Session,
    ) -> X::Output;
}

#[doc(hidden)]
pub struct DefaultDriver;

#[doc(hidden)]
pub trait DriverSelector: _unstable_api_::InternalTrait + Args {}
impl _unstable_api_::InternalTrait for DefaultDriver {}
impl DriverSelector for DefaultDriver {}
impl Args for DefaultDriver {
    type RequiredArgs = DriverArgs;
    type OptionalArgs = DriverOptionalArgs;
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
pub struct Driver<X, TArgs, DArgs, D: DriverSelector + Args = DefaultDriver>
where
    X: Tool,
    DArgs: Into<Params<D::RequiredArgs, D::OptionalArgs>>,
    TArgs: Into<Params<X::RequiredArgs, X::OptionalArgs>>,
{
    /// This is mostly here to guide inference, and generally would be a unitary type.
    pub tool: X,
    /// This is here to guide inference, and allow for future expansion, in the case
    /// that we require a different driver implementation, or changes to driver_args.
    pub driver: D,
    /// Options that get handled by the driver.
    pub driver_args: DArgs,
    // Options specific to a `Tool`.
    pub tool_args: TArgs,
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

impl<X, TOpts, DOpts> Driver<X, TOpts, DOpts, DefaultDriver>
where
    X: Tool,
    DOpts: Into<Params<DriverArgs, DriverOptionalArgs>>,
    TOpts: Into<Params<X::RequiredArgs, X::OptionalArgs>>,
    // This bound is not needed, but perhaps informative.
    DefaultDriver:
        DriverSelector + Args<RequiredArgs = DriverArgs, OptionalArgs = DriverOptionalArgs>,
{
    ///
    /// 1. Populates a `source_cache`
    /// 2. Constructes a `Diagnostics emitter`.
    /// 3. Passes everything above to the tool's implementation of `tool_init`.
    pub fn driver_init<D: Diagnostics<X>>(
        self,
        diagnostics: &mut D,
        source_cache: &mut HashMap<SourceId, (path::PathBuf, String)>,
    ) -> Result<X::Output, DriverError> {
        let mut driver_options = self.driver_args.into();
        let mut source_ids = Vec::new();
        if let Some(source_path) = driver_options.optional.source_path.take() {
            let dir = cap_std::fs::Dir::open_ambient_dir(
                if let Some(path) = driver_options.optional.relative_to_path {
                    path
                } else {
                    std::env::current_dir()?
                },
                cap_std::ambient_authority(),
            )?;
            let mut file = dir.open(&source_path)?;
            let mut source = String::new();
            LAST_SOURCE_ID.fetch_add(1, Ordering::SeqCst);
            let source_id = SourceId(LAST_SOURCE_ID.load(Ordering::SeqCst));
            file.read_to_string(&mut source)?;
            source_cache.insert(source_id, (source_path, source));
            source_ids.push(source_id);
        }
        if let Some((string_path_name, source_string)) = driver_options.optional.source_string {
            LAST_SOURCE_ID.fetch_add(1, Ordering::SeqCst);
            let source_id = SourceId(LAST_SOURCE_ID.load(Ordering::SeqCst));
            source_cache.insert(source_id, (string_path_name, source_string));
            source_ids.push(source_id);
        }
        let source_cache = SourceCache { source_cache };

        let emitter = DiagnosticsEmitter::new(self.tool, diagnostics);
        let session = Session { source_ids };
        Ok(X::Output::tool_init(
            self.tool_args.into(),
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
/// Optional arguments common to a driver.
pub struct DriverOptionalArgs {
    /// Reads a source at the given `path` relative to the
    /// `relative_to_path` argument.
    pub source_path: Option<path::PathBuf>,
    /// Uses a given name, and string.
    pub source_string: Option<(path::PathBuf, String)>,
    /// Allows `source_path` lookup relative to a directory path.
    /// Defaults to the current working directory.
    ///
    /// To allow unrestricted lookups across the filesystem,
    /// you'll need to set this to the root path.
    ///
    /// ```
    /// # use driver::DriverOptionalArgs;
    /// # let _ =
    /// DriverOptionalArgs {
    ///    relative_to_path: Some((&std::path::Component::RootDir).into()),
    ///    .. Default::default()
    /// }
    /// # ;
    /// ````
    ///
    /// Setting this to any other directory will cause
    /// lookups to be done relative to that path instead.
    pub relative_to_path: Option<path::PathBuf>,

    #[doc(hidden)]
    pub _non_exhaustive: _unstable_api_::InternalDefault,
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

/// A pair of required and optional parameters.
pub struct Params<Required, Optional> {
    pub required: Required,
    pub optional: Optional,
}

impl<Required, Optional> From<(Required, Optional)> for Params<Required, Optional> {
    fn from((required, optional): (Required, Optional)) -> Self {
        Self { required, optional }
    }
}

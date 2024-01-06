#![allow(dead_code)]
#![allow(unused_variables)]

/// I don't know how I feel about this, but it works.
use std::io::Read as _;
use std::sync::atomic::AtomicUsize;
use std::{borrow::Cow, collections::HashMap, sync::atomic::Ordering};

use std::{error, fmt, io, path};

mod _unstable_api_ {
    #[derive(Default)]
    pub struct InternalDefault;
}

pub trait Tool
where
    Self: Sized + Copy,
{
    /// The type of errors specific to a tool.
    type Error: error::Error + Spanned;
    /// The type of warnings specific to a tool.
    type Warning: Spanned;
    /// The type output by the tool.
    type Output<'a>: OutputWithDriverControl<'a, Self>;
    /// A tool specific type for arguments must be given.
    type RequiredArgs<'a>;
    /// A tool specific type for arguments which derive `Default`
    type OptionalArgs: Default;
}

/// A `DriverControl`, is built from a `DriverEnv`.
/// Which then gets built within `run_driver`.
///
/// `DriverControl`, just provides functions which the
/// implementer may call to interact with a driver,
/// such as a `DiagnosticsObserver`.
///
/// In the future there is some thoughts, that the `DriverControl`,
/// Can also eventually handle things filesystem interaction
/// presenting a tool with an `&str` without having
/// to concern itself with how it arrives.  Whether from
/// a `Path`, or a string passed in by the user.
pub trait OutputWithDriverControl<'a, T>
where
    T: Tool,
{
    fn build_with_driver_ctl<D: Diagnostics<T>>(
        config: Options<T::RequiredArgs<'a>, T::OptionalArgs>,
        control: DriverControl<'_, T, D>,
    ) -> T::Output<'a>;
}

/// `DriverConfig` gets passed in from within `run_driver`,
/// provded to the implementation of `BuildWithDriverControl`.
/// While `DriverOptions` are *not* passed in, and reserved for the driver.
///
/// There may be some form of `DriverOptions` which we do want to provide
/// Presumably they can be obtained through a `DriverControl`.
///
/// Fields are public so that they are constructable by the caller.
pub struct DriverConfig<'a, X: Tool> {
    /// This is mostly here to guide inference, and generally would be a unitary type.
    pub tool: X,
    /// Options which are specific to the driver and kept hidden
    /// from the tool. For instance whether warnings are errors.
    /// Since `tools` route errors through the driver, tools should
    /// not concern themselves with it.
    ///
    /// Similarly if we implement `Path`/source providing in the driver.
    /// Tools should also probably not concern themselves with that.
    pub driver_options: Options<DriverOptions, DriverOptionalArgs>,
    pub options: Options<X::RequiredArgs<'a>, X::OptionalArgs>,
}

/// This type is owned by driver, but may contain references which outlive it.
pub struct DriverControl<'a, X: Tool, D: Diagnostics<X>> {
    diagnostics_observer: DiagnosticsObserver<'a, X, D>,
    fscache: &'a mut HashMap<SourceId, (Cow<'a, path::Path>, String)>,
}

/// Returned by `DriverControl::take_owned`
pub struct DriverControlOwned<'a, X: Tool, D: Diagnostics<X>> {
    pub diagnostics_observer: DiagnosticsObserver<'a, X, D>,
}

/// Returned by `DriverControl::take_owned`
pub struct DriverControlBorrowed<'a> {
    fscache: &'a mut HashMap<SourceId, (Cow<'a, path::Path>, String)>,
}

/// Used to build a `DriverControl`, and may contain trait implementations
/// specified by the caller.  Currently this is not very future-proof.
pub struct DriverEnv<'a, X, C>
where
    X: Tool,
    C: CallerSpec<X>,
{
    tool: X,

    diagnostics: &'a mut C::Diagnostics,
    fscache: &'a mut HashMap<SourceId, (Cow<'a, path::Path>, String)>,
}

/// Associated types provided by the caller.
pub trait CallerSpec<X: Tool> {
    type Diagnostics: Diagnostics<X>;
}

/// An impl of CallerSpec that can be used with SimpleDiagnostics.
pub struct SimpleSpec;
impl<X: Tool> CallerSpec<X> for SimpleSpec {
    type Diagnostics = SimpleDiagnostics<X>;
}

/// Errors occurred by the driver.
#[derive(thiserror::Error, Debug)]
pub enum DriverError {
    #[error("Io error {0} ")]
    Io(#[from] io::Error),
}

static LAST_SOURCE_ID: AtomicUsize = AtomicUsize::new(0);

impl<'a, X: Tool> DriverConfig<'a, X> {
    /// Builds a DriverControl, calling `build_with_driver_ctl`.
    /// to return a tool specific `Output` type.
    pub fn run_driver<'b: 'a, C: CallerSpec<X>>(
        mut self,
        driver_env: DriverEnv<'b, X, SimpleSpec>,
        caller_spec: C,
    ) -> Result<X::Output<'a>, DriverError>
where {
        if let Some(source_path) = self.driver_options.optional.source_path.take() {
            let dir = cap_std::fs::Dir::open_ambient_dir(".", cap_std::ambient_authority())?;
            let mut file = dir.open(&source_path)?;
            let mut source = String::new();
            LAST_SOURCE_ID.fetch_add(1, Ordering::SeqCst);
            let source_id = SourceId(LAST_SOURCE_ID.load(Ordering::SeqCst));
            file.read_to_string(&mut source)?;
            driver_env
                .fscache
                .insert(source_id, (source_path.into(), source));
        }
        let driver_ctl = DriverControl {
            diagnostics_observer: DiagnosticsObserver::new(self.tool, driver_env.diagnostics),
            fscache: driver_env.fscache,
        };
        Ok(X::Output::build_with_driver_ctl(self.options, driver_ctl))
    }
}

/// This can be used to send ownership of diagnostic errors and warnings.
/// but retain knowledge of errors or warnings having occurred.
pub struct DiagnosticsObserver<'a, X, R>
where
    X: Tool,
    R: Diagnostics<X>,
{
    observed_warning: bool,
    observed_error: bool,
    report: &'a mut R,
    tool: X,
}

impl<'a, X, R> DiagnosticsObserver<'a, X, R>
where
    X: Tool,
    R: Diagnostics<X>,
{
    fn new<'r: 'a>(tool: X, report: &'r mut R) -> Self {
        Self {
            observed_error: false,
            observed_warning: false,
            report,
            tool,
        }
    }
    pub fn error(&mut self, e: X::Error) -> Result<(), DriverToolError> {
        self.observed_error = true;
        self.report.error(e);
        Err(DriverToolError::ToolFailure)
    }
    pub fn non_fatal_error(&mut self, e: X::Error) {
        self.observed_error = true;
        self.report.error(e);
    }
    pub fn warning(&mut self, w: X::Warning) {
        self.observed_warning = true;
        self.report.warning(w);
    }
    pub fn observed_error(&self) -> bool {
        self.observed_error
    }
    pub fn observed_warning(&self) -> bool {
        self.observed_warning
    }
}

/// Required options that are common to all drivers.
pub struct DriverOptions {
    pub foo: (),
}

#[derive(Default)]
/// Optional arguments common to all drivers.
pub struct DriverOptionalArgs {
    // not yet implemented.
    source_path: Option<path::PathBuf>,
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
    fn error(&mut self, e: X::Error) {
        self.errors.push(e);
    }
    /// Indicatation that a `warning` has occurred and the
    /// `Diagnostics` should take ownership of it.
    fn warning(&mut self, w: X::Warning) {
        self.warnings.push(w);
    }
    /// Called by the `DiagnosticsObserver` drop handler.
    fn no_more_data(&mut self) {
        println!("no_more_data");
    }
}

impl<X: Tool, R: Diagnostics<X>> Drop for DiagnosticsObserver<'_, X, R> {
    fn drop(&mut self) {
        self.report.no_more_data()
    }
}

impl<'a, X: Tool, D: Diagnostics<X>> DriverControl<'a, X, D> {
    pub fn take_owned(self) -> (DriverControlOwned<'a, X, D>, DriverControlBorrowed<'a>) {
        let owned = DriverControlOwned {
            diagnostics_observer: self.diagnostics_observer,
        };

        let borrowed = DriverControlBorrowed {
            fscache: self.fscache,
        };
        (owned, borrowed)
    }
}

impl<'a> DriverControlBorrowed<'a> {
    // SourceIds are unique even across multiple runs, and may be different
    // for the same path, when the sources are loaded multiple times,
    pub fn sources(&self) -> impl Iterator<Item = (SourceId, &str)> {
        self.fscache
            .iter()
            .map(|(source_id, (_, src))| (*source_id, src.as_str()))
    }
}

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
/// opaque ID for source strings:
///
/// * A source string may have multiple SourceIDs.
/// * A SourceID refers uniquely to a single source string.
pub struct SourceId(usize);

/// Errors have been emitted by the tool, that were observed by the driver.
#[derive(Debug)]
pub enum DriverToolError {
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
    fn error(&mut self, error: X::Error);
    fn warning(&mut self, warning: X::Warning);
    // Is called automatically if this report is the main Diagnostics
    // From a `DiagnosticsObserver` If this report has children,
    // it will need to propagate the call. Should only be called once.
    //
    // Default implementation does nothing.
    fn no_more_data(&mut self) {}
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

#[cfg(test)]
mod tests {
    #![allow(dead_code)]
    use super::*;
    use std::error::Error;

    #[derive(Copy, Clone)]
    struct Yacc;

    impl Tool for Yacc {
        type Error = YaccGrammarError;
        type Warning = YaccGrammarWarning;
        type OptionalArgs = YaccGrammarOptArgs;
        type RequiredArgs<'a> = YaccConfig;
        type Output<'a> = GrammarASTWithValidationCertificate;
    }

    #[derive(Debug)]
    pub enum YaccOriginalActionKind {
        UserAction,
        GenericParseTree,
        NoAction,
    }
    #[allow(unused)]
    #[derive(Debug)]
    pub enum YaccKind {
        Original(YaccOriginalActionKind),
        Grmtools,
        Eco,
    }

    struct YaccConfig {
        yacc_kind: YaccKind,
    }

    #[derive(Debug)]
    enum YaccGrammarError {
        Testing(Vec<Span>),
    }

    impl Spanned for YaccGrammarError {
        fn spans(&self) -> &[Span] {
            match self {
                Self::Testing(x) => x,
            }
        }
        fn spanskind(&self) -> SpansKind {
            match self {
                Self::Testing(_) => SpansKind::DuplicationError,
            }
        }
    }
    impl fmt::Display for YaccGrammarError {
        fn fmt(&self, _fmt: &mut fmt::Formatter) -> fmt::Result {
            unimplemented!()
        }
    }

    impl Error for YaccGrammarError {}
    impl fmt::Display for YaccGrammarWarning {
        fn fmt(&self, _fmt: &mut fmt::Formatter) -> fmt::Result {
            unimplemented!()
        }
    }
    enum YaccGrammarWarning {
        Testing(Vec<Span>),
    }

    impl Spanned for YaccGrammarWarning {
        fn spans(&self) -> &[Span] {
            match self {
                Self::Testing(x) => x,
            }
        }
        fn spanskind(&self) -> SpansKind {
            match self {
                Self::Testing(_) => SpansKind::DuplicationError,
            }
        }
    }

    #[derive(Default)]
    struct YaccGrammarOptArgs {
        _non_exhaustive: _unstable_api_::InternalDefault,
    }

    struct YaccGrammar;
    struct GrammarAST;
    struct GrammarASTWithValidationCertificate {
        ast: GrammarAST,
        validation_success: bool,
    }

    impl GrammarASTWithValidationCertificate {
        fn grammar(&self) -> Result<YaccGrammar, DriverToolError> {
            if self.validation_success {
                Ok(YaccGrammar)
            } else {
                Err(DriverToolError::ToolFailure)
            }
        }
        fn ast(&self) -> &GrammarAST {
            &self.ast
        }
    }

    impl<'a> OutputWithDriverControl<'a, Yacc> for GrammarASTWithValidationCertificate
    where
        Self: 'a,
    {
        fn build_with_driver_ctl<R: Diagnostics<Yacc>>(
            options: Options<<Yacc as Tool>::RequiredArgs<'a>, <Yacc as Tool>::OptionalArgs>,
            ctl: DriverControl<Yacc, R>,
        ) -> GrammarASTWithValidationCertificate {
            let (
                DriverControlOwned {
                    diagnostics_observer: mut observer,
                },
                ctl,
            ) = ctl.take_owned();
            if let Some((_, source)) = ctl.sources().next() {
                if !source.is_empty() {
                    observer.non_fatal_error(YaccGrammarError::Testing(vec![]));
                }
            }
            println!("{:?}", options.required.yacc_kind,);
            // now at some time in the future.
            GrammarASTWithValidationCertificate {
                ast: GrammarAST,
                validation_success: !observer.observed_error(),
            }
        }
    }

    #[test]
    fn it_works() {
        let mut diagnostics = SimpleDiagnostics::default();
        let mut fscache = HashMap::new();
        let driver_env = DriverEnv {
            tool: Yacc,
            diagnostics: &mut diagnostics,
            fscache: &mut fscache,
        };
        // Test that the DriverEnv outlives the driver.
        {
            // Just pass in `Yacc` to avoid DriverConfig::<Yacc>`.
            let driver = DriverConfig {
                tool: Yacc,
                driver_options: (DriverOptions { foo: () }, Default::default()).into(),
                options: (
                    YaccConfig {
                        yacc_kind: YaccKind::Grmtools,
                    },
                    Default::default(),
                )
                    .into(),
            }
            .run_driver(driver_env, SimpleSpec)
            .unwrap();
            let _ast = driver.ast();
            let _grm = driver.grammar().unwrap();
            #[allow(clippy::drop_non_drop)]
            drop(driver);
        }
    }

    #[should_panic]
    #[test]
    fn it_fails() {
        // These fields should perhaps be combined into something?
        let mut diagnostics = SimpleDiagnostics::default();
        let mut fscache = HashMap::new();

        let driver_env = DriverEnv {
            tool: Yacc,
            diagnostics: &mut diagnostics,
            fscache: &mut fscache,
        };
        // Test that the DriverEnv outlives the driver.
        {
            // Just pass in `Yacc` to avoid DriverConfig::<Yacc>`.
            let driver = DriverConfig {
                tool: Yacc,
                driver_options: (
                    DriverOptions { foo: () },
                    DriverOptionalArgs {
                        source_path: Some("Cargo.toml".into()),
                        ..Default::default()
                    },
                )
                    .into(),
                options: (
                    YaccConfig {
                        yacc_kind: YaccKind::Grmtools,
                    },
                    Default::default(),
                )
                    .into(),
            }
            .run_driver(driver_env, SimpleSpec)
            .unwrap();
            let _ast = driver.ast();
            let _grm = driver.grammar().unwrap();
            #[allow(clippy::drop_non_drop)]
            drop(driver);
        }
    }
}

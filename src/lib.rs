#![allow(dead_code)]
#![allow(unused_variables)]

/// I don't know how I feel about this, but it works.
use std::io::Read as _;
use std::sync::atomic::AtomicUsize;
use std::{collections::HashMap, sync::atomic::Ordering};
use std::{error, fmt, io, path};

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
        source_cache: &HashMap<SourceId, (path::PathBuf, String)>,
        diagnostics: DiagnosticsEmitter<X, D>,
    ) -> X::Output;
}

pub struct DefaultDriver;
/// A Default driver implementation,
///
/// Has required, optional arguments and specifies an environment
pub trait Driver: _unstable_api_::InternalTrait {
    type RequiredArgs;
    type OptionalArgs: Default;
}

impl _unstable_api_::InternalTrait for DefaultDriver {}

impl Driver for DefaultDriver {
    type OptionalArgs = DriverOptionalArgs;
    type RequiredArgs = DriverOptions;
}

/// Used to configure and initialize a driver for a tool.
///
/// Contains the tool to run which must implement `Tool`,
/// `driver_options` for itself, and `options` for the tool.
///
/// Fields are public so that they are constructable by the caller.
pub struct DriverConfig<'args, X: Tool, D: Driver = DefaultDriver> {
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

/// Associated types provided by the caller.
///
/// This one is pretty fragile, need to carefully think about what types/traits
/// It should guarantee, since it shared across all drivers.
///
/// It likely wants a source cache trait for vfs reasons.
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

impl<'args, X: Tool> DriverConfig<'args, X /* Driver = DefaultDriver */> {
    ///
    /// 1. Populates a `source_cache`
    /// 2. Constructes a `Diagnostics emitter`.
    /// 3. Passes everything above to the tool's implementation of `tool_init`.
    pub fn driver_init<C: CallerSpec<X>>(
        mut self,
        diagnostics: &mut C::Diagnostics,
        source_cache: &mut HashMap<SourceId, (path::PathBuf, String)>,
        caller_spec: C,
    ) -> Result<X::Output, DriverError> {
        if let Some(source_path) = self.driver_options.optional.source_path.take() {
            let dir = cap_std::fs::Dir::open_ambient_dir(".", cap_std::ambient_authority())?;
            let mut file = dir.open(&source_path)?;
            let mut source = String::new();
            LAST_SOURCE_ID.fetch_add(1, Ordering::SeqCst);
            let source_id = SourceId(LAST_SOURCE_ID.load(Ordering::SeqCst));
            file.read_to_string(&mut source)?;
            source_cache.insert(source_id, (source_path, source));
        }
        if let Some((string_path_name, source_string)) = self.driver_options.optional.source_string
        {
            LAST_SOURCE_ID.fetch_add(1, Ordering::SeqCst);
            let source_id = SourceId(LAST_SOURCE_ID.load(Ordering::SeqCst));
            source_cache.insert(source_id, (string_path_name, source_string));
        }

        let emitter = DiagnosticsEmitter::new(self.tool, diagnostics);
        Ok(X::Output::tool_init(self.options, source_cache, emitter))
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
pub struct DriverOptions {
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
        type Output = GrammarASTWithValidationCertificate;
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
    struct YaccGrammarError {
        source_id: SourceId,
        kind: YaccGrammarErrorKind,
    }

    #[derive(Debug)]
    enum YaccGrammarErrorKind {
        Testing(Vec<Span>),
    }

    impl SourceArtifact for YaccGrammarError {
        fn source_id(&self) -> SourceId {
            self.source_id
        }
    }

    impl Spanned for YaccGrammarError {
        fn spans(&self) -> &[Span] {
            match &self.kind {
                YaccGrammarErrorKind::Testing(x) => x.as_slice(),
            }
        }
        fn spanskind(&self) -> SpansKind {
            match self.kind {
                YaccGrammarErrorKind::Testing(_) => SpansKind::DuplicationError,
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
        fn grammar(&self) -> Result<YaccGrammar, ToolError> {
            if self.validation_success {
                Ok(YaccGrammar)
            } else {
                Err(ToolError::ToolFailure)
            }
        }
        fn ast(&self) -> &GrammarAST {
            &self.ast
        }
    }

    impl<'args> ToolInit<'args, Yacc> for GrammarASTWithValidationCertificate {
        fn tool_init<R: Diagnostics<Yacc>>(
            options: Options<<Yacc as Tool>::RequiredArgs<'args>, <Yacc as Tool>::OptionalArgs>,
            source_cache: &HashMap<SourceId, (path::PathBuf, String)>,
            mut emitter: DiagnosticsEmitter<Yacc, R>,
        ) -> GrammarASTWithValidationCertificate {
            let source = source_cache.iter().next();
            if let Some((source_id, (path, source))) = source {
                if path == &path::PathBuf::from("Cargo.toml") {
                    emitter.emit_non_fatal_error(YaccGrammarError {
                        source_id: *source_id,
                        kind: YaccGrammarErrorKind::Testing(vec![]),
                    });
                }
            }

            println!("{:?}", options.required.yacc_kind,);
            // now at some time in the future.
            GrammarASTWithValidationCertificate {
                ast: GrammarAST,
                validation_success: !emitter.observed_error(),
            }
        }
    }

    #[test]
    fn it_works() {
        let mut diagnostics: SimpleDiagnostics<Yacc> = SimpleDiagnostics::default();
        let mut source_cache = HashMap::new();
        {
            // Just pass in `Yacc` to avoid DriverConfig::<Yacc>`.
            let driver = DriverConfig {
                driver: DefaultDriver,
                tool: Yacc,
                driver_options: (
                    DriverOptions {},
                    DriverOptionalArgs {
                        source_path: Some("Cargo.lock".into()),
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
            .driver_init(&mut diagnostics, &mut source_cache, SimpleSpec)
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
        let mut source_cache = HashMap::new();

        {
            // Just pass in `Yacc` to avoid DriverConfig::<Yacc>`.
            let driver = DriverConfig {
                tool: Yacc,
                driver: DefaultDriver,
                driver_options: (
                    DriverOptions {},
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
            .driver_init(&mut diagnostics, &mut source_cache, SimpleSpec)
            .unwrap();
            let _ast = driver.ast();
            let _grm = driver.grammar().unwrap();
            #[allow(clippy::drop_non_drop)]
            drop(driver);
        }
    }

    #[test]
    fn unit_driver() {
        impl _unstable_api_::InternalTrait for () {}
        impl Driver for () {
            type RequiredArgs = ();
            type OptionalArgs = bool;
        }
        // These fields should perhaps be combined into something?
        let mut diagnostics = SimpleDiagnostics::default();
        let mut source_cache = HashMap::new();
        impl<X: Tool> DriverConfig<'_, X, ()> {
            pub fn driver_init<C: CallerSpec<X>>(
                self,
                source_cache: &mut HashMap<SourceId, (path::PathBuf, String)>,
                diagnostics: &mut C::Diagnostics,
                caller_spec: C,
            ) -> Result<X::Output, DriverError> {
                let emitter = DiagnosticsEmitter::new(self.tool, diagnostics);

                Ok(X::Output::tool_init(self.options, source_cache, emitter))
            }
        }

        {
            // Just pass in `Yacc` to avoid DriverConfig::<Yacc>`.
            let driver = DriverConfig {
                tool: Yacc,
                driver: (),
                driver_options: ((), true).into(),
                options: (
                    YaccConfig {
                        yacc_kind: YaccKind::Grmtools,
                    },
                    Default::default(),
                )
                    .into(),
            }
            .driver_init(&mut source_cache, &mut diagnostics, SimpleSpec)
            .unwrap();
            let _ast = driver.ast();
            let _grm = driver.grammar().unwrap();
            #[allow(clippy::drop_non_drop)]
            drop(driver);
        }
    }

    #[derive(Copy, Clone)]
    struct Lex;
    struct NeverWarnings(Option<std::convert::Infallible>);
    impl fmt::Display for NeverWarnings {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            Ok(())
        }
    }

    #[derive(Debug)]
    enum LexErrorKind {
        Testing(Vec<Span>),
    }

    #[derive(Debug)]

    struct LexError {
        source_id: SourceId,
        kind: LexErrorKind,
    }
    impl Error for LexError {}
    impl SourceArtifact for LexError {
        fn source_id(&self) -> SourceId {
            self.source_id
        }
    }
    impl Spanned for LexError {
        fn spans(&self) -> &[Span] {
            match &self.kind {
                LexErrorKind::Testing(spans) => spans.as_slice(),
            }
        }
        fn spanskind(&self) -> SpansKind {
            match self.kind {
                LexErrorKind::Testing(_) => SpansKind::Error,
            }
        }
    }

    impl Spanned for NeverWarnings {
        fn spans(&self) -> &[Span] {
            unimplemented!()
        }

        fn spanskind(&self) -> SpansKind {
            unimplemented!()
        }
    }

    impl fmt::Display for LexError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "Lex error test")
        }
    }

    struct LexOutput {}

    impl<'args> ToolInit<'args, tests::Lex> for LexOutput {
        fn tool_init<'diag, 'src, D: Diagnostics<Lex>>(
            config: Options<(), ()>,
            source_cache: &HashMap<SourceId, (path::PathBuf, String)>,
            emitter: DiagnosticsEmitter<Lex, D>,
        ) -> LexOutput {
            LexOutput {}
        }
    }

    impl Tool for Lex {
        type Error = LexError;
        type Warning = NeverWarnings;
        // FIXME look at lex to figure out what all the below should be,
        // This is mostly a test that we can populate the same source_cache
        // from multiple tools.
        type OptionalArgs = ();
        type RequiredArgs<'a> = ();
        type Output = LexOutput;
    }

    #[test]
    fn lex_driver() {
        // These fields should perhaps be combined into something?
        let mut source_cache = HashMap::new();
        {
            let mut diagnostics = SimpleDiagnostics::default();
            // Just pass in `Yacc` to avoid DriverConfig::<Yacc>`.
            let driver = DriverConfig {
                tool: Lex,
                driver: (),
                driver_options: ((), true).into(),
                options: ((), ()).into(),
            }
            .driver_init(&mut source_cache, &mut diagnostics, SimpleSpec)
            .unwrap();
            #[allow(clippy::drop_non_drop)]
            drop(driver);
        }
    }
    #[test]
    fn two_drivers_share_source_cache() {
        let mut source_cache = HashMap::new();
        {
            let mut diagnostics = SimpleDiagnostics::default();
            // Just pass in `Yacc` to avoid DriverConfig::<Yacc>`.
            let driver = DriverConfig {
                tool: Lex,
                driver: DefaultDriver,
                driver_options: (
                    DriverOptions {},
                    DriverOptionalArgs {
                        source_path: Some("Cargo.lock".into()),
                        ..Default::default()
                    },
                )
                    .into(),
                options: ((), ()).into(),
            }
            .driver_init(&mut diagnostics, &mut source_cache, SimpleSpec)
            .unwrap();
            #[allow(clippy::drop_non_drop)]
            drop(driver);
        }

        {
            let mut diagnostics = SimpleDiagnostics::default();
            // Just pass in `Yacc` to avoid DriverConfig::<Yacc>`.
            let driver = DriverConfig {
                tool: Yacc,
                driver: DefaultDriver,
                driver_options: (
                    DriverOptions {},
                    DriverOptionalArgs {
                        source_path: Some("Cargo.lock".into()),
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
            .driver_init(&mut diagnostics, &mut source_cache, SimpleSpec)
            .unwrap();
            let _ast = driver.ast();
            let _grm = driver.grammar().unwrap();
            #[allow(clippy::drop_non_drop)]
            drop(driver);
        }
    }
}

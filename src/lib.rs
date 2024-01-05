#![allow(dead_code)]
#![allow(unused_variables)]

/// I don't know how I feel about this, but it works.
use std::{error, fmt};

mod _unstable_api_ {
    #[derive(Default)]
    pub struct InternalDefault;
}

pub trait Tool
where
    Self: Sized + Copy,
{
    type Error: error::Error + Spanned;
    type Warning: Spanned;
    type Output<'a>: BuildWithDriverEnv<'a, Self>;
    type RequiredArgs<'a>;
    type OptionalArgs: Default;
}

pub trait BuildWithDriverEnv<'a, T>
where
    T: Tool,
{
    fn build_with_driver_env<R: Diagnostics<T>>(
        config: DriverConfig<'a, T>,
        control: DriverEnv<'_, T, R>,
    ) -> T::Output<'a>;
}

pub struct DriverConfig<'a, X: Tool> {
    pub tool: X,
    pub driver_options: Options<DriverOptions, DriverOptionalArgs>,
    pub options: Options<X::RequiredArgs<'a>, X::OptionalArgs>,
}

pub struct DriverEnv<'a, X: Tool, R: Diagnostics<X>> {
    report_observer: DiagnosticsObserver<'a, X, R>,
}

pub struct DriverOptions {
    pub foo: (),
}

struct SimpleDiagnostics<X: Tool> {
    warnings: Vec<X::Warning>,
    errors: Vec<X::Error>,
}

impl<X: Tool> SimpleDiagnostics<X> {
    pub fn new() -> Self {
        Self {
            warnings: vec![],
            errors: vec![],
        }
    }
}

impl<X: Tool> Diagnostics<X> for SimpleDiagnostics<X> {
    fn error(&mut self, e: X::Error) {
        self.errors.push(e);
    }
    fn warning(&mut self, w: X::Warning) {
        self.warnings.push(w);
    }
    fn no_more_data(&mut self) {}
}

struct DiagnosticsObserver<'a, X, R> {
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
    pub fn error(&mut self, e: X::Error) -> Result<(), ConcreteDriverError> {
        self.observed_error = true;
        self.report.error(e);
        Err(ConcreteDriverError::Failure)
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

pub struct DriverControl<'a, X, R>
where
    X: Tool,
    R: Diagnostics<X>,
{
    tool: X,
    report: &'a mut R,
}

#[derive(Default)]
pub struct DriverOptionalArgs {
    // For future use.
    #[doc(hidden)]
    _non_exhaustive: _unstable_api_::InternalDefault,
}

impl<'a, X: Tool> DriverConfig<'a, X> {
    pub fn run_driver<'b: 'a, R: Diagnostics<X>>(
        self,
        driver_ctl: DriverControl<'b, X, R>,
    ) -> X::Output<'a>
where {
        let driver_env = DriverEnv {
            report_observer: DiagnosticsObserver::new(self.tool, driver_ctl.report),
        };
        X::Output::build_with_driver_env(self, driver_env)
    }
}

#[derive(Debug)]
pub enum ConcreteDriverError {
    Failure,
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
    fn no_more_data(&mut self);
}
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
        type RequiredArgs<'a> = YaccConfig<'a>;
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

    struct YaccConfig<'a> {
        source: &'a str,
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
        fn grammar(&self) -> Result<YaccGrammar, ConcreteDriverError> {
            if self.validation_success {
                Ok(YaccGrammar)
            } else {
                Err(ConcreteDriverError::Failure)
            }
        }
        fn ast(&self) -> &GrammarAST {
            &self.ast
        }
    }

    impl<'a> BuildWithDriverEnv<'a, Yacc> for GrammarASTWithValidationCertificate
    where
        Self: 'a,
    {
        fn build_with_driver_env<R: Diagnostics<Yacc>>(
            config: DriverConfig<'a, Yacc>,
            mut ctl: DriverEnv<Yacc, R>,
        ) -> GrammarASTWithValidationCertificate {
            #![allow(clippy::unit_cmp)]
            if config.options.required.source == "invalid sources" {
                ctl.report_observer
                    .non_fatal_error(YaccGrammarError::Testing(vec![]));
            }
            println!(
                "{}{}{:?}",
                config.driver_options.required.foo == (),
                config.options.required.source,
                config.options.required.yacc_kind,
            );
            // now at some time in the future.
            GrammarASTWithValidationCertificate {
                ast: GrammarAST,
                validation_success: !ctl.report_observer.observed_error(),
            }
        }
    }

    #[test]
    fn it_works() -> Result<(), ConcreteDriverError> {
        let mut report = SimpleDiagnostics::new();
        let driver_ctl = DriverControl {
            tool: Yacc,
            report: &mut report,
        };
        // Just pass in `Yacc` to avoid DriverConfig::<Yacc>`.
        let driver = DriverConfig {
            tool: Yacc,
            driver_options: (DriverOptions { foo: () }, Default::default()).into(),
            options: (
                YaccConfig {
                    source: "",
                    yacc_kind: YaccKind::Grmtools,
                },
                Default::default(),
            )
                .into(),
        }
        .run_driver(driver_ctl);
        let _ast = driver.ast();

        let _grm = driver.grammar()?;
        Ok(())
    }

    #[should_panic]
    #[test]
    fn it_fails() {
        let mut report = SimpleDiagnostics::new();
        let driver_ctl = DriverControl {
            tool: Yacc,
            report: &mut report,
        };
        // Just pass in `Yacc` to avoid DriverConfig::<Yacc>`.
        let driver = DriverConfig {
            tool: Yacc,
            driver_options: (DriverOptions { foo: () }, Default::default()).into(),
            options: (
                YaccConfig {
                    source: "invalid sources",
                    yacc_kind: YaccKind::Grmtools,
                },
                Default::default(),
            )
                .into(),
        }
        .run_driver(driver_ctl);
        let _ast = driver.ast();
        let _grm = driver.grammar().unwrap();
    }
}

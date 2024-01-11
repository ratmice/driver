use crate::default_impls::DefaultDriverEnv;
use crate::default_impls::SimpleDiagnostics;
use dir_view::DirView;
use std::{collections::HashMap, fmt, path};
#[cfg(test)]
mod tests {
    #![allow(dead_code)]
    use super::*;
    use crate::*;
    use std::error::Error;

    #[derive(Copy, Clone)]
    struct Yacc;

    enum YaccSourceKind {
        YaccSourceInput,
        YaccRustSourceOutput,
    }

    impl Tool for Yacc {
        type Error = YaccGrammarError;
        type Warning = YaccGrammarWarning;
        type Output = GrammarASTWithValidationCertificate;
        type SourceKind = YaccSourceKind;
    }

    impl Args for Yacc {
        type OptionalArgs = YaccGrammarOptArgs;
        type RequiredArgs = YaccArgs;
    }

    fn cwd_dir_view() -> Result<dir_view::DirView, DriverError> {
        Ok(DirView::open_ambient_dir(
            std::env::current_dir()?,
            dir_view::ViewKind::Readonly,
            cap_std::ambient_authority(),
        )?)
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

    struct YaccArgs {
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
            match &self.kind {
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

    struct YaccGrammarWarning {
        source_id: SourceId,
        kind: YaccGrammarWarningKind,
    }
    impl SourceArtifact for YaccGrammarWarning {
        fn source_id(&self) -> SourceId {
            self.source_id
        }
    }
    enum YaccGrammarWarningKind {
        Testing(Vec<Span>),
    }

    impl Spanned for YaccGrammarWarning {
        fn spans(&self) -> &[Span] {
            match &self.kind {
                YaccGrammarWarningKind::Testing(x) => x,
            }
        }
        fn spanskind(&self) -> SpansKind {
            match self.kind {
                YaccGrammarWarningKind::Testing(_) => SpansKind::DuplicationError,
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

    impl ToolInit<Yacc> for GrammarASTWithValidationCertificate {
        fn tool_init<R: Diagnostics<Yacc>>(
            options: Params<Yacc>,
            tool_env: &mut ToolInitEnv<Yacc, R>,
        ) -> GrammarASTWithValidationCertificate {
            let source_id = tool_env.session.loaded_source_ids().first();
            if let Some(source_id) = source_id.copied() {
                if let Some(path) = tool_env.source_cache.path_for_id(source_id) {
                    if path == path::PathBuf::from("Cargo.toml") {
                        tool_env.emitter.emit_non_fatal_error(YaccGrammarError {
                            source_id,
                            kind: YaccGrammarErrorKind::Testing(vec![]),
                        });
                    }
                }
            }

            println!("{:?}", options.required.yacc_kind,);
            // now at some time in the future.
            GrammarASTWithValidationCertificate {
                ast: GrammarAST,
                validation_success: !tool_env.emitter.observed_error(),
            }
        }
    }

    #[test]
    fn it_works() {
        let mut diagnostics: SimpleDiagnostics<Yacc> = SimpleDiagnostics::default();
        let mut source_cache = HashMap::new();
        {
            let driver_env = DefaultDriverEnv {
                source_cache: &mut source_cache,
                diagnostics: &mut diagnostics,
                tool: Yacc,
            };
            // Just pass in `Yacc` to avoid Driver::<Yacc>`.
            let driver = Driver {
                driver: DefaultDriver,
                tool: Yacc,
                driver_args: (
                    DefaultDriverArgs {},
                    DefaultDriverOptionalArgs {
                        read_source: Some(("Cargo.lock".into(), cwd_dir_view().unwrap())),
                        ..Default::default()
                    },
                ),
                tool_args: (
                    YaccArgs {
                        yacc_kind: YaccKind::Grmtools,
                    },
                    Default::default(),
                ),
            }
            .driver_init(driver_env)
            .unwrap();
            let _session = &driver.session;
            let _ast = driver.output.ast();
            let _grm = driver.output.grammar().unwrap();
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
            let driver_env = DefaultDriverEnv {
                tool: Yacc,
                diagnostics: &mut diagnostics,
                source_cache: &mut source_cache,
            };
            // Just pass in `Yacc` to avoid Driver::<Yacc>`.
            let driver = Driver {
                tool: Yacc,
                driver: DefaultDriver,
                driver_args: (
                    DefaultDriverArgs {},
                    DefaultDriverOptionalArgs {
                        read_source: Some(("Cargo.toml".into(), cwd_dir_view().unwrap())),
                        ..Default::default()
                    },
                ),
                tool_args: (
                    YaccArgs {
                        yacc_kind: YaccKind::Grmtools,
                    },
                    Default::default(),
                ),
            }
            .driver_init(driver_env)
            .unwrap();
            let _ast = driver.output.ast();
            let _grm = driver.output.grammar().unwrap();
            #[allow(clippy::drop_non_drop)]
            drop(driver);
        }
    }

    #[test]
    fn unit_driver() {
        impl _unstable_api_::InternalTrait for () {}
        impl DriverSelector for () {}
        impl Args for () {
            type RequiredArgs = ();
            type OptionalArgs = bool;
        }

        impl<X: Tool> DriverTypes<X> for () {
            type Output<T> = () where T: Tool;
            type DriverEnv<'a, T, D> = () where T: Tool + 'a, D: Diagnostics<T> + 'a;
        }
        // These fields should perhaps be combined into something?
        let mut diagnostics = SimpleDiagnostics::default();
        let mut source_cache = HashMap::new();
        // Note that the args here differ from those of the default `driver_init`.
        // Not for any reason, just to highlight that there can be multiple impls
        // for this struct due to the default type instace. The other being:
        //
        // impl<X, ...> Driver<... , DefaultDriver>.
        //
        // With this we can change both the `driver_init` implementation,
        // and the `DriverArgs`, or just change `driver_init`.
        impl<X> Driver<X, ()>
        where
            X: Tool,
            (<() as Args>::RequiredArgs, <() as Args>::OptionalArgs): Into<Params<()>>,
            (X::RequiredArgs, X::OptionalArgs): Into<Params<X>>,
        {
            pub fn driver_init<D: Diagnostics<X>>(
                self,
                source_cache: &mut HashMap<SourceId, (path::PathBuf, String)>,
                diagnostics: &mut D,
                _extra_param: (),
            ) -> Result<X::Output, DriverError> {
                let _driver_args: Params<()> = self.driver_args.into();
                let emitter = DiagnosticsEmitter::new(self.tool, diagnostics);
                let mut source_cache = SourceCache::new();
                let session: Session<X::SourceKind> = Session {
                    source_ids_from_driver: vec![],
                    source_ids_from_tool: vec![],
                    source_kinds: HashMap::new(),
                };
                let mut tool_env = ToolInitEnv {
                    source_cache: &mut source_cache,
                    emitter,
                    session,
                };
                Ok(X::Output::tool_init(self.tool_args.into(), &mut tool_env))
            }
        }

        {
            // Just pass in `Yacc` to avoid Driver::<Yacc>`.
            let driver = Driver {
                tool: Yacc,
                driver: (),
                driver_args: ((), true),
                tool_args: (
                    YaccArgs {
                        yacc_kind: YaccKind::Grmtools,
                    },
                    Default::default(),
                ),
            }
            .driver_init(&mut source_cache, &mut diagnostics, ())
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
        fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result {
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
    impl SourceArtifact for NeverWarnings {
        fn source_id(&self) -> SourceId {
            unreachable!()
        }
    }
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
            unreachable!()
        }

        fn spanskind(&self) -> SpansKind {
            unreachable!()
        }
    }

    impl fmt::Display for LexError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "Lex error test")
        }
    }

    struct LexOutput {}

    impl ToolInit<Lex> for LexOutput {
        fn tool_init<'diag, 'src, D: Diagnostics<Lex>>(
            _config: Params<Lex>,
            _tool_env: &mut ToolInitEnv<Lex, D>,
        ) -> LexOutput {
            LexOutput {}
        }
    }

    enum LexSourceKind {
        LexSourceInput,
        LexRustSourceOutput,
    }

    impl Tool for Lex {
        type Error = LexError;
        type Warning = NeverWarnings;
        // FIXME look at what lex returns.
        type Output = LexOutput;
        type SourceKind = LexSourceKind;
    }

    impl Args for Lex {
        // FIXME look at lex args.
        type OptionalArgs = ();
        type RequiredArgs = ();
    }

    #[test]
    fn lex_driver() {
        // These fields should perhaps be combined into something?
        let mut source_cache = HashMap::new();
        {
            let mut diagnostics = SimpleDiagnostics::default();
            // Just pass in `Yacc` to avoid Driver::<Yacc>`.
            let driver = Driver {
                tool: Lex,
                driver: (),
                driver_args: ((), true),
                tool_args: ((), ()),
            }
            .driver_init(&mut source_cache, &mut diagnostics, ())
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
            // Just pass in `Yacc` to avoid Driver::<Yacc>`.
            let driver = Driver {
                tool: Lex,
                driver: DefaultDriver,
                driver_args: (
                    DefaultDriverArgs {},
                    DefaultDriverOptionalArgs {
                        read_source: Some(("Cargo.lock".into(), cwd_dir_view().unwrap())),
                        ..Default::default()
                    },
                ),
                tool_args: ((), ()),
            }
            .driver_init(DefaultDriverEnv {
                source_cache: &mut source_cache,
                diagnostics: &mut diagnostics,
                tool: Lex,
            })
            .unwrap();
            #[allow(clippy::drop_non_drop)]
            drop(driver);
        }

        {
            let mut diagnostics = SimpleDiagnostics::default();
            // Just pass in `Yacc` to avoid Driver::<Yacc>`.
            let driver = Driver {
                tool: Yacc,
                driver: DefaultDriver,
                driver_args: (
                    DefaultDriverArgs {},
                    DefaultDriverOptionalArgs {
                        read_source: Some(("Cargo.lock".into(), cwd_dir_view().unwrap())),
                        ..Default::default()
                    },
                ),
                tool_args: (
                    YaccArgs {
                        yacc_kind: YaccKind::Grmtools,
                    },
                    Default::default(),
                ),
            }
            .driver_init(DefaultDriverEnv {
                source_cache: &mut source_cache,
                diagnostics: &mut diagnostics,
                tool: Yacc,
            })
            .unwrap();
            let _ast = driver.output.ast();
            let _grm = driver.output.grammar().unwrap();
            #[allow(clippy::drop_non_drop)]
            drop(driver);
        }
        assert_eq!(source_cache.len(), 2);
    }
}

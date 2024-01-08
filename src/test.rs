#[cfg(test)]
mod tests {
    #![allow(dead_code)]
    use crate::*;
    use std::error::Error;

    #[derive(Copy, Clone)]
    struct Yacc;

    impl Tool for Yacc {
        type Error = YaccGrammarError;
        type Warning = YaccGrammarWarning;
        type Output = GrammarASTWithValidationCertificate;
    }
    impl Args for Yacc {
        type OptionalArgs = YaccGrammarOptArgs;
        type RequiredArgs<'x> = YaccArgs;
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
            options: Params<<Yacc as Args>::RequiredArgs<'_>, <Yacc as Args>::OptionalArgs>,
            source_cache: SourceCache<'_>,
            mut emitter: DiagnosticsEmitter<Yacc, R>,
            session: &mut Session,
        ) -> GrammarASTWithValidationCertificate {
            let src_id = session.loaded_source_ids().next();
            if let Some(src_id) = src_id {
                if let Some(path) = source_cache.path_for_id(src_id) {
                    if path == path::PathBuf::from("Cargo.toml") {
                        emitter.emit_non_fatal_error(YaccGrammarError {
                            source_id: src_id,
                            kind: YaccGrammarErrorKind::Testing(vec![]),
                        });
                    }
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
            // Just pass in `Yacc` to avoid Driver::<Yacc>`.
            let driver = Driver {
                driver: DefaultDriver,
                tool: Yacc,
                driver_args: (
                    DriverArgs {},
                    DriverOptionalArgs {
                        source_path: Some("Cargo.lock".into()),
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
            .driver_init(&mut diagnostics, &mut source_cache)
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
            // Just pass in `Yacc` to avoid Driver::<Yacc>`.
            let driver = Driver {
                tool: Yacc,
                driver: DefaultDriver,
                driver_args: (
                    DriverArgs {},
                    DriverOptionalArgs {
                        source_path: Some("Cargo.toml".into()),
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
            .driver_init(&mut diagnostics, &mut source_cache)
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
            type RequiredArgs<'x> = ();
            type OptionalArgs = bool;
        }

        impl<X: Tool> DriverTypes<X> for () {
            type Output<T> = () where T: Tool;
        }
        // These fields should perhaps be combined into something?
        let mut diagnostics = SimpleDiagnostics::default();
        let mut source_cache = HashMap::new();
        // Note that the args here differ from those of the default `driver_init`.
        // Not for any reason, just to highlight that there can be multiple impls
        // for this struct due to the default type instace. The other being:
        //
        // impl<X: Tool> Driver<'_, X, DefaultDriver>.
        //
        // So in addition to changing the `DriverArgsSelection`,
        // they can differ in their initialization as well.
        impl<X, DArgs, TArgs> Driver<X, DArgs, TArgs, ()>
        where
            X: Tool,
            DArgs: Into<Params<(), bool>>,
            TArgs: for<'x> Into<Params<X::RequiredArgs<'x>, X::OptionalArgs>>,
        {
            pub fn driver_init<D: Diagnostics<X>>(
                self,
                source_cache: &mut HashMap<SourceId, (path::PathBuf, String)>,
                diagnostics: &mut D,
                _extra_param: (),
            ) -> Result<X::Output, DriverError> {
                let _driver_args: Params<(), bool> = self.driver_args.into();
                let emitter = DiagnosticsEmitter::new(self.tool, diagnostics);
                let source_cache = SourceCache { source_cache };
                let mut session = Session { source_ids_from_driver: vec![], source_ids_from_tool: vec![] };
                Ok(X::Output::tool_init(
                    self.tool_args.into(),
                    source_cache,
                    emitter,
                    &mut session,
                ))
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
            _config: Params<(), ()>,
            _source_cache: SourceCache<'_>,
            _emitter: DiagnosticsEmitter<Lex, D>,
            _session: &mut Session,
        ) -> LexOutput {
            LexOutput {}
        }
    }

    impl Tool for Lex {
        type Error = LexError;
        type Warning = NeverWarnings;
        // FIXME look at what lex returns.
        type Output = LexOutput;
    }
    impl Args for Lex {
        // FIXME look at lex args.
        type OptionalArgs = ();
        type RequiredArgs<'x> = ();
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
                    DriverArgs {},
                    DriverOptionalArgs {
                        source_path: Some("Cargo.lock".into()),
                        ..Default::default()
                    },
                ),
                tool_args: ((), ()),
            }
            .driver_init(&mut diagnostics, &mut source_cache)
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
                    DriverArgs {},
                    DriverOptionalArgs {
                        source_path: Some("Cargo.lock".into()),
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
            .driver_init(&mut diagnostics, &mut source_cache)
            .unwrap();
            let _ast = driver.output.ast();
            let _grm = driver.output.grammar().unwrap();
            #[allow(clippy::drop_non_drop)]
            drop(driver);
        }
    }
}

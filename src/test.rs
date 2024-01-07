#[cfg(test)]
mod tests {
    #![allow(dead_code)]
    use super::*;
    use crate::*;
    use std::error::Error;

    #[derive(Copy, Clone)]
    struct Yacc;

    impl Tool for Yacc {
        type Error = YaccGrammarError;
        type Warning = YaccGrammarWarning;
        type OptionalArgs = YaccGrammarOptArgs;
        type RequiredArgs<'a> = YaccArgs;
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
            source_cache: SourceCache<'_>,
            mut emitter: DiagnosticsEmitter<Yacc, R>,
            session: Session,
        ) -> GrammarASTWithValidationCertificate {
            let src_id = session.source_ids().next();
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
                driver_options: (
                    DriverArgs {},
                    DriverOptionalArgs {
                        source_path: Some("Cargo.lock".into()),
                        ..Default::default()
                    },
                )
                    .into(),
                options: (
                    YaccArgs {
                        yacc_kind: YaccKind::Grmtools,
                    },
                    Default::default(),
                )
                    .into(),
            }
            .driver_init(&mut diagnostics, &mut source_cache)
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
            // Just pass in `Yacc` to avoid Driver::<Yacc>`.
            let driver = Driver {
                tool: Yacc,
                driver: DefaultDriver,
                driver_options: (
                    DriverArgs {},
                    DriverOptionalArgs {
                        source_path: Some("Cargo.toml".into()),
                        ..Default::default()
                    },
                )
                    .into(),
                options: (
                    YaccArgs {
                        yacc_kind: YaccKind::Grmtools,
                    },
                    Default::default(),
                )
                    .into(),
            }
            .driver_init(&mut diagnostics, &mut source_cache)
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
        impl DriverArgsSelection for () {
            type RequiredArgs = ();
            type OptionalArgs = bool;
        }
        // These fields should perhaps be combined into something?
        let mut diagnostics = SimpleDiagnostics::default();
        let mut source_cache = HashMap::new();
        impl<X: Tool> Driver<'_, X, ()> {
            pub fn driver_init<D: Diagnostics<X>>(
                self,
                source_cache: &mut HashMap<SourceId, (path::PathBuf, String)>,
                diagnostics: &mut D,
            ) -> Result<X::Output, DriverError> {
                let emitter = DiagnosticsEmitter::new(self.tool, diagnostics);
                let source_cache = SourceCache { source_cache };
                let session = Session { source_ids: vec![] };
                Ok(X::Output::tool_init(
                    self.options,
                    source_cache,
                    emitter,
                    session,
                ))
            }
        }

        {
            // Just pass in `Yacc` to avoid Driver::<Yacc>`.
            let driver = Driver {
                tool: Yacc,
                driver: (),
                driver_options: ((), true).into(),
                options: (
                    YaccArgs {
                        yacc_kind: YaccKind::Grmtools,
                    },
                    Default::default(),
                )
                    .into(),
            }
            .driver_init(&mut source_cache, &mut diagnostics)
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

    impl<'args> ToolInit<'args, Lex> for LexOutput {
        fn tool_init<'diag, 'src, D: Diagnostics<Lex>>(
            _config: Options<(), ()>,
            _source_cache: SourceCache<'_>,
            _emitter: DiagnosticsEmitter<Lex, D>,
            _session: Session,
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
            // Just pass in `Yacc` to avoid Driver::<Yacc>`.
            let driver = Driver {
                tool: Lex,
                driver: (),
                driver_options: ((), true).into(),
                options: ((), ()).into(),
            }
            .driver_init(&mut source_cache, &mut diagnostics)
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
                driver_options: (
                    DriverArgs {},
                    DriverOptionalArgs {
                        source_path: Some("Cargo.lock".into()),
                        ..Default::default()
                    },
                )
                    .into(),
                options: ((), ()).into(),
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
                driver_options: (
                    DriverArgs {},
                    DriverOptionalArgs {
                        source_path: Some("Cargo.lock".into()),
                        ..Default::default()
                    },
                )
                    .into(),
                options: (
                    YaccArgs {
                        yacc_kind: YaccKind::Grmtools,
                    },
                    Default::default(),
                )
                    .into(),
            }
            .driver_init(&mut diagnostics, &mut source_cache)
            .unwrap();
            let _ast = driver.ast();
            let _grm = driver.grammar().unwrap();
            #[allow(clippy::drop_non_drop)]
            drop(driver);
        }
    }
}

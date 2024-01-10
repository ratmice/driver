use crate::{
    Args, Params, _unstable_api_,
    default_impls::DefaultDriver,
    diagnostics::{Diagnostics, DiagnosticsEmitter},
    source::{Session, SourceCache, SourceId},
    tool::{Tool, ToolInit},
    NEXT_SOURCE_ID,
};
use std::{collections::HashMap, io, io::Read as _, path, sync::atomic::Ordering};

/// A marker trait for selecting between drivers.
pub trait DriverSelector: _unstable_api_::InternalTrait {}

/// Used to configure and initialize a driver for a tool.
///
/// Contains the tool to run which must implement `Tool`,
/// `driver_options` for itself, and `options` for the tool.
///
/// Fields are public so that they are constructable by the caller.
pub struct Driver<X, D = DefaultDriver>
where
    X: Tool,
    D: DriverSelector + DriverTypes<X>,
{
    /// This is mostly here to guide inference, and generally would be a unitary type.
    pub tool: X,
    /// This is here to guide inference, and allow for future expansion, in the case
    /// that we require a different driver implementation, or changes to driver_args.
    pub driver: D,
    /// Options that get handled by the driver.
    pub driver_args: (D::RequiredArgs, D::OptionalArgs),
    // Options specific to a `Tool`.
    pub tool_args: (X::RequiredArgs, X::OptionalArgs),
}

impl<X> Driver<X, DefaultDriver>
where
    X: Tool,
    (
        <DefaultDriver as Args>::RequiredArgs,
        <DefaultDriver as Args>::OptionalArgs,
    ): Into<Params<DefaultDriver>>,
    (X::RequiredArgs, X::OptionalArgs): Into<Params<X>>,
    DefaultDriver: DriverTypes<X>,
    // This bound is implied, but perhaps informative.
    // DefaultDriver:
    //     DriverSelector + Args<RequiredArgs = DriverArgs, OptionalArgs = DriverOptionalArgs>,
{
    ///
    /// 1. Populates a `source_cache`
    /// 2. Constructes a `Diagnostics emitter`.
    /// 3. Passes everything above to the tool's implementation of `tool_init`.
    pub fn driver_init<D: Diagnostics<X>>(
        self,
        diagnostics: &mut D,
        source_cache: &mut HashMap<SourceId, (path::PathBuf, String)>,
    ) -> Result<DriverOutput<X>, DriverError> {
        let mut driver_options: Params<DefaultDriver> = self.driver_args.into();
        let mut source_ids_from_driver = Vec::new();
        let mut add_to_src_cache = |source_path, source| {
            let source_id = SourceId(NEXT_SOURCE_ID.fetch_add(1, Ordering::SeqCst));
            source_cache.insert(source_id, (source_path, source));
            source_ids_from_driver.push(source_id);
        };
        if let Some((source_path, source)) = driver_options.optional.named_string.take() {
            add_to_src_cache(source_path, source);
        }
        if let Some((source_path, dir)) = driver_options.optional.read_source {
            let mut file = dir.open(&source_path)?;
            let mut source = String::new();

            file.read_to_string(&mut source)?;
            add_to_src_cache(source_path, source);
        }

        let source_cache = SourceCache { source_cache };

        let emitter = DiagnosticsEmitter::new(self.tool, diagnostics);
        let mut session = Session {
            source_ids_from_driver,
            source_ids_from_tool: vec![],
            source_kinds: HashMap::new(),
        };
        let output =
            X::Output::tool_init(self.tool_args.into(), source_cache, emitter, &mut session);
        Ok(DriverOutput { output, session })
    }
}
/// Errors occurred by the driver.
#[derive(thiserror::Error, Debug)]
pub enum DriverError {
    #[error("Io error {0} ")]
    Io(#[from] io::Error),
}

pub trait DriverTypes<X: Tool>: Args {
    type Output<T>
    where
        T: Tool;
}

/// The return value of `driver_init`.
///
/// Contains a `Session`, and `Tool::Output`
pub struct DriverOutput<X: Tool> {
    pub session: Session<X::SourceKind>,
    pub output: X::Output,
}

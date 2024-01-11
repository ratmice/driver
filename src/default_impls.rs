use crate::{
    _unstable_api_,
    driver::{DriverOutput, DriverSelector, DriverTypes},
    source::SourceId,
    tool::Tool,
    Args,
};
use dir_view::DirView;
use std::{collections::HashMap, path};

/// A [DriverSelector] for the default driver.
pub struct DefaultDriver;

impl _unstable_api_::InternalTrait for DefaultDriver {}
/// Required options for a driver.
pub struct DriverArgs {
    // There are not currently any required options for the tool.
}

#[derive(Default)]
/// Optional arguments for a driver.
pub struct DriverOptionalArgs {
    /// Gives an arbitrary string a name.
    pub named_string: Option<(std::path::PathBuf, String)>,
    /// Takes an arbitrary `DirView`
    pub read_source: Option<(std::path::PathBuf, DirView)>,
    #[doc(hidden)]
    pub _non_exhaustive: _unstable_api_::InternalDefault,
}

impl DriverSelector for DefaultDriver {}
impl<X: Tool> DriverTypes<X> for DefaultDriver {
    type Output<T> = DriverOutput<T> where T: Tool;
    type DriverEnv<'a, T, D> = DefaultDriverEnv<'a, T, D> where D: Diagnostics<T> + 'a, T: Tool + 'a;
}

pub struct DefaultDriverEnv<'a, X, D>
where
    X: Tool,
    D: Diagnostics<X>,
{
    pub diagnostics: &'a mut D,
    pub source_cache: &'a mut HashMap<SourceId, (path::PathBuf, String)>,
    pub tool: X,
}

impl Args for DefaultDriver {
    type RequiredArgs = DriverArgs;
    type OptionalArgs = DriverOptionalArgs;
}

/// A Simple implementation of a `Diagnostics` trait.
/// It uses a vector as a backing store.
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

use crate::diagnostics::Diagnostics;

impl<X: Tool> Diagnostics<X> for SimpleDiagnostics<X> {
    fn emit_error(&mut self, e: X::Error) {
        self.errors.push(e);
    }
    fn emit_warning(&mut self, w: X::Warning) {
        self.warnings.push(w);
    }
    fn no_more_data(&mut self) {
        println!("no_more_data");
    }
}

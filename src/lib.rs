pub use dir_view::DirView;
/// I don't know how I feel about this, but it works.
//use std::io::Read as _;
use std::sync::atomic::AtomicUsize;
//use std::sync::atomic::Ordering;
use std::fmt;

mod default_impls;
mod diagnostics;
mod driver;
mod source;
mod tool;

pub use crate::diagnostics::{Diagnostics, DiagnosticsEmitter};
pub use crate::source::{Session, SourceArtifact, SourceCache, SourceId};
pub use default_impls::{DefaultDriver, DriverArgs, DriverOptionalArgs};
pub use driver::{Driver, DriverError, DriverOutput, DriverSelector, DriverTypes};
pub use tool::{Tool, ToolError, ToolInit};

#[cfg(test)]
mod test;

mod _unstable_api_ {
    /// A sealed trait.
    pub trait InternalTrait {}

    #[derive(Default)]
    pub struct InternalDefault;
}

pub trait Args {
    /// A type for arguments that must be given.
    type RequiredArgs;
    /// A type for arguments which derive `Default`
    type OptionalArgs: Default;
}

pub(crate) static NEXT_SOURCE_ID: AtomicUsize = AtomicUsize::new(0);

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

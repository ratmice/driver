pub use dir_view;
/// I don't know how I feel about this, but it works.
use std::sync::atomic::AtomicUsize;

mod default_impls;
mod diagnostics;
mod driver;
mod source;
mod tool;

pub use {
    crate::default_impls::*, crate::diagnostics::*, crate::driver::*, crate::source::*,
    crate::tool::*,
};

#[cfg(test)]
mod test;

mod _unstable_api_ {
    /// A sealed trait.
    pub trait InternalTrait {}

    #[derive(Default)]
    pub struct InternalDefault;
}

/// Type bounds for `Driver`/`Tool` required and optional arguments.
pub trait Args {
    /// A type for arguments that must be given.
    type RequiredArgs;
    /// A type for arguments which derive `Default`
    type OptionalArgs: Default;
}

pub(crate) static NEXT_SOURCE_ID: AtomicUsize = AtomicUsize::new(0);

/// A `Span` records what portion of the user's input something (e.g. a lexeme or production)
/// references (i.e. the `Span` doesn't hold a reference / copy of the actual input).
#[derive(Debug)]
#[allow(dead_code)]
pub struct Span {
    start: usize,
    end: usize,
}

/// Indicates how to interpret the spans of an error.
pub enum SpansKind {
    /// The first span is the first occurrence, and a span for each subsequent occurrence.
    DuplicationError,
    /// Contains a single span at the site of the error.
    Error,
}

/// Implemented for errors and warnings to provide access to their spans.
pub trait Spanned: std::fmt::Display {
    /// Returns the spans associated with the error, always containing at least 1 span.
    ///
    /// Refer to [SpansKind] via [spanskind](Self::spanskind)
    /// for the meaning and interpretation of spans and their ordering.
    fn spans(&self) -> &[Span];
    /// Returns the `SpansKind` associated with this error.
    fn spanskind(&self) -> SpansKind;
}

/// A pair of required and optional parameters.
pub struct Params<X: Args> {
    pub required: X::RequiredArgs,
    pub optional: X::OptionalArgs,
}

impl<X: Args> From<(X::RequiredArgs, X::OptionalArgs)> for Params<X> {
    fn from((required, optional): (X::RequiredArgs, X::OptionalArgs)) -> Self {
        Self { required, optional }
    }
}

use std::path;
use std::{collections::HashMap, sync::atomic::Ordering};

use crate::NEXT_SOURCE_ID;

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
/// Opaque Id for source strings
///
/// * A source string may have multiple SourceIDs.
/// * A SourceId refers uniquely to a single source string.
#[derive(Debug)]
pub struct SourceId(pub(crate) usize);

/// For obtaining a SourceId from an error.
pub trait SourceArtifact {
    fn source_id(&self) -> SourceId;
}

/// A cache for source text.
///
/// This is a read/write cache that maps [SourceIds](SourceId) to
/// a [Path](std::path::Path) and a [String] of source text. A file can have multiple
/// entries in the cache by having multiple `SourceId`s.
///
/// It can be used as a store for sources loaded from disk, or the output
/// of code generators, but does not perform or require any filesystem
/// operations which are handled by [Driver](crate::Driver).
///
/// An instance where a it is useful to have a file with multiple `SourceId`s
/// present in the source cache is when applying fixes based on error recovery.
///
/// Modifications to a `SourceCache` are tracked in a [Session].
pub struct SourceCache {
    pub(crate) cache: HashMap<SourceId, (std::path::PathBuf, String)>,
}

impl SourceCache {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }
    pub fn source_ids(&self) -> impl Iterator<Item = SourceId> + '_ {
        self.cache.keys().copied()
    }

    pub fn source_for_id(&self, src_id: SourceId) -> Option<&str> {
        self.cache.get(&src_id).map(|(_, src)| src.as_str())
    }

    pub fn path_for_id(&self, src_id: SourceId) -> Option<&path::Path> {
        self.cache.get(&src_id).map(|(path, _)| path.as_path())
    }

    /// This should allow us to populate the source cache with generated code.
    pub fn add_source<SourceKind>(
        &mut self,
        session: &mut Session<SourceKind>,
        path: path::PathBuf,
        src: String,
        kind: SourceKind,
    ) -> SourceId {
        let source_id = SourceId(NEXT_SOURCE_ID.fetch_add(1, Ordering::SeqCst));
        self.cache.insert(source_id, (path, src));
        session.add_source_id(source_id, kind);
        source_id
    }
}
/// A session tracks changes to a `SourceCache`.
///
/// While `source_cache`, and `diagnostics` are allowed to
/// persist across driver runs. `Session` is ephemeral.
///
/// Whenever `Driver` or a tool loads source text
/// into a `SourceCache`, they track their changes here.
pub struct Session<SourceKind> {
    pub(crate) source_ids_from_driver: Vec<SourceId>,
    pub(crate) source_ids_from_tool: Vec<SourceId>,
    pub(crate) source_kinds: HashMap<SourceId, SourceKind>,
}

impl<SourceKind> Session<SourceKind> {
    /// Any new source id's produced by the driver before running the tool.
    pub fn loaded_source_ids(&self) -> &[SourceId] {
        &self.source_ids_from_driver
    }
    /// Any new source id's produced by the tool through `SourceCache::add_source`.
    pub fn added_source_ids(&self) -> &[SourceId] {
        &self.source_ids_from_driver
    }
    pub(crate) fn add_source_id(&mut self, src_id: SourceId, kind: SourceKind) {
        self.source_ids_from_tool.push(src_id);
        self.source_kinds.insert(src_id, kind);
    }
}

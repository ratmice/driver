use std::path;
use std::{collections::HashMap, sync::atomic::Ordering};

use crate::NEXT_SOURCE_ID;

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
/// opaque ID for source strings:
///
/// * A source string may have multiple SourceIDs.
/// * A SourceID refers uniquely to a single source string.
#[derive(Debug)]
pub struct SourceId(pub(crate) usize);

pub trait SourceArtifact {
    fn source_id(&self) -> SourceId;
}

pub struct SourceCache<'a> {
    pub(crate) source_cache: &'a mut HashMap<SourceId, (std::path::PathBuf, String)>,
}

impl<'src> SourceCache<'src> {
    pub fn source_ids(&self) -> impl Iterator<Item = SourceId> + '_ {
        self.source_cache.iter().map(|(src_id, _)| *src_id)
    }

    pub fn source_for_id(&self, src_id: SourceId) -> Option<&str> {
        self.source_cache.get(&src_id).map(|(_, src)| src.as_str())
    }

    pub fn path_for_id(&self, src_id: SourceId) -> Option<&path::Path> {
        self.source_cache
            .get(&src_id)
            .map(|(path, _)| path.as_path())
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
        self.source_cache.insert(source_id, (path, src));
        session.add_source_id(source_id, kind);
        source_id
    }
}
pub struct Session<SourceKind> {
    pub(crate) source_ids_from_driver: Vec<SourceId>,
    pub(crate) source_ids_from_tool: Vec<SourceId>,
    pub(crate) source_kinds: HashMap<SourceId, SourceKind>,
}

/// A session is created during `driver_init`, and contains
/// `SourceId`s for the documents loaded during driver init.
///
/// While `source_cache`, and `diagnostics` are allowed to
/// persist across driver runs. `Session` is ephemeral.
///
/// This can be used to obtain the subset of the files asked to
/// be loaded from the `source_cache`.
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

//! Collaborative document layer.  In v1 this is a thin wrapper
//! around the vault's encrypted blob storage; the CRDT integration
//! is added in a follow-up.  The interface is stable so callers
//! can be written against it today.
//!
//! Each document has a `scope` (e.g. `"notes"`, `"records-index"`)
 //! and a `name` (a stable identifier within the scope).  The body
//! is treated as an opaque byte blob that is encrypted with the
//! vault key before being persisted, and CRDT-merged on read.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Result;

/// A document handle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DocId {
    /// Scope, e.g. "notes".
    pub scope: String,
    /// Stable name within the scope.
    pub name: String,
}

impl DocId {
    /// Construct a new document id.
    pub fn new(scope: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            scope: scope.into(),
            name: name.into(),
        }
    }
}

/// A document version.  UUID v4 is used to keep the format
/// forward-compatible.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocVersion {
    /// UUID v4 identifying this version.
    pub version: Uuid,
    /// Parent versions this version was merged from.  Empty for
    /// the first version.
    pub parents: Vec<Uuid>,
    /// Unix-epoch millis.
    pub ts_millis: i64,
    /// Username that produced this version.
    pub author: String,
    /// Opaque encrypted body.
    pub ciphertext: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_id_constructs() {
        let id = DocId::new("notes", "shopping");
        assert_eq!(id.scope, "notes");
        assert_eq!(id.name, "shopping");
    }
}

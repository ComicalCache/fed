use std::collections::HashMap;

use fed_core::DocumentId;

use crate::type_map::TypeMap;

pub type DocumentStore = HashMap<DocumentId, TypeMap>;

pub mod types {}

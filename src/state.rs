mod document_store;
mod view_store;

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, RwLock,
        atomic::{AtomicU64, Ordering},
    },
};

pub use document_store::{DocumentStore, types as DocumentStoreTypes};
use fed_core::{CoreCommandSender, DocumentId, messages::CoreCommandError};
use view_store::types::Cursors;
pub use view_store::{ViewId, ViewStore, types as ViewStoreTypes};

use crate::{
    render::{WindowId, Workspace},
    state::view_store::types::Mode,
    type_map::TypeMap,
    types::Pos,
};

#[derive(Default, Clone)]
pub struct State {
    pub workspace: Arc<RwLock<Workspace>>,
    pub document_store: Arc<RwLock<DocumentStore>>,
    pub view_store: Arc<RwLock<ViewStore>>,

    pub window_view_map: Arc<RwLock<HashMap<WindowId, ViewId>>>,
}

pub async fn create_document(
    state: &State, core_tx: CoreCommandSender, path: Option<PathBuf>,
) -> Result<DocumentId, CoreCommandError> {
    let id = core_tx.create(path).await?;

    state.document_store.write().unwrap().insert(id, TypeMap::new());

    Ok(id)
}

pub fn create_view(state: &State, doc: DocumentId) -> ViewId {
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    let id = ViewId(NEXT_ID.fetch_add(1, Ordering::Relaxed));

    let mut map = TypeMap::new();
    map.insert(doc);
    map.insert(Cursors { list: vec![Pos::new(0, 0)] });
    map.insert(Mode::Normal);

    state.view_store.write().unwrap().insert(id, map);

    id
}

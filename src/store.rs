use std::sync::{Arc, RwLock};
use tokio::sync::Notify;

use crate::{
    clients::{AsyncOperationRequest, AsyncOperationResult, TableSchema},
    dispatcher::Action,
    ui::events::AppEvent,
};

pub enum Loadable<T> {
    NotLoaded,
    Loading,
    Loaded(T),
}

pub struct SelectableList<T> {
    items: Vec<T>,
    selected: Option<usize>,
}

impl<T> SelectableList<T> {
    pub fn new(items: Vec<T>) -> Self {
        let selected = if items.is_empty() { None } else { Some(0) };

        Self { items, selected }
    }

    pub fn items(&self) -> &[T] {
        &self.items
    }

    pub fn selected(&self) -> Option<&T> {
        self.selected.and_then(|idx| self.items.get(idx))
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    pub fn select_next(&mut self) {
        self.selected = match self.selected {
            Some(idx) if idx + 1 < self.items.len() => Some(idx + 1),
            selected => selected,
        };
    }

    pub fn select_previous(&mut self) {
        self.selected = match self.selected {
            Some(idx) if idx > 0 => Some(idx - 1),
            selected => selected,
        };
    }
}

pub struct Store {
    pub table_list: Loadable<SelectableList<TableSchema>>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            table_list: Loadable::NotLoaded,
        }
    }

    pub fn handle_action(&mut self, action: Action) {
        match action {
            Action::AsyncOperationRequest(request) => match request {
                AsyncOperationRequest::ListTables => self.table_list = Loadable::Loading,
            },
            Action::AsyncOperationResult(result) => match result {
                AsyncOperationResult::ListTables(tables) => {
                    self.table_list = Loadable::Loaded(SelectableList::new(tables));
                }
            },
            Action::SelectNextTable => {
                if let Loadable::Loaded(list) = &mut self.table_list {
                    list.select_next();
                }
            }
            Action::SelectPreviousTable => {
                if let Loadable::Loaded(list) = &mut self.table_list {
                    list.select_previous();
                }
            }
        }
    }
}

struct Inner {
    store: RwLock<Store>,
    changed: Notify,
}

/// A cheaply-cloneable handle to a [`Store`] shared across components.
#[derive(Clone)]
pub struct SharedStore(Arc<Inner>);

impl SharedStore {
    pub fn new() -> Self {
        Self(Arc::new(Inner {
            store: RwLock::new(Store::new()),
            changed: Notify::new(),
        }))
    }

    pub fn handle_action(&self, action: Action) {
        self.0.store.write().unwrap().handle_action(action);
        self.0.changed.notify_one();
    }

    pub fn with<R>(&self, f: impl FnOnce(&Store) -> R) -> R {
        f(&self.0.store.read().unwrap())
    }

    /// Resolves the next time [`SharedStore::handle_action`] mutates the store.
    async fn changed(&self) {
        self.0.changed.notified().await;
    }

    pub async fn next_event(&self) -> AppEvent {
        tokio::select! {
            _ = self.changed() => AppEvent::StoreChanged,
        }
    }
}

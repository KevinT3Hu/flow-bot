use std::sync::Arc;

use dashmap::DashMap;
use turso::Database;

use crate::{
    base::{
        context::BotContext,
        extract::{FromEvent, State},
    },
    error::FlowError,
    event::BotEvent,
};

pub(crate) struct TursoDispatcher {
    database_directory: String,
    databases: DashMap<String, Database>,
}

impl Default for TursoDispatcher {
    fn default() -> Self {
        Self {
            database_directory: "./turso_databases".to_string(),
            databases: DashMap::new(),
        }
    }
}

impl TursoDispatcher {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) async fn get_database(&self, key: &str) -> Result<Database, FlowError> {
        if let Some(db) = self.databases.get(key) {
            return Ok(db.clone());
        }

        let url = format!("{}/{}.db", self.database_directory, key);
        let db = turso::Builder::new_local(&url).build().await?;
        self.databases.insert(url.to_string(), db.clone());
        Ok(db)
    }
}

pub struct TursoDatabase<const KEY: &'static str>(pub Database);

#[async_trait::async_trait]
impl<const KEY: &'static str> FromEvent for TursoDatabase<KEY> {
    async fn from_event(context: BotContext, event: BotEvent) -> Option<Self> {
        let dispatcher = State::<Arc<TursoDispatcher>>::from_event(context, event).await?;

        let database = dispatcher.get_database(KEY).await.ok()?;
        Some(Self(database))
    }
}

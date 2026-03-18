use std::path::PathBuf;

use chrono::Utc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::Subscription,
};

pub struct SubscriptionStore {
    path: PathBuf,
    subscriptions: RwLock<Vec<Subscription>>,
}

impl SubscriptionStore {
    pub async fn load(path: PathBuf) -> AppResult<Self> {
        let subscriptions = if path.exists() {
            let content = tokio::fs::read_to_string(&path).await?;
            serde_json::from_str::<Vec<Subscription>>(&content).unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok(Self {
            path,
            subscriptions: RwLock::new(subscriptions),
        })
    }

    async fn persist(&self, subscriptions: &[Subscription]) -> AppResult<()> {
        let json = serde_json::to_string_pretty(subscriptions)?;
        tokio::fs::write(&self.path, json).await?;
        Ok(())
    }

    pub async fn list(&self) -> Vec<Subscription> {
        self.subscriptions.read().await.clone()
    }

    pub async fn get(&self, id: &str) -> Option<Subscription> {
        self.subscriptions
            .read()
            .await
            .iter()
            .find(|subscription| subscription.id == id)
            .cloned()
    }

    pub async fn upsert_by_url(&self, url: String, name: String) -> AppResult<Subscription> {
        let mut subscriptions = self.subscriptions.write().await;
        let now = Utc::now();

        if let Some(existing) = subscriptions
            .iter_mut()
            .find(|subscription| subscription.url.eq_ignore_ascii_case(url.as_str()))
        {
            existing.name = name;
            existing.updated_at = now;
            let saved = existing.clone();
            self.persist(&subscriptions).await?;
            return Ok(saved);
        }

        let subscription = Subscription {
            id: Uuid::new_v4().to_string(),
            name,
            url,
            created_at: now,
            updated_at: now,
        };
        subscriptions.push(subscription.clone());
        self.persist(&subscriptions).await?;
        Ok(subscription)
    }

    pub async fn delete(&self, id: &str) -> AppResult<Subscription> {
        let mut subscriptions = self.subscriptions.write().await;
        let index = subscriptions
            .iter()
            .position(|subscription| subscription.id == id)
            .ok_or_else(|| AppError::not_found("Subscription was not found"))?;
        let removed = subscriptions.remove(index);
        self.persist(&subscriptions).await?;
        Ok(removed)
    }
}

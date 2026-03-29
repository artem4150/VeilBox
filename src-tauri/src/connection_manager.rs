use tauri::{AppHandle, Manager};

use crate::{
    amnezia_manager,
    error::{AppError, AppResult},
    models::{ConnectionStatusPayload, ProfileEngine, TestConnectionResult},
    state::AppState,
    xray_manager,
};

pub async fn connection_status(state: &AppState) -> ConnectionStatusPayload {
    xray_manager::connection_status(state).await
}

pub async fn connect_profile(
    app: &AppHandle,
    state: &AppState,
    profile_id: String,
) -> AppResult<ConnectionStatusPayload> {
    let profile = state
        .profile_store
        .get(&profile_id)
        .await
        .ok_or_else(|| AppError::not_found("Profile was not found"))?;

    match profile.engine {
        ProfileEngine::Xray => xray_manager::connect_profile(app, state, profile_id).await,
        ProfileEngine::Amneziawg => amnezia_manager::connect_profile(app, state, &profile).await,
    }
}

pub async fn disconnect_profile(
    app: &AppHandle,
    state: &AppState,
) -> AppResult<ConnectionStatusPayload> {
    let session_engine = {
        let session = state.connection.session.lock().await;
        session
            .as_ref()
            .map(|entry| entry.engine)
    };
    let active_engine = if let Some(engine) = session_engine {
        engine
    } else if let Some(active_profile_id) = state.connection.status.read().await.active_profile_id.clone() {
        state
            .profile_store
            .get(&active_profile_id)
            .await
            .map(|profile| profile.engine)
            .unwrap_or(ProfileEngine::Xray)
    } else {
        ProfileEngine::Xray
    };

    match active_engine {
        ProfileEngine::Xray => xray_manager::disconnect_profile(app, state).await,
        ProfileEngine::Amneziawg => amnezia_manager::disconnect_profile(app, state).await,
    }
}

pub async fn cleanup_on_launch(app: &AppHandle) -> AppResult<()> {
    let state = app.state::<AppState>();
    amnezia_manager::cleanup_on_launch(state.inner()).await?;
    xray_manager::cleanup_on_launch(app).await
}

pub async fn xray_version(app: &AppHandle) -> Option<String> {
    xray_manager::xray_version(app).await
}

pub async fn test_profile_connection(
    app: &AppHandle,
    state: &AppState,
    profile_id: String,
) -> AppResult<TestConnectionResult> {
    let profile = state
        .profile_store
        .get(&profile_id)
        .await
        .ok_or_else(|| AppError::not_found("Profile was not found"))?;

    match profile.engine {
        ProfileEngine::Xray => xray_manager::test_profile_connection(app, state, profile_id).await,
        ProfileEngine::Amneziawg => amnezia_manager::test_profile_connection(app, state, &profile).await,
    }
}

use tauri::{AppHandle, State};

use crate::{
    error::AppError,
    latency_manager,
    models::{
        AboutInfo, BootstrapResponse, ConnectionStatusPayload, LogReadMode, LogsResponse, Profile,
        NetworkInterfaceInfo, ProfileCountry, ProfileInput, ProfileLatency, Settings,
        SettingsPatch, Subscription, SubscriptionImportResult, TestConnectionResult,
    },
    network_interface_manager,
    profile_geo,
    state::AppState,
    subscription_import,
    vless_parser::parse_vless_uri,
    xray_manager,
};

#[tauri::command]
pub async fn bootstrap(app: AppHandle, state: State<'_, AppState>) -> Result<BootstrapResponse, AppError> {
    let profiles = state.profile_store.list().await;
    let subscriptions = state.subscription_store.list().await;
    let settings = state.settings_store.get().await;
    let connection_status = xray_manager::connection_status(state.inner()).await;
    let logs = state.log_manager.read_logs(LogReadMode::Preview)?;
    let about = get_about_info(app, state).await?;

    Ok(BootstrapResponse {
        profiles,
        subscriptions,
        settings,
        connection_status,
        logs,
        about,
    })
}

#[tauri::command]
pub async fn list_profiles(state: State<'_, AppState>) -> Result<Vec<Profile>, AppError> {
    Ok(state.profile_store.list().await)
}

#[tauri::command]
pub async fn save_profile(
    state: State<'_, AppState>,
    profile: ProfileInput,
) -> Result<Profile, AppError> {
    let saved = state.profile_store.save(profile).await?;
    Ok(saved)
}

#[tauri::command]
pub async fn delete_profile(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let connection_status = xray_manager::connection_status(state.inner()).await;
    if connection_status.active_profile_id.as_deref() == Some(id.as_str()) {
        let _ = xray_manager::disconnect_profile(&app, state.inner()).await;
    }
    state.profile_store.delete(&id).await
}

#[tauri::command]
pub async fn duplicate_profile(
    state: State<'_, AppState>,
    id: String,
) -> Result<Profile, AppError> {
    state.profile_store.duplicate(&id).await
}

#[tauri::command]
pub async fn import_vless_uri(
    state: State<'_, AppState>,
    uri: String,
) -> Result<Profile, AppError> {
    let imported = parse_vless_uri(&uri)?;
    state.profile_store.save(imported).await
}

#[tauri::command]
pub async fn import_profiles_json(
    state: State<'_, AppState>,
    json: String,
) -> Result<Vec<Profile>, AppError> {
    let value: serde_json::Value = serde_json::from_str(json.trim())
        .map_err(|error| AppError::validation(format!("Invalid JSON: {}", error)))?;

    let inputs: Vec<ProfileInput> = if value.is_array() {
        serde_json::from_value(value)
            .map_err(|error| AppError::validation(format!("Unsupported profile JSON: {}", error)))?
    } else if let Some(profiles) = value.get("profiles") {
        serde_json::from_value(profiles.clone())
            .map_err(|error| AppError::validation(format!("Unsupported profile JSON: {}", error)))?
    } else {
        vec![
            serde_json::from_value(value)
                .map_err(|error| AppError::validation(format!("Unsupported profile JSON: {}", error)))?,
        ]
    };

    let mut saved = Vec::with_capacity(inputs.len());
    for input in inputs {
        saved.push(state.profile_store.save(input).await?);
    }
    Ok(saved)
}

#[tauri::command]
pub async fn import_subscription(
    state: State<'_, AppState>,
    url: String,
) -> Result<SubscriptionImportResult, AppError> {
    sync_subscription(state.inner(), None, url).await
}

#[tauri::command]
pub async fn list_subscriptions(state: State<'_, AppState>) -> Result<Vec<Subscription>, AppError> {
    Ok(state.subscription_store.list().await)
}

#[tauri::command]
pub async fn refresh_subscription(
    state: State<'_, AppState>,
    subscription_id: String,
) -> Result<SubscriptionImportResult, AppError> {
    let subscription = state
        .subscription_store
        .get(&subscription_id)
        .await
        .ok_or_else(|| AppError::not_found("Subscription was not found"))?;
    sync_subscription(state.inner(), Some(subscription.id), subscription.url).await
}

#[tauri::command]
pub async fn refresh_all_subscriptions(
    state: State<'_, AppState>,
) -> Result<Vec<SubscriptionImportResult>, AppError> {
    let subscriptions = state.subscription_store.list().await;
    let mut results = Vec::with_capacity(subscriptions.len());
    for subscription in subscriptions {
        results.push(sync_subscription(
            state.inner(),
            Some(subscription.id),
            subscription.url,
        ).await?);
    }
    Ok(results)
}

#[tauri::command]
pub async fn delete_subscription(
    app: AppHandle,
    state: State<'_, AppState>,
    subscription_id: String,
) -> Result<(), AppError> {
    let connection_status = xray_manager::connection_status(state.inner()).await;
    let active_profile_id = connection_status.active_profile_id;
    let removed_profiles = state
        .profile_store
        .delete_by_subscription_id(&subscription_id)
        .await?;
    if let Some(active_profile_id) = active_profile_id {
        let removed_active = removed_profiles
            .iter()
            .any(|profile| profile.id == active_profile_id);
        if removed_active {
            let _ = xray_manager::disconnect_profile(&app, state.inner()).await;
        }
    }
    let _ = state.subscription_store.delete(&subscription_id).await?;
    Ok(())
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings, AppError> {
    Ok(state.settings_store.get().await)
}

#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    patch: SettingsPatch,
) -> Result<Settings, AppError> {
    if patch.connection_mode.is_some() {
        let status = xray_manager::connection_status(state.inner()).await;
        if status.state != crate::models::ConnectionState::Disconnected {
            return Err(AppError::validation("Cannot change connection mode while active. Please disconnect first."));
        }
    }
    state.settings_store.update(patch).await
}

#[tauri::command]
pub async fn connect(
    app: AppHandle,
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<ConnectionStatusPayload, AppError> {
    xray_manager::connect_profile(&app, state.inner(), profile_id).await
}

#[tauri::command]
pub async fn disconnect(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ConnectionStatusPayload, AppError> {
    xray_manager::disconnect_profile(&app, state.inner()).await
}

#[tauri::command]
pub async fn connection_status(
    state: State<'_, AppState>,
) -> Result<ConnectionStatusPayload, AppError> {
    Ok(xray_manager::connection_status(state.inner()).await)
}

#[tauri::command]
pub async fn test_profile_connection(
    app: AppHandle,
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<TestConnectionResult, AppError> {
    xray_manager::test_profile_connection(&app, state.inner(), profile_id).await
}

#[tauri::command]
pub async fn get_logs(
    state: State<'_, AppState>,
    mode: String,
) -> Result<LogsResponse, AppError> {
    state.log_manager.read_logs(LogReadMode::from_str(&mode))
}

#[tauri::command]
pub async fn clear_logs(state: State<'_, AppState>) -> Result<(), AppError> {
    state.log_manager.clear().await
}

#[tauri::command]
pub async fn get_profile_latencies(
    state: State<'_, AppState>,
) -> Result<Vec<ProfileLatency>, AppError> {
    Ok(latency_manager::measure_profiles(state.profile_store.list().await).await)
}

#[tauri::command]
pub async fn get_profile_countries(
    state: State<'_, AppState>,
) -> Result<Vec<ProfileCountry>, AppError> {
    Ok(profile_geo::resolve_profile_countries(state.profile_store.list().await).await)
}

#[tauri::command]
pub async fn list_network_interfaces() -> Result<Vec<NetworkInterfaceInfo>, AppError> {
    network_interface_manager::list_network_interfaces()
}

#[tauri::command]
pub async fn get_about_info(
    app: AppHandle,
    _state: State<'_, AppState>,
) -> Result<AboutInfo, AppError> {
    Ok(AboutInfo {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        tauri_version: tauri::VERSION.to_string(),
        xray_version: xray_manager::xray_version(&app).await,
        platform: "windows".to_string(),
    })
}

async fn sync_subscription(
    state: &AppState,
    subscription_id: Option<String>,
    url: String,
) -> Result<SubscriptionImportResult, AppError> {
    let imported = subscription_import::import_subscription_url(&url).await?;
    let subscription = state
        .subscription_store
        .upsert_by_url(imported.url, imported.name)
        .await?;
    let effective_subscription_id = subscription_id.unwrap_or_else(|| subscription.id.clone());
    if effective_subscription_id != subscription.id {
        let _ = state
            .profile_store
            .delete_by_subscription_id(&effective_subscription_id)
            .await?;
    }
    let _ = state
        .profile_store
        .delete_by_subscription_id(&subscription.id)
        .await?;

    let mut saved = Vec::with_capacity(imported.profiles.len());
    for mut input in imported.profiles {
        input.source_label = Some(subscription.name.clone());
        input.subscription_id = Some(subscription.id.clone());
        saved.push(state.profile_store.save(input).await?);
    }

    Ok(SubscriptionImportResult {
        subscription,
        profiles: saved,
    })
}

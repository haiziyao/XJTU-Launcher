//! 社区广场 search: one model call with no tools, no persistence and no campus
//! data. The frontend gates the entry on the Agent plugin and the user's
//! `agentSearchEnabled` preference; the backend re-checks both.
use crate::{
    agent::{chat, provider_for},
    plugins::AGENT_ID,
};
use serde::Serialize;
use std::time::Duration;
use tauri::{AppHandle, State, WebviewWindow};

const SYSTEM_PROMPT: &str = "你是西安交通大学校园启动器「社区广场」的搜索助手。\
社区广场分为五个分区：社团、课程攻略、校园生活、生涯分享、资源导航。\
各分区的具体内容仍在开发中，你只能介绍分区用途和回答通用问题。\
不要编造社团、课程、活动、教师评价或个人经历；用户询问具体社区内容时，直接说明该分区内容仍在开发中。\
不要声称搜索过不存在的条目。用中文简洁回答。\
不要索要或输出校园账号、密码、Cookie 等凭据或个人数据。";

const MAX_QUERY_CHARS: usize = 500;
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchStatus {
    pub available: bool,
    pub reason: &'static str,
}

impl SearchStatus {
    fn ready() -> Self {
        Self {
            available: true,
            reason: "ready",
        }
    }

    fn unavailable(reason: &'static str) -> Self {
        Self {
            available: false,
            reason,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchAnswer {
    pub answer: String,
}

/// The first configuration-level blocker, in the order the UI explains it.
fn config_reason(config: &crate::config::AppConfig) -> Option<&'static str> {
    if config.disabled_plugins.iter().any(|id| id == AGENT_ID) {
        Some("plugin-disabled")
    } else if !config.agent_search_enabled {
        Some("disabled")
    } else {
        None
    }
}

/// A configured default provider still needs a model and an API key.
fn provider_reason(provider: Option<&chat::Provider>) -> Option<&'static str> {
    match provider {
        Some(provider)
            if !provider.model.trim().is_empty() && !provider.api_key.trim().is_empty() =>
        {
            None
        }
        _ => Some("no-model"),
    }
}

/// A server that answers but has no `/models` endpoint is still reachable.
fn probe_error_is_reachable(message: &str) -> bool {
    message.contains("不提供模型列表")
}

fn load_config(
    app: &AppHandle,
    state: &State<'_, crate::AppState>,
) -> Result<crate::config::AppConfig, String> {
    let _guard = state.config_lock.lock().map_err(|_| "配置服务繁忙")?;
    crate::config::load(&crate::config_path(app)?)
}

/// Reports whether 社区广场 search can run. Without `probe` this never touches
/// the network; with `probe` it only checks reachability of the model list.
#[tauri::command]
pub async fn agent_search_status(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, crate::AppState>,
    probe: Option<bool>,
) -> Result<SearchStatus, String> {
    crate::main_only(&window)?;
    let config = load_config(&app, &state)?;
    if let Some(reason) = config_reason(&config) {
        return Ok(SearchStatus::unavailable(reason));
    }
    let provider = {
        let _guard = state
            .providers_lock
            .lock()
            .map_err(|_| "模型配置服务繁忙")?;
        provider_for(&app, None, None)
    };
    let Ok(provider) = provider else {
        return Ok(SearchStatus::unavailable("no-model"));
    };
    if let Some(reason) = provider_reason(Some(&provider)) {
        return Ok(SearchStatus::unavailable(reason));
    }
    if !probe.unwrap_or(false) {
        return Ok(SearchStatus::ready());
    }
    let outcome = tokio::time::timeout(
        PROBE_TIMEOUT,
        crate::providers::list_models(&provider.base_url, &provider.protocol, &provider.api_key),
    )
    .await;
    let reachable = match outcome {
        Ok(Ok(_)) => true,
        Ok(Err(message)) => probe_error_is_reachable(&message),
        Err(_) => false,
    };
    Ok(if reachable {
        SearchStatus::ready()
    } else {
        SearchStatus::unavailable("unreachable")
    })
}

/// Answers one 社区广场 search question. Sends only the query text plus the
/// fixed system prompt: no tools, no conversation storage, no campus data.
#[tauri::command]
pub async fn community_agent_search(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, crate::AppState>,
    query: String,
) -> Result<SearchAnswer, String> {
    crate::main_only(&window)?;
    let query = query.trim().to_string();
    if query.is_empty() {
        return Err("请输入搜索内容".into());
    }
    if query.chars().count() > MAX_QUERY_CHARS {
        return Err(format!("搜索内容不能超过 {MAX_QUERY_CHARS} 个字符"));
    }
    let config = load_config(&app, &state)?;
    match config_reason(&config) {
        Some("plugin-disabled") => {
            return Err("Agent 插件已禁用，请先在插件中心启用".into());
        }
        Some(_) => return Err("社区搜索未开启，请先在 Agent 设置中开启".into()),
        None => {}
    }
    let provider = {
        let _guard = state
            .providers_lock
            .lock()
            .map_err(|_| "模型配置服务繁忙")?;
        provider_for(&app, None, None).map_err(|_| "未配置模型服务".to_string())?
    };
    if provider_reason(Some(&provider)).is_some() {
        return Err("未配置模型服务".into());
    }
    let answer = chat::complete_once(&provider, SYSTEM_PROMPT, &query).await?;
    Ok(SearchAnswer { answer })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(key: &str) -> chat::Provider {
        chat::Provider {
            protocol: "openai-compatible".into(),
            base_url: "https://example.com".into(),
            model: "test-model".into(),
            api_key: key.into(),
        }
    }

    #[test]
    fn availability_matrix_reports_the_first_blocker() {
        let mut config = crate::config::AppConfig::default();
        assert_eq!(config_reason(&config), Some("disabled"));

        config.agent_search_enabled = true;
        assert_eq!(config_reason(&config), None);
        assert_eq!(provider_reason(None), Some("no-model"));
        assert_eq!(provider_reason(Some(&provider(""))), Some("no-model"));
        let mut without_model = provider("key");
        without_model.model.clear();
        assert_eq!(provider_reason(Some(&without_model)), Some("no-model"));
        assert_eq!(provider_reason(Some(&provider("key"))), None);

        config.disabled_plugins.push(AGENT_ID.into());
        assert_eq!(config_reason(&config), Some("plugin-disabled"));
    }

    #[test]
    fn missing_models_endpoint_is_treated_as_reachable() {
        assert!(probe_error_is_reachable("该接口地址不提供模型列表"));
        assert!(!probe_error_is_reachable(
            "无法连接模型服务，请检查接口地址和网络"
        ));
        assert!(!probe_error_is_reachable("API Key 无效"));
    }
}

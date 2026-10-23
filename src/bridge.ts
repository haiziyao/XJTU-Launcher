import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import type {
  ImageCrop,
  AgentConversation,
  AgentWorkspace,
  AgentEvent,
  AgentMemory,
  AgentPresetInput,
  AgentPresetKind,
  AgentPresets,
  AgentSummary,
  AgentToolInfo,
  ExposeStatus,
  ExposeTool,
  MarketCatalog,
  McpCatalog,
  McpServer,
  McpTool,
  McpTransport,
  ModelListInput,
  PersonalImageKind,
  SchoolFeature,
  SchoolQuery,
  EvaluationPrepare,
  EvaluationPreview,
  EvaluationOutcome,
  UpdateStatus,
  ModelProviders,
  ProviderBalance,
  ProviderInput,
  CampusAccount,
  CampusAccountList,
  CampusDegree,
  CampusLogin,
  CampusLoginAction,
  AccountStatus,
  NetworkCheck,
  AppConfig,
  AuthEvent,
  PluginList,
  Timetable,
  TimetableEdit,
} from './types';

export const native = isTauri();

export const bridge = {
  pluginSource: () =>
    invoke<{ source: string | null; defaultSource: string | null }>('plugin_source'),
  pluginCatalog: () => invoke<MarketCatalog>('plugin_catalog'),
  schoolCatalog: () => invoke<SchoolFeature[]>('school_catalog'),
  // The backend stores the answer under the account that ran the query when a
  // cache name is given, so an answer that arrives after a switch stays put.
  schoolQuery: (query: SchoolQuery, cacheName?: string) =>
    invoke<unknown>('school_query', { query, cacheKey: cacheName ?? null }),
  lmsOpenFile: (uploadId: number) => invoke<void>('lms_open_file', { uploadId }),
  lmsSaveFile: (uploadId: number, path: string) =>
    invoke<number>('lms_save_file', { uploadId, path }),
  schoolPrepareEvaluations: (input: EvaluationPrepare) =>
    invoke<EvaluationPreview>('school_prepare_evaluations', { input }),
  schoolSubmitEvaluations: (previewId: string) =>
    invoke<EvaluationOutcome[]>('school_submit_evaluations', { previewId }),
  schoolCancelEvaluations: (previewId: string) =>
    invoke<void>('school_cancel_evaluations', { previewId }),
  installPlugin: (id: string, version: string) =>
    invoke<void>('install_market_plugin', { id, version }),
  removePlugin: (id: string) => invoke<void>('remove_plugin', { id }),
  updateStatus: () => invoke<UpdateStatus>('update_status'),
  checkUpdate: () => invoke<UpdateStatus>('check_app_update'),
  installUpdate: () => invoke<void>('install_app_update'),
  restartUpdatedApp: () => invoke<void>('restart_after_update'),
  async modelProviders(): Promise<ModelProviders> {
    requireNative();
    return invoke('list_model_providers');
  },
  async saveProvider(input: ProviderInput): Promise<void> {
    requireNative();
    return invoke('save_model_provider', { input });
  },
  async removeProvider(id: string): Promise<void> {
    requireNative();
    return invoke('remove_model_provider', { id });
  },
  async selectProvider(id: string): Promise<void> {
    requireNative();
    return invoke('select_model_provider', { id });
  },
  async saveProviderModels(id: string, models: string[]): Promise<void> {
    requireNative();
    return invoke('save_provider_models', { id, models });
  },
  async setProviderModel(id: string, model: string): Promise<void> {
    requireNative();
    return invoke('set_provider_model', { id, model });
  },
  async revealProviderKey(id: string): Promise<string> {
    requireNative();
    return invoke('reveal_provider_key', { id });
  },
  async testProvider(id: string): Promise<number> {
    requireNative();
    return invoke('test_provider', { id });
  },
  async providerBalance(id: string): Promise<ProviderBalance | null> {
    requireNative();
    return invoke('provider_balance', { id });
  },
  async listProviderModels(input: ModelListInput): Promise<string[]> {
    requireNative();
    return invoke('list_provider_models', { input });
  },
  /** Picks a source image; the preview is a blob URL the caller must revoke. */
  async pickPersonalImage(
    kind: PersonalImageKind,
  ): Promise<{ path: string; preview: string } | null> {
    requireNative();
    const path = await open({
      multiple: false,
      directory: false,
      title: kind === 'avatar' ? '选择头像' : '选择壁纸',
      filters: [{ name: '图片', extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
    });
    if (!path) return null;
    // Raw bytes let the webview decode the preview natively instead of waiting on Rust.
    const bytes = await invoke<ArrayBuffer>('personal_image_source', { path, kind });
    const ext = path.split('.').pop()?.toLowerCase();
    const type = ext === 'png' ? 'image/png' : ext === 'webp' ? 'image/webp' : 'image/jpeg';
    return { path, preview: URL.createObjectURL(new Blob([bytes], { type })) };
  },
  cropPersonalImage(path: string, kind: PersonalImageKind, crop: ImageCrop): Promise<string> {
    requireNative();
    return invoke('read_personal_image', { path, kind, crop });
  },
  async config(): Promise<AppConfig> {
    requireNative();
    return invoke('load_config');
  },
  async saveConfig(config: AppConfig): Promise<void> {
    requireNative();
    return invoke('save_config', { config });
  },
  async plugins(): Promise<PluginList> {
    requireNative();
    return invoke('list_plugins');
  },
  async importPlugin(): Promise<boolean> {
    requireNative();
    const directory = await open({
      directory: true,
      multiple: false,
      title: '选择包含 plugin.json 的插件目录',
    });
    if (!directory) return false;
    await invoke('import_plugin', { directory });
    return true;
  },
  async account(): Promise<AccountStatus> {
    requireNative();
    return invoke('account_status');
  },
  async checkNetwork(): Promise<NetworkCheck> {
    requireNative();
    return invoke('check_network');
  },
  async refreshDirectSession(): Promise<boolean> {
    requireNative();
    return invoke('refresh_direct_session');
  },
  // Each campus service keeps its own cookie or token session.
  async refreshProfileSession(): Promise<boolean> {
    requireNative();
    return invoke('refresh_profile_session');
  },
  async refreshLmsSession(): Promise<boolean> {
    requireNative();
    return invoke('refresh_lms_session');
  },
  async refreshLibrarySession(): Promise<boolean> {
    requireNative();
    return invoke('refresh_library_session');
  },
  async refreshVenueSession(): Promise<boolean> {
    requireNative();
    return invoke('refresh_venue_session');
  },
  // Native campus authentication and local account management.
  async accounts(): Promise<CampusAccountList> {
    requireNative();
    return invoke('list_accounts');
  },
  async startAccountLogin(degree: CampusDegree, requestId: string): Promise<CampusLogin> {
    requireNative();
    return invoke('start_account_login', { degree, requestId });
  },
  async advanceAccountLogin(flowId: string, action: CampusLoginAction): Promise<CampusLogin> {
    requireNative();
    return invoke('advance_account_login', { flowId, action });
  },
  async accountLoginState(flowId: string): Promise<CampusLogin> {
    requireNative();
    return invoke('account_login_state', { flowId });
  },
  async forgetAccountPassword(accountId: string): Promise<void> {
    requireNative();
    return invoke('forget_account_password', { accountId });
  },
  async accountLoginCaptcha(flowId: string): Promise<string> {
    requireNative();
    return invoke('account_login_captcha', { flowId });
  },
  async cancelAccountLogin(flowId: string): Promise<void> {
    requireNative();
    return invoke('cancel_account_login', { flowId });
  },
  async switchAccount(accountId: string): Promise<void> {
    requireNative();
    return invoke('switch_account', { accountId });
  },
  async removeAccount(accountId: string): Promise<void> {
    requireNative();
    return invoke('remove_account', { accountId });
  },
  async setAccountAvatar(accountId: string): Promise<CampusAccount | null> {
    requireNative();
    const path = await open({
      multiple: false,
      directory: false,
      title: '选择账号头像',
      filters: [{ name: '图片', extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
    });
    if (!path) return null;
    return invoke('set_account_avatar', { accountId, path });
  },
  async clearAccountAvatar(accountId: string): Promise<CampusAccount> {
    requireNative();
    return invoke('set_account_avatar', { accountId, path: null });
  },
  async logout(): Promise<void> {
    requireNative();
    await invoke('logout');
  },
  async saveKey(key: string): Promise<void> {
    requireNative();
    await invoke('save_api_key', { key });
  },
  async timetable(pluginId: string): Promise<Timetable> {
    requireNative();
    return invoke('query_timetable', { pluginId });
  },
  /** Applies one local edit and returns the merged timetable to show. */
  async editTimetable(request: TimetableEdit): Promise<Timetable> {
    requireNative();
    return invoke('edit_timetable', { request });
  },
  async openUrl(url: string): Promise<void> {
    requireNative();
    return invoke('open_external', { url });
  },
  async mcpServers(): Promise<McpServer[]> {
    requireNative();
    return invoke('list_mcp_servers');
  },
  async saveMcpServer(input: {
    id: string | null;
    name: string;
    transport: McpTransport;
  }): Promise<string> {
    requireNative();
    return invoke('save_mcp_server', { input });
  },
  async removeMcpServer(id: string): Promise<void> {
    requireNative();
    return invoke('remove_mcp_server', { id });
  },
  async setMcpEnabled(id: string, enabled: boolean): Promise<void> {
    requireNative();
    return invoke('set_mcp_server_enabled', { id, enabled });
  },
  async probeMcpServer(id: string): Promise<McpTool[]> {
    requireNative();
    return invoke('probe_mcp_server', { id });
  },
  async mcpCatalog(): Promise<McpCatalog> {
    requireNative();
    return invoke('mcp_catalog');
  },
  async installMcpEntry(id: string, values: Record<string, string>): Promise<string> {
    requireNative();
    return invoke('install_mcp_entry', { entryId: id, values });
  },
  async onAuth(callback: (event: AuthEvent) => void): Promise<() => void> {
    requireNative();
    return listen<AuthEvent>('campus-auth', (e) => callback(e.payload));
  },
  async agentList(): Promise<AgentSummary[]> {
    requireNative();
    return invoke('agent_list');
  },
  async agentGet(id: string): Promise<AgentConversation> {
    requireNative();
    return invoke('agent_get', { id });
  },
  async agentNew(workspaceId: string | null = null): Promise<AgentConversation> {
    requireNative();
    return invoke('agent_new', { workspaceId });
  },
  async agentLastOpened(): Promise<string | null> {
    requireNative();
    return invoke('agent_last_opened');
  },
  async agentSetLastOpened(id: string | null): Promise<void> {
    requireNative();
    return invoke('agent_set_last_opened', { id });
  },
  async agentWorkspaces(): Promise<AgentWorkspace[]> {
    requireNative();
    return invoke('agent_workspaces');
  },
  async agentSetWorkspace(id: string, workspaceId: string | null): Promise<void> {
    requireNative();
    return invoke('agent_set_workspace', { id, workspaceId });
  },
  async agentCreateWorkspace(name: string): Promise<AgentWorkspace> {
    requireNative();
    return invoke('agent_create_workspace', { name });
  },
  async agentDelete(id: string): Promise<void> {
    requireNative();
    return invoke('agent_delete', { id });
  },
  async agentRename(id: string, title: string): Promise<void> {
    requireNative();
    return invoke('agent_rename', { id, title });
  },
  async agentSend(input: {
    conversationId: string;
    requestId: string;
    content: string;
    providerId: string | null;
    model: string | null;
    skillIds: string[];
  }): Promise<void> {
    requireNative();
    return invoke('agent_send', { input });
  },
  async agentPresets(): Promise<AgentPresets> {
    requireNative();
    return invoke('agent_presets');
  },
  async agentSavePreset(input: AgentPresetInput): Promise<string> {
    requireNative();
    return invoke('agent_save_preset', { input });
  },
  async agentRemovePreset(kind: AgentPresetKind, id: string): Promise<void> {
    requireNative();
    return invoke('agent_remove_preset', { kind, id });
  },
  async agentMemories(): Promise<AgentMemory[]> {
    requireNative();
    return invoke('agent_memories');
  },
  async agentSaveMemory(id: string | null, content: string): Promise<AgentMemory> {
    requireNative();
    return invoke('agent_save_memory', { id, content });
  },
  async agentRemoveMemory(id: string): Promise<void> {
    requireNative();
    return invoke('agent_remove_memory', { id });
  },
  async agentSetRole(id: string, roleId: string | null): Promise<void> {
    requireNative();
    return invoke('agent_set_role', { id, roleId });
  },
  async agentPin(id: string, pinned: boolean): Promise<void> {
    requireNative();
    return invoke('agent_pin', { id, pinned });
  },
  async agentCancel(requestId: string): Promise<void> {
    requireNative();
    return invoke('agent_cancel', { requestId });
  },
  async agentTools(): Promise<AgentToolInfo[]> {
    requireNative();
    return invoke('agent_tools');
  },
  async exposeStatus(): Promise<ExposeStatus> {
    requireNative();
    return invoke('expose_status');
  },
  async exposeTools(): Promise<ExposeTool[]> {
    requireNative();
    return invoke('expose_tools');
  },
  async setExposeEnabled(enabled: boolean): Promise<ExposeStatus> {
    requireNative();
    return invoke('expose_set_enabled', { enabled });
  },
  async setExposePort(port: number): Promise<ExposeStatus> {
    requireNative();
    return invoke('expose_set_port', { port });
  },
  async probeExpose(): Promise<void> {
    requireNative();
    return invoke('expose_probe');
  },
  async onAgent(callback: (event: AgentEvent) => void): Promise<() => void> {
    requireNative();
    return listen<AgentEvent>('agent-event', (e) => callback(e.payload));
  },
};

function requireNative(): void {
  if (!native) throw new Error('请从 XJTU Launcher 桌面应用打开。');
}

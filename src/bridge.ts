import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open, save } from '@tauri-apps/plugin-dialog';
import type {
  ImageCrop,
  AgentConversation,
  AgentWorkspace,
  AgentEvent,
  AgentMemory,
  AgentPresetInput,
  AgentPresetKind,
  AgentPresets,
  AgentSearchAnswer,
  AgentSearchStatus,
  AgentSummary,
  AgentToolInfo,
  EntryIconSet,
  ExposeStatus,
  ExposeTool,
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
  CampusNoticeBoard,
  PetDesktopView,
  AuthEvent,
  PluginList,
  Timetable,
  TimetableEdit,
  WorkbenchBindingInput,
  WorkbenchFile,
  WorkbenchFolder,
  WorkbenchImportOutcome,
  WorkbenchOverview,
  WorkbenchSyncResult,
  LmsSubmitPreview,
  LmsSubmitResult,
  StorageStatus,
  StoragePreview,
  StorageReport,
  LmsReplayVideos,
  FontDownloadProgress,
  FontLibraryState,
  InstalledFont,
} from './types';

export const native = isTauri();

export const bridge = {
  schoolCatalog: () => invoke<SchoolFeature[]>('school_catalog'),
  // Entry icons under `~/.xjtu-launcher/resources/icons`; never an error, an
  // empty set just means every entry keeps its built-in glyph.
  entryIcons: () => invoke<EntryIconSet>('entry_icons'),
  // The backend stores the answer under the account that ran the query when a
  // cache name is given, so an answer that arrives after a switch stays put.
  // A stored answer younger than the query's TTL is returned without a request;
  // `force` is what a user-initiated refresh passes.
  schoolQuery: (query: SchoolQuery, cacheName?: string, force = false) =>
    invoke<unknown>('school_query', { query, cacheKey: cacheName ?? null, force }),
  lmsOpenFile: (uploadId: number) => invoke<void>('lms_open_file', { uploadId }),
  /** Resolve direct replay videos; the result is presigned and not cached. */
  lmsReplayVideos: (activity: number) => invoke<LmsReplayVideos>('lms_replay_videos', { activity }),
  /** Save a capture export (base64) to the user-chosen absolute path. */
  saveExport: (path: string, data: string) => invoke<number>('save_export', { path, data }),
  lmsSaveFile: (uploadId: number, path: string) =>
    invoke<number>('lms_save_file', { uploadId, path }),
  // Save dialogs start inside the data folder's downloads directory. The
  // backend builds the path so the page never invents one; if the folder
  // cannot be created the dialog falls back to the bare file name.
  async saveTarget(name: string): Promise<string> {
    requireNative();
    try {
      return await invoke<string>('storage_download_path', { name });
    } catch {
      return name;
    }
  },
  // Paths the interface shows use `~/...` so an account name never appears.
  async displayPath(path: string): Promise<string> {
    requireNative();
    try {
      return await invoke<string>('storage_display_path', { path });
    } catch {
      return path;
    }
  },
  storageStatus: () => invoke<StorageStatus>('storage_status'),
  storagePreview: (target: string) => invoke<StoragePreview>('storage_preview', { target }),
  storageMigrate: (target: string) => invoke<StorageReport>('storage_migrate', { target }),
  storageReveal: () => invoke<void>('storage_reveal'),
  // 工作台 is a view of the user's own folders: the backend copies picked files
  // in and keeps only the per-file extras, so Finder stays the source of truth.
  async workbenchOverview(): Promise<WorkbenchOverview> {
    requireNative();
    return invoke('workbench_overview');
  },
  async workbenchFolder(folder: string): Promise<WorkbenchFile[]> {
    requireNative();
    return invoke('workbench_folder', { folder });
  },
  async workbenchCreateFolder(parent: string, name: string): Promise<WorkbenchFolder> {
    requireNative();
    return invoke('workbench_create_folder', { parent, name });
  },
  async workbenchAdd(folder: string): Promise<WorkbenchImportOutcome[]> {
    requireNative();
    const paths = await open({
      multiple: true,
      directory: false,
      title: '添加到工作台',
    });
    if (!paths) return [];
    return invoke('workbench_import', { folder, paths });
  },
  async workbenchUpdate(path: string, input: WorkbenchBindingInput): Promise<WorkbenchFile> {
    requireNative();
    return invoke('workbench_update', { path, input });
  },
  async workbenchDelete(path: string): Promise<void> {
    requireNative();
    return invoke('workbench_delete', { path });
  },
  async workbenchOpen(path: string): Promise<void> {
    requireNative();
    return invoke('workbench_open', { path });
  },
  async workbenchReveal(path: string): Promise<void> {
    requireNative();
    return invoke('workbench_reveal', { path });
  },
  async workbenchSync(): Promise<WorkbenchSyncResult> {
    requireNative();
    return invoke('workbench_sync');
  },
  /** Preview only: the backend reads the activity and local files, no upload. */
  async workbenchSubmitPreview(
    activity: number,
    comment: string,
    paths: string[],
  ): Promise<LmsSubmitPreview> {
    requireNative();
    return invoke('workbench_submit_preview', { activity, comment, paths });
  },
  /** Only after the user saw the preview and explicitly confirmed. */
  async workbenchSubmit(
    activity: number,
    comment: string,
    paths: string[],
  ): Promise<LmsSubmitResult> {
    requireNative();
    return invoke('workbench_submit', { activity, comment, paths });
  },
  schoolPrepareEvaluations: (input: EvaluationPrepare) =>
    invoke<EvaluationPreview>('school_prepare_evaluations', { input }),
  schoolSubmitEvaluations: (previewId: string) =>
    invoke<EvaluationOutcome[]>('school_submit_evaluations', { previewId }),
  schoolCancelEvaluations: (previewId: string) =>
    invoke<void>('school_cancel_evaluations', { previewId }),
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
  async agentSearchStatus(probe = false): Promise<AgentSearchStatus> {
    requireNative();
    return invoke('agent_search_status', { probe });
  },
  async communityAgentSearch(query: string): Promise<AgentSearchAnswer> {
    requireNative();
    return invoke('community_agent_search', { query });
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
  async pluginSettings<T>(id: string): Promise<T> {
    requireNative();
    return invoke<T>('plugin_settings', { id });
  },
  async savePluginSettings<T>(id: string, settings: T): Promise<T> {
    requireNative();
    return invoke<T>('save_plugin_settings', { id, settings });
  },
  async fontsState(): Promise<FontLibraryState> {
    requireNative();
    return invoke('fonts_state');
  },
  async fontDownload(id: string): Promise<InstalledFont> {
    requireNative();
    return invoke('font_download', { id });
  },
  async fontDownloadCancel(id: string): Promise<boolean> {
    requireNative();
    return invoke('font_download_cancel', { id });
  },
  async fontAddLocal(): Promise<InstalledFont | null> {
    requireNative();
    const path = await open({
      multiple: false,
      directory: false,
      title: '添加字体',
      filters: [{ name: '字体', extensions: ['ttf', 'otf', 'woff', 'woff2'] }],
    });
    if (!path) return null;
    return invoke('font_add_local', { path });
  },
  async fontRemove(id: string): Promise<void> {
    requireNative();
    return invoke('font_remove', { id });
  },
  /** Raw font bytes for `FontFace`; no file path ever reaches the page. */
  async fontData(id: string): Promise<ArrayBuffer> {
    requireNative();
    return invoke('font_data', { id });
  },
  async onFontDownload(callback: (progress: FontDownloadProgress) => void): Promise<() => void> {
    requireNative();
    return listen<FontDownloadProgress>('font-download', (event) => callback(event.payload));
  },
  async account(): Promise<AccountStatus> {
    requireNative();
    return invoke('account_status');
  },
  async checkNetwork(): Promise<NetworkCheck> {
    requireNative();
    return invoke('check_network');
  },
  async refreshDirectSession(force = false): Promise<boolean> {
    requireNative();
    return invoke('refresh_direct_session', { force });
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
  async accountLoginQr(flowId: string): Promise<string> {
    requireNative();
    return invoke('account_login_qr', { flowId });
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
  async notices(channel: string, limit: number): Promise<CampusNoticeBoard> {
    requireNative();
    return invoke('campus_notices', { channel, limit });
  },
  async petView(): Promise<PetDesktopView> {
    requireNative();
    return invoke('pet_view');
  },
  async petRestore(): Promise<void> {
    requireNative();
    return invoke('pet_restore');
  },
  async petOpenSettings(): Promise<void> {
    requireNative();
    return invoke('pet_open_settings');
  },
  async petSetBarHidden(hidden: boolean): Promise<PetDesktopView> {
    requireNative();
    return invoke('pet_set_bar_hidden', { hidden });
  },
  async petPreview(): Promise<void> {
    requireNative();
    return invoke('pet_preview');
  },
  async petPushNotification(source: string, text: string, key?: string): Promise<void> {
    requireNative();
    return invoke('pet_push_notification', { source, text, key: key ?? null });
  },
  async onPetNotifications(callback: () => void): Promise<() => void> {
    requireNative();
    return listen('pet-notifications', () => callback());
  },
  async onPetSettings(callback: () => void): Promise<() => void> {
    requireNative();
    return listen('pet-settings', () => callback());
  },
  async onPetOpenSettings(callback: () => void): Promise<() => void> {
    requireNative();
    return listen('pet-open-settings', () => callback());
  },
};

function requireNative(): void {
  if (!native) throw new Error('请从 XJTU Launcher 桌面应用打开。');
}

import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import type {
  MarketCatalog,
  SchoolFeature,
  UpdateStatus,
  ModelProviders,
  ProviderInput,
  CampusAccount,
  CampusAccountList,
  CampusDegree,
  CampusLogin,
  CampusLoginAction,
  AccountStatus,
  AppConfig,
  AuthEvent,
  PluginList,
  Timetable,
} from './types';

export const native = isTauri();

export const bridge = {
  pluginSource: () =>
    invoke<{ source: string | null; defaultSource: string | null }>('plugin_source'),
  pluginCatalog: () => invoke<MarketCatalog>('plugin_catalog'),
  schoolCatalog: () => invoke<SchoolFeature[]>('school_catalog'),
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
  async personalImage(kind: 'avatar' | 'wallpaper'): Promise<string | null> {
    requireNative();
    const path = await open({
      multiple: false,
      directory: false,
      title: kind === 'avatar' ? '选择头像' : '选择壁纸',
      filters: [{ name: '图片', extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
    });
    return path ? invoke('read_personal_image', { path, kind }) : null;
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
  async openUrl(url: string): Promise<void> {
    requireNative();
    return invoke('open_external', { url });
  },
  async onAuth(callback: (event: AuthEvent) => void): Promise<() => void> {
    requireNative();
    return listen<AuthEvent>('campus-auth', (e) => callback(e.payload));
  },
};

function requireNative(): void {
  if (!native) throw new Error('请从 XJTU Launcher 桌面应用打开。');
}

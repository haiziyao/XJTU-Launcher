//! accounts domain types.

export interface AccountStatus {
  connected: boolean;
  name?: string | null;
  studentId?: string | null;
  avatar?: string | null;
  degree: 'undergraduate' | 'postgraduate' | null;
  apiKeySet: boolean;
  accessMode?: 'direct' | 'webvpn' | null;
  /** Whether this account also holds a direct session for card/fitness. */
  directSession?: boolean;
}
/** Result of the sidebar network self-check. */
export interface NetworkCheck {
  tone: 'campus' | 'vpn' | 'manual' | 'neutral';
  message: string;
  /** Whether the account holds a direct CAS session for card/fitness. */
  directSession: boolean;
}
export interface AuthEvent {
  state: 'success' | 'error' | 'cancelled';
  message: string;
}

export type CampusDegree = 'undergraduate' | 'postgraduate';
export interface CampusAccount {
  id: string;
  username: string;
  degree: CampusDegree;
  name: string | null;
  studentId: string | null;
  avatar: string | null;
  accessMode: 'direct' | 'webvpn';
  passwordSaved: boolean;
}
export interface CampusAccountList {
  activeId: string | null;
  accounts: CampusAccount[];
}
export interface CampusLogin {
  flowId: string;
  accessMode: 'direct' | 'webvpn';
  step: 'webvpn' | 'campus';
  phase: 'credentials' | 'captcha' | 'mfa' | 'identity' | 'success';
  phone: string | null;
  choices: { label: string; name: string }[];
  message: string | null;
  canContinue: boolean;
  retryAfterSeconds: number;
  savePending: boolean;
}
export type CampusLoginAction =
  | {
      type: 'credentials';
      username: string;
      password: string;
      captcha?: string;
      rememberPassword: boolean;
      trustDevice?: boolean;
    }
  | {
      type: 'savedCredentials';
      accountId: string;
      captcha?: string;
      rememberPassword: boolean;
      trustDevice?: boolean;
    }
  | { type: 'continue'; captcha?: string }
  | { type: 'sendMfa' }
  | { type: 'verifyMfa'; code: string }
  | { type: 'chooseIdentity'; label: string }
  | { type: 'sendSmsCode'; phone: string; captcha?: string }
  | { type: 'smsLogin'; phone: string; code: string; captcha?: string }
  | { type: 'pollQr' };

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { Eye, EyeOff, LoaderCircle, RefreshCw, ShieldCheck, ArrowLeft } from 'lucide-vue-next';
import { bridge } from '../bridge';
import { createCampusLogin } from '../campusLogin';
import type { CampusAccount, CampusDegree, CampusLoginAction } from '../types';
import PageHeader from './ui/PageHeader.vue';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import UiSwitch from './ui/UiSwitch.vue';
import EmptyState from './ui/EmptyState.vue';

import UiButton from './ui/UiButton.vue';
import UiIconButton from './ui/UiIconButton.vue';
const props = defineProps<{ account?: CampusAccount | null }>();
const emit = defineEmits<{ cancel: []; success: [] }>();
const degree = ref<CampusDegree>(props.account?.degree || 'undergraduate');
const username = ref(props.account?.username || '');
const password = ref('');
const remember = ref(true);
const trust = ref(true);
const showPassword = ref(false);
const useSaved = ref(!!props.account?.passwordSaved);
const captcha = ref('');
const method = ref<'password' | 'phone' | 'qrcode'>('password');
const phone = ref('');
const smsCaptcha = ref('');
const smsCode = ref('');
const smsResendAt = ref(0);
const code = ref('');
const identity = ref('');
const now = ref(Date.now());
const resendAt = ref(0);
let timer: ReturnType<typeof setInterval> | undefined;
const login = createCampusLogin(bridge, () => {
  password.value = '';
  emit('success');
});
const { flow, busy, error, captchaImage, qrImage } = login;
const retrySeconds = computed(() => Math.max(0, Math.ceil((resendAt.value - now.value) / 1000)));
const smsRetrySeconds = computed(() =>
  Math.max(0, Math.ceil((smsResendAt.value - now.value) / 1000)),
);
const phoneValid = computed(() => /^1\d{10}$/.test(phone.value.trim()));
// QR status lives in the QR card; keep it out of the general notice bar so it
// does not linger after switching back to password or phone.
const QR_STATUS = new Set([
  '请使用移动交通大学 App 扫码登录。',
  '已扫码，请在手机上确认登录。',
  '已在手机上取消登录，请刷新二维码。',
  '二维码已过期，请刷新。',
  '二维码状态异常，请刷新。',
]);
const visibleMessage = computed(() => {
  const message = flow.value?.message;
  return message && !QR_STATUS.has(message) ? message : null;
});
const credentials = computed(
  () => flow.value && ['credentials', 'captcha'].includes(flow.value.phase),
);
const hasCredentials = computed(() => useSaved.value || (username.value.trim() && password.value));
const canSubmit = computed(
  () =>
    !busy.value &&
    !!flow.value &&
    (credentials.value
      ? method.value === 'phone'
        ? phoneValid.value && /^\d{4,8}$/.test(smsCode.value.trim())
        : method.value === 'qrcode'
          ? false
          : hasCredentials.value && (flow.value.phase !== 'captcha' || !!captcha.value.trim())
      : flow.value.phase === 'mfa'
        ? /^\d{4,16}$/.test(code.value.trim())
        : flow.value.phase === 'identity'
          ? !!identity.value
          : flow.value.phase === 'success'),
);
watch(flow, (next, previous) => {
  now.value = Date.now();
  resendAt.value = Date.now() + (next?.retryAfterSeconds || 0) * 1000;
  if (next?.phase !== 'identity' || next.flowId !== previous?.flowId) identity.value = '';
  if (next?.phase !== 'credentials' && next?.phase !== 'captcha') {
    password.value = '';
    showPassword.value = false;
    void login.stopQr();
  }
  if (next?.message === '验证码已发送') smsResendAt.value = Date.now() + 60_000;
});
function switchMethod(next: 'password' | 'phone' | 'qrcode') {
  if (busy.value || !credentials.value || method.value === next) return;
  method.value = next;
  error.value = '';
  captcha.value = '';
  smsCaptcha.value = '';
  smsCode.value = '';
  if (next !== 'qrcode') void login.stopQr();
  if (next === 'qrcode') void login.startQr();
  if (next === 'phone') void login.refreshCaptcha();
}
async function sendCode() {
  if (!phoneValid.value || busy.value) return;
  await login.sendSmsCode(phone.value.trim(), smsCaptcha.value.trim());
  smsCaptcha.value = '';
  void login.refreshCaptcha();
}
async function restart() {
  method.value = 'password';
  phone.value = '';
  smsCaptcha.value = '';
  smsCode.value = '';
  smsResendAt.value = 0;
  void login.stopQr();
  password.value = '';
  code.value = '';
  captcha.value = '';
  identity.value = '';
  showPassword.value = false;
  await login.begin(degree.value);
}
async function submit() {
  if (!canSubmit.value || !flow.value) return;
  let action: CampusLoginAction;
  if (credentials.value) {
    if (method.value === 'qrcode') return;
    if (method.value === 'phone') {
      action = {
        type: 'smsLogin',
        phone: phone.value.trim(),
        code: smsCode.value.trim(),
        captcha: smsCaptcha.value.trim(),
      };
      smsCode.value = '';
      smsCaptcha.value = '';
      await login.submit(action);
      return;
    }
    if (useSaved.value && props.account)
      action = {
        type: 'savedCredentials',
        accountId: props.account.id,
        captcha: captcha.value.trim(),
        rememberPassword: remember.value,
        trustDevice: trust.value,
      };
    else
      action = {
        type: 'credentials',
        username: username.value.trim(),
        password: password.value,
        captcha: captcha.value.trim(),
        rememberPassword: remember.value,
        trustDevice: trust.value,
      };
  } else if (flow.value.phase === 'mfa') action = { type: 'verifyMfa', code: code.value.trim() };
  else if (flow.value.phase === 'identity')
    action = { type: 'chooseIdentity', label: identity.value };
  else action = { type: 'continue' }; // Retry a completed login whose local save failed.
  password.value = '';
  captcha.value = '';
  code.value = '';
  showPassword.value = false;
  await login.submit(action);
}
async function cancel() {
  void login.stopQr();
  password.value = '';
  code.value = '';
  captcha.value = '';
  await login.cancel();
  emit('cancel');
}
function enterPassword() {
  useSaved.value = false;
  password.value = '';
}
onMounted(() => {
  timer = setInterval(() => (now.value = Date.now()), 1000);
  void login.begin(degree.value);
});
onBeforeUnmount(() => {
  if (timer) clearInterval(timer);
  void login.stopQr();
  password.value = '';
  code.value = '';
  login.dispose();
});
</script>

<template>
  <section class="desktop-page narrow-page">
    <UiButton variant="ghost" size="sm" class="login-back" @click="cancel">
      <ArrowLeft :size="14" />校园账号
    </UiButton>
    <PageHeader
      :title="account ? '重新登录' : '添加校园账号'"
      description="使用西安交通大学统一身份认证账号"
    >
      <template #actions
        ><UiButton variant="ghost" size="sm" :disabled="busy" @click="restart">
          <RefreshCw :size="14" />重新开始
        </UiButton></template
      >
    </PageHeader>
    <SettingGroup title="校园身份">
      <SettingRow title="身份类型"
        ><select
          v-model="degree"
          class="text-field"
          aria-label="身份类型"
          :disabled="busy || !!account"
          @change="restart"
        >
          <option value="undergraduate">本科生</option>
          <option value="postgraduate">研究生</option>
        </select></SettingRow
      >
      <SettingRow
        v-if="flow"
        title="连接方式"
        :description="
          flow.accessMode === 'webvpn'
            ? flow.step === 'webvpn'
              ? '当前网络无法直连校园服务，先完成 WebVPN 认证'
              : 'WebVPN 已通过，继续校园业务认证'
            : '通过校园服务直连认证'
        "
        ><span class="badge" :class="{ accent: flow.accessMode === 'direct' }">{{
          flow.accessMode === 'webvpn' ? 'WebVPN' : '直连'
        }}</span></SettingRow
      >
    </SettingGroup>
    <p v-if="error" class="notice" role="alert">{{ error }}</p>
    <p
      v-if="visibleMessage"
      class="notice"
      :class="{ success: visibleMessage === '验证码已发送' }"
      role="status"
    >
      {{ visibleMessage }}
    </p>
    <SettingGroup
      :title="
        !flow
          ? '连接校园服务'
          : flow.phase === 'mfa'
            ? '短信验证'
            : flow.phase === 'identity'
              ? '选择校园身份'
              : '统一身份认证'
      "
    >
      <EmptyState
        v-if="!flow"
        compact
        :icon="LoaderCircle"
        :busy="busy"
        :title="busy ? '正在检测校园网络…' : '连接尚未完成'"
      >
        <UiButton size="sm" v-if="!busy" @click="restart">重试</UiButton>
      </EmptyState>
      <form v-else class="campus-login-form" @submit.prevent="submit">
        <fieldset :disabled="busy">
          <template v-if="credentials">
            <div class="login-methods" role="tablist" aria-label="登录方式">
              <button
                type="button"
                role="tab"
                :aria-selected="method === 'password'"
                :class="{ active: method === 'password' }"
                @click="switchMethod('password')"
              >
                账号密码
              </button>
              <button
                type="button"
                role="tab"
                :aria-selected="method === 'phone'"
                :class="{ active: method === 'phone' }"
                @click="switchMethod('phone')"
              >
                手机号
              </button>
              <button
                type="button"
                role="tab"
                :aria-selected="method === 'qrcode'"
                :class="{ active: method === 'qrcode' }"
                @click="switchMethod('qrcode')"
              >
                扫码登录
              </button>
            </div>
            <template v-if="method === 'password'">
              <label for="campus-username">账号</label
              ><input
                id="campus-username"
                v-model="username"
                class="text-field"
                autocomplete="username"
                placeholder="NetID / 学号"
                maxlength="256"
                :readonly="useSaved"
                autocapitalize="none"
                spellcheck="false"
              />
              <label :for="useSaved ? undefined : 'campus-password'">密码</label>
              <div v-if="useSaved" class="saved-password">
                <ShieldCheck :size="16" /><span>使用本机保存的密码</span
                ><UiButton variant="ghost" size="sm" type="button" @click="enterPassword"
                  >更换</UiButton
                >
              </div>
              <div v-else class="field-shell">
                <input
                  id="campus-password"
                  v-model="password"
                  :type="showPassword ? 'text' : 'password'"
                  autocomplete="current-password"
                  :placeholder="'输入统一认证密码'"
                  maxlength="512"
                /><UiIconButton
                  :label="showPassword ? '隐藏密码' : '显示密码'"
                  size="sm"
                  type="button"
                  @click="showPassword = !showPassword"
                >
                  <EyeOff v-if="showPassword" :size="15" /><Eye v-else :size="15" />
                </UiIconButton>
              </div>
              <template v-if="flow.phase === 'captcha'">
                <label for="campus-captcha">图片验证码</label>
                <div class="captcha-field">
                  <input
                    id="campus-captcha"
                    v-model="captcha"
                    class="text-field"
                    autocomplete="off"
                    maxlength="32"
                  /><button
                    type="button"
                    class="captcha-image"
                    title="刷新验证码"
                    aria-label="刷新图片验证码"
                    @click="
                      captcha = '';
                      login.refreshCaptcha();
                    "
                  >
                    <img v-if="captchaImage" :src="captchaImage" alt="校园验证码" /><span v-else
                      >加载验证码</span
                    >
                  </button>
                </div>
              </template>
              <div v-if="!flow.canContinue || hasCredentials" class="login-remember">
                <div class="login-option">
                  <div>
                    <strong>保存账号密码</strong
                    ><small>登录成功后保存在本机，之后可直接重新登录</small>
                  </div>
                  <UiSwitch
                    :checked="remember"
                    label="保存账号密码"
                    :disabled="busy"
                    @toggle="remember = !remember"
                  />
                </div>
                <div class="login-option">
                  <div>
                    <strong>信任本设备</strong
                    ><small
                      >学校会记住这台电脑，短信验证通过一次后，之后登录通常不再要求验证码</small
                    >
                  </div>
                  <UiSwitch
                    :checked="trust"
                    label="信任本设备"
                    :disabled="busy"
                    @toggle="trust = !trust"
                  />
                </div>
              </div>
            </template>
            <template v-else-if="method === 'phone'">
              <label for="campus-phone">手机号</label>
              <input
                id="campus-phone"
                v-model="phone"
                class="text-field"
                inputmode="tel"
                autocomplete="tel"
                placeholder="11 位手机号"
                maxlength="11"
              />
              <label for="campus-sms-captcha">图片验证码</label>
              <div class="captcha-field">
                <input
                  id="campus-sms-captcha"
                  v-model="smsCaptcha"
                  class="text-field"
                  autocomplete="off"
                  maxlength="32"
                /><button
                  type="button"
                  class="captcha-image"
                  title="刷新验证码"
                  aria-label="刷新图片验证码"
                  @click="
                    smsCaptcha = '';
                    login.refreshCaptcha();
                  "
                >
                  <img v-if="captchaImage" :src="captchaImage" alt="校园验证码" /><span v-else
                    >加载验证码</span
                  >
                </button>
              </div>
              <label for="campus-phone-code">短信验证码</label>
              <div class="captcha-field">
                <input
                  id="campus-phone-code"
                  v-model="smsCode"
                  class="text-field"
                  inputmode="numeric"
                  autocomplete="one-time-code"
                  maxlength="8"
                /><UiButton
                  size="sm"
                  type="button"
                  :disabled="busy || smsRetrySeconds > 0 || !phoneValid"
                  @click="sendCode"
                >
                  {{ smsRetrySeconds ? smsRetrySeconds + ' 秒后重发' : '发送验证码' }}
                </UiButton>
              </div>
              <p class="login-help">
                先发送短信验证码，再输入收到的 4–8 位数字。手机号登录不会保存密码。
              </p>
            </template>
            <template v-else>
              <div class="qr-login">
                <div class="qr-frame">
                  <img v-if="qrImage" :src="qrImage" alt="扫码登录二维码" />
                  <span v-else>正在生成二维码…</span>
                </div>
                <div class="qr-copy">
                  <strong>使用手机扫码登录</strong>
                  <p>
                    {{
                      flow.message ||
                      '打开移动交通大学 App 扫描二维码并在手机上确认；微信扫一扫若能打开学校登录页也可使用。'
                    }}
                  </p>
                  <UiButton size="sm" type="button" :disabled="busy" @click="login.startQr()"
                    ><RefreshCw :size="14" />刷新二维码</UiButton
                  >
                </div>
              </div>
            </template>
          </template>
          <template v-else-if="flow.phase === 'mfa'">
            <p class="login-help">
              向学校绑定的手机{{ flow.phone ? '（' + flow.phone + '）' : '' }}发送验证码。
            </p>
            <label for="campus-sms">短信验证码</label>
            <div class="captcha-field">
              <input
                id="campus-sms"
                v-model="code"
                class="text-field"
                inputmode="numeric"
                autocomplete="one-time-code"
                maxlength="16"
              /><UiButton
                size="sm"
                type="button"
                :disabled="busy || retrySeconds > 0"
                @click="login.submit({ type: 'sendMfa' })"
              >
                {{ retrySeconds ? retrySeconds + ' 秒后重发' : '发送验证码' }}
              </UiButton>
            </div>
          </template>
          <template v-else-if="flow.phase === 'identity'">
            <p class="login-help">请选择本次使用的校园身份。</p>
            <label v-for="choice in flow.choices" :key="choice.label" class="identity-choice"
              ><input
                v-model="identity"
                type="radio"
                :value="choice.label"
                name="campus-identity"
              /><span
                >{{ choice.name }}<small>{{ choice.label }}</small></span
              ></label
            >
          </template>
          <p v-else class="login-help">学校认证已完成，重试保存本机账号。</p>
          <div class="login-actions">
            <UiButton type="button" @click="cancel">取消</UiButton
            ><UiButton
              v-if="!(credentials && method === 'qrcode')"
              variant="primary"
              :disabled="!canSubmit"
            >
              <LoaderCircle v-if="busy" :size="14" class="spinning" />{{
                busy
                  ? '处理中…'
                  : flow.phase === 'success'
                    ? '重试保存'
                    : flow.phase === 'mfa'
                      ? '验证并继续'
                      : flow.phase === 'identity'
                        ? '继续'
                        : '登录'
              }}
            </UiButton>
          </div>
        </fieldset>
      </form>
    </SettingGroup>
  </section>
</template>

<style scoped>
.login-methods {
  grid-column: 1 / -1;
  display: flex;
  gap: 4px;
  padding: 3px;
  border-radius: var(--radius);
  background: var(--control-sunken);
}
.login-methods button {
  flex: 1;
  border: 0;
  border-radius: var(--radius-sm);
  padding: 6px 8px;
  background: transparent;
  color: var(--text-secondary);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}
.login-methods button.active {
  background: var(--surface);
  color: var(--text);
  box-shadow: var(--shadow-xs);
}
.qr-login {
  grid-column: 1 / -1;
  display: flex;
  gap: 22px;
  align-items: center;
  padding: 4px 0 8px;
}
.qr-frame {
  display: grid;
  flex: none;
  place-items: center;
  width: 168px;
  height: 168px;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: #fff;
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.qr-frame img {
  display: block;
  width: 160px;
  height: 160px;
}
.qr-copy strong {
  font-size: 13px;
}
.qr-copy p {
  max-width: 280px;
  margin: 6px 0 12px;
  color: var(--text-secondary);
  font-size: 12px;
  line-height: 1.7;
}
@media (max-width: 560px) {
  .qr-login {
    flex-direction: column;
    align-items: flex-start;
  }
}
.login-back {
  margin-bottom: 12px;
}
.campus-login-form {
  padding: 22px;
}
.campus-login-form fieldset {
  display: grid;
  grid-template-columns: 90px minmax(0, 1fr);
  align-items: center;
  gap: 18px 16px;
  padding: 0;
  margin: 0;
  border: 0;
}
.campus-login-form label {
  font-size: 12px;
  color: var(--text-secondary);
}
.captcha-field,
.saved-password {
  display: flex;
  gap: 10px;
  align-items: center;
  min-width: 0;
}
.captcha-field .text-field {
  flex: 1;
  width: 80px;
}
.captcha-image {
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface);
  padding: 0;
  width: 110px;
  height: 36px;
  overflow: hidden;
  flex-shrink: 0;
  color: var(--text-secondary);
  font-size: 11px;
}
.captcha-image img {
  width: 100%;
  height: 100%;
  object-fit: contain;
}
.saved-password {
  font-size: 12px;
  color: var(--text-secondary);
}
.saved-password .btn {
  margin-left: auto;
}
.login-remember {
  grid-column: 1 / -1;
  display: grid;
  gap: 16px;
  padding-top: 16px;
  border-top: 1px solid var(--divider);
}
.login-option {
  display: flex;
  gap: 16px;
  align-items: center;
  justify-content: space-between;
}
.login-remember strong {
  font-size: 12px;
  font-weight: 500;
}
.login-remember small {
  display: block;
  font-size: 11px;
  color: var(--text-tertiary);
  margin-top: 5px;
  line-height: 1.6;
}
.login-help,
.identity-choice,
.login-actions {
  grid-column: 1 / -1;
}
.login-help {
  color: var(--text-secondary);
  font-size: 12px;
  line-height: 1.7;
}
.identity-choice {
  display: flex;
  gap: 12px;
  padding: 12px;
  border-radius: var(--radius-sm);
  background: var(--surface);
}
.identity-choice small {
  display: block;
  color: var(--text-tertiary);
  margin-top: 4px;
}
.login-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 8px;
}
</style>

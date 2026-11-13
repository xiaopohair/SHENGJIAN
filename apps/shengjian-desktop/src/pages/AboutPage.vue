<script setup lang="ts">
import { computed, onMounted, ref, shallowRef } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { ArrowUpCircle, Boxes, RefreshCw, ShieldCheck } from "lucide-vue-next";
import { getDeviceSupport, getHostCapabilities, openExternalUrl, errorMessage } from "../lib/api";
import { deviceProductNameKey } from "../lib/deviceProduct";
import { statusLabel, tr, trDynamic } from "../lib/i18n";
import { useUpdateStore } from "../stores/update";
import { usePairingStore } from "../stores/pairing";
import PageHeader from "../components/PageHeader.vue";
import Button from "../components/ui/Button.vue";
import Card from "../components/ui/Card.vue";
import Checkbox from "../components/ui/Checkbox.vue";
import EmptyState from "../components/ui/EmptyState.vue";
import Notice from "../components/ui/Notice.vue";
import SectionHeader from "../components/ui/SectionHeader.vue";
import StatusPill from "../components/ui/StatusPill.vue";
import type { DeviceSupport, HostCapabilities } from "../lib/types";
import BrandMark from "../components/BrandMark.vue";

type StatusTone = "neutral" | "success" | "warning" | "danger" | "info" | "accent";

const deviceSupport = ref<DeviceSupport[]>([]);
const capabilities = ref<HostCapabilities | null>(null);
const appVersion = shallowRef("");
const platformName = ref("windows");
const error = shallowRef<string | null>(null);
const updateStore = useUpdateStore();
const pairing = usePairingStore();
const updateState = computed(() => updateStore.status);
/** 参与版本比较的当前版本（与横幅「当前版本 → 最新版本」保持一致）。 */
const comparisonVersion = computed(() => updateStore.currentVersion || appVersion.value);
const latestRelease = computed(() => updateStore.latest);
const includePrereleases = computed({
  get: () => updateStore.includePrereleases,
  set: (value: boolean) => {
    void updateStore.setIncludePrereleases(value);
  },
});

const PLATFORM_NAMES: Record<string, string> = {
  windows: "Windows",
  macos: "macOS",
  linux: "Linux",
};

const capabilityItems = computed(() => {
  const caps = capabilities.value;
  if (!caps) return [];
  return [
    { label: "增强捕获", ok: caps.enhancedCapture },
    { label: "宏运行时", ok: caps.macroRuntime },
    { label: "托盘", ok: caps.tray },
    { label: "开机自启", ok: caps.autostart },
    { label: "常用语", ok: caps.commonPhrases },
  ];
});

function platformNameFor(id: string): string {
  return PLATFORM_NAMES[id] ?? id;
}

function supportDeviceName(device: DeviceSupport): string {
  const key = deviceProductNameKey(device.model);
  return key ? trDynamic(key) : device.displayName;
}

function supportTone(status: string): StatusTone {
  if (status === "supported" || status === "implemented") return "success";
  if (status === "partial" || status === "experimental" || status === "research") {
    return "warning";
  }
  if (status === "unsupported") return "danger";
  return "neutral";
}

function orderedPlatforms(device: DeviceSupport) {
  const priority = (platform: string, index: number) => {
    if (platform === platformName.value) return 0;
    if (platform === "windows") return 1;
    if (platform === "macos") return 2;
    if (platform === "linux") return 3;
    return 4 + index;
  };
  return [...device.platforms].sort(
    (left, right) =>
      priority(left.platform, device.platforms.indexOf(left)) -
      priority(right.platform, device.platforms.indexOf(right)),
  );
}

async function runUpdateCheck() {
  await updateStore.check(true);
}

async function openReleasePage() {
  if (!latestRelease.value) return;
  try {
    await openExternalUrl(latestRelease.value.url);
  } catch (cause) {
    updateStore.reportError(errorMessage(cause));
  }
}

/** 用户主动忽略这个版本：横幅、Toast 与托盘提示一起收起。 */
function dismissUpdate() {
  updateStore.dismiss();
}

onMounted(async () => {
  const [version, caps, devices] = await Promise.all([
    getVersion().catch(() => ""),
    getHostCapabilities().catch(() => null),
    getDeviceSupport().catch(() => []),
  ]);
  appVersion.value = version;
  capabilities.value = caps;
  if (caps) platformName.value = caps.platform;
  deviceSupport.value = devices;
  await updateStore.hydrate();
  updateStore.setCurrentVersion(version);
});
</script>

<template>
  <section class="page">
    <PageHeader
      eyebrow="About"
      title="关于"
      description="版本、运行环境、组件许可与支持的遥控器。"
    />

    <Notice v-if="error" kind="error">{{ error }}</Notice>

    <!-- 发现新版本：置顶横幅。比 hero 里的小胶囊更抓眼，直接给「查看更新」入口。
         用 hasUpdate 而不是 status === 'available'：用户点过「忽略此版本」后
         status 仍是 available，但横幅必须跟着一起收起。 -->
    <div v-if="updateStore.hasUpdate && latestRelease" class="update-banner" role="status">
      <span class="update-banner-icon" aria-hidden="true">
        <ArrowUpCircle :size="22" />
      </span>
      <div class="update-banner-copy">
        <p class="update-banner-title">
          {{ tr("发现新版本") }}
          <strong class="update-banner-version">{{ latestRelease.tagName }}</strong>
        </p>
        <p class="update-banner-sub">
          {{ tr("当前版本") }}
          {{ comparisonVersion ? `v${comparisonVersion}` : "—" }}
          <span class="update-banner-arrow" aria-hidden="true">→</span>
          {{ tr("最新版本") }} {{ latestRelease.tagName }}
        </p>
      </div>
      <div class="update-banner-actions">
        <Button size="sm" @click="openReleasePage">{{ tr("查看更新") }}</Button>
        <Button variant="ghost" size="sm" @click="dismissUpdate">{{ tr("忽略此版本") }}</Button>
      </div>
    </div>

    <Card data-about-hero>
      <div class="about-hero">
        <div class="about-brand">
          <BrandMark :height="84" />
          <div class="about-copy">
            <p class="about-roman">SHENGJIAN</p>
            <h2 class="about-name">{{ tr("声键") }}</h2>
            <p class="about-slogan">{{ tr("声音，就是快捷键。") }}</p>
            <p class="about-tagline">
              {{ tr("声键把小米蓝牙语音遥控器接进 Windows：语音输入、按键映射、宏与方案。") }}
            </p>
            <div class="about-pills">
              <StatusPill :label="appVersion ? `v${appVersion}` : '—'" tone="accent" />
              <StatusPill :label="platformNameFor(platformName)" tone="info" />
              <StatusPill :label="trDynamic('本地构建')" tone="neutral" />
              <StatusPill :label="trDynamic('未签名')" tone="warning" dot />
              <StatusPill
                v-if="updateState === 'checking'"
                :label="trDynamic('检查中')"
                tone="neutral"
                dot
              />
              <StatusPill
                v-else-if="updateState === 'current'"
                :label="trDynamic('已是最新')"
                tone="success"
                dot
              />
              <StatusPill
                v-else-if="updateState === 'unavailable'"
                :label="trDynamic('暂无发布')"
                tone="neutral"
                dot
              />
              <StatusPill
                v-else-if="updateState === 'error'"
                :label="trDynamic('检查失败')"
                tone="danger"
                dot
              />
            </div>
          </div>
        </div>
        <div class="about-side">
          <Checkbox v-model="includePrereleases" class="about-prerelease">
            {{ tr("包含预发布版本") }}
          </Checkbox>
          <div class="about-actions">
            <Button
              variant="primary"
              :disabled="updateState === 'checking'"
              @click="runUpdateCheck"
            >
              <RefreshCw :size="16" :class="{ spin: updateState === 'checking' }" />
              {{ trDynamic(updateState === "checking" ? "检查中" : "检查更新") }}
            </Button>
          </div>
        </div>
      </div>
    </Card>

    <Card>
      <SectionHeader title="运行信息" hint="当前版本与运行环境" />
      <div class="metric-grid about-metrics">
        <div class="metric">
          <p class="metric-label">{{ tr("应用版本") }}</p>
          <p class="metric-value">{{ appVersion ? `v${appVersion}` : "—" }}</p>
        </div>
        <div class="metric">
          <p class="metric-label">{{ tr("运行平台") }}</p>
          <p class="metric-value">{{ platformNameFor(platformName) }}</p>
        </div>
        <div class="metric">
          <p class="metric-label">{{ tr("构建") }}</p>
          <p class="metric-value">NSIS · x64</p>
        </div>
        <div class="metric">
          <p class="metric-label">{{ tr("签名") }}</p>
          <div class="metric-pill">
            <StatusPill :label="trDynamic('未签名')" tone="warning" dot />
          </div>
        </div>
      </div>
    </Card>

    <Card v-if="capabilityItems.length">
      <SectionHeader title="本机能力" hint="当前平台已启用的能力" />
      <div class="capability-grid">
        <div v-for="item in capabilityItems" :key="item.label" class="capability-row">
          <span class="capability-name">{{ trDynamic(item.label) }}</span>
          <StatusPill
            :label="trDynamic(item.ok ? '可用' : '不可用')"
            :tone="item.ok ? 'success' : 'neutral'"
            dot
          />
        </div>
      </div>
    </Card>

    <Card>
      <SectionHeader title="支持的遥控器" hint="已验证的型号与平台支持情况" />
      <ul v-if="deviceSupport.length" class="support-list">
        <li v-for="device in deviceSupport" :key="device.id" class="support-row">
          <div class="support-main">
            <div class="support-title">
              <span class="support-name">{{ supportDeviceName(device) }}</span>
              <span class="mono muted">{{ device.model }}</span>
            </div>
            <div class="support-platforms">
              <span
                v-for="item in orderedPlatforms(device)"
                :key="item.platform"
                class="support-platform"
              >
                <span>{{ platformNameFor(item.platform) }}</span>
                <StatusPill :label="statusLabel(item.status)" :tone="supportTone(item.status)" />
              </span>
            </div>
          </div>
          <div class="support-actions">
            <Button size="sm" variant="ghost" @click="pairing.show(device.model)">
              {{ tr("配对帮助") }}
            </Button>
            <StatusPill
              :label="statusLabel(device.status)"
              :tone="supportTone(device.status)"
              dot
            />
          </div>
        </li>
      </ul>
      <EmptyState
        v-else
        title="未读取到设备支持信息"
        description="请稍后重试，或打开日志目录查看诊断。"
      />
    </Card>

    <Card>
      <SectionHeader title="组件与许可" hint="随安装包分发的开源组件" />
      <ul class="about-list">
        <li class="about-list-row">
          <span class="about-list-name">
            <Boxes :size="15" />
            {{ tr("声键核心") }}
          </span>
          <span class="mono muted">Rust · GPL-3.0-only</span>
        </li>
        <li class="about-list-row">
          <span class="about-list-name">
            <Boxes :size="15" />
            {{ tr("桌面宿主") }}
          </span>
          <span class="mono muted">Tauri 2 · Vue 3</span>
        </li>
        <li class="about-list-row">
          <span class="about-list-name">
            <ShieldCheck :size="15" />
            RC003 helper
          </span>
          <span class="mono muted">Frida Gadget</span>
        </li>
      </ul>
      <p class="about-license">
        {{ tr("许可全文与第三方声明随安装包分发。") }}
      </p>
    </Card>
  </section>
</template>

<style scoped>
.about-hero {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  gap: var(--space-5);
}

.about-brand {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-5);
  min-width: 0;
}

.about-copy {
  flex: 1 1 0;
  min-width: 0;
}

.about-roman {
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: var(--text-2xs);
  font-weight: 600;
  letter-spacing: 0.22em;
  line-height: 1.3;
}

.about-name {
  margin-top: 3px;
  font-size: var(--text-2xl);
  font-weight: 650;
  letter-spacing: 0.02em;
  line-height: 1.2;
}

.about-slogan {
  margin-top: 6px;
  color: var(--text);
  font-size: var(--text-lg);
  font-weight: 600;
  letter-spacing: 0.01em;
  line-height: 1.4;
}

.about-tagline {
  margin-top: 6px;
  max-width: 100%;
  color: var(--text-muted);
  font-size: var(--text-sm);
  line-height: 1.6;
}

.about-pills {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  margin-top: var(--space-3);
}

.about-side {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  align-self: stretch;
  justify-content: space-between;
  gap: var(--space-4);
}

.about-prerelease {
  flex: 0 0 auto;
}

/* 「发现新版本」置顶横幅：品牌色描边 + 渐变浅底 + 左侧粗条，全页最醒目的元素。 */
.update-banner {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  border: 1px solid color-mix(in srgb, var(--accent) 60%, var(--border));
  border-left: 4px solid var(--accent);
  border-radius: var(--radius-md);
  background:
    linear-gradient(
      100deg,
      color-mix(in srgb, var(--accent-soft-strong) 90%, transparent),
      color-mix(in srgb, var(--accent-soft) 55%, transparent)
    ),
    var(--bg-elevated);
  padding: var(--space-3) var(--space-4);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent) 14%, transparent);
  animation: update-banner-in var(--duration-base) var(--ease-out);
}

.update-banner-icon {
  display: grid;
  flex: 0 0 auto;
  place-items: center;
  width: 36px;
  height: 36px;
  border-radius: 50%;
  background: color-mix(in srgb, var(--accent) 18%, transparent);
  color: var(--accent);
}

.update-banner-copy {
  flex: 1;
  min-width: 0;
}

.update-banner-title {
  margin: 0;
  color: var(--text);
  font-size: var(--text-md);
  font-weight: 700;
}

.update-banner-version {
  margin-left: 4px;
  color: var(--accent);
  font-family: var(--font-mono);
}

.update-banner-sub {
  margin: 2px 0 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.update-banner-arrow {
  margin: 0 4px;
  color: var(--accent);
}

.update-banner-actions {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: var(--space-2);
}

@keyframes update-banner-in {
  from {
    opacity: 0;
    transform: translateY(-6px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.about-actions {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: var(--space-2);
}

.about-metrics {
  margin-top: var(--space-4);
}

.metric-pill {
  margin-top: 3px;
}

.capability-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: var(--space-2);
  margin-top: var(--space-4);
}

.capability-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-subtle);
  padding: var(--space-2) var(--space-3);
}

.capability-name {
  color: var(--text-secondary);
  font-size: var(--text-sm);
}

.support-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  margin: var(--space-4) 0 0;
  padding: 0;
  list-style: none;
}

.support-row {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-3);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-3);
}

.support-main {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--space-2);
  min-width: 0;
}

.support-title {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: var(--space-2);
}

.support-name {
  font-weight: 600;
}

.support-platforms {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.support-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: flex-end;
  gap: var(--space-2);
}

.support-platform {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.about-list {
  display: flex;
  flex-direction: column;
  gap: 0;
  margin: var(--space-3) 0 0;
  padding: 0;
  list-style: none;
}

.about-list-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  border-bottom: 1px solid var(--border);
  padding: var(--space-2) 0;
}

.about-list-name {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  color: var(--text-secondary);
  font-size: var(--text-sm);
}

.about-license {
  margin-top: var(--space-3);
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.55;
}

@media (min-width: 1200px) {
  .about-actions {
    flex-direction: row;
    flex-wrap: wrap;
    justify-content: flex-end;
  }
}

/* Only collapse to a single column below the application minimum width, so the
   version/update side keeps its top-right / bottom-right corners at 1200px. */
@media (max-width: 1100px) {
  .about-hero {
    grid-template-columns: 1fr;
  }

  .about-brand {
    align-items: flex-start;
  }

  .about-side {
    width: 100%;
    align-self: auto;
    align-items: flex-start;
    justify-content: flex-start;
  }

  .about-actions {
    flex-direction: row;
    flex-wrap: wrap;
  }
}
</style>

/**
 * 「推荐按键方案」的本地状态：隐藏开关与「上次查看的版本」。
 *
 * 只存 localStorage，不进 settings/方案文件——隐藏是纯界面偏好，不需要跟配置备份
 * 一起迁移；版本记录用于判断工具栏按钮要不要显示「有更新」徽标。
 */
export const RECOMMENDED_HIDDEN_KEY = "shengjian.recommended-profiles.hidden";
export const RECOMMENDED_SEEN_VERSION_KEY = "shengjian.recommended-profiles.seen-version";

function readStorage(key: string): string {
  try {
    return window.localStorage.getItem(key) ?? "";
  } catch {
    return "";
  }
}

function writeStorage(key: string, value: string | null): void {
  try {
    if (value === null) {
      window.localStorage.removeItem(key);
    } else {
      window.localStorage.setItem(key, value);
    }
  } catch {
    // localStorage 不可用时只是少了一层记忆，不影响主流程。
  }
}

/** 用户是否在推荐对话框里勾选了「不再显示推荐方案」。 */
export function isRecommendedProfilesHidden(): boolean {
  return readStorage(RECOMMENDED_HIDDEN_KEY) === "1";
}

export function setRecommendedProfilesHidden(hidden: boolean): void {
  writeStorage(RECOMMENDED_HIDDEN_KEY, hidden ? "1" : null);
}

/** 上次看到过的推荐包版本；空串表示从没看过。 */
export function recommendedSeenVersion(): string {
  return readStorage(RECOMMENDED_SEEN_VERSION_KEY).trim();
}

export function rememberRecommendedVersion(version: string | null | undefined): void {
  writeStorage(RECOMMENDED_SEEN_VERSION_KEY, (version ?? "").trim() || null);
}

/**
 * 当前推荐包是否比上次查看的新。
 * 版本号是不透明字符串（如 "2026.11.1"），只做相等比较，不解析大小。
 */
export function hasRecommendedUpdate(
  bundleVersion: string | null | undefined,
  seenVersion: string = recommendedSeenVersion(),
): boolean {
  const version = (bundleVersion ?? "").trim();
  return version.length > 0 && version !== seenVersion.trim();
}

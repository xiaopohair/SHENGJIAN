/**
 * 「推荐按键方案」的本地状态：上次查看的版本。
 *
 * 只存 localStorage，不进 settings/方案文件；版本记录用于判断工具栏按钮
 * 要不要显示「有更新」徽标。
 */
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

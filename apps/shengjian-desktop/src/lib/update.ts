export const RELEASES_API_URL =
  "https://api.github.com/repos/xiaopohair/SHENGJIAN/releases?per_page=30";

export interface GitHubRelease {
  tag_name: string;
  name: string | null;
  html_url: string;
  draft: boolean;
  prerelease: boolean;
  published_at: string | null;
}

export interface UpdateRelease {
  version: string;
  tagName: string;
  name: string;
  url: string;
  prerelease: boolean;
  publishedAt: string | null;
}

export interface UpdateCheckResult {
  currentVersion: string;
  latest: UpdateRelease | null;
  updateAvailable: boolean;
}

interface ParsedVersion {
  major: number;
  minor: number;
  patch: number;
  prerelease: string[];
}

function parseVersion(raw: string): ParsedVersion | null {
  const match = /^v?(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?/.exec(raw.trim());
  if (!match) return null;
  return {
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
    prerelease: match[4] ? match[4].split(".") : [],
  };
}

function comparePrerelease(left: string[], right: string[]): number {
  if (left.length === 0 && right.length === 0) return 0;
  if (left.length === 0) return 1;
  if (right.length === 0) return -1;

  const length = Math.max(left.length, right.length);
  for (let index = 0; index < length; index += 1) {
    const leftPart = left[index];
    const rightPart = right[index];
    if (leftPart === undefined) return -1;
    if (rightPart === undefined) return 1;

    const leftNumber = /^\d+$/.test(leftPart) ? Number(leftPart) : null;
    const rightNumber = /^\d+$/.test(rightPart) ? Number(rightPart) : null;

    if (leftNumber !== null && rightNumber !== null) {
      if (leftNumber !== rightNumber) return leftNumber < rightNumber ? -1 : 1;
    } else if (leftNumber !== null) {
      return -1;
    } else if (rightNumber !== null) {
      return 1;
    } else if (leftPart !== rightPart) {
      return leftPart < rightPart ? -1 : 1;
    }
  }
  return 0;
}

export function compareVersions(left: string, right: string): number {
  const parsedLeft = parseVersion(left);
  const parsedRight = parseVersion(right);
  if (!parsedLeft || !parsedRight) return 0;

  for (const key of ["major", "minor", "patch"] as const) {
    if (parsedLeft[key] !== parsedRight[key]) {
      return parsedLeft[key] < parsedRight[key] ? -1 : 1;
    }
  }
  return comparePrerelease(parsedLeft.prerelease, parsedRight.prerelease);
}

export function selectLatestRelease(
  releases: GitHubRelease[],
  includePrereleases: boolean,
): UpdateRelease | null {
  const candidates = releases
    .filter((release) => !release.draft)
    .filter((release) => includePrereleases || !release.prerelease)
    .map((release) => ({
      version: release.tag_name.replace(/^v/i, ""),
      tagName: release.tag_name,
      name: release.name || release.tag_name,
      url: release.html_url,
      prerelease: release.prerelease,
      publishedAt: release.published_at,
    }))
    .filter((release) => parseVersion(release.version) !== null)
    .sort((left, right) => compareVersions(right.version, left.version));

  return candidates[0] ?? null;
}

export async function fetchReleases(): Promise<GitHubRelease[]> {
  const response = await fetch(RELEASES_API_URL, {
    headers: { Accept: "application/vnd.github+json" },
  });
  if (!response.ok) {
    throw new Error(`GitHub Releases API returned ${response.status}`);
  }
  const data = (await response.json()) as unknown;
  if (!Array.isArray(data)) {
    throw new Error("GitHub Releases API returned an invalid payload");
  }
  return data as GitHubRelease[];
}

export async function checkForUpdates(
  currentVersion: string,
  includePrereleases: boolean,
): Promise<UpdateCheckResult> {
  const latest = selectLatestRelease(await fetchReleases(), includePrereleases);
  return {
    currentVersion,
    latest,
    updateAvailable: latest !== null && compareVersions(latest.version, currentVersion) > 0,
  };
}

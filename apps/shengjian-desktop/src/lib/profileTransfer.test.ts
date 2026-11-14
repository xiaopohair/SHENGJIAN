import { describe, expect, it } from "vitest";
import type { ButtonProfile, ButtonProfilesDocument } from "./types";
import {
  buildImportRows,
  composeImportDocument,
  dateStamp,
  estimateDocumentBytes,
  formatKilobytes,
  importCounts,
  importedTotal,
  presetIdsFor,
  profileStats,
  resolveImportRows,
  transferSummary,
} from "./profileTransfer";

function profile(overrides: Partial<ButtonProfile> = {}): ButtonProfile {
  return {
    id: "profile-a",
    name: "方案 A",
    appTargets: [],
    autoApply: false,
    deviceModel: "RC003",
    bindings: {},
    ...overrides,
  };
}

function withBindings(buttons: string[], action: Record<string, unknown> = { type: "disabled" }) {
  const bindings: Record<string, Record<string, unknown>> = {};
  for (const button of buttons) bindings[button] = { single: action };
  return bindings as ButtonProfile["bindings"];
}

describe("profileStats", () => {
  it("counts binds, apps and referenced macros", () => {
    const value = profile({
      appTargets: ["chrome.exe", "  ", "PotPlayer"],
      bindings: withBindings(["ok", "back"], { type: "run-macro", macro_id: "macro.play" }),
    });
    const stats = profileStats(value);
    expect(stats.binds).toBe(2);
    expect(stats.appCount).toBe(2);
    expect(stats.macroIds).toEqual(["macro.play"]);
  });

  it("summarises a set of profiles", () => {
    const summary = transferSummary([
      profile({ bindings: withBindings(["ok", "back"]) }),
      profile({
        id: "p2",
        bindings: withBindings(["menu"], { type: "run-macro", macro_id: "m1" }),
      }),
    ]);
    expect(summary).toEqual({ count: 2, binds: 3, macros: 1 });
  });
});

describe("export helpers", () => {
  it("estimates a readable size", () => {
    const bytes = estimateDocumentBytes({ profiles: [profile()] });
    expect(bytes).toBeGreaterThan(10);
    expect(formatKilobytes(bytes)).toMatch(/KB$/);
  });

  it("stamps the local date", () => {
    expect(dateStamp(new Date(2026, 10, 14))).toBe("2026-11-14");
  });

  it("builds unique preset ids per model", () => {
    const ids = presetIdsFor(
      [
        profile({ id: "a", name: "客厅 观影", deviceModel: "RC003" }),
        profile({ id: "b", name: "客厅 观影", deviceModel: "RC003" }),
        profile({ id: "c", name: "通用方案", deviceModel: null }),
      ],
      "ARN9",
    );
    expect(ids.get("a")).toBe("preset.rc003.客厅-观影");
    expect(ids.get("b")).toBe("preset.rc003.客厅-观影-2");
    expect(ids.get("c")).toBe("preset.arn9.通用方案");
  });
});

describe("buildImportRows", () => {
  const current = [
    profile({ id: "default-rc003", name: "默认", isDefault: true }),
    profile({ id: "profile-a", name: "方案 A" }),
    profile({ id: "profile-b", name: "方案 B", deviceModel: null }),
  ];

  it("separates updates, conflicts and new profiles", () => {
    const rows = buildImportRows(
      [
        profile({ id: "profile-a", name: "方案 A" }),
        profile({ id: "profile-c", name: "方案 C" }),
        profile({ id: "profile-b-copy", name: "方案 B" }),
      ],
      current,
      "RC003",
    );
    expect(rows.map((row) => row.status)).toEqual(["update", "new", "conflict"]);
    expect(rows[0].matchId).toBe("profile-a");
    expect(rows[2].matchId).toBe("profile-b");
  });

  it("flags incompatible models and locked defaults", () => {
    const rows = buildImportRows(
      [
        profile({ id: "arn", name: "ARN 方案", deviceModel: "ARN9" }),
        profile({ id: "default-rc003", name: "默认", isDefault: true }),
      ],
      current,
      "RC003",
    );
    expect(rows[0].compatible).toBe(false);
    expect(rows[1].locked).toBe(true);
  });
});

describe("resolveImportRows", () => {
  const rows = buildImportRows(
    [
      profile({ id: "profile-a", name: "方案 A" }),
      profile({ id: "profile-b-copy", name: "方案 B" }),
      profile({ id: "arn", name: "ARN 方案", deviceModel: "ARN9" }),
    ],
    [
      profile({ id: "profile-a", name: "方案 A" }),
      profile({ id: "profile-b", name: "方案 B", deviceModel: null }),
    ],
    "RC003",
  );

  it("follows the policy for conflicts and skips incompatible rows", () => {
    const resolved = resolveImportRows(rows, {
      policy: "keep",
      overrides: {},
      onlyCompatible: true,
    });
    expect(resolved.map((row) => row.action)).toEqual(["overwrite", "keep", "skip"]);
    const counts = importCounts(resolved);
    expect(importedTotal(counts)).toBe(2);
    expect(counts).toEqual({ add: 0, overwrite: 1, keep: 1, skip: 1 });
  });

  it("lets a per-row choice win over the policy", () => {
    const resolved = resolveImportRows(rows, {
      policy: "overwrite",
      overrides: { "profile-b-copy": "skip" },
      onlyCompatible: false,
    });
    expect(resolved.map((row) => row.action)).toEqual(["overwrite", "skip", "add"]);
  });
});

describe("composeImportDocument", () => {
  const current: ButtonProfilesDocument = {
    schemaVersion: 2,
    activeManualProfileId: "profile-a",
    profiles: [
      profile({ id: "profile-a", name: "方案 A" }),
      profile({ id: "profile-b", name: "方案 B", deviceModel: null }),
    ],
  };

  it("keeps the local id when overwriting so references stay valid", () => {
    const rows = resolveImportRows(
      buildImportRows(
        [profile({ id: "incoming-a", name: "方案 A", autoApply: true })],
        current.profiles,
        "RC003",
      ),
      { policy: "overwrite", overrides: {}, onlyCompatible: false },
    );
    const next = composeImportDocument(current, rows);
    expect(next.profiles).toHaveLength(2);
    const replaced = next.profiles.find((item) => item.name === "方案 A");
    expect(replaced?.id).toBe("profile-a");
    expect(replaced?.autoApply).toBe(true);
    expect(next.activeManualProfileId).toBe("profile-a");
  });

  it("adds a renamed copy when keeping both", () => {
    const rows = resolveImportRows(
      buildImportRows([profile({ id: "incoming-b", name: "方案 B" })], current.profiles, "RC003"),
      { policy: "keep", overrides: {}, onlyCompatible: false },
    );
    const next = composeImportDocument(current, rows);
    expect(next.profiles).toHaveLength(3);
    expect(next.profiles[2].name).toBe("方案 B（副本）");
    expect(next.profiles[2].id).toBe("incoming-b");
    // 原方案没被动过。
    expect(next.profiles[1].id).toBe("profile-b");
  });

  it("never removes local profiles when everything is skipped", () => {
    const rows = resolveImportRows(
      buildImportRows([profile({ id: "x", name: "新方案" })], current.profiles, "RC003"),
      { policy: "overwrite", overrides: { x: "skip" }, onlyCompatible: false },
    );
    const next = composeImportDocument(current, rows);
    expect(next.profiles.map((item) => item.id)).toEqual(["profile-a", "profile-b"]);
  });
});

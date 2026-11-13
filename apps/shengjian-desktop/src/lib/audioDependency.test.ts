import { describe, expect, it } from "vitest";
import {
  audioDependencyDetailKey,
  audioDependencyTitleKey,
  audioDependencyTone,
  blocksOnboardingProgress,
  cableEndpointNames,
  isAudioDependencyReady,
  mergeAudioEndpoints,
  suggestedEndpointId,
  VB_CABLE_INSTALL_URL,
} from "./audioDependency";
import { hasTranslation } from "./i18n";
import type { AudioDependencyStatus, AudioEndpoint } from "./types";

function endpoint(id: string, name = id): AudioEndpoint {
  return { id, name, recommended: false };
}

function status(overrides: Partial<AudioDependencyStatus> = {}): AudioDependencyStatus {
  return {
    state: "ready",
    cableEndpoints: [endpoint("cable-in", "CABLE Input")],
    suggestedEndpointId: "cable-in",
    installUrl: "https://vb-audio.com/Cable/",
    ...overrides,
  };
}

describe("audio dependency helpers", () => {
  it("maps each state to a translated title and detail", () => {
    for (const state of ["ready", "missing", "disabled"] as const) {
      expect(hasTranslation(audioDependencyTitleKey(state))).toBe(true);
      const detail = audioDependencyDetailKey(state);
      if (detail) expect(hasTranslation(detail)).toBe(true);
    }
    expect(hasTranslation(audioDependencyTitleKey(null))).toBe(true);
    expect(audioDependencyDetailKey(null)).toBeNull();
    expect(audioDependencyTitleKey("ready")).toBe("已检测到 VB-CABLE");
    expect(audioDependencyTitleKey("missing")).toBe("未检测到 VB-CABLE，远程语音将没有声音");
  });

  it("only treats ready as satisfied", () => {
    expect(isAudioDependencyReady(status())).toBe(true);
    for (const state of ["missing", "disabled"] as const) {
      expect(isAudioDependencyReady(status({ state }))).toBe(false);
    }
    expect(isAudioDependencyReady(null)).toBe(false);
  });

  it("blocks only the onboarding step 3 when not ready", () => {
    expect(blocksOnboardingProgress(3, status({ state: "missing" }))).toBe(true);
    expect(blocksOnboardingProgress(3, status({ state: "disabled" }))).toBe(true);
    expect(blocksOnboardingProgress(3, null)).toBe(true);
    expect(blocksOnboardingProgress(3, status())).toBe(false);
    expect(blocksOnboardingProgress(2, status({ state: "missing" }))).toBe(false);
    expect(blocksOnboardingProgress(4, status({ state: "disabled" }))).toBe(false);
  });

  it("suggests an endpoint only when ready", () => {
    expect(suggestedEndpointId(status())).toBe("cable-in");
    expect(suggestedEndpointId(status({ suggestedEndpointId: "  " }))).toBeNull();
    expect(suggestedEndpointId(status({ state: "missing" }))).toBeNull();
    expect(suggestedEndpointId(null)).toBeNull();
  });

  it("merges CABLE endpoints by id without disturbing the original list", () => {
    const base = [endpoint("speakers"), endpoint("cable-in", "CABLE Input")];
    const merged = mergeAudioEndpoints(base, [
      endpoint("cable-in", "CABLE Input (dup)"),
      endpoint("cable-out", "CABLE Output"),
    ]);

    expect(merged.map((item) => item.id)).toEqual(["speakers", "cable-in", "cable-out"]);
    expect(merged[1].name).toBe("CABLE Input");
    expect(base).toHaveLength(2);
  });

  it("describes the tone and cable endpoint names", () => {
    expect(audioDependencyTone("ready")).toBe("success");
    expect(audioDependencyTone("missing")).toBe("warning");
    expect(audioDependencyTone("disabled")).toBe("warning");
    expect(audioDependencyTone(undefined)).toBe("neutral");
    expect(cableEndpointNames(status())).toBe("CABLE Input");
    expect(cableEndpointNames(status({ cableEndpoints: [] }))).toBe("");
  });

  it("has a fallback install url for old backends", () => {
    expect(VB_CABLE_INSTALL_URL).toContain("vb-audio.com");
  });
});

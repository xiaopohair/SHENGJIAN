import { describe, expect, it } from "vitest";
import { shouldHintVoiceSelfForeground } from "./voiceFocusHint";

const base = {
  voiceState: "streaming",
  previousVoiceState: "idle",
  onboardingOpen: false,
  windowFocused: true,
  alreadyHinted: false,
};

describe("shouldHintVoiceSelfForeground", () => {
  it("warns when a voice session starts while our own window is focused", () => {
    expect(shouldHintVoiceSelfForeground(base)).toBe(true);
  });

  it("stays quiet when another window is in the foreground", () => {
    expect(shouldHintVoiceSelfForeground({ ...base, windowFocused: false })).toBe(false);
  });

  it("stays quiet during onboarding: its step 3 records voice inside our own window", () => {
    expect(shouldHintVoiceSelfForeground({ ...base, onboardingOpen: true })).toBe(false);
  });

  it("warns once per session, not on every streaming update", () => {
    expect(shouldHintVoiceSelfForeground({ ...base, alreadyHinted: true })).toBe(false);
    expect(shouldHintVoiceSelfForeground({ ...base, previousVoiceState: "streaming" })).toBe(false);
  });

  it("ignores non-streaming states", () => {
    for (const state of ["idle", "starting", "stopping", "unknown"]) {
      expect(shouldHintVoiceSelfForeground({ ...base, voiceState: state })).toBe(false);
    }
  });
});

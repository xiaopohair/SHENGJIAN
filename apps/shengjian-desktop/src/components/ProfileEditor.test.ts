/** @vitest-environment jsdom */
import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import ProfileEditor from "./ProfileEditor.vue";
import type { ProfileDraft } from "../lib/buttons";
import { setLanguagePreference } from "../lib/i18n";

setLanguagePreference("zh-Hans");

const draft: ProfileDraft = {
  id: "profile-test",
  name: "Test",
  appTargets: "",
  autoApply: false,
  holdRepeat: "",
  voiceInputTool: "",
  bindings: {},
};

describe("ProfileEditor", () => {
  it("renders the full binding matrix and marks edited rows", async () => {
    const baselineJson = JSON.stringify(draft);
    const wrapper = mount(ProfileEditor, {
      props: {
        draft,
        editingNew: true,
        busy: false,
        dirty: false,
        baselineJson,
        foregroundApp: "",
        commonPhrasesAvailable: false,
        latestMacros: [],
      },
    });

    expect(wrapper.findAll(".binding-row")).toHaveLength(12);
    expect(wrapper.text()).toContain("已修改 0 个按键");

    await wrapper.find(".binding-cell select").setValue("disabled");
    expect(draft.bindings.back?.single?.type).toBe("disabled");
    expect(wrapper.findAll(".binding-row.modified")).toHaveLength(1);
    wrapper.unmount();
  });
});

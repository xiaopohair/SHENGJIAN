/** @vitest-environment jsdom */
import { beforeEach, describe, expect, it } from "vitest";
import { LANGUAGE_STORAGE_KEY, setLanguagePreference } from "./i18n";

describe("splash language preference", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("mirrors the in-app language for the next launch", () => {
    setLanguagePreference("en");
    expect(localStorage.getItem(LANGUAGE_STORAGE_KEY)).toBe("en");

    setLanguagePreference("zh-Hans");
    expect(localStorage.getItem(LANGUAGE_STORAGE_KEY)).toBe("zh-Hans");

    setLanguagePreference("system");
    expect(localStorage.getItem(LANGUAGE_STORAGE_KEY)).toBe("system");
  });
});

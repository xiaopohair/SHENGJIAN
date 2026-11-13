import { setLanguagePreference } from "./lib/i18n";

// Tests assert Chinese source strings in several places. Force a deterministic
// locale so CI (en-US) and local machines (zh-CN) behave the same; individual
// tests that exercise English call setLanguagePreference("en") themselves.
setLanguagePreference("zh-Hans");

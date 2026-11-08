import pluginVue from "eslint-plugin-vue";
import vueTsEslintConfig from "@vue/eslint-config-typescript";

export default [
  {
    ignores: ["dist/**", "src-tauri/**", "node_modules/**", "coverage/**"],
  },
  ...pluginVue.configs["flat/essential"],
  ...vueTsEslintConfig(),
  {
    rules: {
      "vue/multi-word-component-names": "off",
      "vue/no-v-html": "error",
      "@typescript-eslint/no-explicit-any": "warn",
    },
  },
];

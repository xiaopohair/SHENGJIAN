import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import "./styles/tokens.css";
import "./styles/global.css";

// The remote voice key is F5; WebView2 would otherwise reload the whole UI and
// wipe the event history right when the voice session starts.
window.addEventListener(
  "keydown",
  (event) => {
    if (event.key === "F5" || (event.ctrlKey && event.key.toLowerCase() === "r")) {
      event.preventDefault();
      event.stopPropagation();
    }
  },
  { capture: true },
);

createApp(App).use(createPinia()).mount("#app");

// Lucide icons are decorative unless they carry their own accessible name.
// Mark them once and as Vue renders new icons, so the accessibility tree stays
// focused on labels, not "image" nodes.
function hideDecorativeIcons(root: ParentNode) {
  root.querySelectorAll("svg").forEach((icon) => {
    if (!icon.hasAttribute("aria-label") && !icon.hasAttribute("role")) {
      icon.setAttribute("aria-hidden", "true");
    }
  });
}

hideDecorativeIcons(document);
new MutationObserver((mutations) => {
  for (const mutation of mutations) {
    mutation.addedNodes.forEach((node) => {
      if (node instanceof SVGElement) {
        if (!node.hasAttribute("aria-label") && !node.hasAttribute("role")) {
          node.setAttribute("aria-hidden", "true");
        }
      } else if (node instanceof Element) {
        hideDecorativeIcons(node);
      }
    });
  }
}).observe(document.body, { childList: true, subtree: true });

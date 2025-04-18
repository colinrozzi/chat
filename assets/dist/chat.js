import { initializeApp } from "./components/app.js";
import { setupEventListeners } from "./components/events.js";
import { checkMobileView } from "./utils/responsive.js";
document.addEventListener("DOMContentLoaded", () => {
  initializeApp();
  setupEventListeners();
  checkMobileView();
  window.addEventListener("resize", checkMobileView);
});

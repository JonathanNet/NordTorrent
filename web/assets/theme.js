(function () {
  "use strict";

  var THEME_KEY = "nordtorrent.theme";
  var LANGUAGE_KEY = "nordtorrent.language";
  var root = document.documentElement;
  var preference = "system";
  try {
    var saved = localStorage.getItem(THEME_KEY);
    if (saved === "system" || saved === "dark" || saved === "light") preference = saved;
  } catch (error) {}

  try {
    var language = localStorage.getItem(LANGUAGE_KEY);
    if (language === "da" || language === "en") {
      root.lang = language;
      root.dataset.language = language;
    }
  } catch (error) {}

  var media = window.matchMedia ? window.matchMedia("(prefers-color-scheme: light)") : null;

  function resolved() {
    if (preference === "light" || preference === "dark") return preference;
    return media && media.matches ? "light" : "dark";
  }

  function apply() {
    var theme = resolved();
    root.dataset.theme = theme;
    root.dataset.themePreference = preference;
    root.style.colorScheme = theme;
    var meta = document.querySelector('meta[name="theme-color"]');
    if (meta) meta.setAttribute("content", theme === "light" ? "#f4f6f0" : "#0a0d10");
  }

  window.__nordtorrentTheme = {
    getPreference: function () {
      return preference;
    },
    setPreference: function (value) {
      preference = value === "light" || value === "dark" ? value : "system";
      try {
        localStorage.setItem(THEME_KEY, preference);
      } catch (error) {}
      apply();
    },
    resolved: resolved,
  };

  apply();
  if (media) {
    var onChange = function () {
      if (preference === "system") apply();
    };
    if (media.addEventListener) media.addEventListener("change", onChange);
    else if (media.addListener) media.addListener(onChange);
  }
})();

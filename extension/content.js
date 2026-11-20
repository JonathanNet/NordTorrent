(() => {
  const MARKER = "__nordtorrent_magnet__";
  const MAGNET_RE = /magnet:\?[^\s"'<>]+/i;
  let lastMagnet = "";
  let lastAt = 0;

  function extract(value) {
    const text = String(value || "");
    const match = text.match(MAGNET_RE);
    return match ? match[0].slice(0, 8192) : "";
  }

  function report(value) {
    const magnet = extract(value);
    if (!magnet) return;
    const now = Date.now();
    if (magnet === lastMagnet && now - lastAt < 1500) return;
    lastMagnet = magnet;
    lastAt = now;
    try {
      chrome.runtime.sendMessage({ type: "magnet", magnet });
    } catch (error) {
      // Extension was reloaded; the next copy will reconnect.
    }
  }

  document.addEventListener(
    "copy",
    () => {
      const selection = String(window.getSelection?.() || "");
      const active = document.activeElement;
      const activeValue =
        active && "value" in active ? String(active.value || "") : "";
      report(`${selection}\n${activeValue}`);
    },
    true
  );

  document.addEventListener(
    "click",
    (event) => {
      const target = event.target;
      if (!(target instanceof Element)) return;
      const element = target.closest(
        "a, button, [data-clipboard-text], [data-magnet], [data-url], [data-link]"
      );
      if (!element) return;
      const values = [
        element.getAttribute("href"),
        element.getAttribute("value"),
        element.getAttribute("data-clipboard-text"),
        element.getAttribute("data-magnet"),
        element.getAttribute("data-url"),
        element.getAttribute("data-link"),
      ];
      const selector = element.getAttribute("data-clipboard-target");
      if (selector) {
        const source = document.querySelector(selector);
        if (source) values.push(source.getAttribute("value"), source.textContent);
      }
      report(values.join("\n"));
    },
    true
  );

  window.addEventListener("message", (event) => {
    if (event.source !== window || !event.data || typeof event.data !== "object") return;
    const value = event.data[MARKER];
    if (typeof value === "string") report(value);
  });
})();

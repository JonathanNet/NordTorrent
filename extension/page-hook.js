(() => {
  const MARKER = "__nordtorrent_magnet__";

  function report(text) {
    const value = String(text || "").trim();
    if (!/^magnet:\?/i.test(value)) return;
    window.postMessage({ [MARKER]: value }, "*");
  }

  try {
    const clipboard = navigator.clipboard;
    if (clipboard && typeof clipboard.writeText === "function") {
      const original = clipboard.writeText.bind(clipboard);
      clipboard.writeText = async (text) => {
        report(text);
        return original(text);
      };
    }
  } catch (error) {
    // Some pages lock down navigator.clipboard; the copy listener still works.
  }

  try {
    const originalExec = document.execCommand.bind(document);
    document.execCommand = (command, ...args) => {
      if (String(command).toLowerCase() === "copy") {
        const selected = String(window.getSelection?.() || "").trim();
        report(selected);
      }
      return originalExec(command, ...args);
    };
  } catch (error) {
    // Ignore pages that do not allow wrapping execCommand.
  }
})();

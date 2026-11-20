const HANDLED_KEY = "handledTorrentDownloads";
const WORKER_STARTED_AT = Date.now();
const STARTUP_GRACE_MS = 15000;
let activePopupWindowId = null;
let popupEnabled = true;

chrome.storage.local.get(["popupEnabled"]).then((stored) => {
  popupEnabled = stored.popupEnabled !== false;
});

chrome.storage.onChanged.addListener((changes, area) => {
  if (area === "local" && changes.popupEnabled) {
    popupEnabled = changes.popupEnabled.newValue !== false;
  }
});

chrome.runtime.onInstalled.addListener(async () => {
  const stored = await chrome.storage.local.get(["popupEnabled"]);
  if (typeof stored.popupEnabled !== "boolean") {
    popupEnabled = true;
    await chrome.storage.local.set({ popupEnabled: true });
  }
});

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (
    !message ||
    message.type !== "magnet" ||
    typeof message.magnet !== "string" ||
    !/^magnet:\?/i.test(message.magnet) ||
    message.magnet.length > 8192
  ) {
    return false;
  }
  openWindow(`popup.html?magnet=${encodeURIComponent(message.magnet)}`)
    .then(() => sendResponse({ ok: true }))
    .catch(() => sendResponse({ ok: false }));
  return true;
});

chrome.downloads.onDeterminingFilename.addListener((item) => {
  if (!isTorrentDownload(item)) return;
  if (popupEnabled) {
    cancelDownload(item.id);
    return;
  }
  // The cached value may be stale if the worker was asleep when the user
  // changed the setting.  Re-check before leaving the download alone.
  chrome.storage.local.get(["popupEnabled"]).then((stored) => {
    popupEnabled = stored.popupEnabled !== false;
    if (popupEnabled) cancelDownload(item.id);
  });
});

function cancelDownload(downloadId) {
  try {
    chrome.downloads.cancel(downloadId, () => {
      // Read lastError so a download that has already disappeared does not
      // produce "Unchecked runtime.lastError: Invalid downloadId".
      void chrome.runtime.lastError;
    });
  } catch (error) {
    // The download is already gone.
  }
}

chrome.downloads.onCreated.addListener(async (item) => {
  if (!isTorrentDownload(item)) return;
  if (item.state && item.state !== "in_progress" && item.state !== "interrupted") {
    return;
  }

  // Chrome restores the download list on startup and can replay onCreated for
  // old .torrent downloads.  Those are not new user actions and must not open
  // a popup window (or a long list of windows).
  const startedAt = Date.parse(item.startTime || "");
  if (Number.isFinite(startedAt) && startedAt < WORKER_STARTED_AT - STARTUP_GRACE_MS) {
    return;
  }
  if (await wasHandled(item.id)) return;

  // Read the setting fresh: a service worker can be asleep when the user
  // flips the toggle, so the cached value may still be stale.
  const stored = await chrome.storage.local.get(["popupEnabled"]);
  popupEnabled = stored.popupEnabled !== false;
  if (!popupEnabled) return;

  await markHandled(item.id);
  await openWindow(`popup.html?downloadId=${encodeURIComponent(item.id)}`);
});

chrome.windows.onRemoved.addListener((windowId) => {
  if (activePopupWindowId === windowId) activePopupWindowId = null;
});

function isTorrentDownload(item) {
  const filename = String(item.filename || "").toLowerCase();
  const url = String(item.finalUrl || item.url || "").toLowerCase();
  return (
    filename.endsWith(".torrent") ||
    /\.torrent(?:$|[?#])/.test(url) ||
    String(item.mime || "").toLowerCase() === "application/x-bittorrent"
  );
}

async function wasHandled(downloadId) {
  const stored = await chrome.storage.local.get([HANDLED_KEY]);
  const handled = Array.isArray(stored[HANDLED_KEY]) ? stored[HANDLED_KEY] : [];
  return handled.includes(downloadId);
}

async function markHandled(downloadId) {
  const stored = await chrome.storage.local.get([HANDLED_KEY]);
  const handled = Array.isArray(stored[HANDLED_KEY]) ? stored[HANDLED_KEY] : [];
  handled.push(downloadId);
  await chrome.storage.local.set({ [HANDLED_KEY]: handled.slice(-100) });
}

async function openWindow(relativeUrl) {
  const url = chrome.runtime.getURL(relativeUrl);

  // Keep exactly one NordTorrent popup window.  A burst of downloads must not
  // create a wall of windows that Chrome restores on the next startup.
  if (activePopupWindowId != null) {
    try {
      await chrome.windows.remove(activePopupWindowId);
    } catch (error) {
      activePopupWindowId = null;
    }
  }

  try {
    const created = await chrome.windows.create({
      url,
      type: "popup",
      width: 445,
      height: 720,
      focused: true,
    });
    activePopupWindowId = created.id ?? null;
  } catch (error) {
    console.error("Could not open NordTorrent window", error);
  }
}

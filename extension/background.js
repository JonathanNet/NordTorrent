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
  try {
    if (!isTorrentDownload(item)) return;
    if (popupEnabled) {
      cancelDownload(item.id);
      return;
    }
    // The cached value may be stale if the worker was asleep when the user
    // changed the setting.  Re-check before leaving the download alone.
    chrome.storage.local
      .get(["popupEnabled"])
      .then((stored) => {
        popupEnabled = stored.popupEnabled !== false;
        if (popupEnabled) cancelDownload(item.id);
      })
      .catch((error) => console.debug("NordTorrent: storage read failed", error));
  } catch (error) {
    console.debug("NordTorrent: onDeterminingFilename failed", error);
  }
});

// Ids we already tried to cancel, so a replayed event cannot trigger a second
// cancel on a download that is no longer in progress.
const cancelAttempts = new Map();
const CANCEL_TTL_MS = 10000;

async function cancelDownload(downloadId) {
  if (!Number.isInteger(downloadId) || downloadId < 0) return;

  const previous = cancelAttempts.get(downloadId);
  if (previous && Date.now() - previous < CANCEL_TTL_MS) return;
  cancelAttempts.set(downloadId, Date.now());
  if (cancelAttempts.size > 200) {
    const cutoff = Date.now() - CANCEL_TTL_MS;
    for (const [id, at] of cancelAttempts) {
      if (at < cutoff) cancelAttempts.delete(id);
    }
  }

  // Only an in-progress download can be cancelled.  Checking first avoids
  // "Download must be in progress" when the file was already fetched, which
  // otherwise shows up as an unchecked runtime error.
  try {
    const found = await chrome.downloads
      .search({ id: downloadId })
      .catch(() => []);
    const item = Array.isArray(found) ? found[0] : null;
    if (!item || item.state !== "in_progress") return;
  } catch (error) {
    return;
  }

  try {
    const pending = chrome.downloads.cancel(downloadId);
    if (pending && typeof pending.catch === "function") {
      pending.catch((error) => {
        console.debug("NordTorrent: cancel ignored", error?.message || error);
      });
    } else {
      chrome.downloads.cancel(downloadId, () => {
        if (chrome.runtime.lastError) {
          console.debug("NordTorrent: cancel ignored", chrome.runtime.lastError.message);
        }
      });
    }
  } catch (error) {
    try {
      chrome.downloads.cancel(downloadId, () => {
        if (chrome.runtime.lastError) {
          console.debug("NordTorrent: cancel ignored", chrome.runtime.lastError.message);
        }
      });
    } catch (fallbackError) {
      console.debug("NordTorrent: cancel threw", fallbackError);
    }
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

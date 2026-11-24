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
  // Nothing to do here: cancelling inside this event is what made Chrome
  // report "Unchecked runtime.lastError".  onCreated below handles it in its
  // own task instead.
  void item;
});

chrome.downloads.onCreated.addListener(async (item) => {
  try {
    if (!isTorrentDownload(item)) return;
    if (item.state && item.state !== "in_progress") return;

    // Read the setting fresh: a service worker can be asleep when the user
    // flips the toggle, so the cached value may still be stale.
    const stored = await chrome.storage.local.get(["popupEnabled"]);
    popupEnabled = stored.popupEnabled !== false;
    if (!popupEnabled) return;

    scheduleCancel(item.id);

    // Chrome restores the download list on startup and can replay onCreated
    // for old .torrent downloads.  Those are not new user actions and must not
    // open a popup window (or a long list of windows).
    const startedAt = Date.parse(item.startTime || "");
    if (Number.isFinite(startedAt) && startedAt < WORKER_STARTED_AT - STARTUP_GRACE_MS) {
      return;
    }
    if (await wasHandled(item.id)) return;

    await markHandled(item.id);
    const params = new URLSearchParams({
      downloadId: String(item.id),
      name: String(item.filename || "").split(/[\\/]/).pop() || "",
      url: String(item.finalUrl || item.url || ""),
    });
    await openWindow(`popup.html?${params.toString()}`);
  } catch (error) {
    console.debug("NordTorrent: onCreated failed", error);
  }
});

// Ids we already tried to cancel, so a replayed event cannot trigger a second
// cancel on a download that is no longer in progress.
const cancelAttempts = new Map();
const CANCEL_TTL_MS = 10000;

function cancelDownload(downloadId, state) {
  if (!Number.isInteger(downloadId) || downloadId < 1) return;
  if (state && state !== "in_progress") return;

  const previous = cancelAttempts.get(downloadId);
  if (previous && Date.now() - previous < CANCEL_TTL_MS) return;
  cancelAttempts.set(downloadId, Date.now());
  if (cancelAttempts.size > 200) {
    const cutoff = Date.now() - CANCEL_TTL_MS;
    for (const [id, at] of cancelAttempts) {
      if (at < cutoff) cancelAttempts.delete(id);
    }
  }

  // Callback form with lastError read inside the callback: that is the only
  // way Chrome treats the error as handled.
  try {
    chrome.downloads.cancel(downloadId, () => {
      if (chrome.runtime.lastError) {
        console.debug("NordTorrent: cancel ignored", chrome.runtime.lastError.message);
      }
    });
  } catch (error) {
    console.debug("NordTorrent: cancel threw", error);
  }
}

function scheduleCancel(downloadId) {
  if (!Number.isInteger(downloadId) || downloadId < 1) return;
  setTimeout(() => cancelDownload(downloadId, "in_progress"), 0);
}

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

(() => {
  const DEFAULT_SERVER = "http://192.168.1.10:8080";
  const MAX_TORRENT_BYTES = 20 * 1024 * 1024;
  const DEFAULT_LANGUAGE = "en";

  const STRINGS = {
    en: {
      "header.subtitle": "Download helper",
      "common.server": "Server address",
      "common.username": "Username",
      "common.password": "Password",
      "common.label": "Label",
      "common.noLabel": "No label",
      "common.popupToggle": "Open popup for .torrent downloads",
      "common.signOut": "Sign out",

      "setup.eyebrow": "CONNECT",
      "setup.title": "Connect to your server",
      "setup.lead":
        "Sign in with your NordTorrent account. The password is not stored - only the login token.",
      "setup.submit": "Sign in and save",

      "settings.eyebrow": "SETTINGS",
      "settings.title": "Settings",
      "settings.lead": "You stay signed in. Changes here never sign you out.",
      "settings.language": "Language",
      "settings.savedNow": "Saved immediately.",
      "settings.save": "Save settings",
      "settings.back": "Back",
      "settings.serverNote": "Changing the server requires a new sign-in.",
      "settings.popupOn": "On: the popup opens automatically for .torrent downloads.",
      "settings.popupOff":
        "Off: .torrent downloads continue as a normal download in Chrome.",

      "main.none": "No .torrent found",
      "main.noneSub": "Download a .torrent to get started.",
      "main.new": "New",
      "main.ready": "Ready",
      "main.added": "Added",
      "main.notFound": "Download not found",
      "main.selectManually": "Select the .torrent file manually below.",
      "main.fromHost": "From {host}",
      "main.fetchingFile": "Fetching the .torrent file…",

      "magnet.eyebrow": "MAGNET LINK",
      "magnet.label": "Magnet link or 40-character info hash",
      "magnet.add": "Add magnet",

      "folder.eyebrow": "DOWNLOAD FOLDER",
      "folder.up": "Go up",
      "folder.search": "Search this folder…",
      "folder.clear": "Clear search",
      "folder.choose": "Choose this folder",
      "folder.pick": "Select a folder on the server.",
      "folder.selected": "Selected: {path}",
      "folder.loading": "Loading folders…",
      "folder.searchEmpty": "No folders match the search.",
      "folder.emptyHere": "No subfolders here.",
      "folder.loadFailed": "Could not load download folders.",

      "actions.pick": "Choose .torrent file",
      "actions.add": "Add to server",
      "actions.adding": "Adding…",
      "footer.open": "Open NordTorrent",

      "status.magnetReady": "Magnet link ready. Pick a folder and add it.",
      "status.fetching": "Fetching the .torrent file…",
      "status.fetchManual":
        "Could not fetch automatically. Select the .torrent file manually.",
      "status.fetchManualUrl":
        "Cannot fetch this download automatically. Select the file manually.",
      "status.torrentReady": "Torrent file ready. Pick a folder and add it.",
      "status.tooLarge": "The torrent file is larger than 20 MB.",
      "status.empty": "The torrent file is empty.",
      "status.html": "Received a web page instead of a torrent.",
      "status.badDownload": "Download responded {status}.",
      "status.addingMagnet": "Adding magnet link…",
      "status.magnetAdded": "Magnet added: {name} in {folder}",
      "status.uploading": "Uploading torrent to the server…",
      "status.added": "Added: {name} in {folder}",
      "status.noMagnet": "Paste a magnet link or a 40-character info hash.",
      "status.chooseFolderFirst": "Pick a download folder first.",
      "status.addMagnetFailed": "Could not add the magnet link.",
      "status.addTorrentFailed": "Could not add the torrent.",
      "status.popupOn": "Popup enabled.",
      "status.popupOff": "Popup disabled. Downloads continue in Chrome.",
      "status.sessionExpired": "Your session expired. Sign in again.",
      "status.serverResponded": "The server responded {status}.",
      "status.noToken": "The server returned no token.",
      "status.missingCredentials": "Enter a username and password.",
      "status.connecting": "Connecting…",
      "status.connectFailed": "Could not reach the server.",
      "status.serverChanged": "The server changed. Sign in again on the new server.",
    },
    da: {
      "header.subtitle": "Download-hjælper",
      "common.server": "Serveradresse",
      "common.username": "Brugernavn",
      "common.password": "Adgangskode",
      "common.label": "Label",
      "common.noLabel": "Ingen label",
      "common.popupToggle": "Åbn popup ved .torrent-downloads",
      "common.signOut": "Log ud",

      "setup.eyebrow": "FORBIND",
      "setup.title": "Forbind til serveren",
      "setup.lead":
        "Log ind med din NordTorrent-konto. Adgangskoden gemmes ikke – kun login-tokenet.",
      "setup.submit": "Log ind og gem",

      "settings.eyebrow": "INDSTILLINGER",
      "settings.title": "Indstillinger",
      "settings.lead": "Du er stadig logget ind. Ændringer her logger dig ikke ud.",
      "settings.language": "Sprog",
      "settings.savedNow": "Gemmes med det samme.",
      "settings.save": "Gem indstillinger",
      "settings.back": "Tilbage",
      "settings.serverNote": "Skift af server kræver et nyt login.",
      "settings.popupOn": "Slået til: popup åbner automatisk ved .torrent-downloads.",
      "settings.popupOff":
        "Slået fra: .torrent-downloads fortsætter som en normal download.",

      "main.none": "Ingen .torrent fundet",
      "main.noneSub": "Download en .torrent for at komme i gang.",
      "main.new": "Ny",
      "main.ready": "Klar",
      "main.added": "Tilføjet",
      "main.notFound": "Download ikke fundet",
      "main.selectManually": "Vælg .torrent-filen manuelt nedenfor.",
      "main.fromHost": "Fra {host}",
      "main.fetchingFile": "Henter .torrent-filen…",

      "magnet.eyebrow": "MAGNET-LINK",
      "magnet.label": "Magnet-link eller 40-tegns info-hash",
      "magnet.add": "Tilføj magnet",

      "folder.eyebrow": "DOWNLOADMAPPE",
      "folder.up": "Gå op",
      "folder.search": "Søg i denne mappe…",
      "folder.clear": "Ryd søgning",
      "folder.choose": "Vælg denne mappe",
      "folder.pick": "Vælg en mappe på serveren.",
      "folder.selected": "Valgt: {path}",
      "folder.loading": "Henter mapper…",
      "folder.searchEmpty": "Ingen mapper matcher søgningen.",
      "folder.emptyHere": "Ingen undermapper her.",
      "folder.loadFailed": "Kunne ikke hente downloadmapper.",

      "actions.pick": "Vælg .torrent-fil",
      "actions.add": "Føj til server",
      "actions.adding": "Tilføjer…",
      "footer.open": "Åbn NordTorrent",

      "status.magnetReady": "Magnet-link klar. Vælg en mappe og tilføj.",
      "status.fetching": "Henter .torrent-filen…",
      "status.fetchManual": "Kunne ikke hente automatisk. Vælg .torrent-filen manuelt.",
      "status.fetchManualUrl":
        "Kan ikke hente denne download automatisk. Vælg filen manuelt.",
      "status.torrentReady": "Torrentfilen er klar. Vælg en mappe og tilføj.",
      "status.tooLarge": "Torrentfilen er større end 20 MB.",
      "status.empty": "Torrentfilen er tom.",
      "status.html": "Fik en webside i stedet for en torrent.",
      "status.badDownload": "Download svarede {status}.",
      "status.addingMagnet": "Tilføjer magnet-link…",
      "status.magnetAdded": "Magnet tilføjet: {name} i {folder}",
      "status.uploading": "Uploader torrent til serveren…",
      "status.added": "Tilføjet: {name} i {folder}",
      "status.noMagnet": "Indsæt et magnet-link eller en 40-tegns info-hash.",
      "status.chooseFolderFirst": "Vælg en downloadmappe først.",
      "status.addMagnetFailed": "Kunne ikke tilføje magnet-linket.",
      "status.addTorrentFailed": "Kunne ikke tilføje torrenten.",
      "status.popupOn": "Popup er slået til.",
      "status.popupOff": "Popup er slået fra. Downloads fortsætter i Chrome.",
      "status.sessionExpired": "Sessionen er udløbet. Log ind igen.",
      "status.serverResponded": "Serveren svarede {status}.",
      "status.noToken": "Serveren returnerede ingen token.",
      "status.missingCredentials": "Udfyld brugernavn og adgangskode.",
      "status.connecting": "Forbinder…",
      "status.connectFailed": "Kunne ikke forbinde til serveren.",
      "status.serverChanged": "Serveren blev ændret. Log ind igen på den nye server.",
    },
  };

  const state = {
    server: "",
    token: "",
    username: "",
    popupEnabled: true,
    language: DEFAULT_LANGUAGE,
    downloadId: null,
    download: null,
    bytes: null,
    fileName: "",
    currentPath: "",
    currentDisplay: "/downloads",
    parentPath: null,
    folderEntries: [],
    folderQuery: "",
    selectedPath: null,
    selectedDisplay: "",
    labels: [],
    selectedLabel: "",
  };

  const $ = (selector) => document.querySelector(selector);

  function t(key, vars) {
    const table = STRINGS[state.language] || STRINGS[DEFAULT_LANGUAGE];
    let text = table[key] ?? STRINGS[DEFAULT_LANGUAGE][key] ?? key;
    if (vars) {
      for (const [name, value] of Object.entries(vars)) {
        text = text.split(`{${name}}`).join(String(value));
      }
    }
    return text;
  }

  function applyI18n(root = document) {
    document.documentElement.lang = state.language;
    for (const node of root.querySelectorAll("[data-i18n]")) {
      node.textContent = t(node.dataset.i18n);
    }
    for (const node of root.querySelectorAll("[data-i18n-title]")) {
      node.title = t(node.dataset.i18nTitle);
      if (node.hasAttribute("aria-label")) {
        node.setAttribute("aria-label", node.title);
      }
    }
    for (const node of root.querySelectorAll("[data-i18n-placeholder]")) {
      node.placeholder = t(node.dataset.i18nPlaceholder);
    }
  }

  function applyLanguage(language) {
    state.language = STRINGS[language] ? language : DEFAULT_LANGUAGE;
    applyI18n();
    const select = $("#language-select");
    if (select) select.value = state.language;
  }

  document.addEventListener("DOMContentLoaded", init);

  async function init() {
    const stored = await chrome.storage.local.get([
      "server",
      "token",
      "username",
      "popupEnabled",
      "language",
    ]);
    state.server = stored.server || DEFAULT_SERVER;
    state.token = stored.token || "";
    state.username = stored.username || "";
    state.popupEnabled = stored.popupEnabled !== false;
    state.language = STRINGS[stored.language] ? stored.language : DEFAULT_LANGUAGE;

    bindEvents();
    applyLanguage(state.language);

    const params = new URLSearchParams(location.search);
    const queryId = Number(params.get("downloadId"));
    state.downloadId = Number.isFinite(queryId) && queryId > 0 ? queryId : null;
    const queryMagnet = params.get("magnet") || "";
    if (/^magnet:\?/i.test(queryMagnet)) {
      $("#magnet-input").value = queryMagnet;
    }
    await chrome.storage.local.remove(["lastTorrentDownloadId"]);

    const version = chrome.runtime.getManifest?.()?.version;
    if (version && $("#extension-version")) {
      $("#extension-version").textContent = `v${version}`;
    }

    $("#server-input").value = state.server;
    $("#username-input").value = state.username;
    syncPopupToggles();

    if (state.token) {
      showMain();
      await loadLabels();
      await loadDownload();
      await loadFolder("");
      if ($("#magnet-input").value.trim()) {
        setStatus(t("status.magnetReady"), false);
      }
    } else {
      showSetup();
    }
  }

  function bindEvents() {
    $("#setup-form").addEventListener("submit", onSetupSubmit);
    $("#settings-button").addEventListener("click", showSettings);
    $("#settings-form").addEventListener("submit", onSettingsSubmit);
    $("#settings-back").addEventListener("click", () => {
      if (state.token) showMain();
      else showSetup();
    });
    $("#settings-logout").addEventListener("click", logout);
    $("#language-select").addEventListener("change", onLanguageChange);
    $("#folder-up").addEventListener("click", () => {
      if (state.parentPath !== null) loadFolder(state.parentPath);
    });
    $("#folder-search").addEventListener("input", (event) => {
      state.folderQuery = event.target.value;
      renderFolderEntries();
    });
    $("#folder-search-clear").addEventListener("click", () => {
      state.folderQuery = "";
      $("#folder-search").value = "";
      $("#folder-search").focus();
      renderFolderEntries();
    });
    $("#choose-folder").addEventListener("click", chooseFolder);
    $("#pick-file").addEventListener("click", () => $("#file-fallback").click());
    $("#file-fallback").addEventListener("change", onFilePicked);
    $("#add-button").addEventListener("click", addTorrent);
    $("#magnet-add").addEventListener("click", addMagnet);
    $("#magnet-input").addEventListener("input", updateAddButton);
    $("#label-select").addEventListener("change", (event) => {
      state.selectedLabel = event.target.value;
    });
    $("#logout-button").addEventListener("click", logout);
    $("#popup-toggle").addEventListener("change", (event) => {
      setPopupEnabled(event.target.checked);
    });
    $("#settings-popup-toggle").addEventListener("change", (event) => {
      setPopupEnabled(event.target.checked);
    });
    $("#main-popup-toggle").addEventListener("change", (event) => {
      setPopupEnabled(event.target.checked);
    });
  }

  async function onLanguageChange(event) {
    const language = STRINGS[event.target.value] ? event.target.value : DEFAULT_LANGUAGE;
    await chrome.storage.local.set({ language });
    state.language = language;
    // A reload is the simplest way to retranslate every dynamically built
    // string; nothing unsaved is lost because settings are stored instantly.
    location.reload();
  }

  function normalizeServer(raw) {
    let value = String(raw || "").trim();
    if (!value) value = DEFAULT_SERVER;
    if (!/^https?:\/\//i.test(value)) value = `http://${value}`;
    return value.replace(/\/+$/, "");
  }

  function showSetup() {
    $("#setup-view").hidden = false;
    $("#settings-view").hidden = true;
    $("#main-view").hidden = true;
    $("#server-input").value = state.server || DEFAULT_SERVER;
    $("#username-input").value = state.username || "";
    $("#password-input").value = "";
    syncPopupToggles();
    setTimeout(() => $("#password-input").focus(), 50);
  }

  function showMain() {
    $("#setup-view").hidden = true;
    $("#settings-view").hidden = true;
    $("#main-view").hidden = false;
    syncPopupToggles();
    $("#open-server").href = state.server || DEFAULT_SERVER;
  }

  function showSettings() {
    $("#setup-view").hidden = true;
    $("#main-view").hidden = true;
    $("#settings-view").hidden = false;
    $("#settings-server").value = state.server || DEFAULT_SERVER;
    $("#language-select").value = state.language;
    syncPopupToggles();
    $("#settings-error").hidden = true;
  }

  async function onSettingsSubmit(event) {
    event.preventDefault();
    const server = normalizeServer($("#settings-server").value);
    const popupEnabled = $("#settings-popup-toggle").checked;
    const serverChanged = server !== state.server;
    state.server = server;
    state.popupEnabled = popupEnabled;
    await chrome.storage.local.set({ server, popupEnabled });
    if (serverChanged && state.token) {
      state.token = "";
      await chrome.storage.local.remove(["token"]);
      showSetup();
      const error = $("#setup-error");
      error.textContent = t("status.serverChanged");
      error.hidden = false;
      return;
    }
    showMain();
  }

  async function onSetupSubmit(event) {
    event.preventDefault();
    const error = $("#setup-error");
    error.hidden = true;
    const server = normalizeServer($("#server-input").value);
    const username = $("#username-input").value.trim();
    const password = $("#password-input").value;
    if (!username || !password) {
      error.textContent = t("status.missingCredentials");
      error.hidden = false;
      return;
    }
    setBusy($("#setup-submit"), true, t("status.connecting"));
    try {
      const response = await fetch(`${server}/api/v1/auth/login`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-NordTorrent-Client": "chrome-extension",
          "X-NordTorrent-Client-Version":
            chrome.runtime.getManifest()?.version || "",
        },
        body: JSON.stringify({ username, password }),
      });
      const payload = await response.json().catch(() => null);
      if (!response.ok) {
        throw new Error(
          payload?.error?.message || t("status.serverResponded", { status: response.status })
        );
      }
      if (!payload?.access_token) throw new Error(t("status.noToken"));
      state.server = server;
      state.token = payload.access_token;
      state.username = username;
      state.popupEnabled = $("#popup-toggle").checked;
      await chrome.storage.local.set({
        server,
        token: state.token,
        username,
        popupEnabled: state.popupEnabled,
      });
      showMain();
      await loadLabels();
      await loadDownload();
      await loadFolder("");
      if ($("#magnet-input").value.trim()) {
        setStatus(t("status.magnetReady"), false);
      }
    } catch (err) {
      error.textContent = err?.message || t("status.connectFailed");
      error.hidden = false;
    } finally {
      setBusy($("#setup-submit"), false, t("setup.submit"));
    }
  }

  function syncPopupToggles() {
    const enabled = state.popupEnabled;
    for (const selector of [
      "#popup-toggle",
      "#settings-popup-toggle",
      "#main-popup-toggle",
    ]) {
      const input = $(selector);
      if (input) input.checked = enabled;
    }
    const hint = $("#settings-popup-hint");
    if (hint) {
      hint.textContent = t(enabled ? "settings.popupOn" : "settings.popupOff");
    }
  }

  async function setPopupEnabled(enabled) {
    state.popupEnabled = Boolean(enabled);
    syncPopupToggles();
    await chrome.storage.local.set({ popupEnabled: state.popupEnabled });
    setStatus(
      t(state.popupEnabled ? "status.popupOn" : "status.popupOff"),
      false,
      true
    );
  }

  async function logout() {
    state.token = "";
    state.bytes = null;
    await chrome.storage.local.remove(["token"]);
    showSetup();
  }

  async function loadDownload() {
    $("#download-card").classList.remove("success");
    $("#download-state").textContent = t("main.new");
    $("#main-status").hidden = true;
    state.bytes = null;
    state.fileName = "";
    $("#add-button").disabled = true;
    if (!state.downloadId) {
      $("#download-name").textContent = t("main.none");
      $("#download-source").textContent = t("main.noneSub");
      $("#pick-file").hidden = false;
      return;
    }
    let item = null;
    try {
      // .catch() keeps a stale id from surfacing as an unchecked runtime error.
      const found = await chrome.downloads.search({ id: state.downloadId }).catch(() => []);
      item = Array.isArray(found) && found.length ? found[0] : null;
    } catch (error) {
      console.warn("Could not read download", error);
    }
    state.download = item;
    if (!item) {
      $("#download-name").textContent = t("main.notFound");
      $("#download-source").textContent = t("main.selectManually");
      $("#pick-file").hidden = false;
      return;
    }
    state.fileName = filenameFromDownload(item);
    $("#download-name").textContent = state.fileName || "Torrent download";
    try {
      const host = new URL(item.finalUrl || item.url).host;
      $("#download-source").textContent = t("main.fromHost", { host });
    } catch (error) {
      $("#download-source").textContent = t("main.fetchingFile");
    }
    $("#pick-file").hidden = true;
    await fetchTorrent(item);
  }

  async function fetchTorrent(item) {
    const url = item.finalUrl || item.url || "";
    if (!/^https?:\/\//i.test(url)) {
      showFetchFallback(t("status.fetchManualUrl"));
      return;
    }
    setStatus(t("status.fetching"), false);
    try {
      const response = await fetch(url, {
        credentials: "include",
        cache: "no-store",
      });
      if (!response.ok) {
        throw new Error(t("status.badDownload", { status: response.status }));
      }
      const contentType = String(response.headers.get("content-type") || "").toLowerCase();
      if (contentType.includes("text/html")) throw new Error(t("status.html"));
      const buffer = await response.arrayBuffer();
      if (buffer.byteLength > MAX_TORRENT_BYTES) throw new Error(t("status.tooLarge"));
      if (buffer.byteLength < 8) throw new Error(t("status.empty"));
      acceptBytes(buffer, state.fileName || "download.torrent");
    } catch (error) {
      showFetchFallback(t("status.fetchManual"));
    }
  }

  function showFetchFallback(message) {
    $("#pick-file").hidden = false;
    setStatus(message, true);
  }

  function torrentFileName(fileName) {
    const base = String(fileName || "download.torrent").split(/[\\/]/).pop() || "download.torrent";
    return /\.torrent$/i.test(base) ? base : base + ".torrent";
  }

  function acceptBytes(buffer, fileName) {
    state.bytes = buffer;
    state.fileName = torrentFileName(fileName);
    $("#download-name").textContent = state.fileName;
    $("#download-state").textContent = t("main.ready");
    $("#pick-file").hidden = true;
    setStatus(t("status.torrentReady"), false);
    updateAddButton();
  }

  async function onFilePicked(event) {
    const file = event.target.files && event.target.files[0];
    event.target.value = "";
    if (!file) return;
    if (file.size > MAX_TORRENT_BYTES) {
      setStatus(t("status.tooLarge"), true);
      return;
    }
    const buffer = await file.arrayBuffer();
    acceptBytes(buffer, file.name);
  }

  async function loadLabels() {
    try {
      const response = await apiFetch("/api/v1/labels");
      if (!response.ok) return;
      const data = await response.json();
      state.labels = Array.isArray(data?.labels) ? data.labels : [];
      renderLabelSelect();
    } catch (error) {
      // Labels are optional; adding a torrent still works without them.
    }
  }

  function renderLabelSelect() {
    const select = $("#label-select");
    if (!select) return;
    const fragment = document.createDocumentFragment();
    const none = document.createElement("option");
    none.value = "";
    none.textContent = t("common.noLabel");
    fragment.appendChild(none);
    for (const label of state.labels) {
      const option = document.createElement("option");
      option.value = label;
      option.textContent = label;
      fragment.appendChild(option);
    }
    select.replaceChildren(fragment);
    if (!state.labels.includes(state.selectedLabel)) state.selectedLabel = "";
    select.value = state.selectedLabel;
  }

  async function loadFolder(path) {
    $("#folder-entries").replaceChildren();
    $("#folder-status").textContent = t("folder.loading");
    $("#folder-status").classList.remove("error-text");
    state.folderQuery = "";
    state.folderEntries = [];
    $("#folder-search").value = "";
    state.selectedPath = null;
    state.selectedDisplay = "";
    updateAddButton();
    try {
      const response = await apiFetch(
        `/api/v1/download-browser?path=${encodeURIComponent(path || "")}`
      );
      if (!response.ok) throw new Error(await errorText(response));
      const data = await response.json();
      state.currentPath = data.path || "";
      state.currentDisplay = data.display_path || "/downloads";
      state.parentPath = data.parent;
      $("#current-path").textContent = `${state.currentDisplay}/`;
      $("#folder-up").disabled = data.parent === null;
      renderEntries(Array.isArray(data.entries) ? data.entries : []);
      $("#folder-status").textContent = t("folder.pick");
    } catch (error) {
      $("#folder-status").textContent = error?.message || t("folder.loadFailed");
      $("#folder-status").classList.add("error-text");
    }
  }

  function renderEntries(entries) {
    state.folderEntries = Array.isArray(entries) ? entries : [];
    renderFolderEntries();
  }

  function renderFolderEntries() {
    const host = $("#folder-entries");
    host.replaceChildren();
    const query = state.folderQuery.trim().toLocaleLowerCase();
    const entries = query
      ? state.folderEntries.filter((entry) =>
          String(entry.name || "")
            .toLocaleLowerCase()
            .includes(query)
        )
      : state.folderEntries;
    if (!entries.length) {
      const empty = document.createElement("p");
      empty.className = "empty";
      empty.textContent = state.folderQuery.trim()
        ? t("folder.searchEmpty")
        : t("folder.emptyHere");
      host.appendChild(empty);
      return;
    }
    for (const entry of entries) {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "folder-entry";
      const icon = document.createElement("span");
      icon.className = "folder-icon";
      icon.textContent = "▸";
      const label = document.createElement("span");
      label.textContent = entry.name;
      button.append(icon, label);
      button.addEventListener("click", () => loadFolder(entry.path));
      host.appendChild(button);
    }
  }

  function chooseFolder() {
    state.selectedPath = state.currentPath || ".";
    state.selectedDisplay = `${state.currentDisplay}/`;
    $("#folder-status").textContent = t("folder.selected", {
      path: state.selectedDisplay,
    });
    $("#folder-status").classList.remove("error-text");
    updateAddButton();
  }

  function updateAddButton() {
    $("#add-button").disabled = !(state.token && state.bytes && state.selectedPath);
    $("#magnet-add").disabled = !(
      state.token &&
      state.selectedPath &&
      $("#magnet-input").value.trim()
    );
  }

  async function addMagnet() {
    const magnet = $("#magnet-input").value.trim();
    const hash = /^[a-f0-9]{40}$/i.test(magnet);
    if (!magnet || (!magnet.startsWith("magnet:?") && !hash)) {
      setStatus(t("status.noMagnet"), true);
      return;
    }
    if (!state.selectedPath) {
      setStatus(t("status.chooseFolderFirst"), true);
      return;
    }
    setBusy($("#magnet-add"), true, t("actions.adding"));
    setStatus(t("status.addingMagnet"), false);
    try {
      const response = await apiFetch("/api/v1/torrents", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          magnet,
          paused: false,
          folder: state.selectedPath,
          label: state.selectedLabel || null,
        }),
      });
      const payload = await response.json().catch(() => null);
      if (!response.ok) {
        throw new Error(
          payload?.error?.message || t("status.serverResponded", { status: response.status })
        );
      }
      $("#magnet-input").value = "";
      setStatus(
        t("status.magnetAdded", {
          name: payload?.name || "torrent",
          folder: state.selectedDisplay,
        }),
        false,
        true
      );
    } catch (error) {
      setStatus(error?.message || t("status.addMagnetFailed"), true);
    } finally {
      setBusy($("#magnet-add"), false, t("magnet.add"));
      updateAddButton();
    }
  }

  async function addTorrent() {
    if (!state.bytes || !state.selectedPath) return;
    setBusy($("#add-button"), true, t("actions.adding"));
    setStatus(t("status.uploading"), false);
    try {
      const form = new FormData();
      form.append(
        "file",
        new Blob([state.bytes], { type: "application/x-bittorrent" }),
        state.fileName
      );
      form.append("paused", "false");
      form.append("folder", state.selectedPath);
      form.append("label", state.selectedLabel || "");
      const response = await apiFetch("/api/v1/torrents/upload", {
        method: "POST",
        body: form,
      });
      const payload = await response.json().catch(() => null);
      if (!response.ok) {
        throw new Error(
          payload?.error?.message || t("status.serverResponded", { status: response.status })
        );
      }
      $("#download-card").classList.add("success");
      $("#download-state").textContent = t("main.added");
      setStatus(
        t("status.added", {
          name: payload?.name || state.fileName,
          folder: state.selectedDisplay,
        }),
        false,
        true
      );
      $("#add-button").disabled = true;
    } catch (error) {
      setStatus(error?.message || t("status.addTorrentFailed"), true);
    } finally {
      setBusy($("#add-button"), false, t("actions.add"));
      updateAddButton();
    }
  }

  async function apiFetch(path, options = {}) {
    const headers = new Headers(options.headers || {});
    if (state.token) headers.set("Authorization", `Bearer ${state.token}`);
    const response = await fetch(`${state.server}${path}`, { ...options, headers });
    if (response.status === 401) {
      state.token = "";
      await chrome.storage.local.remove(["token"]);
      showSetup();
      throw new Error(t("status.sessionExpired"));
    }
    return response;
  }

  async function errorText(response) {
    const payload = await response.json().catch(() => null);
    return (
      payload?.error?.message || t("status.serverResponded", { status: response.status })
    );
  }

  function filenameFromDownload(item) {
    const raw = String(item.filename || "").split(/[\\/]/).pop();
    if (raw) return raw;
    try {
      const url = new URL(item.finalUrl || item.url);
      return decodeURIComponent(url.pathname.split("/").pop() || "download.torrent");
    } catch (error) {
      return "download.torrent";
    }
  }

  function setStatus(message, isError, isSuccess = false) {
    const target = $("#main-status");
    target.textContent = message;
    target.hidden = false;
    target.classList.toggle("error-text", Boolean(isError));
    target.classList.toggle("success-text", Boolean(isSuccess));
  }

  function setBusy(button, busy, label) {
    button.disabled = busy;
    button.textContent = label;
  }
})();

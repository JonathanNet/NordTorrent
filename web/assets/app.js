const API = "/api/v1";
const TV_LIBRARY_KEY = "nordtorrent.tvLibrary";
const LABELS_VISIBLE_KEY = "nordtorrent.labelsVisible";
const VIEW_PATHS = {
  all: "/",
  downloading: "/downloading",
  completed: "/completed",
  library: "/library",
  stats: "/stats",
  rss: "/rss",
  settings: "/settings",
};
const t = (key, vars) => window.NordTorrentI18n?.t(key, vars) ?? key;
const getLanguage = () => window.NordTorrentI18n?.getLanguage?.() ?? "da";

const state = {
  user: null,
  version: "",
  torrents: [],
  totals: emptyTotals(),
  library: [],
  tvLibraryVisible: true,
  labelsVisible: true,
  system: null,
  network: null,
  networkFetchedAt: 0,
  users: [],
  sessions: [],
  statsData: null,
  speedLimits: null,
  seedingSettings: null,
  dhtSettings: null,
  rssFeeds: [],
  rssEditingId: null,
  labels: [],
  labelAssignments: {},
  labelFilter: "",
  rssFolderBrowser: {
    currentPath: "",
    currentDisplay: "/downloads",
    rootDisplay: "/downloads",
    parent: null,
    entries: [],
    query: "",
  },
  view: "all",
  search: "",
  selectedTorrent: null,
  detailTab: "files",
  pollTimer: null,
  refreshing: false,
  confirmAction: null,
  moveTorrentTarget: null,
  booted: false,
  folderBrowser: {
    open: false,
    currentPath: "",
    currentDisplay: "/downloads",
    rootDisplay: "/downloads",
    parent: null,
    entries: [],
    query: "",
  },
};

const $ = (selector, root = document) => root.querySelector(selector);
const $$ = (selector, root = document) => Array.from(root.querySelectorAll(selector));

class ApiError extends Error {
  constructor(message, status = 0, code = "") {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.code = code;
  }
}

function emptyTotals() {
  return {
    torrent_count: 0,
    active: 0,
    paused: 0,
    completed: 0,
    download_speed_bps: 0,
    upload_speed_bps: 0,
    downloaded_bytes: 0,
    uploaded_bytes: 0,
  };
}

function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined && text !== null) node.textContent = String(text);
  return node;
}

function icon(name, className = "icon") {
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("class", className);
  svg.setAttribute("aria-hidden", "true");
  const use = document.createElementNS("http://www.w3.org/2000/svg", "use");
  use.setAttribute("href", `#i-${name}`);
  svg.appendChild(use);
  return svg;
}

function iconButton(name, label, action) {
  const button = el("button", "icon-btn");
  button.type = "button";
  button.title = label;
  button.setAttribute("aria-label", label);
  button.appendChild(icon(name));
  if (action) {
    button.addEventListener("click", (event) => {
      event.stopPropagation();
      action();
    });
  }
  return button;
}

function compactNumber(value, maximumFractionDigits = 1) {
  return new Intl.NumberFormat("da-DK", { maximumFractionDigits }).format(value);
}

function formatBytes(value) {
  const number = Number(value) || 0;
  if (number < 1024) return `${Math.round(number)} B`;
  const units = ["KB", "MB", "GB", "TB", "PB"];
  let size = number / 1024;
  let index = 0;
  while (size >= 1024 && index < units.length - 1) {
    size /= 1024;
    index += 1;
  }
  const digits = size >= 100 ? 0 : size >= 10 ? 1 : 2;
  return `${compactNumber(size, digits)} ${units[index]}`;
}

function formatSpeed(value) {
  return `${formatBytes(value)}/s`;
}

function speedValue(value) {
  const number = Number(value) || 0;
  if (number < 1024) return Math.round(number).toString();
  const units = ["KB", "MB", "GB"];
  let size = number / 1024;
  let index = 0;
  while (size >= 1024 && index < units.length - 1) {
    size /= 1024;
    index += 1;
  }
  return `${compactNumber(size, size >= 10 ? 0 : 1)} ${units[index]}`;
}

function formatEta(seconds) {
  if (!seconds || seconds <= 0) return "–";
  const total = Math.round(seconds);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const secs = total % 60;
  if (hours > 0) return `${hours}t ${minutes}m`;
  if (minutes > 0) return `${minutes}m ${secs}s`;
  return `${secs}s`;
}

function formatDate(value) {
  if (!value) return "–";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "–";
  return date.toLocaleDateString(getLanguage() === "en" ? "en-GB" : "da-DK", {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

function formatUptime(seconds) {
  const total = Math.max(0, Math.floor(seconds || 0));
  const days = Math.floor(total / 86400);
  const hours = Math.floor((total % 86400) / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const hourUnit = getLanguage() === "en" ? "h" : "t";
  if (days > 0) return `${days}d ${hours}${hourUnit}`;
  if (hours > 0) return `${hours}${hourUnit} ${minutes}m`;
  return `${minutes}m`;
}

function clampPercent(value) {
  return Math.min(100, Math.max(0, Number(value) || 0));
}

function stateLabel(torrent) {
  switch (torrent.state) {
    case "downloading":
      return t("Downloader");
    case "seeding":
      return t("Seeder");
    case "paused":
      return t("Pauset");
    case "initializing":
      return t("Henter metadata");
    case "error":
      return t("Fejl");
    default:
      return torrent.finished ? t("Færdig") : t("Klar");
  }
}

function stateClass(torrent) {
  if (torrent.state === "error") return "error";
  if (torrent.state === "paused") return "paused";
  if (torrent.state === "seeding" || (torrent.finished && torrent.state !== "downloading")) return "seeding";
  if (torrent.state === "initializing") return "initializing";
  return "downloading";
}

function roleLabel(role) {
  return role === "admin" ? t("Administrator") : t("Seer");
}

function initials(name) {
  const clean = String(name || "N").trim();
  return clean ? clean.slice(0, 1).toUpperCase() : "N";
}

function hueFor(text) {
  let hash = 0;
  for (const char of String(text || "")) {
    hash = (hash * 31 + char.charCodeAt(0)) % 360;
  }
  return hash;
}

async function api(path, options = {}) {
  const { method = "GET", json, form, skipAuth = false } = options;
  const headers = new Headers({ "X-Requested-With": "NordTorrent" });
  let body;
  if (json !== undefined) {
    headers.set("Content-Type", "application/json");
    body = JSON.stringify(json);
  } else if (form) {
    body = form;
  }

  let response;
  try {
    response = await fetch(`${API}${path}`, {
      method,
      headers,
      body,
      credentials: "same-origin",
      cache: "no-store",
    });
  } catch {
    throw new ApiError("Kunne ikke få forbindelse til serveren.", 0);
  }

  let data = null;
  const contentType = response.headers.get("content-type") || "";
  if (contentType.includes("application/json")) {
    data = await response.json().catch(() => null);
  }

  if (!response.ok) {
    if (response.status === 401 && !skipAuth) {
      handleSessionLost();
    }
    const message = data?.error?.message || `Serveren svarede med fejl ${response.status}.`;
    throw new ApiError(message, response.status, data?.error?.code || "");
  }
  return data;
}

function handleSessionLost() {
  if (!state.user) return;
  state.user = null;
  stopPolling();
  showAuth("login");
  toast("Sessionen er udløbet. Log ind igen.", "error");
}

/* Boot and auth */

async function init() {
  try {
    state.tvLibraryVisible = localStorage.getItem(TV_LIBRARY_KEY) !== "off";
    state.labelsVisible = localStorage.getItem(LABELS_VISIBLE_KEY) !== "off";
  } catch (error) {
    state.tvLibraryVisible = true;
    state.labelsVisible = true;
  }
  bindGlobalEvents();
  syncPreferences();
  try {
    const status = await api("/status", { skipAuth: true });
    state.version = status.version || "";
    setVersionLabels();
    if (status.setup_required) {
      showAuth("setup");
    } else {
      try {
        const user = await api("/auth/me", { skipAuth: true });
        await enterApp(user);
      } catch (error) {
        if (!(error instanceof ApiError) || error.status !== 401) {
          toast(error.message || "Kunne ikke kontakte serveren.", "error");
        }
        showAuth("login");
      }
    }
  } catch (error) {
    showAuth("login");
    toast(error.message || "Kunne ikke kontakte serveren.", "error");
  } finally {
    state.booted = true;
    const boot = $("#boot-screen");
    if (boot) {
      boot.classList.add("is-done");
      setTimeout(() => boot.remove(), 400);
    }
  }
}

function setVersionLabels() {
  const version = state.version || "–";
  $("#auth-version").textContent = version;
  $("#footer-version").textContent = version;
}

function showAuth(mode) {
  stopPolling();
  syncPreferences();
  const totpField = $("#login-totp-field");
  if (totpField) totpField.hidden = true;
  const totpInput = $("#login-totp");
  if (totpInput) totpInput.value = "";
  $("#app").hidden = true;
  $("#auth-view").hidden = false;
  $("#login-panel").hidden = mode !== "login";
  $("#setup-panel").hidden = mode !== "setup";
  if (mode === "setup") {
    $("#setup-username").focus();
  } else {
    $("#login-username").focus();
  }
}

async function enterApp(user) {
  state.user = user;
  $("#auth-view").hidden = true;
  $("#app").hidden = false;
  renderUser();
  await loadLabels();
  setView(viewFromPath(), { replace: true });
  await refreshAll();
  startPolling();
}

function renderUser() {
  const user = state.user;
  if (!user) return;
  $("#user-name").textContent = user.display_name || user.username;
  $("#user-avatar").textContent = initials(user.display_name || user.username);
  $("#user-role").textContent = roleLabel(user.role);
  $("#settings-username").textContent = user.username;
  $("#settings-role").textContent = roleLabel(user.role);
  $("#settings-created").textContent = formatDate(user.created_at);
  const isAdmin = user.role === "admin";
  $("#add-btn").hidden = !isAdmin;
  $("#users-card").hidden = !isAdmin;
  $("#sessions-card").hidden = !isAdmin;
  $("#network-card").hidden = !isAdmin;
  $("#speed-card").hidden = !isAdmin;
  $("#seeding-card").hidden = !isAdmin;
  $("#dht-card").hidden = !isAdmin;
  const rssNav = $('.nav-item[data-view="rss"]');
  if (rssNav) rssNav.hidden = !isAdmin;
}

/* Navigation */

function viewFromPath() {
  const path = (window.location.pathname || "/").replace(/\/+$/, "") || "/";
  const route = {
    "/": "all",
    "/all": "all",
    "/downloading": "downloading",
    "/completed": "completed",
    "/library": "library",
    "/settings": "settings",
    "/rss": "rss",
  }[path];
  return route || "all";
}

function updateViewUrl(view, options = {}) {
  const path = VIEW_PATHS[view] || "/";
  if (window.location.pathname === path) return;
  const method = options.replace ? "replaceState" : "pushState";
  window.history[method]({ view }, "", path);
}

function setView(view, options = {}) {
  if (view === "rss" && state.user?.role !== "admin") {
    view = "all";
  }
  if (view === "library" && !state.tvLibraryVisible) {
    view = "all";
    if (options.updateUrl === false) updateViewUrl("all", { replace: true });
  }
  state.view = view;
  if (options.updateUrl !== false) updateViewUrl(view, options);
  for (const item of $$(".nav-item")) {
    item.classList.toggle("is-active", item.dataset.view === view);
  }

  const library = view === "library";
  const settings = view === "settings";
  const stats = view === "stats";
  const rss = view === "rss";
  $("#torrent-section").hidden = library || settings || stats || rss;
  $("#library-section").hidden = !library;
  $("#stats-section").hidden = !stats;
  $("#rss-section").hidden = !rss;
  $("#settings-section").hidden = !settings;

  const titles = {
    all: "Alle torrents",
    downloading: "Downloader",
    completed: "Færdige",
    library: "TV-bibliotek",
    stats: "Stats",
    rss: "RSS",
    settings: "Indstillinger",
  };
  $("#section-title").textContent = titles[view] || "Alle torrents";

  if (library) {
    renderLibrary();
    refreshLibrary().catch((error) => toast(error.message, "error"));
  }
  if (stats) {
    loadStats();
  }
  if (rss) {
    loadRss().catch((error) => toast(error.message, "error"));
  }
  if (settings) {
    renderSettings();
    renderNetwork();
    loadUsers();
    loadSessions();
    refreshSystem().catch((error) => toast(error.message, "error"));
    refreshNetwork(false).catch((error) => toast(error.message, "error"));
    loadSpeedLimits().catch((error) => toast(error.message, "error"));
    loadSeedingSettings().catch((error) => toast(error.message, "error"));
    loadDhtSettings().catch((error) => toast(error.message, "error"));
  }
  if (!library && !settings && !stats && !rss) renderTorrents();
}

/* Data refresh */

async function refreshAll() {
  if (state.refreshing) return;
  state.refreshing = true;
  try {
    const tasks = [
      refreshTorrents(),
      refreshSystem(),
      refreshNetwork(false),
      state.tvLibraryVisible && (state.view === "library" || state.library.length)
        ? refreshLibrary()
        : Promise.resolve(),
      state.view === "stats" ? loadStats() : Promise.resolve(),
    ];
    await Promise.allSettled(tasks);
  } finally {
    state.refreshing = false;
  }
}

async function refreshTorrents() {
  const data = await api("/torrents");
  state.torrents = Array.isArray(data?.torrents) ? data.torrents : [];
  state.totals = data?.totals || emptyTotals();
  renderStats();
  if (state.view !== "library" && state.view !== "settings") renderTorrents();
}

async function refreshLibrary() {
  if (!state.tvLibraryVisible) return;
  const data = await api("/library");
  state.library = Array.isArray(data?.items) ? data.items : [];
  $("#library-count").textContent = String(state.library.length);
  if (state.view === "library") renderLibrary();
}

async function loadStats() {
  try {
    const data = await api("/stats");
    state.statsData = data;
    renderStatsPage();
  } catch (error) {
    if (error.status !== 401) toast(error.message, "error");
  }
}

function monthLabel(value) {
  if (!value) return "–";
  const date = new Date(`${value}-01T12:00:00`);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleDateString(getLanguage() === "en" ? "en-GB" : "da-DK", {
    month: "short",
    year: "numeric",
  });
}

function renderStatsPage() {
  const data = state.statsData;
  if (!data) return;
  const allTimeRatio = data.all_time_downloaded_bytes > 0
    ? data.all_time_uploaded_bytes / data.all_time_downloaded_bytes
    : 0;
  $("#stats-month-badge").textContent = monthLabel(data.month);
  $("#stats-month-download").textContent = formatBytes(data.downloaded_bytes);
  $("#stats-month-download-sub").textContent = `${t("I alt")} ${formatBytes(data.all_time_downloaded_bytes)}`;
  $("#stats-month-upload").textContent = formatBytes(data.uploaded_bytes);
  $("#stats-month-upload-sub").textContent = `${t("I alt")} ${formatBytes(data.all_time_uploaded_bytes)}`;
  $("#stats-month-ratio").textContent = Number(data.ratio || 0).toFixed(2);
  $("#stats-month-ratio-sub").textContent = `${t("Samlet ratio")} ${allTimeRatio.toFixed(2)}`;
  $("#stats-seeding").textContent = String(data.seeding_count || 0);
  $("#stats-seeding-sub").textContent = `${data.torrent_count || 0} ${t("torrents i alt")}`;

  const months = Array.isArray(data.months) ? data.months : [];
  const monthsHost = $("#stats-months");
  if (!months.length) {
    monthsHost.replaceChildren(el("p", "muted stats-empty", t("Ingen statistik endnu.")));
  } else {
    const max = Math.max(
      1,
      ...months.flatMap((item) => [
        Number(item.downloaded_bytes || 0),
        Number(item.uploaded_bytes || 0),
      ])
    );
    const fragment = document.createDocumentFragment();
    for (const item of months) {
      const row = el("div", "stats-month-row");
      const bars = el("div", "stats-bars");
      const track = (kind, value) => {
        const barTrack = el("div", "stats-bar-track");
        const bar = el("span", `stats-bar ${kind}`);
        bar.style.width = `${Math.max(0, Math.min(100, (Number(value || 0) / max) * 100))}%`;
        barTrack.appendChild(bar);
        return barTrack;
      };
      const values = el("div", "stats-month-values");
      values.append(
        el("span", "", `↓ ${formatBytes(item.downloaded_bytes)}`),
        el("span", "", `↑ ${formatBytes(item.uploaded_bytes)}`)
      );
      bars.append(track("download", item.downloaded_bytes), track("upload", item.uploaded_bytes), values);
      row.append(el("span", "stats-month-name", monthLabel(item.month)), bars);
      fragment.appendChild(row);
    }
    monthsHost.replaceChildren(fragment);
  }

  const top = Array.isArray(data.top_torrents) ? data.top_torrents : [];
  const topHost = $("#stats-top");
  if (!top.length) {
    topHost.replaceChildren(el("p", "muted stats-empty", t("Ingen uploadet data endnu.")));
  } else {
    const fragment = document.createDocumentFragment();
    top.forEach((item, index) => {
      const row = el("div", "stats-top-row");
      const name = el("div", "stats-top-name");
      name.append(
        el("strong", "", item.name || item.info_hash.slice(0, 12)),
        el("small", "", `${t("Ratio")} ${Number(item.ratio || 0).toFixed(2)}`)
      );
      row.append(
        el("span", "stats-top-rank", String(index + 1)),
        name,
        el("span", "stats-top-upload", formatBytes(item.uploaded_bytes))
      );
      fragment.appendChild(row);
    });
    topHost.replaceChildren(fragment);
  }
}

async function refreshSystem() {
  const data = await api("/system");
  state.system = data;
  if (data?.torrents?.totals) {
    state.totals = data.torrents.totals;
  }
  renderStats();
  if (state.view === "settings") renderSettings();
}

async function refreshNetwork(force = false) {
  if (!force && state.network && Date.now() - state.networkFetchedAt < 30000) return;
  const data = await api(force ? "/network/check" : "/network", force ? { method: "POST" } : {});
  state.network = data;
  state.networkFetchedAt = Date.now();
  renderNetwork();
}

function renderNetwork() {
  const net = state.network;
  const button = $("#port-status");
  if (!button) return;

  const status = net?.status || "unknown";
  const endpoint =
    net?.external_endpoint ||
    (net?.external_ip && net?.external_port ? `${net.external_ip}:${net.external_port}` : "");
  button.className = `port-status ${status}`;
  $("#port-status-text").textContent = endpoint || t("Port ukendt");
  button.title = net?.message ? t(net.message) : t("Tjekker torrent-porten…");
  button.disabled = !state.user;

  const badge = $("#network-badge");
  if (badge) {
    badge.className = `network-pill ${status}`;
    badge.textContent =
      status === "open" ? t("Åben") : status === "closed" ? t("Lukket") : t("Ukendt");
  }

  if (!net) return;
  const localPort = $("#network-local-port");
  const range = $("#network-range");
  if (localPort) localPort.textContent = net.listen_port || "–";
  if (range) range.textContent = `${net.port_start}–${net.port_end}`;
  const autoIp = $("#network-auto-ip");
  if (autoIp) autoIp.textContent = net.external_ip || t("Ikke fundet");

  const ipInput = $("#network-external-ip");
  const portInput = $("#network-external-port");
  if (ipInput && document.activeElement !== ipInput) {
    ipInput.value = net.configured_external_ip || "";
    ipInput.placeholder = net.external_ip || t("Findes automatisk");
  }
  if (portInput && document.activeElement !== portInput) {
    portInput.value = net.configured_external_port || "";
    portInput.placeholder = net.listen_port ? String(net.listen_port) : t("Fx 50000");
  }
  const message = $("#network-message");
  if (message) {
    message.className = `network-message ${status}`;
    message.textContent = net.message ? t(net.message) : "";
  }
}

async function saveNetworkSettings(event) {
  event.preventDefault();
  const error = $("#network-error");
  if (error) error.hidden = true;
  const ip = $("#network-external-ip").value.trim();
  const portValue = $("#network-external-port").value.trim();
  const port = portValue ? Number(portValue) : null;
  if (port !== null && (!Number.isInteger(port) || port < 1 || port > 65535)) {
    toast("Porten skal være mellem 1 og 65535.", "error");
    return;
  }
  const button = event.submitter;
  if (button) setButtonLoading(button, true);
  try {
    state.network = await api("/network", {
      method: "PUT",
      json: {
        external_ip: ip || null,
        external_port: port,
      },
    });
    state.networkFetchedAt = Date.now();
    renderNetwork();
    toast("Netværksindstillinger gemt.", "success");
  } catch (err) {
    toast(err.message, "error");
  } finally {
    if (button) setButtonLoading(button, false);
  }
}

async function checkNetworkPort() {
  const button = $("#network-test");
  setButtonLoading(button, true);
  try {
    state.network = await api("/network/check", { method: "POST" });
    state.networkFetchedAt = Date.now();
    renderNetwork();
    toast(
      state.network.status === "open"
        ? "Porten er åben."
        : state.network.status === "closed"
          ? "Porten er lukket."
          : "Portstatus kunne ikke afgøres.",
      state.network.status === "open" ? "success" : "error"
    );
  } catch (err) {
    toast(err.message, "error");
  } finally {
    setButtonLoading(button, false);
  }
}

async function loadSpeedLimits() {
  if (!state.user || state.user.role !== "admin") return;
  const data = await api("/limits");
  state.speedLimits = data;
  renderSpeedLimits(data);
}

function renderSpeedLimits(data) {
  if (!data) return;
  $("#speed-error").hidden = true;
  $("#speed-download-enabled").checked = Boolean(data.download_enabled);
  $("#speed-download-kbps").value = String(data.download_kbps ?? 1000);
  $("#speed-upload-enabled").checked = Boolean(data.upload_enabled);
  $("#speed-upload-kbps").value = String(data.upload_kbps ?? 200);
  updateSpeedRows();
  $("#speed-status").textContent = speedLimitSummary(data);
}

function updateSpeedRows() {
  $("#speed-download-row").classList.toggle(
    "is-disabled",
    !$("#speed-download-enabled").checked
  );
  $("#speed-upload-row").classList.toggle(
    "is-disabled",
    !$("#speed-upload-enabled").checked
  );
  $("#speed-download-kbps").disabled = !$("#speed-download-enabled").checked;
  $("#speed-upload-kbps").disabled = !$("#speed-upload-enabled").checked;
}

function speedLimitSummary(data) {
  const parts = [];
  if (data.download_enabled) {
    parts.push(t("Download {speed} KB/s", { speed: data.download_kbps }));
  }
  if (data.upload_enabled) {
    parts.push(t("Upload {speed} KB/s", { speed: data.upload_kbps }));
  }
  return parts.length ? parts.join(" · ") : t("Ingen hastighedsgrænse");
}

async function saveSpeedLimits() {
  const error = $("#speed-error");
  error.hidden = true;
  const downloadKbps = Number($("#speed-download-kbps").value);
  const uploadKbps = Number($("#speed-upload-kbps").value);
  for (const [value, label] of [
    [downloadKbps, t("Download")],
    [uploadKbps, t("Upload")],
  ]) {
    if (!Number.isInteger(value) || value < 1 || value > 4_194_303) {
      error.textContent = t("{label}-hastigheden skal være mellem 1 og 4.194.303 KB/s.", {
        label,
      });
      error.hidden = false;
      return;
    }
  }
  const settings = {
    download_enabled: $("#speed-download-enabled").checked,
    download_kbps: downloadKbps,
    upload_enabled: $("#speed-upload-enabled").checked,
    upload_kbps: uploadKbps,
  };
  const button = $("#speed-save");
  setButtonLoading(button, true);
  try {
    state.speedLimits = await api("/limits", { method: "PUT", json: settings });
    renderSpeedLimits(state.speedLimits);
    toast(t("Hastighedsgrænser gemt."), "success");
  } catch (err) {
    error.textContent = err.message;
    error.hidden = false;
  } finally {
    setButtonLoading(button, false);
  }
}

async function loadSeedingSettings() {
  if (!state.user || state.user.role !== "admin") return;
  const data = await api("/seeding");
  state.seedingSettings = data;
  renderSeedingSettings(data);
}

function renderSeedingSettings(data) {
  if (!data) return;
  $("#seeding-error").hidden = true;
  $("#seeding-keep").checked = Boolean(data.keep_seeding);
  $("#seeding-ratio").value = String(data.stop_ratio ?? 0);
  $("#seeding-minutes").value = String(data.stop_minutes ?? 0);
  updateSeedingRows();
  $("#seeding-status").textContent = seedingSummary(data);
}

function updateSeedingRows() {
  const keep = $("#seeding-keep").checked;
  $("#seeding-ratio-row").classList.toggle("is-disabled", !keep);
  $("#seeding-time-row").classList.toggle("is-disabled", !keep);
  $("#seeding-ratio").disabled = !keep;
  $("#seeding-minutes").disabled = !keep;
}

function seedingSummary(data) {
  if (!data.keep_seeding) return t("Seeding stoppes når torrenten er færdig");
  const limits = [];
  if (data.stop_ratio > 0) {
    limits.push(t("Stop ved ratio {ratio}", { ratio: data.stop_ratio }));
  }
  if (data.stop_minutes > 0) {
    limits.push(t("Stop efter {minutes} min.", { minutes: data.stop_minutes }));
  }
  return limits.length ? limits.join(" · ") : t("Seeding fortsætter uden grænse");
}

async function saveSeedingSettings() {
  const error = $("#seeding-error");
  error.hidden = true;
  const ratio = Number($("#seeding-ratio").value);
  const minutes = Number($("#seeding-minutes").value);
  if (!Number.isFinite(ratio) || ratio < 0 || ratio > 1000) {
    error.textContent = t("Ratio skal være mellem 0 og 1000.");
    error.hidden = false;
    return;
  }
  if (!Number.isInteger(minutes) || minutes < 0 || minutes > 525600) {
    error.textContent = t("Stop-tid skal være mellem 0 og 525.600 minutter.");
    error.hidden = false;
    return;
  }
  const settings = {
    keep_seeding: $("#seeding-keep").checked,
    stop_ratio: ratio,
    stop_minutes: minutes,
  };
  const button = $("#seeding-save");
  setButtonLoading(button, true);
  try {
    state.seedingSettings = await api("/seeding", { method: "PUT", json: settings });
    renderSeedingSettings(state.seedingSettings);
    toast(t("Seeding-indstillinger gemt."), "success");
  } catch (err) {
    error.textContent = err.message;
    error.hidden = false;
  } finally {
    setButtonLoading(button, false);
  }
}

async function loadDhtSettings() {
  if (!state.user || state.user.role !== "admin") return;
  const data = await api("/dht");
  state.dhtSettings = data;
  renderDhtSettings(data);
}

function renderDhtSettings(data) {
  if (!data) return;
  $("#dht-error").hidden = true;
  $("#dht-enabled").checked = Boolean(data.enabled);
  $("#dht-note").textContent = data.enabled
    ? t("DHT er slået til. Ændringen træder i kraft efter genstart af serveren.")
    : t("DHT er slået fra. Ændringen træder i kraft efter genstart af serveren.");
}

async function saveDhtSettings() {
  const error = $("#dht-error");
  error.hidden = true;
  const button = $("#dht-save");
  setButtonLoading(button, true);
  try {
    state.dhtSettings = await api("/dht", {
      method: "PUT",
      json: { enabled: $("#dht-enabled").checked },
    });
    renderDhtSettings(state.dhtSettings);
    toast(t("DHT-indstilling gemt. Genstart serveren for at anvende den."), "success");
  } catch (err) {
    error.textContent = err.message;
    error.hidden = false;
  } finally {
    setButtonLoading(button, false);
  }
}

async function loadSessions() {
  if (!state.user || state.user.role !== "admin") return;
  try {
    const sessions = await api("/auth/sessions");
    state.sessions = Array.isArray(sessions) ? sessions : [];
    renderSessions();
  } catch (error) {
    if (error.status !== 401) toast(error.message, "error");
  }
}

function describeUserAgent(userAgent) {
  if (!userAgent) return t("Ukendt enhed");
  const ua = String(userAgent);
  const extension = ua.match(/NordTorrent Chrome Extension(?:\s+([\w.-]+))?/i);
  if (extension) {
    return extension[1]
      ? `${t("Chrome-udvidelse")} ${extension[1]}`
      : t("Chrome-udvidelse");
  }
  let browser = "Browser";
  if (/Edg\//.test(ua)) browser = "Edge";
  else if (/OPR\//.test(ua)) browser = "Opera";
  else if (/Firefox\//.test(ua)) browser = "Firefox";
  else if (/Chrome\//.test(ua)) browser = "Chrome";
  else if (/Safari\//.test(ua)) browser = "Safari";
  else if (/okhttp/i.test(ua)) browser = "NordTorrent TV";
  else if (/curl/i.test(ua)) browser = "curl";
  let os = "Ukendt OS";
  if (/Android TV/i.test(ua)) os = "Android TV";
  else if (/Android/i.test(ua)) os = "Android";
  else if (/iPhone|iPad|iPod/i.test(ua)) os = "iOS";
  else if (/Windows/i.test(ua)) os = "Windows";
  else if (/Mac OS X|Macintosh/i.test(ua)) os = "macOS";
  else if (/Linux/i.test(ua)) os = "Linux";
  if (browser === "NordTorrent TV" && os === "Ukendt OS") os = "Android";
  return `${browser} · ${os}`;
}

function formatDateTime(value) {
  if (!value) return "–";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "–";
  return date.toLocaleString(getLanguage() === "en" ? "en-GB" : "da-DK", {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function renderSessions() {
  const list = $("#sessions-list");
  const count = $("#sessions-count");
  if (!list) return;
  if (count) count.textContent = String(state.sessions.length);
  if (!state.sessions.length) {
    list.replaceChildren(el("p", "muted detail-empty", t("Ingen aktive sessioner.")));
    return;
  }

  const fragment = document.createDocumentFragment();
  for (const session of state.sessions) {
    const row = el("div", `session-row${session.current ? " current" : ""}`);
    const main = el("div", "session-main");
    const device = el("div", "session-device");
    device.append(icon("devices"), el("strong", "", describeUserAgent(session.user_agent)));
    if (session.current) device.appendChild(el("span", "session-current", t("Denne enhed")));
    const meta = el(
      "div",
      "session-meta",
      `${session.username} · ${session.ip || "–"}`
    );
    main.append(device, meta);
    const times = el("div", "session-times");
    times.append(
      el("span", "", `${t("Oprettet")}: ${formatDateTime(session.created_at)}`),
      el("span", "", `${t("Sidst aktiv")}: ${formatDateTime(session.last_seen_at)}`)
    );
    const revoke = el("button", "icon-btn session-revoke");
    revoke.type = "button";
    revoke.title = t("Tilbagekald session");
    revoke.setAttribute("aria-label", t("Tilbagekald session"));
    revoke.appendChild(icon("logout"));
    revoke.addEventListener("click", () => confirmRevokeSession(session));
    row.append(main, times, revoke);
    fragment.appendChild(row);
  }
  list.replaceChildren(fragment);
}

function confirmRevokeSession(session) {
  const device = describeUserAgent(session.user_agent);
  const text = session.current
    ? t("Er du sikker på, at du vil tilbagekalde denne session? Du bliver logget ud med det samme.")
    : t("Er du sikker på, at du vil tilbagekalde sessionen for {device}?", { device });
  confirmDialog({
    title: t("Tilbagekald session?"),
    text,
    confirmLabel: t("Tilbagekald"),
    danger: true,
    onConfirm: async () => {
      const result = await api(`/auth/sessions/${encodeURIComponent(session.id)}`, {
        method: "DELETE",
      });
      if (session.current || result.revoked_current) {
        state.user = null;
        stopPolling();
        window.history.replaceState(null, "", "/");
        showAuth("login");
        toast(t("Sessionen er tilbagekaldt. Du er logget ud."));
        return;
      }
      toast(t("Sessionen er tilbagekaldt."));
      await loadSessions();
    },
  });
}

async function loadUsers() {
  if (!state.user || state.user.role !== "admin") return;
  try {
    const users = await api("/users");
    state.users = Array.isArray(users) ? users : [];
    renderUsers();
  } catch (error) {
    if (error.status !== 401) toast(error.message, "error");
  }
}

function startPolling() {
  stopPolling();
  state.pollTimer = window.setInterval(() => {
    if (!document.hidden && state.user && !$("#player-modal").hidden) return;
    if (document.hidden || !state.user) return;
    refreshAll();
  }, 3500);
}

function stopPolling() {
  if (state.pollTimer) {
    window.clearInterval(state.pollTimer);
    state.pollTimer = null;
  }
}

/* Stats */

function renderStats() {
  const totals = state.totals || emptyTotals();
  const download = totals.download_speed_bps || 0;
  const upload = totals.upload_speed_bps || 0;
  const active = totals.active || 0;
  const count = totals.torrent_count || 0;

  $("#top-down").textContent = formatSpeed(download);
  $("#top-up").textContent = formatSpeed(upload);
  $("#stat-down").textContent = formatSpeed(download);
  $("#stat-up").textContent = formatSpeed(upload);
  $("#stat-active").textContent = String(active);
  $("#stat-downloaded").textContent = formatBytes(totals.downloaded_bytes);
  $("#stat-active-sub").textContent = count ? t("af {count}", { count }) : "–";
  $("#stat-downloaded-sub").textContent = totals.uploaded_bytes
    ? `op ${formatBytes(totals.uploaded_bytes)}`
    : "–";
  $("#stat-down-sub").textContent = active ? t("{count} aktive", { count: active }) : "–";
  $("#stat-up-sub").textContent = totals.completed
    ? t("{count} færdige", { count: totals.completed })
    : "–";
  $("#sidebar-uptime").textContent = state.system
    ? t("Oppetid {uptime}", { uptime: formatUptime(state.system.uptime_seconds) })
    : t("Forbinder…");

  document.title = active ? `NordTorrent · ${formatSpeed(download)}` : "NordTorrent";
}

/* Torrents */

function filteredTorrents() {
  const query = state.search.trim().toLowerCase();
  return state.torrents.filter((torrent) => {
    if (state.view === "downloading" && torrent.state !== "downloading" && torrent.state !== "initializing") {
      return false;
    }
    if (state.view === "completed" && !torrent.finished) return false;
    if (state.labelFilter && torrent.label !== state.labelFilter) return false;
    if (!query) return true;
    return (
      String(torrent.name || "").toLowerCase().includes(query) ||
      String(torrent.info_hash || "").toLowerCase().includes(query)
    );
  });
}

function renderTorrents() {
  const torrents = filteredTorrents();
  renderLabels();
  $("#torrent-count").textContent = String(torrents.length);
  const empty = $("#torrent-empty");
  const table = $("#torrent-table");
  const rows = $("#torrent-rows");

  const noAny = state.torrents.length === 0;
  empty.hidden = !noAny;
  table.hidden = noAny;
  if (noAny) {
    const button = empty.querySelector("button");
    if (button) button.hidden = state.user?.role !== "admin";
    rows.replaceChildren();
    return;
  }

  if (torrents.length === 0) {
    empty.hidden = false;
    table.hidden = true;
    empty.querySelector("h3").textContent = "Ingen match";
    empty.querySelector("p").textContent = "Prøv en anden søgning eller et andet filter.";
    const button = empty.querySelector("button");
    if (button) button.hidden = true;
    rows.replaceChildren();
    return;
  }

  empty.hidden = true;
  table.hidden = false;
  empty.querySelector("h3").textContent = "Ingen torrents endnu";
  empty.querySelector("p").textContent = "Tilføj et magnet-link eller en .torrent-fil for at komme i gang.";
  const button = empty.querySelector("button");
  if (button) button.hidden = state.user?.role !== "admin";

  const fragment = document.createDocumentFragment();
  for (const torrent of torrents) {
    fragment.appendChild(torrentRow(torrent));
  }
  rows.replaceChildren(fragment);
}

function torrentRow(torrent) {
  const row = el("div", "torrent-row");
  row.tabIndex = 0;
  row.setAttribute("role", "button");
  row.setAttribute("aria-label", `Åbn ${torrent.name}`);
  row.addEventListener("click", () => openDetail(torrent.id));
  row.addEventListener("keydown", (event) => {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      openDetail(torrent.id);
    }
  });

  const main = el("div", "torrent-main");
  const fileIcon = el("div", `file-icon ${torrent.state === "error" ? "error" : torrent.media_files ? "video" : ""}`);
  fileIcon.appendChild(icon(torrent.state === "error" ? "alert" : torrent.media_files ? "film" : "file"));
  const totalFiles = Number(torrent.total_files || 0);
  const mediaFiles = Number(torrent.media_files || 0);
  const mediaLabel = mediaFiles
    ? `${mediaFiles} ${t(mediaFiles === 1 ? "medie" : "medier")}`
    : null;
  const fileLabel = `${totalFiles} ${t(totalFiles === 1 ? "fil" : "filer")}`;
  const folderLabel = torrent.folder ? ` · ${torrent.folder}` : "";
  const text = el("div", "torrent-text");
  text.appendChild(el("span", "torrent-title", torrent.name || "Ukendt torrent"));
  if (torrent.label) text.appendChild(el("span", "torrent-label", torrent.label));
  text.appendChild(
    el(
      "span",
      "torrent-sub",
      `${String(torrent.info_hash || "").slice(0, 12)} · ${mediaLabel ? `${mediaLabel} · ` : ""}${fileLabel}${folderLabel}`
    )
  );
  main.append(fileIcon, text);

  const status = el("span", `status-pill ${stateClass(torrent)}`);
  status.append(el("span", "status-dot"), document.createTextNode(stateLabel(torrent)));
  if (torrent.error) status.title = torrent.error;

  const size = el("span", "cell-size", formatBytes(torrent.total_bytes));

  const progressCell = el("div", "progress-cell");
  const meta = el("div", "progress-meta");
  meta.append(
    el("strong", "", `${compactNumber(clampPercent(torrent.progress), 1)}%`),
    el("span", "", `${formatBytes(torrent.progress_bytes)} / ${formatBytes(torrent.total_bytes)}`)
  );
  const track = el("div", "progress-track");
  const fill = el("div", `progress-fill ${stateClass(torrent)}`);
  fill.style.width = `${clampPercent(torrent.progress)}%`;
  track.appendChild(fill);
  progressCell.append(meta, track);

  const speed = el("div", "speed-cell");
  speed.append(el("span", "", formatSpeed(torrent.download_speed_bps)));
  speed.append(
    el(
      "span",
      "",
      torrent.finished
        ? `op ${formatSpeed(torrent.upload_speed_bps)}`
        : torrent.eta_seconds
          ? `ETA ${formatEta(torrent.eta_seconds)}`
          : "–"
    )
  );

  speed.append(
    el("span", "speed-ratio", `${t("Ratio")} ${Number(torrent.ratio || 0).toFixed(2)}`)
  );

  const actions = el("div", "row-actions");
  actions.appendChild(
    iconButton("users", t("Vis peers"), () => openDetail(torrent.id, false, "peers"))
  );
  if (torrent.media_files > 0) {
    actions.appendChild(iconButton("film", t("Vis filer og afspil"), () => openDetail(torrent.id)));
  }
  if (state.user?.role === "admin") {
    if (torrent.state === "paused") {
      actions.appendChild(iconButton("play", "Start", () => torrentAction(torrent.id, "resume")));
    } else {
      actions.appendChild(iconButton("pause", "Pause", () => torrentAction(torrent.id, "pause")));
    }
    const remove = iconButton("trash", "Fjern torrent", () => confirmRemove(torrent));
    remove.classList.add("danger");
    actions.appendChild(remove);
  }

  row.append(main, status, size, progressCell, speed, actions);
  return row;
}

async function torrentAction(id, action) {
  try {
    await api(`/torrents/${id}/${action}`, { method: "POST", json: {} });
    toast(action === "pause" ? "Torrentet er sat på pause." : "Torrentet kører igen.");
    await refreshTorrents();
    if (state.selectedTorrent?.id === id) await openDetail(id, true);
  } catch (error) {
    toast(error.message, "error");
  }
}

/* Library */

function renderLibrary() {
  const query = state.search.trim().toLowerCase();
  const items = state.library.filter((item) => {
    if (!query) return true;
    return (
      String(item.title || "").toLowerCase().includes(query) ||
      String(item.torrent_name || "").toLowerCase().includes(query) ||
      String(item.file_name || "").toLowerCase().includes(query)
    );
  });
  $("#library-count").textContent = String(items.length);
  const grid = $("#library-grid");
  const empty = $("#library-empty");
  empty.hidden = items.length > 0;
  if (!items.length) {
    grid.replaceChildren();
    return;
  }

  const fragment = document.createDocumentFragment();
  for (const item of items) {
    const card = el("article", "library-card");
    card.tabIndex = 0;
    card.style.setProperty("--hue", String(hueFor(item.title)));
    card.setAttribute("aria-label", `Afspil ${item.title}`);
    card.addEventListener("click", () => openPlayer(item));
    card.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        openPlayer(item);
      }
    });

    const art = el("div", "library-art");
    const fileIcon = el("div", "file-icon");
    fileIcon.appendChild(icon(item.kind === "audio" ? "music" : "film"));
    art.appendChild(fileIcon);
    if (item.preview_url) {
      const preview = el("img", "library-thumb");
      preview.src = item.preview_url;
      preview.alt = "";
      preview.loading = "lazy";
      preview.decoding = "async";
      preview.addEventListener("error", () => preview.remove());
      art.appendChild(preview);
    }
    const playBadge = el("span", "play-badge");
    playBadge.appendChild(icon("play"));
    art.appendChild(playBadge);

    const info = el("div", "library-info");
    info.appendChild(el("strong", "", item.title || item.file_name));
    info.appendChild(
      el(
        "span",
        "",
        `${formatBytes(item.size)} · ${compactNumber(clampPercent(item.progress), 1)}% ${t("hentet")}${
          item.torrent_state === "paused" ? ` · ${t("pauset")}` : ""
        }`
      )
    );
    const progress = el("div", "library-progress");
    const bar = el("span");
    bar.style.width = `${clampPercent(item.progress)}%`;
    progress.appendChild(bar);
    info.appendChild(progress);
    card.append(art, info);
    fragment.appendChild(card);
  }
  grid.replaceChildren(fragment);
}

/* Detail drawer */

async function openDetail(id, preserveScroll = false, tab = null) {
  const overlay = $("#detail-overlay");
  const body = $("#detail-body");
  overlay.hidden = false;
  document.body.classList.add("no-scroll");
  if (tab) state.detailTab = tab;
  else if (!preserveScroll) state.detailTab = "files";
  if (!preserveScroll) {
    body.replaceChildren(el("p", "muted", t("Henter torrentdetaljer…")));
  }
  try {
    const detail = await api(`/torrents/${id}`);
    state.selectedTorrent = detail;
    renderDetail(detail);
  } catch (error) {
    closeModal("detail-overlay");
    toast(error.message, "error");
  }
}

function renderDetail(detail) {
  $("#detail-title").textContent = detail.name || "Ukendt torrent";
  $("#detail-eyebrow").textContent = `TORRENT · ${String(detail.info_hash || "").slice(0, 12)}`;
  const body = $("#detail-body");
  const fragment = document.createDocumentFragment();

  const hero = el("div", "detail-hero");
  const heroIcon = el("div", `file-icon ${detail.media_files ? "video" : ""}`);
  heroIcon.appendChild(icon(detail.media_files ? "film" : "file"));
  const heroText = el("div");
  const mediaLabel = `${detail.media_files} ${t(detail.media_files === 1 ? "medie" : "medier")}`;
  const fileLabel = `${detail.total_files} ${t(detail.total_files === 1 ? "fil" : "filer")}`;
  heroText.append(
    el("strong", "", `${mediaLabel} · ${fileLabel}`),
    el(
      "span",
      "",
      detail.folder
        ? t("Downloadmappe: {path}", { path: detail.folder })
        : detail.output_folder
          ? t("Gemt i {path}", { path: detail.output_folder })
          : t("Gemt i downloadmappen")
    )
  );
  hero.append(heroIcon, heroText, statusPill(detail));
  fragment.appendChild(hero);

  const actions = el("div", "detail-actions");
  if (state.user?.role === "admin") {
    if (detail.state === "paused") {
      actions.appendChild(actionButton("play", t("Start"), () => torrentAction(detail.id, "resume")));
    } else {
      actions.appendChild(actionButton("pause", t("Pause"), () => torrentAction(detail.id, "pause")));
    }
    actions.appendChild(
      actionButton("folder", t("Flyt"), () => openMoveModal(detail))
    );
    const remove = actionButton("trash", t("Fjern"), () => confirmRemove(detail));
    remove.classList.add("btn-danger");
    actions.appendChild(remove);
  }
  if (detail.media_files > 0) {
    const first = detail.files.find((file) => file.playable && file.included);
    if (first) {
      actions.appendChild(
        actionButton("play", t("Afspil"), () =>
          openPlayer({
            title: detail.name,
            stream_url: first.stream_url,
            file_name: first.name,
            kind: first.kind,
          })
        )
      );
    }
  }
  fragment.appendChild(actions);

  if (state.user?.role === "admin") {
    const labelRow = el("div", "detail-label-row");
    labelRow.appendChild(el("span", "", t("Label")));
    const select = el("select");
    renderLabelOptions(select, detail.label || "");
    select.addEventListener("change", () => setTorrentLabel(detail, select.value));
    labelRow.appendChild(select);
    fragment.appendChild(labelRow);
  } else if (detail.label) {
    fragment.appendChild(el("span", "detail-label-chip", detail.label));
  }

  const grid = el("div", "detail-grid");
  grid.append(
    detailStat(t("Størrelse"), formatBytes(detail.total_bytes)),
    detailStat(t("Hentet"), formatBytes(detail.progress_bytes)),
    detailStat(t("Uploadet"), formatBytes(detail.uploaded_bytes)),
    detailStat(t("Ratio"), Number(detail.ratio || 0).toFixed(2)),
    detailStat(t("Download"), formatSpeed(detail.download_speed_bps)),
    detailStat(t("ETA"), detail.eta_seconds ? formatEta(detail.eta_seconds) : "–")
  );
  fragment.appendChild(grid);

  const progress = el("div", "detail-progress");
  const meta = el("div", "progress-meta");
  meta.append(
    el("strong", "", `${compactNumber(clampPercent(detail.progress), 1)}%`),
    el("span", "", `${formatBytes(detail.progress_bytes)} / ${formatBytes(detail.total_bytes)}`)
  );
  const track = el("div", "progress-track");
  const fill = el("div", `progress-fill ${stateClass(detail)}`);
  fill.style.width = `${clampPercent(detail.progress)}%`;
  track.appendChild(fill);
  progress.append(meta, track);
  fragment.appendChild(progress);

  const tabs = el("div", "detail-tabs");
  const tabDefinitions = [
    { id: "files", label: t("Filer"), icon: "file", count: detail.files.length },
    { id: "trackers", label: t("Trackere"), icon: "globe", count: detail.trackers?.length || 0 },
    { id: "peers", label: t("Peers"), icon: "users", count: detail.peers?.length || 0 },
  ];
  for (const definition of tabDefinitions) {
    const button = el(
      "button",
      `detail-tab${state.detailTab === definition.id ? " is-active" : ""}`
    );
    button.type = "button";
    button.append(
      icon(definition.icon),
      document.createTextNode(definition.label),
      el("span", "detail-tab-count", String(definition.count))
    );
    button.addEventListener("click", () => {
      state.detailTab = definition.id;
      renderDetail(detail);
    });
    tabs.appendChild(button);
  }
  fragment.appendChild(tabs);

  const panel = el("div", "detail-tab-panel");
  fragment.appendChild(panel);
  renderDetailTab(detail, panel);

  body.replaceChildren(fragment);
}

function renderDetailTab(detail, panel) {
  if (state.detailTab === "trackers") {
    renderTrackersTab(detail, panel);
  } else if (state.detailTab === "peers") {
    renderPeersTab(detail, panel);
  } else {
    renderFilesTab(detail, panel);
  }
}

function renderFilesTab(detail, panel) {
  const filesHead = el("div", "drawer-section-head");
  filesHead.append(el("h3", "", t("Filer")), el("span", "", `${detail.files.length} ${t("i alt")}`));
  panel.appendChild(filesHead);

  const fileList = el("div", "file-list");
  for (const file of detail.files) {
    fileList.appendChild(fileRow(detail, file));
  }
  panel.appendChild(fileList);

  if (state.user?.role === "admin") {
    const bar = el("div", "file-actions-bar");
    bar.appendChild(el("span", "", t("Normal eller Spring over pr. fil.")));
    const save = el("button", "btn btn-ghost btn-compact");
    save.type = "button";
    save.append(icon("check"), document.createTextNode(t("Gem filvalg")));
    save.addEventListener("click", () => saveFileSelection(detail, save));
    bar.appendChild(save);
    panel.appendChild(bar);
  }
}

function renderTrackersTab(detail, panel) {
  const trackers = detail.trackers || [];
  const head = el("div", "drawer-section-head");
  head.append(
    el("h3", "", t("Trackere")),
    el("span", "", `${trackers.length} ${t("i alt")}`)
  );
  panel.appendChild(head);

  if (!trackers.length) {
    panel.appendChild(
      el("p", "muted detail-empty", t("Ingen trackere fundet for denne torrent."))
    );
    return;
  }

  const table = el("div", "peer-table tracker-table");
  const header = el("div", "peer-table-head");
  header.append(
    el("span", "", t("Tracker-URL")),
    el("span", "", t("Type")),
    el("span", "", t("Status"))
  );
  table.appendChild(header);
  for (const tracker of trackers) {
    const row = el("div", "peer-table-row");
    const url =
      tracker.url.startsWith("http://") || tracker.url.startsWith("https://")
        ? el("a", "tracker-url", tracker.url)
        : el("span", "tracker-url", tracker.url);
    if (url.tagName === "A") {
      url.href = tracker.url;
      url.target = "_blank";
      url.rel = "noopener noreferrer";
    }
    row.append(
      url,
      el("span", "peer-mono", String(tracker.kind || "–").toUpperCase()),
      el("span", "tracker-status", t("Ukendt"))
    );
    table.appendChild(row);
  }
  panel.appendChild(table);
  panel.appendChild(
    el("p", "muted detail-empty", t("Tracker-status er ikke eksponeret af torrentmotoren."))
  );
}

function renderPeersTab(detail, panel) {
  const peers = detail.peers || [];
  const head = el("div", "drawer-section-head");
  head.append(
    el("h3", "", t("Peers")),
    el("span", "", `${peers.length} ${t("forbundet")}`)
  );
  panel.appendChild(head);

  if (!peers.length) {
    panel.appendChild(
      el(
        "p",
        "muted detail-empty",
        t("Ingen live peers lige nu. Peers vises, når der er forbindelse til andre.")
      )
    );
    return;
  }

  const table = el("div", "peer-table peers-table");
  const header = el("div", "peer-table-head");
  header.append(
    el("span", "", t("Adresse")),
    el("span", "", t("Status")),
    el("span", "", t("Hentet")),
    el("span", "", t("Pieces")),
    el("span", "", t("Fejl")),
    el("span", "", t("Retning"))
  );
  table.appendChild(header);
  for (const peer of peers) {
    const row = el("div", "peer-table-row");
    row.append(
      el("span", "peer-address", peer.address),
      el("span", "peer-state", peer.state || "–"),
      el("span", "peer-mono", formatBytes(peer.downloaded_bytes || 0)),
      el("span", "peer-mono", String(peer.pieces || 0)),
      el("span", "peer-mono", String(peer.errors || 0)),
      el("span", "peer-direction", peer.incoming ? t("Ind") : t("Ud"))
    );
    table.appendChild(row);
  }
  panel.appendChild(table);
}

function statusPill(detail) {
  const pill = el("span", `status-pill ${stateClass(detail)}`);
  pill.append(el("span", "status-dot"), document.createTextNode(stateLabel(detail)));
  return pill;
}

function detailStat(label, value) {
  const stat = el("div", "detail-stat");
  stat.append(el("span", "", label), el("strong", "", value));
  return stat;
}

function actionButton(iconName, label, action) {
  const button = el("button", "btn btn-ghost btn-compact");
  button.type = "button";
  button.append(icon(iconName), document.createTextNode(label));
  button.addEventListener("click", action);
  return button;
}

function fileRow(detail, file) {
  const row = el("div", "file-row");
  if (!file.included) row.classList.add("is-excluded");

  if (state.user?.role === "admin") {
    const priority = el("select", "file-priority");
    priority.dataset.fileId = String(file.id);
    priority.setAttribute("aria-label", `${t("Prioritet")}: ${file.name}`);
    for (const [value, label] of [
      ["normal", t("Normal")],
      ["skip", t("Spring over")],
    ]) {
      const option = el("option", "", label);
      option.value = value;
      priority.appendChild(option);
    }
    priority.value = file.included ? "normal" : "skip";
    priority.addEventListener("change", () => {
      row.classList.toggle("is-excluded", priority.value === "skip");
    });
    row.appendChild(priority);
  }

  const fileIcon = el("div", "file-icon");
  fileIcon.appendChild(icon(file.kind === "video" ? "film" : file.kind === "audio" ? "music" : "file"));
  const name = el("span", "file-name", file.path || file.name);
  name.title = file.path || file.name;
  row.append(fileIcon, name, el("span", "file-size", formatBytes(file.length)));

  if (file.playable) {
    row.appendChild(
      iconButton("play", t("Afspil"), () =>
        openPlayer({
          title: file.name,
          stream_url: file.stream_url,
          file_name: file.name,
          kind: file.kind,
        })
      )
    );
  }
  if (detail.state === "error") row.title = detail.error || "";
  return row;
}

async function saveFileSelection(detail, button) {
  const selected = $$(".file-priority", $("#detail-body"))
    .filter((select) => select.value !== "skip")
    .map((select) => Number(select.dataset.fileId));
  if (!selected.length) {
    toast("Vælg mindst én fil.", "error");
    return;
  }
  setButtonLoading(button, true);
  try {
    await api(`/torrents/${detail.id}/files`, { method: "POST", json: { included: selected } });
    toast("Filvalget er gemt.");
    await openDetail(detail.id, true);
    await refreshTorrents();
  } catch (error) {
    toast(error.message, "error");
  } finally {
    setButtonLoading(button, false);
  }
}

/* Remove and confirm */

function confirmRemove(torrent) {
  confirmDialog({
    title: t("Fjern torrent?"),
    text: t("“{name}” fjernes fra listen. Du kan vælge også at slette de downloadede filer.", {
      name: torrent.name,
    }),
    confirmLabel: t("Fjern"),
    danger: true,
    showFiles: true,
    onConfirm: async () => {
      const deleteFiles = $("#confirm-delete-files").checked;
      await api(`/torrents/${torrent.id}?delete_files=${deleteFiles ? "true" : "false"}`, { method: "DELETE" });
      toast(deleteFiles ? "Torrent og filer er slettet." : "Torrentet er fjernet.");
      closeModal("detail-overlay");
      await refreshAll();
    },
  });
}

function confirmDialog({ title, text, confirmLabel, danger = false, showFiles = false, onConfirm }) {
  $("#confirm-title").textContent = title;
  $("#confirm-text").textContent = text;
  $("#confirm-ok").textContent = confirmLabel;
  $("#confirm-ok").className = `btn ${danger ? "btn-danger" : "btn-primary"}`;
  $("#confirm-files-row").hidden = !showFiles;
  $("#confirm-delete-files").checked = false;
  state.confirmAction = onConfirm;
  openModal("confirm-modal");
  $("#confirm-ok").focus();
}

async function runConfirm() {
  if (!state.confirmAction) return;
  const action = state.confirmAction;
  state.confirmAction = null;
  setButtonLoading($("#confirm-ok"), true);
  try {
    await action();
    closeModal("confirm-modal");
  } catch (error) {
    toast(error.message || "Handlingen mislykkedes.", "error");
  } finally {
    setButtonLoading($("#confirm-ok"), false);
  }
}

/* Players */

function openPlayer(item) {
  const modal = $("#player-modal");
  const video = $("#player-video");
  $("#player-title").textContent = item.title || item.file_name || "Afspiller";
  $("#player-message").hidden = true;
  video.hidden = false;
  video.src = item.stream_url;
  $("#player-download").href = item.stream_url;
  modal.hidden = false;
  document.body.classList.add("no-scroll");
  video.load();
  video.play().catch(() => {
    // Autoplay can be blocked; controls remain available.
  });
  $("#player-close").focus();
}

function closePlayer() {
  const video = $("#player-video");
  video.pause();
  video.removeAttribute("src");
  video.load();
  $("#player-modal").hidden = true;
  document.body.classList.remove("no-scroll");
}

/* Labels */

async function loadLabels() {
  try {
    const data = await api("/labels");
    state.labels = Array.isArray(data?.labels) ? data.labels : [];
    state.labelAssignments = data?.assignments || {};
    renderLabels();
    renderAddLabelPicker($("#add-label")?.value || "");
    if (state.labelFilter && !state.labels.includes(state.labelFilter)) {
      state.labelFilter = "";
    }
  } catch (error) {
    if (error.status !== 401) toast(error.message, "error");
  }
}

function renderLabels() {
  const list = $("#label-list");
  if (!list) return;
  $("#label-add-btn").hidden = state.user?.role !== "admin";
  const fragment = document.createDocumentFragment();
  const all = el("button", `label-filter${state.labelFilter ? "" : " active"}`, t("Alle"));
  all.type = "button";
  all.append(el("span", "label-count", String(state.torrents.length)));
  all.addEventListener("click", () => {
    state.labelFilter = "";
    renderLabels();
    renderTorrents();
  });
  fragment.appendChild(all);
  for (const label of state.labels) {
    const row = el("button", `label-filter${state.labelFilter === label ? " active" : ""}`);
    row.type = "button";
    row.append(document.createTextNode(label));
    row.append(
      el(
        "span",
        "label-count",
        String(state.torrents.filter((torrent) => torrent.label === label).length)
      )
    );
    row.addEventListener("click", () => {
      state.labelFilter = label;
      if (state.view !== "all") setView("all");
      else renderTorrents();
      renderLabels();
    });
    if (state.user?.role === "admin") {
      const remove = el("span", "label-remove", "×");
      remove.title = t("Slet label");
      remove.addEventListener("click", (event) => {
        event.stopPropagation();
        removeLabel(label);
      });
      row.appendChild(remove);
    }
    fragment.appendChild(row);
  }
  list.replaceChildren(fragment);
  renderMobileLabels();
}

function renderMobileLabels() {
  const strip = $("#mobile-labels");
  if (!strip) return;
  strip.hidden = false;
  const hasLabels = state.labels.length > 0 || state.user?.role === "admin";
  strip.dataset.empty = hasLabels ? "false" : "true";
  const fragment = document.createDocumentFragment();

  const toggle = el("button", `mobile-label-toggle${state.labelsVisible ? "" : " is-off"}`);
  toggle.type = "button";
  toggle.appendChild(icon(state.labelsVisible ? "eye" : "eye-off"));
  toggle.title = state.labelsVisible ? t("Skjul labels") : t("Vis labels");
  toggle.setAttribute("aria-label", toggle.title);
  toggle.addEventListener("click", toggleLabelsVisibility);
  fragment.appendChild(toggle);

  if (state.labelsVisible) {
    const all = el("button", `mobile-label-chip${state.labelFilter ? "" : " is-active"}`);
    all.type = "button";
    all.append(document.createTextNode(t("Alle")));
    all.append(el("span", "label-count", String(state.torrents.length)));
    all.addEventListener("click", () => {
      state.labelFilter = "";
      renderLabels();
      renderTorrents();
    });
    fragment.appendChild(all);

    for (const label of state.labels) {
      const chip = el(
        "button",
        `mobile-label-chip${state.labelFilter === label ? " is-active" : ""}`
      );
      chip.type = "button";
      chip.append(document.createTextNode(label));
      chip.append(
        el(
          "span",
          "label-count",
          String(state.torrents.filter((torrent) => torrent.label === label).length)
        )
      );
      chip.addEventListener("click", () => {
        state.labelFilter = label;
        if (state.view !== "all") setView("all");
        else renderTorrents();
        renderLabels();
      });
      fragment.appendChild(chip);
    }

    if (state.user?.role === "admin") {
      const add = el("button", "mobile-label-add");
      add.type = "button";
      add.appendChild(icon("plus"));
      add.title = t("Ny label");
      add.setAttribute("aria-label", add.title);
      add.addEventListener("click", openLabelModal);
      fragment.appendChild(add);
    }
  }

  strip.replaceChildren(fragment);
}

function renderLabelOptions(select, selected = "") {
  if (!select) return;
  const fragment = document.createDocumentFragment();
  const none = document.createElement("option");
  none.value = "";
  none.textContent = t("Ingen label");
  fragment.appendChild(none);
  for (const label of state.labels) {
    const option = document.createElement("option");
    option.value = label;
    option.textContent = label;
    fragment.appendChild(option);
  }
  select.replaceChildren(fragment);
  select.value = state.labels.includes(selected) ? selected : "";
}

function renderAddLabelPicker(selected = "") {
  const input = $("#add-label");
  if (!input) return;
  const value = state.labels.includes(selected) ? selected : "";
  input.value = value;
  $("#add-label-text").textContent = value || t("Ingen label");
  const list = $("#label-picker-options");
  const fragment = document.createDocumentFragment();
  const none = el("button", `label-option${value ? "" : " active"}`, t("Ingen label"));
  none.type = "button";
  none.addEventListener("click", () => selectAddLabel(""));
  fragment.appendChild(none);
  for (const label of state.labels) {
    const option = el("button", `label-option${value === label ? " active" : ""}`);
    option.type = "button";
    option.append(icon("folder"), document.createTextNode(label));
    option.append(
      el(
        "span",
        "label-count",
        String(state.torrents.filter((torrent) => torrent.label === label).length)
      )
    );
    option.addEventListener("click", () => selectAddLabel(label));
    fragment.appendChild(option);
  }
  list.replaceChildren(fragment);
}

function toggleAddLabelPicker() {
  const picker = $("#label-picker");
  picker.hidden = !picker.hidden;
  if (!picker.hidden) renderAddLabelPicker($("#add-label").value || "");
}

function selectAddLabel(label) {
  $("#add-label").value = label;
  $("#add-label-text").textContent = label || t("Ingen label");
  $("#label-picker").hidden = true;
  renderAddLabelPicker(label);
}

function openLabelModal() {
  $("#label-name").value = "";
  $("#label-error").hidden = true;
  openModal("label-modal");
  setTimeout(() => $("#label-name").focus(), 80);
}

async function createLabel(event) {
  event.preventDefault();
  const error = $("#label-error");
  error.hidden = true;
  try {
    const data = await api("/labels", {
      method: "POST",
      json: { name: $("#label-name").value.trim() },
    });
    state.labels = data.labels;
    state.labelAssignments = data.assignments;
    closeModal("label-modal");
    renderLabels();
    renderAddLabelPicker($("#add-label")?.value || "");
    toast(t("Labelen er oprettet."), "success");
  } catch (err) {
    error.textContent = err.message;
    error.hidden = false;
  }
}

function removeLabel(label) {
  confirmDialog({
    title: t("Slet label?"),
    text: t("“{name}” fjernes fra alle torrents.", { name: label }),
    confirmLabel: t("Slet"),
    danger: true,
    onConfirm: async () => {
      const data = await api(`/labels/${encodeURIComponent(label)}`, {
        method: "DELETE",
      });
      state.labels = data.labels;
      state.labelAssignments = data.assignments;
      if (state.labelFilter === label) state.labelFilter = "";
      renderLabels();
      renderAddLabelPicker("");
      renderTorrents();
      toast(t("Labelen er slettet."), "success");
    },
  });
}

async function setTorrentLabel(detail, label) {
  try {
    const updated = await api(`/torrents/${detail.id}/label`, {
      method: "PUT",
      json: { label: label || null },
    });
    state.selectedTorrent = updated;
    await loadLabels();
    await openDetail(updated.id, true);
    toast(t("Labelen er gemt."), "success");
  } catch (error) {
    toast(error.message, "error");
  }
}

/* RSS */

async function loadRss() {
  if (!state.user || state.user.role !== "admin") return;
  state.rssFeeds = await api("/rss");
  renderRss();
}

function renderRss() {
  const feeds = Array.isArray(state.rssFeeds) ? state.rssFeeds : [];
  $("#rss-empty").hidden = feeds.length > 0;
  const list = $("#rss-list");
  const fragment = document.createDocumentFragment();
  for (const feed of feeds) {
    const card = el("article", "rss-card");
    const head = el("div", "rss-card-head");
    const title = el("div", "rss-card-title");
    title.append(
      el("strong", "", feed.name || feed.url),
      el("span", "rss-feed-url", feed.url)
    );
    const badges = el("div", "rss-badges");
    badges.append(
      el("span", `rss-badge ${feed.enabled ? "on" : ""}`, feed.enabled ? t("Aktiv") : t("Slået fra")),
      el("span", `rss-badge ${feed.auto_download ? "on" : ""}`, feed.auto_download ? t("Auto") : t("Kun overvågning"))
    );
    head.append(title, badges);

    const meta = el("div", "rss-card-meta");
    meta.append(
      el("span", "", `${t("Mappe")}: ${feed.folder || t("standard")}`),
      el(
        "span",
        "",
        `${t("Filtre")}: ${feed.include || "–"}${feed.exclude ? ` · ${t("Udeluk")}: ${feed.exclude}` : ""}`
      ),
      el(
        "span",
        "",
        feed.last_checked
          ? `${t("Sidst tjekket")}: ${formatRssDate(feed.last_checked)} · ${feed.last_items} ${t("elementer")}`
          : t("Ikke tjekket endnu")
      )
    );
    if (feed.last_error) {
      const error = el("span", "error-text", feed.last_error);
      meta.appendChild(error);
    }

    const actions = el("div", "rss-card-actions");
    const check = el("button", "btn btn-ghost btn-compact", t("Tjek nu"));
    check.type = "button";
    check.addEventListener("click", () => checkRssFeed(feed, check));
    const edit = el("button", "btn btn-ghost btn-compact", t("Rediger"));
    edit.type = "button";
    edit.addEventListener("click", () => openRssModal(feed));
    const remove = el("button", "btn btn-ghost btn-compact btn-danger", t("Slet"));
    remove.type = "button";
    remove.addEventListener("click", () => deleteRssFeed(feed));
    actions.append(check, edit, remove);

    card.append(head, meta, actions);
    fragment.appendChild(card);
  }
  list.replaceChildren(fragment);
}

function formatRssDate(timestamp) {
  try {
    return new Date(timestamp * 1000).toLocaleString(
      getLanguage() === "en" ? "en-GB" : "da-DK"
    );
  } catch (error) {
    return "–";
  }
}

async function openRssFolderBrowser() {
  const panel = $("#rss-folder-browser");
  if (!panel.hidden) return;
  panel.hidden = false;
  await loadRssFolderBrowser(state.rssFolderBrowser.currentPath || "");
  $("#rss-folder-search").focus();
}

function closeRssFolderBrowser() {
  $("#rss-folder-browser").hidden = true;
}

async function loadRssFolderBrowser(path) {
  const entries = $("#rss-folder-entries");
  entries.replaceChildren(el("div", "folder-empty", "Henter mapper…"));
  try {
    const data = await api(`/download-browser?path=${encodeURIComponent(path || "")}`);
    const browser = state.rssFolderBrowser;
    browser.currentPath = data.path || "";
    browser.currentDisplay = data.display_path || browser.rootDisplay;
    if (!data.path) browser.rootDisplay = data.display_path || browser.rootDisplay;
    browser.parent = data.parent;
    browser.entries = Array.isArray(data.entries) ? data.entries : [];
    browser.query = "";
    $("#rss-folder-search").value = "";
    renderRssFolderBrowser();
  } catch (error) {
    entries.replaceChildren(
      el("div", "folder-empty", error.message || "Kunne ikke læse mappen.")
    );
  }
}

function renderRssFolderBrowser() {
  const browser = state.rssFolderBrowser;
  const entries = $("#rss-folder-entries");
  const crumbs = $("#rss-folder-breadcrumbs");
  const query = browser.query.trim().toLowerCase();

  crumbs.replaceChildren();
  const rootButton = el("button", "", browser.rootDisplay || "/downloads");
  rootButton.type = "button";
  rootButton.addEventListener("click", () => loadRssFolderBrowser(""));
  crumbs.appendChild(rootButton);

  let walked = "";
  for (const segment of browser.currentPath.split("/").filter(Boolean)) {
    walked = walked ? `${walked}/${segment}` : segment;
    const separator = el("span", "crumb-sep", "/");
    const button = el("button", "", segment);
    button.type = "button";
    const target = walked;
    button.addEventListener("click", () => loadRssFolderBrowser(target));
    crumbs.append(separator, button);
  }

  const filtered = browser.entries.filter((entry) => {
    if (!query) return true;
    return String(entry.name || "").toLowerCase().includes(query);
  });

  const rows = document.createDocumentFragment();
  if (browser.currentPath && browser.parent !== null) {
    const parent = el("button", "folder-row parent");
    parent.type = "button";
    parent.append(icon("arrow-up"), document.createTextNode(".."));
    parent.addEventListener("click", () => loadRssFolderBrowser(browser.parent));
    rows.appendChild(parent);
  }
  const selected = $("#rss-folder").value;
  for (const entry of filtered) {
    const row = el("button", `folder-row${entry.path === selected ? " selected" : ""}`);
    row.type = "button";
    row.append(icon("folder"), document.createTextNode(entry.name));
    row.addEventListener("click", () => loadRssFolderBrowser(entry.path));
    rows.appendChild(row);
  }
  if (!filtered.length) {
    rows.appendChild(
      el(
        "div",
        "folder-empty",
        query ? "Ingen mapper matcher søgningen." : "Der er ingen undermapper her."
      )
    );
  }
  entries.replaceChildren(rows);
  $("#rss-folder-current").textContent = t("Vælg: {path}", {
    path: `${browser.currentDisplay}/`,
  });
}

function selectRssFolder() {
  const browser = state.rssFolderBrowser;
  $("#rss-folder").value = browser.currentPath || "";
  $("#rss-folder-display").value = `${browser.currentDisplay}/`;
  $("#rss-folder-help").textContent = t("Valgt: {path}", {
    path: `${browser.currentDisplay}/`,
  });
  closeRssFolderBrowser();
}

async function openRssModal(feed = null) {
  state.rssEditingId = feed?.id || null;
  $("#rss-title").textContent = feed ? t("Rediger RSS-feed") : t("Tilføj RSS-feed");
  const folder = feed?.folder || "";
  $("#rss-name").value = feed?.name || "";
  $("#rss-url").value = feed?.url || "";
  $("#rss-folder").value = folder;
  $("#rss-folder-display").value = folder ? `/downloads/${folder}/` : "/downloads/";
  $("#rss-folder-help").textContent = folder
    ? t("Valgt: {path}", { path: `/downloads/${folder}/` })
    : "Vælg en mappe under downloadroden.";
  $("#rss-folder-browser").hidden = true;
  state.rssFolderBrowser.currentPath = folder;
  state.rssFolderBrowser.currentDisplay = folder
    ? `/downloads/${folder}`
    : "/downloads";
  $("#rss-include").value = feed?.include || "";
  $("#rss-exclude").value = feed?.exclude || "";
  $("#rss-enabled").checked = feed ? feed.enabled : true;
  $("#rss-auto").checked = feed ? feed.auto_download : true;
  $("#rss-paused").checked = feed ? feed.paused : false;
  $("#rss-error").hidden = true;
  openModal("rss-modal");
  setTimeout(() => $("#rss-name").focus(), 80);
}

async function onRssSubmit(event) {
  event.preventDefault();
  const error = $("#rss-error");
  error.hidden = true;
  const payload = {
    name: $("#rss-name").value.trim(),
    url: $("#rss-url").value.trim(),
    folder: $("#rss-folder").value.trim() || null,
    include: $("#rss-include").value.trim(),
    exclude: $("#rss-exclude").value.trim(),
    enabled: $("#rss-enabled").checked,
    auto_download: $("#rss-auto").checked,
    paused: $("#rss-paused").checked,
  };
  const button = $("#rss-submit");
  setButtonLoading(button, true);
  try {
    if (state.rssEditingId) {
      await api(`/rss/${state.rssEditingId}`, { method: "PUT", json: payload });
    } else {
      await api("/rss", { method: "POST", json: payload });
    }
    closeModal("rss-modal");
    toast(t("RSS-feedet er gemt."), "success");
    await loadRss();
  } catch (err) {
    error.textContent = err.message;
    error.hidden = false;
  } finally {
    setButtonLoading(button, false);
  }
}

async function checkRssFeed(feed, button) {
  setButtonLoading(button, true);
  try {
    await api(`/rss/${feed.id}/check`, { method: "POST" });
    toast(t("RSS-feedet er tjekket."), "success");
    await loadRss();
  } catch (error) {
    toast(error.message, "error");
  } finally {
    setButtonLoading(button, false);
  }
}

function deleteRssFeed(feed) {
  confirmDialog({
    title: t("Slet RSS-feed?"),
    text: t("“{name}” og dets automatiske regler slettes.", { name: feed.name || feed.url }),
    confirmLabel: t("Slet"),
    danger: true,
    onConfirm: async () => {
      await api(`/rss/${feed.id}`, { method: "DELETE" });
      toast(t("RSS-feedet er slettet."), "success");
      await loadRss();
    },
  });
}

/* Settings */

function renderSettings() {
  const system = state.system;
  if (system) {
    $("#settings-version").textContent = system.version || state.version;
    $("#settings-uptime").textContent = formatUptime(system.uptime_seconds);
    $("#settings-ports").textContent = `${system.torrent_port_start}–${system.torrent_port_end}`;
    const pathsRow = $("#settings-paths-row");
    if (system.paths) {
      pathsRow.hidden = false;
      $("#settings-download-dir").textContent = system.paths.download_dir;
    } else {
      pathsRow.hidden = true;
    }
  }
  syncPreferences();
  renderTotpStatus();
  renderUsers();
}

function renderTotpStatus() {
  const enabled = Boolean(state.user?.totp_enabled);
  $("#totp-status").textContent = enabled
    ? t("2FA er slået til for denne konto.")
    : t("2FA er slået fra.");
  $("#totp-enable-btn").hidden = enabled;
  $("#totp-disable-btn").hidden = !enabled;
}

async function openTotpSetup() {
  const error = $("#totp-setup-error");
  error.hidden = true;
  $("#totp-code").value = "";
  $("#totp-confirm").disabled = true;
  try {
    const setup = await api("/auth/totp/setup", { method: "POST" });
    $("#totp-secret").value = setup.secret || "";
    $("#totp-uri").textContent = setup.otpauth_url || "";
    const qr = $("#totp-qr");
    if (setup.qr_svg) {
      qr.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(setup.qr_svg)}`;
      qr.hidden = false;
    } else {
      qr.removeAttribute("src");
      qr.hidden = true;
    }
    // The confirm button is enabled once a 6-digit code is entered.
    const update = () => {
      $("#totp-confirm").disabled = !/^\d{6}$/.test($("#totp-code").value.trim());
    };
    $("#totp-code").addEventListener("input", update);
    openModal("totp-setup-modal");
    setTimeout(() => $("#totp-code").focus(), 80);
  } catch (err) {
    toast(err.message, "error");
  }
}

async function confirmTotpSetup() {
  const error = $("#totp-setup-error");
  error.hidden = true;
  const code = $("#totp-code").value.trim();
  if (!/^\d{6}$/.test(code)) {
    error.textContent = t("Indtast en 6-cifret kode.");
    error.hidden = false;
    return;
  }
  const button = $("#totp-confirm");
  setButtonLoading(button, true);
  try {
    state.user = await api("/auth/totp/enable", { method: "POST", json: { code } });
    closeModal("totp-setup-modal");
    renderTotpStatus();
    toast(t("2FA er nu slået til."), "success");
  } catch (err) {
    error.textContent = err.message;
    error.hidden = false;
  } finally {
    setButtonLoading(button, false);
    $("#totp-confirm").disabled = false;
  }
}

function openTotpDisable() {
  $("#totp-disable-password").value = "";
  $("#totp-disable-error").hidden = true;
  openModal("totp-disable-modal");
  setTimeout(() => $("#totp-disable-password").focus(), 80);
}

async function confirmTotpDisable() {
  const error = $("#totp-disable-error");
  error.hidden = true;
  const password = $("#totp-disable-password").value;
  if (!password) {
    error.textContent = t("Indtast din adgangskode.");
    error.hidden = false;
    return;
  }
  const button = $("#totp-disable-confirm");
  setButtonLoading(button, true);
  try {
    state.user = await api("/auth/totp/disable", {
      method: "POST",
      json: { password },
    });
    closeModal("totp-disable-modal");
    renderTotpStatus();
    toast(t("2FA er slået fra."), "success");
  } catch (err) {
    error.textContent = err.message;
    error.hidden = false;
  } finally {
    setButtonLoading(button, false);
  }
}

function copyTotpSecret() {
  const secret = $("#totp-secret").value;
  if (!secret) return;
  navigator.clipboard?.writeText(secret).then(
    () => toast(t("Nøglen er kopieret."), "success"),
    () => {
      $("#totp-secret").select();
      toast(t("Kunne ikke kopiere automatisk. Nøglen er markeret."), "error");
    }
  );
}

function applyTvLibraryVisibility() {
  const navItem = $('.nav-item[data-view="library"]');
  if (navItem) navItem.hidden = !state.tvLibraryVisible;
  const toggle = $("#tv-library-toggle");
  if (toggle) toggle.checked = state.tvLibraryVisible;
}

function applyLabelsVisibility() {
  const list = $("#label-list");
  if (list) list.hidden = !state.labelsVisible;
  const toggle = $("#labels-visible-toggle");
  if (toggle) toggle.checked = state.labelsVisible;
  const button = $("#labels-visibility-btn");
  if (button) {
    button.title = state.labelsVisible ? t("Skjul labels") : t("Vis labels");
    button.setAttribute("aria-label", button.title);
    const use = button.querySelector("use");
    if (use) use.setAttribute("href", state.labelsVisible ? "#i-eye" : "#i-eye-off");
  }
  renderMobileLabels();
}

function setLabelsVisibility(visible) {
  state.labelsVisible = Boolean(visible);
  try {
    localStorage.setItem(LABELS_VISIBLE_KEY, state.labelsVisible ? "on" : "off");
  } catch (error) {}
  applyLabelsVisibility();
  if (!state.labelsVisible && state.labelFilter) {
    state.labelFilter = "";
    renderTorrents();
  }
}

function onLabelsVisibilityToggle(event) {
  setLabelsVisibility(event.target.checked);
}

function toggleLabelsVisibility() {
  setLabelsVisibility(!state.labelsVisible);
}

function onTvLibraryToggle(event) {
  state.tvLibraryVisible = event.target.checked;
  try {
    localStorage.setItem(TV_LIBRARY_KEY, state.tvLibraryVisible ? "on" : "off");
  } catch (error) {}
  applyTvLibraryVisibility();
  if (!state.tvLibraryVisible && state.view === "library") {
    setView("all");
  }
}

function syncPreferences() {
  applyTvLibraryVisibility();
  applyLabelsVisibility();
  const themeSelect = $("#theme-select");
  const languageSelect = $("#language-select");
  if (themeSelect && window.__nordtorrentTheme) {
    themeSelect.value = window.__nordtorrentTheme.getPreference();
  }
  if (languageSelect && window.NordTorrentI18n) {
    languageSelect.value = window.NordTorrentI18n.getLanguage();
  }
  const authLanguage = $("#auth-language-select");
  if (authLanguage && window.NordTorrentI18n) {
    authLanguage.value = window.NordTorrentI18n.getLanguage();
  }
  const authTheme = $("#auth-theme-toggle");
  if (authTheme && window.__nordtorrentTheme) {
    const light = window.__nordtorrentTheme.resolved() === "light";
    authTheme.innerHTML = "";
    authTheme.appendChild(icon(light ? "moon" : "sun"));
    authTheme.title = t(light ? "Skift til mørkt tema" : "Skift til lyst tema");
    authTheme.setAttribute("aria-label", authTheme.title);
  }
}

function onThemeChange(event) {
  if (window.__nordtorrentTheme) {
    window.__nordtorrentTheme.setPreference(event.target.value);
  }
}

function onLanguageChange(event) {
  if (window.NordTorrentI18n) {
    window.NordTorrentI18n.setLanguage(event.target.value);
  }
}

function renderUsers() {
  if (!state.user || state.user.role !== "admin") return;
  const list = $("#users-list");
  const fragment = document.createDocumentFragment();
  for (const user of state.users) {
    const row = el("div", "user-row");
    const avatar = el("span", "avatar", initials(user.display_name || user.username));
    const meta = el("div", "user-meta");
    meta.append(el("strong", "", user.display_name || user.username), el("span", "", `${user.username} · ${roleLabel(user.role)}`));
    row.append(avatar, meta);
    if (user.username !== state.user.username) {
      const remove = iconButton("trash", "Slet bruger", () => {
        confirmDialog({
          title: "Slet bruger?",
          text: `${user.username} mister adgang med det samme.`,
          confirmLabel: "Slet",
          danger: true,
          onConfirm: async () => {
            await api(`/users/${encodeURIComponent(user.username)}`, { method: "DELETE" });
            toast("Brugeren er slettet.");
            await loadUsers();
          },
        });
      });
      remove.classList.add("danger");
      row.appendChild(remove);
    } else {
      row.appendChild(el("span", "badge", "Dig"));
    }
    fragment.appendChild(row);
  }
  list.replaceChildren(fragment);
}

/* Add modal */

function openAddModal() {
  if (state.user?.role !== "admin") return;
  state.moveTorrentTarget = null;
  $("#add-modal").classList.remove("is-move");
  $("#add-eyebrow").textContent = t("NY DOWNLOAD");
  $("#add-title").textContent = t("Tilføj torrent");
  resetAddForms();
  loadDownloadFolders();
  loadLabels().then(() => renderAddLabelPicker(""));
  openModal("add-modal");
  switchAddTab("magnet");
  setTimeout(() => $("#magnet-input").focus(), 80);
}

async function openMoveModal(detail) {
  if (state.user?.role !== "admin") return;
  state.moveTorrentTarget = detail;
  resetAddForms();
  $("#add-modal").classList.add("is-move");
  $("#add-eyebrow").textContent = t("FLYT TORRENT");
  $("#add-title").textContent = t("Flyt torrent");
  $("#download-folder-help").textContent = t(
    "Vælg den nye downloadmappe. Torrenten pauses, flyttes og startes igen."
  );
  openModal("add-modal");
  await loadDownloadFolders();
  const folder = String(detail.folder || "");
  state.folderBrowser.currentPath = folder.split("/").slice(0, -1).join("/");
  $("#download-folder-help").textContent = t(
    "Vælg den nye downloadmappe. Torrenten pauses, flyttes og startes igen."
  );
  await openFolderBrowser();
}

function moveTorrentTo(detail, folder) {
  const target = folder === "." ? t("downloadroden") : folder;
  confirmDialog({
    title: t("Flyt torrent?"),
    text: t("Flyt torrenten til {path}? Den pauses, flyttes og startes igen.", {
      path: target,
    }),
    confirmLabel: t("Flyt"),
    danger: false,
    onConfirm: async () => {
      const moved = await api(`/torrents/${detail.id}/move`, {
        method: "POST",
        json: { folder },
      });
      state.moveTorrentTarget = null;
      state.selectedTorrent = moved;
      closeModal("add-modal");
      toast(t("Torrenten er flyttet."));
      await refreshAll();
      await openDetail(moved.id, true);
    },
  });
}

async function loadDownloadFolders() {
  const help = $("#download-folder-help");
  $("#download-folder-display").value = "Henter mapper…";
  $("#folder-browser").hidden = true;
  state.folderBrowser.open = false;
  try {
    const [folders, root] = await Promise.all([
      api("/download-folders"),
      api("/download-browser?path="),
    ]);
    state.folderBrowser.rootDisplay = root.display_path || "/downloads";
    state.folderBrowser.currentPath = "";
    state.folderBrowser.currentDisplay = state.folderBrowser.rootDisplay;
    state.folderBrowser.parent = null;
    state.folderBrowser.entries = Array.isArray(root.entries) ? root.entries : [];
    state.folderBrowser.query = "";
    $("#folder-search").value = "";
    const defaultFolder = Array.isArray(folders)
      ? folders.find((folder) => folder.default) || folders[0]
      : null;
    if (defaultFolder) {
      const display =
        defaultFolder.path === "."
          ? state.folderBrowser.rootDisplay
          : `${state.folderBrowser.rootDisplay.replace(/\/$/, "")}/${defaultFolder.path}`;
      const parentPath =
        defaultFolder.path === "."
          ? ""
          : defaultFolder.path.split("/").slice(0, -1).join("/");
      state.folderBrowser.currentPath = parentPath;
      state.folderBrowser.currentDisplay = parentPath
        ? `${state.folderBrowser.rootDisplay.replace(/\/$/, "")}/${parentPath}`
        : state.folderBrowser.rootDisplay;
      setFolderSelection(defaultFolder.path, display);
      help.textContent =
        folders.length === 1
          ? t("Serveren fandt én downloadmappe.")
          : t("Serveren fandt {count} downloadmapper. Klik Gennemse for at vælge undermappe.", {
              count: folders.length,
            });
    } else {
      setFolderSelection(".", state.folderBrowser.rootDisplay);
      help.textContent = "Ingen særskilte downloadmapper fundet; downloadroden bruges.";
    }
  } catch (error) {
    setFolderSelection("", state.folderBrowser.rootDisplay);
    help.textContent = error.message || "Kunne ikke hente downloadmapper.";
  }
}

function setFolderSelection(path, displayPath) {
  $("#download-folder").value = path;
  const display = String(displayPath || "/downloads").replace(/\/+$/, "");
  $("#download-folder-display").value = `${display}/`;
}

async function openFolderBrowser() {
  const panel = $("#folder-browser");
  if (!panel.hidden) return;
  state.folderBrowser.open = true;
  panel.hidden = false;
  await loadFolderBrowser(state.folderBrowser.currentPath || "");
  $("#folder-search").focus();
}

function closeFolderBrowser() {
  $("#folder-browser").hidden = true;
  state.folderBrowser.open = false;
}

async function loadFolderBrowser(path) {
  const selectedPath = path || "";
  const entries = $("#folder-entries");
  entries.replaceChildren(el("div", "folder-empty", "Henter mapper…"));
  try {
    const data = await api(`/download-browser?path=${encodeURIComponent(selectedPath)}`);
    state.folderBrowser.currentPath = data.path || "";
    state.folderBrowser.currentDisplay = data.display_path || state.folderBrowser.rootDisplay;
    state.folderBrowser.parent = data.parent;
    state.folderBrowser.entries = Array.isArray(data.entries) ? data.entries : [];
    state.folderBrowser.query = "";
    $("#folder-search").value = "";
    renderFolderBrowser();
  } catch (error) {
    entries.replaceChildren(el("div", "folder-empty", error.message || "Kunne ikke læse mappen."));
  }
}

function renderFolderBrowser() {
  const browser = state.folderBrowser;
  const entries = $("#folder-entries");
  const crumbs = $("#folder-breadcrumbs");
  const query = browser.query.trim().toLowerCase();

  crumbs.replaceChildren();
  const rootButton = el("button", "", browser.rootDisplay || "/downloads");
  rootButton.type = "button";
  rootButton.addEventListener("click", () => loadFolderBrowser(""));
  crumbs.appendChild(rootButton);

  let walked = "";
  for (const segment of browser.currentPath.split("/").filter(Boolean)) {
    walked = walked ? `${walked}/${segment}` : segment;
    const separator = el("span", "crumb-sep", "/");
    const button = el("button", "", segment);
    button.type = "button";
    const target = walked;
    button.addEventListener("click", () => loadFolderBrowser(target));
    crumbs.append(separator, button);
  }

  const filtered = browser.entries.filter((entry) => {
    if (!query) return true;
    return String(entry.name || "").toLowerCase().includes(query);
  });

  const rows = document.createDocumentFragment();
  if (browser.currentPath && browser.parent !== null) {
    const parent = el("button", "folder-row parent");
    parent.type = "button";
    parent.append(icon("arrow-up"), document.createTextNode(".."));
    parent.addEventListener("click", () => loadFolderBrowser(browser.parent));
    rows.appendChild(parent);
  }
  const selected = $("#download-folder").value;
  for (const entry of filtered) {
    const row = el("button", `folder-row${entry.path === selected ? " selected" : ""}`);
    row.type = "button";
    row.append(icon("folder"), document.createTextNode(entry.name));
    row.addEventListener("click", () => loadFolderBrowser(entry.path));
    rows.appendChild(row);
  }
  if (!filtered.length) {
    rows.appendChild(
      el("div", "folder-empty", query ? "Ingen mapper matcher søgningen." : "Der er ingen undermapper her.")
    );
  }
  entries.replaceChildren(rows);
  $("#folder-current").textContent = t("Vælg: {path}", { path: `${browser.currentDisplay}/` });
}

function selectCurrentFolder() {
  const browser = state.folderBrowser;
  const path = browser.currentPath ? browser.currentPath : ".";
  if (state.moveTorrentTarget) {
    const target = state.moveTorrentTarget;
    closeFolderBrowser();
    moveTorrentTo(target, path);
    return;
  }
  setFolderSelection(path, browser.currentDisplay);
  closeFolderBrowser();
  $("#download-folder-help").textContent = t("Valgt: {path}", {
    path: `${browser.currentDisplay}/`,
  });
}

function resetAddForms() {
  $("#magnet-form").reset();
  $("#file-form").reset();
  $("#magnet-error").hidden = true;
  $("#file-error").hidden = true;
  $("#folder-browser").hidden = true;
  $("#label-picker").hidden = true;
  $("#download-folder").value = "";
  $("#download-folder-display").value = "/downloads/";
  $("#folder-search").value = "";
  state.folderBrowser.open = false;
  state.folderBrowser.query = "";
  $("#dropzone-title").textContent = "Vælg eller slip en .torrent-fil";
  $("#dropzone-sub").textContent = "Maks. 20 MB · filen bliver på din server";
  setButtonLoading($("#magnet-submit"), false);
  setButtonLoading($("#file-submit"), false);
}

function switchAddTab(tab) {
  for (const button of $$("[data-add-tab]")) {
    button.classList.toggle("is-active", button.dataset.addTab === tab);
  }
  $("#magnet-form").hidden = tab !== "magnet";
  $("#file-form").hidden = tab !== "file";
}

/* Modals */

function openModal(id) {
  const modal = document.getElementById(id);
  if (!modal) return;
  modal.hidden = false;
  document.body.classList.add("no-scroll");
}

function closeModal(id) {
  const modal = document.getElementById(id);
  if (!modal) return;
  modal.hidden = true;
  if (id === "detail-overlay") state.selectedTorrent = null;
  if (id === "confirm-modal") state.confirmAction = null;
  if ($$(".modal:not([hidden]), .drawer-overlay:not([hidden]), .player-modal:not([hidden])").length === 0) {
    document.body.classList.remove("no-scroll");
  }
}

function setButtonLoading(button, loading) {
  if (!button) return;
  button.classList.toggle("loading", loading);
  button.disabled = loading;
}

/* Toasts */

function toast(message, type = "info") {
  const container = $("#toasts");
  const node = el("div", `toast ${type === "error" ? "error" : ""}`);
  node.appendChild(icon(type === "error" ? "alert" : "check"));
  node.appendChild(el("span", "", message));
  container.appendChild(node);
  window.setTimeout(() => {
    node.classList.add("is-leaving");
    window.setTimeout(() => node.remove(), 190);
  }, type === "error" ? 6000 : 3800);
}

/* Events */

function bindGlobalEvents() {
  window.addEventListener("popstate", () => {
    if (state.user) setView(viewFromPath(), { updateUrl: false });
  });
  for (const item of $$(".nav-item")) {
    item.addEventListener("click", () => setView(item.dataset.view));
  }

  $("#login-form").addEventListener("submit", onLogin);
  $("#setup-form").addEventListener("submit", onSetup);
  $("#logout-btn").addEventListener("click", onLogout);
  $("#settings-logout-btn").addEventListener("click", onLogout);
  $("#refresh-btn").addEventListener("click", async () => {
    const button = $("#refresh-btn");
    button.classList.add("loading");
    await refreshAll();
    button.classList.remove("loading");
  });
  $("#port-status").addEventListener("click", () => setView("settings"));
  $("#network-form").addEventListener("submit", saveNetworkSettings);
  $("#network-test").addEventListener("click", checkNetworkPort);
  $("#speed-save").addEventListener("click", saveSpeedLimits);
  $("#speed-download-enabled").addEventListener("change", updateSpeedRows);
  $("#speed-upload-enabled").addEventListener("change", updateSpeedRows);
  $("#seeding-save").addEventListener("click", saveSeedingSettings);
  $("#seeding-keep").addEventListener("change", updateSeedingRows);
  $("#dht-save").addEventListener("click", saveDhtSettings);
  $("#totp-enable-btn").addEventListener("click", openTotpSetup);
  $("#totp-disable-btn").addEventListener("click", openTotpDisable);
  $("#totp-confirm").addEventListener("click", confirmTotpSetup);
  $("#totp-disable-confirm").addEventListener("click", confirmTotpDisable);
  $("#totp-copy-secret").addEventListener("click", copyTotpSecret);
  $("#theme-select").addEventListener("change", onThemeChange);
  $("#language-select").addEventListener("change", onLanguageChange);
  $("#tv-library-toggle").addEventListener("change", onTvLibraryToggle);
  $("#labels-visible-toggle").addEventListener("change", onLabelsVisibilityToggle);
  $("#sessions-refresh").addEventListener("click", loadSessions);
  const authLanguage = $("#auth-language-select");
  if (authLanguage) authLanguage.addEventListener("change", onLanguageChange);
  const authTheme = $("#auth-theme-toggle");
  if (authTheme) {
    authTheme.addEventListener("click", () => {
      if (!window.__nordtorrentTheme) return;
      const current = window.__nordtorrentTheme.resolved();
      window.__nordtorrentTheme.setPreference(current === "light" ? "dark" : "light");
      syncPreferences();
    });
  }

  for (const button of $$("#add-btn, [data-action='open-add']")) {
    button.addEventListener("click", openAddModal);
  }
  $("#rss-add-btn").addEventListener("click", () => openRssModal());
  for (const button of $$("[data-action='rss-add']")) {
    button.addEventListener("click", () => openRssModal());
  }
  $("#rss-form").addEventListener("submit", onRssSubmit);
  $("#labels-visibility-btn").addEventListener("click", toggleLabelsVisibility);
  $("#label-add-btn").addEventListener("click", openLabelModal);
  $("#label-form").addEventListener("submit", createLabel);
  $("#add-label-display").addEventListener("click", toggleAddLabelPicker);
  $("#label-picker-new").addEventListener("click", () => {
    $("#label-picker").hidden = true;
    openLabelModal();
  });
  document.addEventListener("click", (event) => {
    if (!event.target.closest(".add-label")) {
      $("#label-picker").hidden = true;
    }
  });
  $("#rss-folder-browse").addEventListener("click", openRssFolderBrowser);
  $("#rss-folder-browser-close").addEventListener("click", closeRssFolderBrowser);
  $("#rss-folder-select").addEventListener("click", selectRssFolder);
  $("#rss-folder-search").addEventListener("input", (event) => {
    state.rssFolderBrowser.query = event.target.value;
    renderRssFolderBrowser();
  });

  $("#search-input").addEventListener("input", (event) => {
    state.search = event.target.value;
    if (state.view === "library") renderLibrary();
    else if (state.view !== "settings") renderTorrents();
  });

  for (const button of $$("[data-add-tab]")) {
    button.addEventListener("click", () => switchAddTab(button.dataset.addTab));
  }
  $("#magnet-form").addEventListener("submit", onAddMagnet);
  $("#file-form").addEventListener("submit", onUploadFile);
  $("#download-folder-browse").addEventListener("click", openFolderBrowser);
  $("#folder-browser-close").addEventListener("click", closeFolderBrowser);
  $("#folder-select").addEventListener("click", selectCurrentFolder);
  $("#folder-search").addEventListener("input", (event) => {
    state.folderBrowser.query = event.target.value;
    renderFolderBrowser();
  });
  $("#torrent-file").addEventListener("change", (event) => {
    const file = event.target.files?.[0];
    if (file) updateDropzone(file.name);
  });

  const dropzone = $("#dropzone");
  for (const eventName of ["dragenter", "dragover"]) {
    dropzone.addEventListener(eventName, (event) => {
      event.preventDefault();
      dropzone.classList.add("is-dragging");
    });
  }
  for (const eventName of ["dragleave", "drop"]) {
    dropzone.addEventListener(eventName, (event) => {
      event.preventDefault();
      dropzone.classList.remove("is-dragging");
    });
  }
  dropzone.addEventListener("drop", (event) => {
    const file = event.dataTransfer?.files?.[0];
    if (!file) return;
    const transfer = new DataTransfer();
    transfer.items.add(file);
    $("#torrent-file").files = transfer.files;
    updateDropzone(file.name);
  });

  for (const element of $$("[data-close]")) {
    element.addEventListener("click", () => closeModal(element.dataset.close));
  }
  $("#confirm-ok").addEventListener("click", runConfirm);
  $("#player-close").addEventListener("click", closePlayer);
  $("#player-video").addEventListener("error", () => {
    $("#player-message").hidden = false;
    $("#player-video").hidden = true;
  });

  $("#password-form").addEventListener("submit", onChangePassword);
  $("#create-user-form").addEventListener("submit", onCreateUser);

  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      if (!$("#player-modal").hidden) return closePlayer();
      if (!$("#confirm-modal").hidden) return closeModal("confirm-modal");
      if (!$("#add-modal").hidden) return closeModal("add-modal");
      if (!$("#detail-overlay").hidden) return closeModal("detail-overlay");
    }
    if (event.key === "/" && !isTypingTarget(event.target)) {
      event.preventDefault();
      $("#search-input").focus();
    }
    if ((event.key === "n" || event.key === "N") && !isTypingTarget(event.target) && state.user?.role === "admin") {
      event.preventDefault();
      openAddModal();
    }
  });

  document.addEventListener("visibilitychange", () => {
    if (!document.hidden && state.user) refreshAll();
  });
}

function isTypingTarget(target) {
  if (!(target instanceof HTMLElement)) return false;
  return ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName) || target.isContentEditable;
}

/* Auth actions */

async function onLogin(event) {
  event.preventDefault();
  const errorNode = $("#login-error");
  errorNode.hidden = true;
  const button = $("#login-submit");
  setButtonLoading(button, true);
  try {
    const result = await api("/auth/login", {
      method: "POST",
      skipAuth: true,
      json: {
        username: $("#login-username").value.trim(),
        password: $("#login-password").value,
        totp_code: $("#login-totp-field").hidden
          ? null
          : $("#login-totp").value.trim(),
      },
    });
    $("#login-form").reset();
    $("#login-totp-field").hidden = true;
    await enterApp(result.user);
    toast(`Velkommen, ${result.user.display_name || result.user.username}.`);
  } catch (error) {
    if (error instanceof ApiError && error.code === "totp_required") {
      $("#login-totp-field").hidden = false;
      $("#login-totp").focus();
    }
    if (error instanceof ApiError && error.code === "totp_invalid") {
      $("#login-totp-field").hidden = false;
      $("#login-totp").select();
    }
    errorNode.textContent = error.message;
    errorNode.hidden = false;
  } finally {
    setButtonLoading(button, false);
  }
}

async function onSetup(event) {
  event.preventDefault();
  const errorNode = $("#setup-error");
  errorNode.hidden = true;
  const password = $("#setup-password").value;
  if (password !== $("#setup-password-confirm").value) {
    errorNode.textContent = "Adgangskoderne er ikke ens.";
    errorNode.hidden = false;
    return;
  }
  const button = $("#setup-submit");
  setButtonLoading(button, true);
  try {
    const result = await api("/auth/setup", {
      method: "POST",
      skipAuth: true,
      json: {
        username: $("#setup-username").value.trim(),
        display_name: $("#setup-display-name").value.trim(),
        password,
      },
    });
    $("#setup-form").reset();
    await enterApp(result.user);
    toast("Serveren er klar. Velkommen!");
  } catch (error) {
    errorNode.textContent = error.message;
    errorNode.hidden = false;
  } finally {
    setButtonLoading(button, false);
  }
}

async function onLogout() {
  try {
    await api("/auth/logout", { method: "POST" });
  } catch {
    // The cookie is cleared locally even if the request fails.
  }
  state.user = null;
  state.torrents = [];
  state.library = [];
  stopPolling();
  window.history.replaceState(null, "", "/");
  showAuth("login");
  toast("Du er logget ud.");
}

/* Add actions */

async function onAddMagnet(event) {
  event.preventDefault();
  const errorNode = $("#magnet-error");
  errorNode.hidden = true;
  const magnet = $("#magnet-input").value.trim();
  if (!magnet) {
    errorNode.textContent = "Indsæt et magnet-link eller en info-hash.";
    errorNode.hidden = false;
    return;
  }
  const button = $("#magnet-submit");
  setButtonLoading(button, true);
  try {
    await api("/torrents", {
      method: "POST",
      json: {
        magnet,
        paused: $("#magnet-paused").checked,
        folder: $("#download-folder").value || null,
        label: $("#add-label").value || null,
      },
    });
    closeModal("add-modal");
    toast(t("Torrentet er tilføjet."));
    await refreshAll();
    await loadLabels();
  } catch (error) {
    errorNode.textContent = error.message;
    errorNode.hidden = false;
  } finally {
    setButtonLoading(button, false);
  }
}

async function onUploadFile(event) {
  event.preventDefault();
  const errorNode = $("#file-error");
  errorNode.hidden = true;
  const file = $("#torrent-file").files?.[0];
  if (!file) {
    errorNode.textContent = "Vælg en .torrent-fil først.";
    errorNode.hidden = false;
    return;
  }
  const form = new FormData();
  const fileName = /\.torrent$/i.test(file.name) ? file.name : file.name + ".torrent";
  form.append("file", file, fileName);
  form.append("paused", $("#file-paused").checked ? "true" : "false");
  form.append("folder", $("#download-folder").value || "");
  form.append("label", $("#add-label").value || "");
  const button = $("#file-submit");
  setButtonLoading(button, true);
  try {
    await api("/torrents/upload", { method: "POST", form });
    closeModal("add-modal");
    toast(t("Torrentet er tilføjet."));
    await refreshAll();
    await loadLabels();
  } catch (error) {
    errorNode.textContent = error.message;
    errorNode.hidden = false;
  } finally {
    setButtonLoading(button, false);
  }
}

function updateDropzone(name) {
  $("#dropzone-title").textContent = name;
  $("#dropzone-sub").textContent = "Klar til upload · filen bliver på din server";
}

/* Settings actions */

async function onChangePassword(event) {
  event.preventDefault();
  const errorNode = $("#password-error");
  errorNode.hidden = true;
  const button = event.submitter;
  setButtonLoading(button, true);
  try {
    await api("/auth/password", {
      method: "POST",
      json: {
        current_password: $("#current-password").value,
        new_password: $("#new-password").value,
      },
    });
    $("#password-form").reset();
    toast("Adgangskoden er ændret. Andre enheder skal logge ind igen.");
  } catch (error) {
    errorNode.textContent = error.message;
    errorNode.hidden = false;
  } finally {
    setButtonLoading(button, false);
  }
}

async function onCreateUser(event) {
  event.preventDefault();
  const errorNode = $("#user-error");
  errorNode.hidden = true;
  const button = event.submitter;
  setButtonLoading(button, true);
  try {
    await api("/users", {
      method: "POST",
      json: {
        username: $("#new-user-username").value.trim(),
        display_name: $("#new-user-username").value.trim(),
        password: $("#new-user-password").value,
        role: $("#new-user-role").value,
      },
    });
    $("#create-user-form").reset();
    toast("Brugeren er oprettet.");
    await loadUsers();
  } catch (error) {
    errorNode.textContent = error.message;
    errorNode.hidden = false;
  } finally {
    setButtonLoading(button, false);
  }
}

init();

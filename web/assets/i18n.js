/* NordTorrent UI language layer.
 * Danish is the source language. English is applied to exact UI strings and
 * through the small t() helper for interpolated text. */
(function () {
  "use strict";

  var LANGUAGE_KEY = "nordtorrent.language";
  var SUPPORTED = ["da", "en"];

  var en = {
    "PRIVAT. HURTIGT. DIT.": "PRIVATE. FAST. YOURS.",
    "Filmene bliver": "Your movies stay",
    "hjemme hos dig.": "right at home.",
    "En rolig, hurtig torrentklient til din egen server – med webpanel, sikker API og Android TV-app.":
      "A calm, fast torrent client for your own server – with a web panel, secure API and Android TV app.",
    "Ingen cloud-konto. Dine data bliver lokalt.": "No cloud account. Your data stays local.",
    "Stream direkte til TV'et.": "Stream straight to your TV.",
    "Bygget til Proxmox og Docker.": "Built for Proxmox and Docker.",
    "VELKOMMEN TILBAGE": "WELCOME BACK",
    "Log ind": "Sign in",
    "Brug din konto fra NordTorrent-serveren.": "Use your NordTorrent server account.",
    "Brugernavn": "Username",
    "Adgangskode": "Password",
    "Forbindelsen bør køre over HTTPS, når serveren nås udefra.":
      "Use HTTPS when the server is reachable from the internet.",
    "FØRSTE OPSÆTNING": "FIRST-TIME SETUP",
    "Opret administrator": "Create administrator",
    "Lav den første konto. Den kan oprette seere til Android TV bagefter.":
      "Create the first account. You can add viewers for Android TV afterwards.",
    "Visningsnavn": "Display name",
    "Mindst 12 tegn. Brug en sætning, du kan huske.":
      "At least 12 characters. Use a phrase you can remember.",
    "Gentag adgangskode": "Confirm password",
    "Opret server": "Create server",
    "· Selvhostet med omtanke": "· Self-hosted with care",
    "Alle torrents": "All torrents",
    "Downloader": "Downloading",
    "Færdige": "Completed",
    "TV-bibliotek": "TV library",
    "Indstillinger": "Settings",
    "Serveren kører": "Server is running",
    "Port ukendt": "Port unknown",
    "Tilføj": "Add",
    "DIT PERSONLIGE NETVÆRK": "YOUR PERSONAL NETWORK",
    "Roligt overblik.": "Calm overview.",
    "Fuld fart.": "Full speed.",
    "Henter status fra serveren…": "Fetching server status…",
    "Tilføj torrent": "Add torrent",
    "Åbn TV-bibliotek": "Open TV library",
    "B/s samlet": "B/s total",
    "Download": "Download",
    "Upload": "Upload",
    "Aktive": "Active",
    "Hentet": "Downloaded",
    "hentet": "downloaded",
    "pauset": "paused",
    "OVERSIGT": "OVERVIEW",
    "Vælg": "Select",
    "Ingen torrents endnu": "No torrents yet",
    "Tilføj et magnet-link eller en .torrent-fil for at komme i gang.":
      "Add a magnet link or a .torrent file to get started.",
    "Tilføj din første torrent": "Add your first torrent",
    "Navn": "Name",
    "Status": "Status",
    "Størrelse": "Size",
    "Fremskridt": "Progress",
    "Hastighed": "Speed",
    "Handling": "Action",
    "ANDROID TV": "ANDROID TV",
    "Biblioteket er tomt": "The library is empty",
    "Video- og lydfiler fra dine torrents vises her, klar til afspilning.":
      "Video and audio files from your torrents appear here, ready to play.",
    "KONTO & SERVER": "ACCOUNT & SERVER",
    "Din konto": "Your account",
    "Rolle": "Role",
    "Oprettet": "Created",
    "Log ud": "Sign out",
    "Server": "Server",
    "Version": "Version",
    "Oppetid": "Uptime",
    "Torrent-porte": "Torrent ports",
    "Downloadmappe": "Download folder",
    "Port & netværk": "Port & network",
    "Ukendt": "Unknown",
    "Lokal port": "Local port",
    "Portinterval": "Port range",
    "Offentlig IP": "Public IP",
    "Ekstern IP": "External IP",
    "Ekstern port": "External port",
    "Tjekker porten…": "Checking the port…",
    "Test port": "Test port",
    "Gem": "Save",
    "Skift adgangskode": "Change password",
    "Nuværende adgangskode": "Current password",
    "Ny adgangskode": "New password",
    "Gem ny adgangskode": "Save new password",
    "Brugere": "Users",
    "Admin": "Admin",
    "Seer": "Viewer",
    "Administrator": "Administrator",
    "Opret bruger": "Create user",
    "Kører lokalt på din server": "Runs locally on your server",
    "NY DOWNLOAD": "NEW DOWNLOAD",
    "Magnet-link": "Magnet link",
    ".torrent-fil": ".torrent file",
    "Gennemse": "Browse",
    "Vælg en mappe under downloadroden.": "Choose a folder under the download root.",
    "Vælg denne mappe": "Choose this folder",
    "Magnet-link eller info-hash": "Magnet link or info hash",
    "Torrenten hentes direkte af serveren. Intet sendes til tredjepart.":
      "The torrent is fetched directly by the server. Nothing is sent to a third party.",
    "Tilføj i pauset tilstand": "Add in paused state",
    "Annuller": "Cancel",
    "Vælg eller slip en .torrent-fil": "Choose or drop a .torrent file",
    "Maks. 20 MB · filen bliver på din server": "Max 20 MB · the file stays on your server",
    "Upload og tilføj": "Upload and add",
    "TORRENT": "TORRENT",
    "BEKRÆFT": "CONFIRM",
    "Er du sikker?": "Are you sure?",
    "Slet også de downloadede filer fra disken": "Also delete the downloaded files from disk",
    "Bekræft": "Confirm",
    "AFSPILLER": "PLAYER",
    "Browseren kunne ikke afspille filen. Download den i stedet, eller brug Android TV-appen.":
      "The browser could not play the file. Download it instead, or use the Android TV app.",
    "STATISTIK": "STATISTICS",
    "Denne måned · downloadet": "This month · downloaded",
    "Denne måned · uploadet": "This month · uploaded",
    "Ratio denne måned": "Ratio this month",
    "Seedende torrents": "Seeding torrents",
    "Månedlig overførsel": "Monthly transfer",
    "Download / upload": "Download / upload",
    "Mest uploadet denne måned": "Most uploaded this month",
    "Top torrents": "Top torrents",
    "I alt": "All time",
    "Samlet ratio": "All-time ratio",
    "torrents i alt": "torrents total",
    "Ingen statistik endnu.": "No stats yet.",
    "Ingen uploadet data endnu.": "No uploaded data yet.",
    "Vis hero-kort på forsiden": "Show the hero card on the overview",
    "Tema, sprog og hero-kort gemmes i denne browser.":
      "Theme, language and hero card are saved in this browser.",
    "Aktive sessioner": "Active sessions",
    "Enheder og browsere, der er logget ind på serveren. Kun administratorer kan tilbagekalde sessioner.":
      "Devices and browsers signed in to the server. Only administrators can revoke sessions.",
    "Ingen aktive sessioner.": "No active sessions.",
    "Ukendt enhed": "Unknown device",
    "Denne enhed": "This device",
    "Sidst aktiv": "Last active",
    "Tilbagekald session": "Revoke session",
    "Tilbagekald session?": "Revoke session?",
    "Tilbagekald": "Revoke",
    "Er du sikker på, at du vil tilbagekalde denne session? Du bliver logget ud med det samme.":
      "Are you sure you want to revoke this session? You will be signed out immediately.",
    "Er du sikker på, at du vil tilbagekalde sessionen for {device}?":
      "Are you sure you want to revoke the session for {device}?",
    "Sessionen er tilbagekaldt.": "Session revoked.",
    "Sessionen er tilbagekaldt. Du er logget ud.": "Session revoked. You are signed out.",
    "Sessionen findes ikke.": "Session not found.",
    "Udseende & sprog": "Appearance & language",
    "Tema": "Theme",
    "System": "System",
    "Mørk": "Dark",
    "Lys": "Light",
    "Sprog": "Language",
    "Dansk": "Danish",
    "Vis TV-bibliotek": "Show TV library",
    "Vis labels i sidepanelet": "Show labels in the sidebar",
    "Skjul labels": "Hide labels",
    "Vis labels": "Show labels",
    "Tema, sprog og TV-bibliotek gemmes i denne browser.":
      "Theme, language and TV library are saved in this browser.",
    "Tema, sprog, TV-bibliotek og labels gemmes i denne browser.":
      "Theme, language, TV library and labels are saved in this browser.",
    "Chrome-udvidelse": "Chrome extension",
    "Label": "Label",
    "Alle": "All",
    "Ny label": "New label",
    "Opret label": "Create label",
    "Slet label": "Delete label",
    "Slet label?": "Delete label?",
    "“{name}” fjernes fra alle torrents.":
      "“{name}” will be removed from all torrents.",
    "Labelen er oprettet.": "Label created.",
    "Labelen er slettet.": "Label deleted.",
    "Labelen er gemt.": "Label saved.",
    "Ingen label": "No label",
    "To-faktor-godkendelse": "Two-factor authentication",
    "2FA er slået fra.": "2FA is disabled.",
    "2FA er slået til for denne konto.": "2FA is enabled for this account.",
    "Ved login udefra kræves en kode fra Google Authenticator. Lokale login på serverens netværk springes over.":
      "Signing in from outside requires a Google Authenticator code. Local logins on the server network skip it.",
    "Aktivér 2FA": "Enable 2FA",
    "Slå 2FA fra": "Disable 2FA",
    "Aktivér Google Authenticator": "Enable Google Authenticator",
    "Tilføj kontoen i Google Authenticator med den hemmelige nøgle nedenfor, og indtast derefter den viste 6-cifrede kode.":
      "Add the account in Google Authenticator with the secret below, then enter the displayed 6-digit code.",
    "Hemmelig nøgle": "Secret key",
    "6-cifret kode": "6-digit code",
    "Bekræft og aktivér": "Confirm and enable",
    "Slå 2FA fra?": "Disable 2FA?",
    "Bekræft med din adgangskode for at slå to-faktor-godkendelse fra.":
      "Confirm with your password to disable two-factor authentication.",
    "Indtast en 6-cifret kode.": "Enter a 6-digit code.",
    "Indtast din adgangskode.": "Enter your password.",
    "2FA er nu slået til.": "2FA is now enabled.",
    "Nøglen er kopieret.": "The key was copied.",
    "Kunne ikke kopiere automatisk. Nøglen er markeret.":
      "Could not copy automatically. The key is selected.",
    "Tilføj feed": "Add feed",
    "Ingen RSS-feeds endnu": "No RSS feeds yet",
    "Tilføj et RSS-feed for automatisk at downloade matching torrents.":
      "Add an RSS feed to automatically download matching torrents.",
    "Tilføj dit første feed": "Add your first feed",
    "Rediger RSS-feed": "Edit RSS feed",
    "Tilføj RSS-feed": "Add RSS feed",
    "Navn": "Name",
    "Mappe": "Folder",
    "standard": "default",
    "Filtre": "Filters",
    "Udeluk": "Exclude",
    "Sidst tjekket": "Last checked",
    "elementer": "items",
    "Ikke tjekket endnu": "Not checked yet",
    "Tjek nu": "Check now",
    "Rediger": "Edit",
    "Slået fra": "Disabled",
    "Kun overvågning": "Monitoring only",
    "RSS-adresse": "RSS address",
    "Skal indeholde": "Must contain",
    "Må ikke indeholde": "Must not contain",
    "Aktiv": "Active",
    "Automatisk download": "Automatic download",
    "Tilføj pauset": "Add paused",
    "Gem feed": "Save feed",
    "RSS-feedet er gemt.": "RSS feed saved.",
    "RSS-feedet er tjekket.": "RSS feed checked.",
    "Slet RSS-feed?": "Delete RSS feed?",
    "“{name}” og dets automatiske regler slettes.":
      "“{name}” and its automatic rules will be deleted.",
    "RSS-feedet er slettet.": "RSS feed deleted.",
    "Tema og sprog gemmes i denne browser.": "Theme and language are saved in this browser.",
    "Downloadmappe": "Download folder",
    "Søg i torrents…": "Search torrents…",
    "Opdatér": "Refresh",
    "Samlet downloadhastighed": "Total download speed",
    "Samlet uploadhastighed": "Total upload speed",
    "Tjekker torrent-porten…": "Checking the torrent port…",
    "Findes automatisk": "Detected automatically",
    "Ikke fundet": "Not found",
    "Fx 50000": "e.g. 50000",
    "Søg i denne mappe…": "Search this folder…",
    "Luk mappe-browser": "Close folder browser",
    "Valgt: {path}": "Selected: {path}",
    "Vælg: {path}": "Select: {path}",
    "Ingen mapper matcher søgningen.": "No folders match the search.",
    "Der er ingen undermapper her.": "There are no subfolders here.",
    "Henter mapper…": "Loading folders…",
    "Henter…": "Loading…",
    "Serveren fandt én downloadmappe.": "The server found one download folder.",
    "Serveren fandt {count} downloadmapper. Klik Gennemse for at vælge undermappe.":
      "The server found {count} download folders. Click Browse to choose a subfolder.",
    "Ingen særskilte downloadmapper fundet; downloadroden bruges.":
      "No separate download folders found; using the download root.",
    "Kunne ikke hente downloadmapper.": "Could not load download folders.",
    "Kunne ikke læse mappen.": "Could not read the folder.",
    "Ingen match": "No matches",
    "Prøv en anden søgning eller et andet filter.": "Try another search or filter.",
    "Ukendt torrent": "Unknown torrent",
    "Afspiller": "Player",
    "Henter torrentdetaljer…": "Loading torrent details…",
    "Downloadmappe: {path}": "Download folder: {path}",
    "Gemt i {path}": "Saved in {path}",
    "Gemt i downloadmappen": "Saved in the download folder",
    "Uploadet": "Uploaded",
    "Afspil": "Play",
    "Filer": "Files",
    "Trackere": "Trackers",
    "Peers": "Peers",
    "i alt": "total",
    "forbundet": "connected",
    "Prioritet": "Priority",
    "Normal": "Normal",
    "Spring over": "Skip",
    "Vælg download-prioritet for hver fil.": "Choose download priority for each file.",
    "Normal eller Spring over pr. fil.": "Normal or Skip per file.",
    "Gem filvalg": "Save file selection",
    "Ingen trackere fundet for denne torrent.": "No trackers found for this torrent.",
    "Tracker-URL": "Tracker URL",
    "Type": "Type",
    "Tracker-status er ikke eksponeret af torrentmotoren.":
      "Tracker status is not exposed by the torrent engine.",
    "Ingen live peers lige nu. Peers vises, når der er forbindelse til andre.":
      "No live peers right now. Peers appear when connections are established.",
    "Adresse": "Address",
    "Pieces": "Pieces",
    "Retning": "Direction",
    "Ind": "In",
    "Ud": "Out",
    "Vis peers": "Show peers",
    "medie": "media item",
    "medier": "media items",
    "fil": "file",
    "filer": "files",
    "Vis filer og afspil": "Show files and play",
    "Fjern": "Remove",
    "Slet": "Delete",
    "Seeding": "Seeding",
    "Seeder": "Seeding",
    "Pauset": "Paused",
    "Henter metadata": "Fetching metadata",
    "Fejl": "Error",
    "Færdig": "Complete",
    "Klar": "Ready",
    "Fjern torrent?": "Remove torrent?",
    "Torrentet er sat på pause.": "Torrent paused.",
    "Torrentet kører igen.": "Torrent resumed.",
    "Torrentet er tilføjet.": "Torrent added.",
    "Torrentfilen er tilføjet.": "Torrent file added.",
    "Torrentet er fjernet.": "Torrent removed.",
    "Torrent og filer er slettet.": "Torrent and files deleted.",
    "Flyt": "Move",
    "FLYT TORRENT": "MOVE TORRENT",
    "Flyt torrent": "Move torrent",
    "Flyt torrent?": "Move torrent?",
    "Vælg den nye downloadmappe. Torrenten pauses, flyttes og startes igen.":
      "Choose the new download folder. The torrent will be paused, moved and started again.",
    "Flyt torrenten til {path}? Den pauses, flyttes og startes igen.":
      "Move the torrent to {path}? It will be paused, moved and started again.",
    "Torrenten er flyttet.": "Torrent moved.",
    "downloadroden": "the download root",
    "Filvalget er gemt.": "File selection saved.",
    "Vælg mindst én fil.": "Select at least one file.",
    "Brugeren er oprettet.": "User created.",
    "Brugeren er slettet.": "User deleted.",
    "Adgangskoden er ændret. Andre enheder skal logge ind igen.":
      "Password changed. Other devices must sign in again.",
    "Adgangskoderne er ikke ens.": "The passwords do not match.",
    "Indsæt et magnet-link eller en info-hash.": "Paste a magnet link or info hash.",
    "Vælg en .torrent-fil først.": "Choose a .torrent file first.",
    "Du er logget ud.": "You are signed out.",
    "Serveren er klar. Velkommen!": "Server ready. Welcome!",
    "Sessionen er udløbet. Log ind igen.": "Your session expired. Sign in again.",
    "Kunne ikke kontakte serveren.": "Could not contact the server.",
    "Handlingen mislykkedes.": "The action failed.",
    "Netværksindstillinger gemt.": "Network settings saved.",
    "Begræns torrent-downloads": "Limit torrent downloads",
    "Samlet for alle torrents, adskilt fra HTTP-downloads":
      "For all torrents together, apart from HTTP downloads",
    "Maksimal hastighed": "Maximum speed",
    "Begræns torrent-uploads": "Limit torrent uploads",
    "Samlet for alle torrents": "For all torrents together",
    "Gem hastighed": "Save speed",
    "Ingen hastighedsgrænse": "No speed limit",
    "Download {speed} KB/s": "Download {speed} KB/s",
    "Upload {speed} KB/s": "Upload {speed} KB/s",
    "Hastighedsgrænser gemt.": "Speed limits saved.",
    "{label}-hastigheden skal være mellem 1 og 4.194.303 KB/s.":
      "{label} speed must be between 1 and 4,194,303 KB/s.",
    "Seeding": "Seeding",
    "Behold seeding efter torrenten er færdig":
      "Keep seeding after the torrent finishes",
    "Del med andre peers indtil en grænse nedenfor":
      "Share with other peers until a limit below",
    "Stop ved ratio": "Stop at ratio",
    "Uploaded divideret med downloaded; 0 = ingen grænse":
      "Uploaded divided by downloaded; 0 = no limit",
    "Stop efter": "Stop after",
    "0 = ingen grænse": "0 = no limit",
    "minutter": "minutes",
    "Gem seeding": "Save seeding",
    "Seeding stoppes når torrenten er færdig":
      "Seeding stops when the torrent finishes",
    "Stop ved ratio {ratio}": "Stop at ratio {ratio}",
    "Stop efter {minutes} min.": "Stop after {minutes} min.",
    "Seeding fortsætter uden grænse": "Seeding continues without a limit",
    "Ratio skal være mellem 0 og 1000.":
      "Ratio must be between 0 and 1000.",
    "Stop-tid skal være mellem 0 og 525.600 minutter.":
      "Stop time must be between 0 and 525,600 minutes.",
    "Seeding-indstillinger gemt.": "Seeding settings saved.",
    "DHT": "DHT",
    "Find peers via DHT": "Find peers through DHT",
    "Bruges til magnet-links uden trackere. Private torrents bruger den ikke.":
      "Used for magnet links without trackers. Private torrents never use it.",
    "DHT er slået til. Ændringen træder i kraft efter genstart af serveren.":
      "DHT is enabled. The change takes effect after restarting the server.",
    "DHT er slået fra. Ændringen træder i kraft efter genstart af serveren.":
      "DHT is disabled. The change takes effect after restarting the server.",
    "Gem DHT": "Save DHT",
    "DHT-indstilling gemt. Genstart serveren for at anvende den.":
      "DHT setting saved. Restart the server to apply it.",
    "Porten skal være mellem 1 og 65535.": "The port must be between 1 and 65535.",
    "Porten er åben.": "The port is open.",
    "Porten er lukket.": "The port is closed.",
    "Portstatus kunne ikke afgøres.": "Port status could not be determined.",
    "Åben": "Open",
    "Lukket": "Closed",
    "Oppetid {uptime}": "Uptime {uptime}",
    "Forbinder…": "Connecting…",
    "Tilføj dit første magnet-link eller en .torrent-fil. Serveren henter resten, og TV-appen finder mediet automatisk.":
      "Add your first magnet link or .torrent file. The server fetches the rest, and the TV app finds the media automatically.",
    "{count} torrents i biblioteket. Ingen aktive overførsler lige nu.":
      "{count} torrents in the library. No active transfers right now.",
    "{count} torrent i biblioteket. Ingen aktive overførsler lige nu.":
      "{count} torrent in the library. No active transfers right now.",
    "op {size}": "up {size}",
    "Skift til mørkt tema": "Switch to dark theme",
    "Skift til lyst tema": "Switch to light theme",
    "Skift tema": "Switch theme",
    "Luk": "Close",
    "Luk mappe-browser": "Close folder browser",
    "Porten svarede fra den offentlige IP. Andre bør kunne forbinde.":
      "The port responded from the public IP. Others should be able to connect.",
    "Forbindelsen blev afvist. Tjek portforwarding og firewall. Nogle routere afviser også NAT-loopback, selv om porten er åben udefra.":
      "The connection was refused. Check port forwarding and firewall. Some routers also reject NAT loopback even when the port is open externally.",
    "Forbindelsen blev afvist. Tjek portforwarding og firewall.":
      "The connection was refused. Check port forwarding and firewall.",
    "Kunne ikke afgøre portstatus:": "Could not determine port status:",
    "Porttjek fik timeout. Tjek portforwarding, firewall og at routeren tillader loopback.":
      "Port check timed out. Check port forwarding, firewall, and that the router allows loopback.",
    "Angiv en ekstern IP og port, eller start torrentmotoren.":
      "Enter an external IP and port, or start the torrent engine.",
    "Kunne ikke finde den offentlige IP automatisk:": "Could not find the public IP automatically:",
    "Udfyld den manuelt.": "Enter it manually.",
    "Den angivne eksterne IP er ugyldig.": "The specified external IP is invalid.",
    "Velkommen, {name}.": "Welcome, {name}.",
    "{count} aktive": "{count} active",
    "{count} færdige": "{count} complete",
    "af {count}": "of {count}",
    "{count} torrent": "{count} torrent",
    "{count} torrents": "{count} torrents",
    "{count} torrents arbejder lige nu. Alt kører lokalt på din server.":
      "{count} torrents are working right now. Everything runs locally on your server.",
    "{count} torrent arbejder lige nu. Alt kører lokalt på din server.":
      "{count} torrent is working right now. Everything runs locally on your server.",
    "{count} torrents i biblioteket. Ingen aktive downloads.":
      "{count} torrents in the library. No active downloads.",
    "Vælg: {path}": "Select: {path}",
    "Valgt: {path}": "Selected: {path}",
    "“{name}” fjernes fra listen. Du kan vælge også at slette de downloadede filer.":
      "“{name}” will be removed from the list. You can also choose to delete the downloaded files.",
    "“{name}” slettes permanent fra serveren.": "“{name}” will be permanently deleted from the server.",
    "{count} dage": "{count} days",
    "{count} timer": "{count} hours",
    "{count} minutter": "{count} minutes",
    "{count} sekunder": "{count} seconds",
  };

  function getLanguage() {
    try {
      var saved = localStorage.getItem(LANGUAGE_KEY);
      return SUPPORTED.indexOf(saved) >= 0 ? saved : "da";
    } catch (error) {
      return "da";
    }
  }

  function format(template, vars) {
    if (!vars) return template;
    return template.replace(/\{(\w+)\}/g, function (match, key) {
      return vars[key] === undefined || vars[key] === null ? match : String(vars[key]);
    });
  }

  function t(key, vars) {
    if (getLanguage() !== "en") return format(key, vars);
    return format(en[key] || key, vars);
  }

  function translateAttributes(element) {
    if (!element || element.nodeType !== 1) return;
    ["placeholder", "title", "aria-label"].forEach(function (name) {
      var value = element.getAttribute(name);
      if (!value) return;
      var translation = en[value.trim()];
      if (translation) element.setAttribute(name, translation);
    });
  }

  function shouldSkip(node) {
    var parent = node.parentElement;
    while (parent) {
      var tag = parent.tagName;
      if (tag === "SCRIPT" || tag === "STYLE" || tag === "TEXTAREA" || tag === "CODE" || tag === "PRE") {
        return true;
      }
      if (parent.hasAttribute("data-no-i18n")) return true;
      parent = parent.parentElement;
    }
    return false;
  }

  function translateNode(node) {
    if (!node || !node.nodeValue || shouldSkip(node)) return;
    var raw = node.nodeValue;
    var key = raw.replace(/\s+/g, " ").trim();
    if (!key) return;
    var translation = en[key];
    if (!translation) return;
    var leading = raw.match(/^\s*/)[0];
    var trailing = raw.match(/\s*$/)[0];
    node.nodeValue = leading + translation + trailing;
  }

  function translateDom(root) {
    if (getLanguage() !== "en" || !root) return;
    var walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    var nodes = [];
    while (walker.nextNode()) nodes.push(walker.currentNode);
    nodes.forEach(translateNode);
    if (root.nodeType === 1) translateAttributes(root);
    root.querySelectorAll("[placeholder], [title], [aria-label]").forEach(translateAttributes);
  }

  var scheduled = false;
  function scheduleTranslate() {
    if (getLanguage() !== "en" || scheduled) return;
    scheduled = true;
    window.requestAnimationFrame(function () {
      scheduled = false;
      translateDom(document.body);
    });
  }

  function applyLanguageAttributes() {
    var language = getLanguage();
    document.documentElement.lang = language;
    document.documentElement.dataset.language = language;
  }

  function setLanguage(language) {
    var next = SUPPORTED.indexOf(language) >= 0 ? language : "da";
    try {
      localStorage.setItem(LANGUAGE_KEY, next);
    } catch (error) {}
    applyLanguageAttributes();
    window.location.reload();
  }

  window.NordTorrentI18n = {
    t: t,
    getLanguage: getLanguage,
    setLanguage: setLanguage,
    translateDom: translateDom,
  };

  applyLanguageAttributes();
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", function () {
      translateDom(document.body);
      new MutationObserver(scheduleTranslate).observe(document.body, {
        childList: true,
        characterData: true,
        subtree: true,
      });
    });
  } else {
    translateDom(document.body);
    new MutationObserver(scheduleTranslate).observe(document.body, {
      childList: true,
      characterData: true,
      subtree: true,
    });
  }
})();

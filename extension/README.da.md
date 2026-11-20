# NordTorrent Download Helper

Chrome-extension (Manifest V3) til at sende `.torrent`-downloads direkte til
NordTorrent-serveren i stedet for at gemme dem lokalt.

## Installation

1. Åbn `chrome://extensions`.
2. Slå **Udviklertilstand** til.
3. Vælg **Indlæs upakket** og vælg denne `extension/`-mappe.
4. Klik på extension-ikonet.
5. Indtast:
   - Server: `http://192.168.1.10:8080`
   - Brugernavn og adgangskode til en **admin**-konto
6. Gem.

Adgangskoden gemmes ikke. Kun login-tokenet gemmes i `chrome.storage.local`.

## Sprog

Extensionen er på engelsk som standard. Åbn indstillingerne (tandhjulet) og vælg **Dansk** under **Sprog** for at skifte til dansk. Valget gemmes pr. browser og slår igennem med det samme.

## Popup til/fra

**Åbn popup ved .torrent-downloads** kan slås til og fra tre steder:

- i hovedvisningen
- under Indstillinger
- i login-visningen

Indstillingen gemmes med det samme. Når den er slået fra, fortsætter
`.torrent`-downloads som en helt normal Chrome-download uden popup.

## Labels

Ved tilføjelse af en `.torrent`-fil eller et magnet-link kan du vælge en af
serverens labels i **Label**-dropdownen. Labels oprettes og administreres i
webpanelet under **Labels**.

## Magnet-links

Når du trykker **Copy magnet link** på en side, åbner extensionen automatisk
sit popupvindue med magnet-linket indsat. Teknikken dækker både
`navigator.clipboard.writeText`, `document.execCommand("copy")`, kopiering af
markeret tekst og knapper med `data-clipboard-text`/magnet-attributter.

Du kan også åbne extensionen og indsætte et `magnet:?xt=...`-link eller en
40-tegns info-hash i magnetfeltet. Vælg en downloadmappe, og tryk
**Tilføj magnet**.

Downloadmappelisten har et søgefelt, så du hurtigt kan filtrere mapper i den
aktuelle mappe.

## Sådan virker den

Når Chrome starter en `.torrent`-download:

- åbnes et lille NordTorrent-vindue
- Chromes normale download stoppes med det samme, så **Save As**-vinduet ikke
  bliver hængende
- extensionen henter `.torrent`-filen med browserens cookies
- du vælger en downloadmappe på serveren
- filen uploades til `/api/v1/torrents/upload`
- den lokale Chrome-download annulleres og filen fjernes

Hvis Chrome ikke kan hente filen automatisk (fx en beskyttet eller midlertidig
download-URL), kan man vælge `.torrent`-filen manuelt i vinduet.

## Tilladelser

- `downloads`: opdage, annullere og fjerne den lokale `.torrent`-download.
- `storage`: gemme server, brugernavn og login-token.
- `<all_urls>`: hente `.torrent`-filen fra den side, du downloader fra, samt
  kalde NordTorrent-serveren.

Extensionen har ikke adgang til serverens filer ud over de API-kald, som
brugerens konto er autoriseret til.

## Begrænsninger

- Kun `.torrent`-downloads; magnet-links håndteres i webpanelet.
- Nogle private trackere kræver en bestemt `Referer` eller engangstoken, som
  Chrome ikke giver extensionen. Brug filvælgeren i så fald.
- Serveren skal være opdateret med `extension-api`-funktionen.
- Chrome skal genstartes efter installation, hvis download-popup ikke vises.

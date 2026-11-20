# NordTorrent til Android TV og telefon

Native Android-klient til NordTorrent-serveren. Virker på både Android TV/Google TV og Android-telefoner. Skrevet i Kotlin med klassiske Android Views, Material/AppCompat og Media3/ExoPlayer.

![Android TV-bibliotek](../docs/screenshots/android-tv-library.png)

## Sprog

Appen er på engelsk som standard. På en enhed med dansk sprog bruges dansk automatisk, fordi de danske tekster ligger i `values-da/`. Al synlig tekst kommer fra `strings.xml`, så et nyt sprog kun kræver endnu en `values-xx`-mappe.

## Funktioner

- Login mod serverens `/api/v1/auth/login`
- Husker server og bruger; bearer-token gemmes krypteret med Android Keystore (AES-GCM)
- D-pad-venligt biblioteksgrid på TV og touchvenligt 2-kolonne-grid på telefon
- Afspiller video/lyd via ExoPlayer med `Authorization`-header
- Fortsæt-afspilning gemmes lokalt pr. stream
- Overførselsliste med pause/genoptag for administratorer
- Indstillinger, logout og serverstatus
- Ingen Google-konto, analytics eller tredjeparts-CDN'er

## Krav

- Android 6.0 (API 23) eller nyere
- Android TV / Google TV eller Android-telefon
- En kørende NordTorrent-server
- En `viewer`- eller `admin`-konto

## Byg

```bash
cd android-tv
./gradlew :app:assembleDebug
```

På Windows:

```powershell
cd android-tv
.\gradlew.bat :app:assembleDebug
```

APK'en ligger i:

```
app/build/outputs/apk/debug/app-debug.apk
```

## Installér på TV eller telefon

Slå ADB-debugging til på TV'et (oftest under Indstillinger → Om → Byg-nummer, tryk 7 gange), og kør:

```bash
adb connect <tv-ip>:5555
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

Eller kopiér APK'en til en USB-stick og installér med en filhåndtering på TV'et.

## Første login

1. Åbn appen.
2. Serveradresse: `http://192.168.1.10:8080` eller din HTTPS-adresse.
3. Brugernavn og adgangskode for den konto, du oprettede i webpanelet.
4. Vælg en titel i biblioteket med OK.

Hvis serveren endnu ikke er sat op, viser appen en henvisning til webpanelet. Opret administratoren der først.

## Sikkerhed

- Tokenet gemmes ikke i klartekst. Det krypteres med en nøgle i Android Keystore og kan ikke læses af andre apps.
- Appen tillader HTTP til LAN-brug. Brug HTTPS, når serveren nås udefra.
- Appen har kun `INTERNET`, `ACCESS_NETWORK_STATE` og `WAKE_LOCK`.
- Der er ingen WebView og ingen eksterne scripts.
- Logout sletter token, serveradresse og brugeroplysninger fra enheden.

## Projektstruktur

```
app/src/main/java/com/nordtorrent/tv/
├── ApiClient.kt          HTTP/JSON-klient
├── SessionStore.kt       Keystore-krypteret tokenlagring
├── LoginActivity.kt      Login og servervalidering
├── MainActivity.kt       Navigation, bibliotek og overførsler
├── PlayerActivity.kt     Media3/ExoPlayer
├── MediaAdapter.kt       TV-grid
├── TransferAdapter.kt    Overførselsliste
├── Models.kt             API-modeller
├── Format.kt             Danske formateringshelpers
└── GridSpacingDecoration.kt
```

## Kendte begrænsninger

- Ingen indbygget søgning på TV (brug webpanelet).
- Ingen undertekstvalg i afspilleren endnu.
- Transkodning foregår ikke; ExoPlayer skal kunne afspille codec'et.
- Appen er kun testet mod API v1.




På telefon: aktivér trådløs ADB-debugging, par enheden, og installér samme APK:

```powershell
adb connect 192.168.1.206:5555
adb -s 192.168.1.206:5555 install -r app/build/outputs/apk/debug/app-debug.apk
```

Telefon-UI'en bruger 2 kolonner i portræt, touchvenlige knapper og respekterer status- og navigationsbjælker. Telefonen og TV'et kan bruge samme APK.
# NordTorrent for Android TV and phones

Native Android client for the NordTorrent server. Works on Android TV/Google TV and on Android phones. Written in Kotlin with classic Android Views, Material/AppCompat and Media3/ExoPlayer.

## Languages

The app is in English by default. On a device set to Danish it uses Danish automatically, because the Danish strings live in `values-da/`. Everything user-visible comes from `strings.xml`, so adding a language only needs one more `values-xx` file.

## Features

- Signs in against the server's `/api/v1/auth/login`
- Remembers server and user; the bearer token is encrypted with Android Keystore (AES-GCM)
- D-pad friendly library grid on TV and a touch friendly two-column grid on phones
- Plays video/audio through ExoPlayer with the `Authorization` header
- Resume position is stored locally per stream
- Transfer list with pause/resume for administrators
- Settings, logout and server status
- No Google account, analytics or third-party CDNs

## Requirements

- Android 6.0 (API 23) or newer
- Android TV / Google TV or an Android phone
- A running NordTorrent server
- A `viewer` or `admin` account

## Build

```bash
cd android-tv
./gradlew :app:assembleDebug
```

On Windows:

```powershell
cd android-tv
.\gradlew.bat :app:assembleDebug
```

The APK is written to:

```
app/build/outputs/apk/debug/app-debug.apk
```

## Install on a TV or phone

Enable ADB debugging on the TV (usually under Settings -> About -> Build number, tap seven times) and run:

```bash
adb connect <tv-ip>:5555
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

Alternatively copy the APK to a USB stick and install it with a file manager on the TV.

For a phone, enable wireless ADB debugging, pair the device and install the same APK:

```powershell
adb connect 192.168.1.20:5555
adb -s 192.168.1.20:5555 install -r app/build/outputs/apk/debug/app-debug.apk
```

The phone UI uses two columns in portrait, touch friendly buttons and respects the status and navigation bars. The TV and the phone can use the same APK.

## First login

1. Open the app.
2. Server address: `http://192.168.1.10:8080` or your HTTPS address.
3. Username and password for the account created in the web panel.
4. Pick a title in the library with OK.

If the server has not been set up yet, the app points you to the web panel. Create the administrator there first.

## Security

- The token is not stored in plain text. It is encrypted with a key in Android Keystore and cannot be read by other apps.
- The app allows HTTP for LAN use. Use HTTPS when the server is reachable from the internet.
- The app only requests `INTERNET`, `ACCESS_NETWORK_STATE` and `WAKE_LOCK`.
- There is no WebView and no external scripts.
- Logout removes the token, server address and user details from the device.

## Project structure

```
app/src/main/java/com/nordtorrent/tv/
├── ApiClient.kt          HTTP/JSON client
├── SessionStore.kt       Keystore-encrypted token storage
├── LoginActivity.kt      Login and server validation
├── MainActivity.kt       Navigation, library and transfers
├── PlayerActivity.kt     Media3/ExoPlayer
├── MediaAdapter.kt       TV grid
├── TransferAdapter.kt    Transfer list
├── Models.kt             API models
├── Format.kt             Formatting helpers
└── GridSpacingDecoration.kt
```

## Known limitations

- No built-in search on the TV (use the web panel).
- No subtitle selection in the player yet.
- No transcoding; ExoPlayer must be able to play the codec.
- The app is only tested against API v1.

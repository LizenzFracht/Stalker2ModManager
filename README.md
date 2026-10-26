# S2MM – Stalker 2 Mod Manager

Mod-Manager für **S.T.A.L.K.E.R. 2: Heart of Chornobyl** – inspiriert vom
[Definitive Mod Manager](https://www.nexusmods.com/crimsondesert/mods/633) und
[CDUMM](https://github.com/faisalkindi/CrimsonDesert-UltimateModsManager) für Crimson Desert.

Tauri 2 (Rust) + React + Tailwind. Eine einzelne EXE ohne Laufzeit-Abhängigkeiten.

## Funktionen

- **Alle gängigen Mod-Arten**: Pak-Mods (`.pak` sowie IoStore `.utoc`/`.ucas`), UE4SS-Lua-Mods,
  DLL/ASI-Plugins (z. B. UE4SS selbst) und Dateiersetzungen im Spielordner.
- **Installation per Drag & Drop**: ZIP, 7z, RAR, lose Pak-Dateien oder Ordner. Verschachtelte
  Archivstrukturen werden automatisch erkannt (`Stalker2/Content/Paks/~mods/…`, `Binaries/Win64`,
  `ue4ss/Mods`, einzelne Wrapper-Ordner …).
- **Mod-Optionen**:
  - **FOMOD**-Installer (Schritte, Gruppen, Beschreibungen, Vorschaubilder, Flag-Bedingungen).
  - Erkennung von **Varianten-Ordnern** (z. B. `x2/`, `x5/`) in Archiven ohne FOMOD – wählbar wie FOMOD.
  - Auswahl jederzeit änderbar, ohne neu zu installieren.
- **Konfigurationsdateien bearbeiten** (`.ini`, `.lua`, `.json`, `.cfg`, …) direkt im Mod.
- **Ladereihenfolge per Drag & Drop**: Weiter unten = gewinnt. Pak-Dateien bekommen beim Anwenden ein
  Nummern-Präfix (`003_modname_Datei_P.pak`), optional mit erzwungenem `_P`-Suffix.
- **Konflikterkennung auf Asset-Ebene**: S2MM liest die Inhaltsverzeichnisse aller Paks (Pak v11 +
  IoStore) und zeigt, welche Mod welche `.cfg`/Assets einer anderen überschreibt – inkl. „Umkehren“-Knopf.
- **Sicheres Anwenden**: Mods liegen in einer eigenen Bibliothek und werden per Hardlink (oder Kopie)
  ins Spiel gebracht. Nur von S2MM platzierte Dateien werden je entfernt; überschriebene Originaldateien
  werden gesichert und wiederhergestellt. „Vanilla herstellen“ entfernt alles mit einem Klick.
- **Profile**: Mehrere Mod-Zusammenstellungen mit eigener Reihenfolge. Das schreibgeschützte Profil **„Vanilla“**
  ist immer vorhanden (keine Mods); wer dort eine Mod aktiviert, landet automatisch im eigenen Mod-Profil.
- **Vanilla herstellen** (auch als Angebot beim ersten Start): entfernt alle S2MM-Mods und auf Wunsch manuell
  installiertes UE4SS, fremde Mods in `~mods`/`Stalker2\Mods` sowie alle Engine.ini-Tweaks (Steam- und
  Game-Pass-Konfiguration; die vom Spiel geschriebenen Abschnitte bleiben). Nichts wird gelöscht – alles landet in
  einem datierten Sicherungsordner.
- **Automatisch sortieren**: Vorschlag mit Begründung – Frameworks zuerst, große Pak-Überarbeitungen vor kleinen
  Änderungen, Patches/Kompatibilitäts-Mods zuletzt bzw. direkt hinter ihrer Basis-Mod.
- **Fremde Dateien** im `~mods`-Ordner werden erkannt und können übernommen werden.
- **Spiel-Tweaks**: Engine.ini-Editor (`[SystemSettings]`) mit kuratierten Schaltern (Chromatische
  Aberration, Vignette, Tiefenunschärfe, Streaming-Pool, Shader-Cache …), eigenen CVars und Sicherung.
- **Nexus Mods**:
  - Anmeldung per persönlichem API-Schlüssel – es reicht der Schlüssel, der auf der Nexus-API-Seite beim Eintrag
    „Vortex (old)“ steht.
  - `nxm://`-Handler für den Button **„Mod Manager Download“** → Download mit Fortschritt, automatische
    Installation, Metadaten (Name, Version, Autor, Bild).
  - Update-Prüfung (Datei-Update-Kette + Versionsvergleich). Premium: Update direkt herunterladen;
    kostenlos: Nexus-Seite öffnet sich, der Download kommt per „Mod Manager Download“ zurück und ersetzt
    die alte Version (Optionen und Reihenfolge bleiben).
  - Nexus-IDs werden auch aus Dateinamen manuell geladener Archive erkannt (`Name-123-1-0-1732400000.zip`).
- **Zone-Kit-Mods** (offizielles Format `Stalker2\Mods\<Name>\Content\Paks\Windows`) werden erkannt und
  unverändert dorthin installiert – z. B. das Mod Configuration Menu.
- **Mod-Einstellungen (MCM)**: Liest und schreibt `MCM_Settings.sav` (Unreal-SaveGame/GVAS, byte-genau über
  [uesave](https://crates.io/crates/uesave)) – Schalter für Bool-Werte, Zahlen, Texte, Tasten. MCM selbst lässt sich
  per Toggle aktivieren/deaktivieren. Aus dem Konfigurations-Tab einer Mod springt man direkt zu ihren MCM-Werten.
- **Tastenbelegung**: Übersicht aller Mod-Tasten (UE4SS `RegisterKeyBind`, `Key.X`-Konfigurationen, Hotkeys in
  ini/json/lua, MCM-Tasten) mit Erkennung doppelter Belegungen (auch gegen Standardtasten des Spiels) und
  Neubelegung per Tastendruck – inkl. Strg/Shift/Alt bei UE4SS.
- **UE4SS bei Bedarf**: Download der aktuellen UE4SS-Version von GitHub inklusive der Stalker-2-Konfiguration
  (Steam bzw. Game Pass), installiert als abschaltbare Mod (Toggle).
- **Live-Karte**: Lokaler Server (nur 127.0.0.1) mit [joric's interaktiver Karte](https://github.com/joric/stalker);
  eine mitgelieferte UE4SS-Lua-Mod meldet die Spielerposition, die Karte zeigt sie live mit Blickrichtung und
  „Folgen“-Modus. Ein Ebenenfilter (deutsche Namen, Markeranzahl, Suche, Alle/Keine, Grundkarte) blendet
  Markierungsgruppen ein und aus. Kartendaten werden lokal zwischengespeichert.
- **Engine.ini Export/Import**: Export der kompletten Datei, Import als Zusammenführung aller Abschnitte oder als
  vollständiger Ersatz (mit Sicherung).
- **Spielerkennung**: Steam, Game Pass/Xbox, GOG, Epic. Start über Steam oder direkt, mit Parametern.
  Erkennt Spiel-Updates (Steam-Build) und weist darauf hin.
- **Portabler Modus**: `portable.txt` neben die EXE legen → Daten in `S2MM-Daten\`.

## Entwicklung

Voraussetzungen: Rust (MSVC), Node.js 20+, WebView2 (unter Windows 11 vorhanden).

```bash
npm install
npm run tauri dev      # App mit echtem Backend
npm run dev            # nur UI im Browser mit Testdaten (src/devMock.ts)
npm run tauri build    # EXE + Installer unter src-tauri/target/release
cd src-tauri && cargo test
cargo test -- --ignored                  # zusätzlich Netzwerk-Tests (UE4SS-Download, Kartenserver)
cargo run --example mapserver            # nur den Kartenserver starten (Port 27421)
```

Die Tests enthalten einen End-to-End-Durchlauf (Installieren → Optionen → Konflikte → Anwenden →
Backup/Restore → Vanilla) gegen einen künstlichen Spielordner sowie – falls vorhanden – einen
Lese-Test gegen die echten Spiel-Paks.

### Aufbau

| Datei | Aufgabe |
| --- | --- |
| `src-tauri/src/game.rs` | Installationen finden, Pfade, Spielstart |
| `src-tauri/src/archive.rs` | ZIP/7z/RAR entpacken |
| `src-tauri/src/layout.rs` | Archivstruktur analysieren, Varianten erkennen, Dateien Zielen zuordnen |
| `src-tauri/src/fomod.rs` | FOMOD-`ModuleConfig.xml` |
| `src-tauri/src/plan.rs` | Profil → Dateiliste, Pak-Benennung, Konflikte |
| `src-tauri/src/pak.rs` | Pak-v11- und IoStore-Verzeichnisindex lesen |
| `src-tauri/src/deploy.rs` | Anwenden/Entfernen mit Backups |
| `src-tauri/src/tweaks.rs` | Engine.ini |
| `src-tauri/src/nexus.rs` | Nexus-API, nxm-Links, Protokoll-Registrierung |
| `src-tauri/src/gvas.rs` | SaveGame-Editor (MCM) |
| `src-tauri/src/keybinds.rs` | Tasten finden und umschreiben |
| `src-tauri/src/ue4ss.rs` | UE4SS-Download |
| `src-tauri/src/livemap.rs`, `livemap/` | Kartenserver, Overlay-Skript, Lua-Positions-Mod |
| `src/` | React-Oberfläche |

Daten: `%APPDATA%\de.s2mm.stalker2modmanager\` (`state.json`, `deployment.json`, `mods\`, `downloads\`, `backups\`).

## Ideen für v2

- Nexus-SSO-Anmeldung (erfordert eine bei Nexus registrierte App) und Mod-Browser in der App.
- Zusammenführen kollidierender `.cfg`-Dateien zu einer Merge-Pak (benötigt Pak-Schreiber + Oodle/Zlib).
- „Culprit finden“: automatische Halbierungssuche nach einer absturzverursachenden Mod.
- Export/Import von Profilen (Modlisten teilen).
- Steam Workshop / mod.io-Mods anzeigen.

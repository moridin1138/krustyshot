# Rustshot interface text: German.
# Machine-drafted translation (not yet reviewed by a native speaker).

## Capture actions

capture-fullscreen = Ganzen Bildschirm aufnehmen
capture-last-region = Letzten Bereich aufnehmen
capture-region = Bereich aufnehmen
capture-window = Fenster aufnehmen
record-fullscreen = Ganzen Bildschirm aufnehmen
record-region = Bereich aufnehmen …
record-stop = Aufnahme beenden
record-window = Fenster aufnehmen …

## Menu-bar / tray menu

menu-about = Über Rustshot
menu-open-clipboard = Bild aus der Zwischenablage öffnen
menu-open-file = Bild öffnen …
menu-open-folder = Ausgabeordner öffnen
menu-quit = Rustshot beenden
menu-settings = Einstellungen …
tray-click-menu = Menü anzeigen

## After-capture destinations

dest-clipboard = In die Zwischenablage kopieren
dest-editor = Im Bildeditor öffnen
dest-file = Direkt sichern
dest-file-as = Sichern unter …
dest-picker = Nachfragen (Schnellmenü)
dest-open-with = In anderer App öffnen
dest-print = Drucken…
dest-email = Per E-Mail senden
dest-share = Teilen…
dest-copy-text = Text kopieren (OCR)
picker-heading = Bildschirmfoto { $width } × { $height }

## Capture overlay

overlay-help-region = Ziehen, um einen Bereich zu wählen  ·  Auf ein Fenster klicken, um es aufzunehmen  ·  Leertaste: Fenstermodus  ·  Esc: abbrechen
overlay-help-window = Auf ein Fenster klicken, um es aufzunehmen  ·  Leertaste: Bereichsmodus  ·  Esc: abbrechen
overlay-record-help-region = Aufnehmen: Bereich aufziehen  ·  Auf ein Fenster oder den Schreibtisch klicken  ·  Leertaste: Fenstermodus  ·  Esc: abbrechen
overlay-record-help-window = Aufnehmen: Auf ein Fenster klicken  ·  Leertaste: Bereichsmodus  ·  Esc: abbrechen

## Recording

recording-bar-title = Rustshot-Aufnahme
recording-frame-title = Rustshot-Aufnahmebereich
recording-discard = Verwerfen
recording-discard-body = Die Aufnahme wird beendet und das Video gelöscht. Das lässt sich nicht widerrufen.
recording-discard-tip = Aufnahme beenden und verwerfen
recording-mute-microphone = Mikrofon stummschalten
recording-unmute-microphone = Mikrofon einschalten
recording-mute-system = Systemton stummschalten
recording-unmute-system = Systemton einschalten
recording-discard-title = Diese Aufnahme verwerfen?
recording-keep = Weiter aufnehmen
recording-no-display = Es ließ sich nicht feststellen, welcher Bildschirm aufgenommen werden soll. Versuche es erneut.
recording-stop = Beenden

## Settings: recording

rec-estimate = Bis zu etwa { $size } pro Minute für eine Aufnahme des ganzen Bildschirms
rec-estimate-sub = { $width } × { $height } mit { $fps } fps auf deinem Hauptbildschirm. Ruhige Bildschirme und kleinere Bereiche brauchen deutlich weniger.
rec-format = Format
rec-format-gif-hint = GIF: lässt sich überall abspielen, auch in Chat-Apps und auf Webseiten, aber die Dateien werden schnell groß. Ideal für kurze Clips.
rec-format-gif-only-hint = MP4-Aufnahmen kommen bald auf diese Plattform; bis dahin werden Aufnahmen als GIF gesichert, ideal für kurze Clips.
rec-format-mp4-hint = MP4: kleine Dateien, flüssiges Video. Ideal für alles, was länger als ein paar Sekunden dauert.
rec-fps = Bildrate
rec-fps-value = { $fps } fps
rec-full-resolution = Volle Auflösung
rec-full-resolution-sub = Auf hochauflösenden (Retina-)Bildschirmen. Ausgeschaltet sieht es in normaler Größe gleich aus und braucht etwa ein Viertel des Speicherplatzes.
rec-microphone = Mikrofon
rec-microphone-default = Systemstandard
rec-microphone-device = Welches Mikrofon
rec-microphone-missing = Nicht verbunden (Standard wird verwendet)
rec-microphone-sub = Deine Stimme mit dem Video aufnehmen
rec-quality = Qualität
rec-quality-high = Hoch
rec-quality-low = Niedrig
rec-quality-medium = Mittel
rec-quality-sub = Höhere Qualität braucht mehr Speicherplatz
rec-shortcut-hint = Starte und beende Aufnahmen über das Menü oder mit dem Kurzbefehl unter „Kurzbefehle“.
rec-show-clicks = Mausklicks anzeigen
rec-show-clicks-sub = Wo du klickst, erscheint ein Ring, damit Zuschauer folgen können
rec-system-audio = Systemton
rec-system-audio-sub = Aufnehmen, was dein Computer abspielt
rec-system-audio-unavailable = Erfordert macOS 13 oder neuer
rec-microphone-denied = Der Mikrofonzugriff ist für Rustshot ausgeschaltet

## Disk space

disk-change-folder = Ordner ändern …
disk-continue = Trotzdem fortfahren
disk-low-body =
    Auf dem Volume, auf dem Rustshot Bildschirmfotos und Aufnahmen sichert, sind nur noch { $free } ({ $percent } %) frei:

    { $folder }

    Das Sichern kann fehlschlagen, wenn das Volume voll ist. Gib Speicherplatz frei oder wähle einen Ordner auf einem anderen Volume.
disk-low-title = Wenig Speicherplatz

## Notifications

toast-copied = In die Zwischenablage kopiert
toast-no-text = Kein Text im Bild gefunden
toast-text-copied = { $lines ->
    [one] 1 Textzeile kopiert
   *[other] { $lines } Textzeilen kopiert
}
toast-recording-discarded = Aufnahme verworfen
toast-recording-saved = { $name } gesichert ({ $size })
toast-saved = { $name } gesichert
toast-show-file = Klicken, um die Datei anzuzeigen
toast-show-finder = Klicken, um sie im Finder zu zeigen

## Messages

notice-capture-failed = Aufnahme fehlgeschlagen
notice-disk-full-body = Das Volume war fast voll. Rustshot hat die Aufnahme deshalb beendet und das Bisherige gesichert. Gib Speicherplatz frei, bevor du erneut aufnimmst.
notice-disk-full-title = Aufnahme beendet
notice-hotkeys-body =
    Einige Kurzbefehle konnten nicht registriert werden (vielleicht verwendet eine andere App sie):

    { $errors }

    Ändere sie unter Einstellungen > Kurzbefehle.
notice-hotkeys-title = Kurzbefehle
notice-no-clipboard-image = In der Zwischenablage ist kein Bild.
notice-open-image = Bild öffnen
notice-recording-failed = Aufnahme fehlgeschlagen
notice-no-microphone-title = Aufnahme ohne Mikrofon
notice-no-microphone-body = Rustshot hat noch keinen Mikrofonzugriff. Schalte Mikrofon unter Einstellungen → Aufnahme aus und wieder ein, um gefragt zu werden, oder erlaube es unter Systemeinstellungen → Datenschutz & Sicherheit → Mikrofon.

## Screen Recording permission (macOS)

permission-body =
    Rustshot braucht die Berechtigung „Bildschirmaufnahme“, um andere Apps aufzunehmen. Ohne sie erlaubt macOS nur, den Schreibtischhintergrund aufzunehmen.

    1. Klicke auf „Systemeinstellungen öffnen“.
    2. Aktiviere Rustshot unter „Datenschutz & Sicherheit“ > „Bildschirmaufnahme“.
    3. Klicke auf „Rustshot beenden“ und öffne Rustshot erneut.

    Ist Rustshot schon aktiviert, aber die Aufnahmen zeigen trotzdem keine Fenster? Klicke auf „Berechtigung zurücksetzen“ und beantworte die Anfrage von macOS.

    Startest du Rustshot aus einem Terminal? Gib die Berechtigung stattdessen deiner Terminal-App.
permission-open-settings = Systemeinstellungen öffnen
permission-reset = Berechtigung zurücksetzen
permission-reset-tip = Den alten Bildschirmaufnahme-Eintrag von Rustshot entfernen und erneut fragen
permission-title = Berechtigung „Bildschirmaufnahme“ erforderlich

## About

about-body =
    Rustshot { $version }

    Ein plattformübergreifendes Bildschirmfoto-Werkzeug nach dem Vorbild von Greenshot, geschrieben in Rust mit egui.

    Lizenziert unter der PolyForm Shield License 1.0.0: frei für jeden Zweck, auch kommerziell, außer für ein Produkt, das mit Rustshot konkurriert.
about-credits = Inspiriert von Greenshot. Symbole von Phosphor Icons.
about-project-page = Projektseite
about-reset-all = Alle Einstellungen zurücksetzen
about-reset-all-tip = Alle Einstellungen auf ihre Standardwerte zurücksetzen
about-settings-file = Einstellungsdatei

## Window and file titles

title-clipboard = Zwischenablage
title-full-screen = Ganzer Bildschirm
title-recording = Aufnahme
title-screenshot = Bildschirmfoto

## Editor status bar

confirm-save-changes = Änderungen an diesem Bildschirmfoto vor dem Schließen sichern?
status-error = Fehler: { $error }
status-saved-to = Gesichert in { $path }

## File dialogs

filter-images = Bilder

## Buttons

button-cancel = Abbrechen
button-change = Ändern …
button-close = Schließen
button-open-settings = Einstellungen öffnen
button-default = Standard
button-ok = OK
button-show = Anzeigen
button-show-file = Datei anzeigen
button-show-in-finder = Im Finder zeigen

## Editor tools

tool-arrow = Pfeil
tool-counter = Schrittzähler
tool-crop = Zuschneiden
tool-ellipse = Ellipse
tool-freehand = Freihand
tool-highlight = Hervorheben
tool-line = Linie
tool-obfuscate = Unkenntlich machen
tool-rect = Rechteck
tool-select = Auswählen
tool-speech = Sprechblase
tool-text = Text

## Editor toolbar

editor-apply-crop = Zuschneiden anwenden
editor-copy-tip = Bild in die Zwischenablage kopieren ({ $keys })
editor-send-tip = In anderer App öffnen, drucken, per E-Mail senden, teilen oder Text kopieren
editor-crop-help = Ziehe, um den Bereich zu wählen, der bleiben soll, und drücke dann den Zeilenschalter.
editor-delete-tip = Auswahl löschen (Entf)
editor-effects-tip = Effekte: Schatten, gerissene Ränder, Rahmen, Graustufen, Drehen, Größe ändern
editor-redo-tip = Wiederholen ({ $keys })
editor-save-as-tip = Sichern unter … ({ $keys })
editor-save-tip = Im Ausgabeordner sichern ({ $keys })
editor-select-help = Klicke auf eine Anmerkung, um sie auszuwählen; ziehe sie zum Verschieben und ziehe die Ecken, um die Größe zu ändern.
editor-undo-tip = Widerrufen ({ $keys })

## Editor effects

effect-border = Rahmen
effect-drop-shadow = Schlagschatten
effect-greyscale = Graustufen
effect-invert = Farben umkehren
effect-resize = Größe ändern
effect-rotate-left = Nach links drehen
effect-rotate-right = Nach rechts drehen
effect-torn-edges = Gerissene Ränder

## Editor properties bar

highlight-grey = Rest ausgrauen
highlight-magnify = Lupe
highlight-marker = Textmarker
highlight-spotlight = Scheinwerfer
prop-background = Hintergrund
prop-blur = Weichzeichnen
prop-blur-radius = Weichzeichnerradius
prop-colour = Farbe
prop-fill = Füllung
prop-line-colour = Linienfarbe
prop-line-text-colour = Linien- und Textfarbe
prop-mode = Modus
prop-no-fill = Keine Füllung
prop-pixel-size = Pixelgröße
prop-pixelate = Verpixeln
prop-shadow = Schatten
prop-size = Größe
prop-style = Stil
prop-text-colour = Textfarbe
prop-text-size = Textgröße
prop-thickness = Stärke

## Editor zoom

zoom-fit = Einpassen
zoom-fit-window = In Fenster einpassen ({ $keys })
zoom-label = Zoom: { $zoom }

## Settings window

settings-about = Über
settings-after-capture = Nach einer Aufnahme
settings-always = Immer:
settings-ask = Jedes Mal fragen
settings-ask-sub = Ein Schnellmenü zeigen: bearbeiten, kopieren oder sichern
settings-capture = Aufnahme
settings-copy-path = Dateipfad nach dem Sichern kopieren
settings-delay = Verzögerung vor der Aufnahme
settings-delay-sub = Zeit, um ein Menü zu öffnen oder auf etwas zu zeigen
settings-dest-copy = Kopieren
settings-dest-editor = Im Editor öffnen
settings-dest-folder = Im Ordner sichern
settings-open-with = In anderer App öffnen mit
settings-open-with-sub = Für „In anderer App öffnen“
settings-open-with-default = Standard-Bild-App
settings-example = Beispiel: { $name }
settings-file-name = Dateiname
settings-folder = Ordner
settings-format = Format
settings-free-space = Freier Speicher
settings-free-space-low = Dieses Volume ist fast voll. Rustshot warnt vor dem Aufnehmen; wähle am besten einen Ordner auf einem anderen Volume.
settings-free-space-value = { $free } von { $total } ({ $percent } %)
settings-general = Allgemein
settings-icon-click = Klick auf das Symbol
settings-jpeg-quality = JPEG-Qualität
settings-language = Sprache
settings-language-system = System ({ $language })
settings-magnifier = Lupe beim Auswählen
settings-menubar-icon = Symbol in der Menüleiste
settings-notifications = Mitteilungen
settings-notifications-sub = Nach dem Sichern oder Kopieren
settings-off = Aus
settings-open-at-login = Bei der Anmeldung öffnen
settings-otherwise-always = Sonst immer:
settings-pointer = Mauszeiger einbeziehen
settings-recording = Videoaufnahme
settings-right-click-hint = Klicke mit der rechten Maustaste auf das Symbol, um das Menü zu öffnen.
settings-saving = Sichern
settings-seconds = { $n } s
settings-shortcuts = Kurzbefehle
settings-title = Rustshot-Einstellungen
settings-tray-icon = Symbol im Infobereich
token-date = Datum
token-time = Uhrzeit
token-title = Fenstertitel
token-user = Benutzer

## Settings: shortcuts

shortcut-help = Klicke auf einen Kurzbefehl und drücke dann die neuen Tasten.
shortcut-help-recording = Drücke den neuen Kurzbefehl. Esc bricht ab, Entf löscht ihn.
shortcut-need-modifier = Nimm eine Sondertaste dazu: Strg, Alt, Umschalt oder Windows.
shortcut-need-modifier-mac = Nimm eine Sondertaste dazu: Wahl, Control, Umschalt oder Befehl.
shortcut-none = Keiner
shortcut-press-keys = Tasten drücken …
shortcut-record = Aufnahme starten oder beenden
shortcut-restore = Standardkurzbefehle wiederherstellen
shortcut-tip = Klicken und dann den neuen Kurzbefehl drücken

## Settings: updates

update-available = Version { $version } ist verfügbar.
update-check = Nach Updates suchen
update-checking = GitHub wird geprüft …
update-download = Laden
update-failed = Nach Updates suchen fehlgeschlagen: { $error }
update-latest = Du hast die neueste Version.

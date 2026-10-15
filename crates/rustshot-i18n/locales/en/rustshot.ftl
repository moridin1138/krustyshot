# Rustshot interface text: English (the reference language).
# Every other language file must define exactly these messages.

## Capture actions

capture-fullscreen = Capture full screen
capture-last-region = Capture last region
capture-region = Capture region
capture-window = Capture window
record-fullscreen = Record full screen
record-region = Record region…
record-stop = Stop recording
record-window = Record window…

## Menu-bar / tray menu

menu-about = About Rustshot
menu-open-clipboard = Open image from clipboard
menu-open-file = Open image from file…
menu-open-folder = Open output folder
menu-quit = Quit Rustshot
menu-update-available = Update available: { $version }…
menu-settings = Settings…
tray-click-menu = Show the menu

## After-capture destinations

dest-clipboard = Copy to clipboard
dest-editor = Open in image editor
dest-file = Save directly
dest-file-as = Save as…
dest-picker = Ask me (quick menu)
dest-open-with = Open in another app
dest-print = Print…
dest-email = Email
dest-share = Share…
dest-copy-text = Copy text (OCR)
picker-heading = Screenshot { $width } × { $height }

## Capture overlay

overlay-help-region = Drag to select a region  ·  Click a window to capture it  ·  Space: window mode  ·  Esc: cancel
overlay-help-window = Click a window to capture it  ·  Space: region mode  ·  Esc: cancel
overlay-record-help-region = Record: drag a region  ·  Click a window or the desktop  ·  Space: window mode  ·  Esc: cancel
overlay-record-help-window = Record: click a window  ·  Space: region mode  ·  Esc: cancel

## Recording

recording-bar-title = Rustshot Recording
recording-frame-title = Rustshot recording area
recording-discard = Discard
recording-discard-body = The recording will stop and the video will be deleted. This can't be undone.
recording-discard-tip = Stop and discard this recording
recording-mute-microphone = Mute microphone
recording-unmute-microphone = Unmute microphone
recording-mute-system = Mute system sound
recording-unmute-system = Unmute system sound
recording-discard-title = Discard this recording?
recording-keep = Keep recording
recording-no-display = Couldn't tell which display to record. Try again.
recording-stop = Stop

## Settings: recording

rec-estimate = Up to about { $size } per minute for a full-screen recording
rec-estimate-sub = { $width } × { $height } at { $fps } fps on your main display. Quiet screens and smaller areas use much less.
rec-format = Format
rec-format-gif-hint = GIF: plays anywhere, including chat apps and web pages, but files grow quickly. Best for short clips.
rec-format-gif-only-hint = MP4 recording is coming to this platform; for now recordings are GIFs, which suit short clips.
rec-format-mp4-hint = MP4: small files, smooth video. Best for anything longer than a few seconds.
rec-fps = Frame rate
rec-fps-value = { $fps } fps
rec-full-resolution = Full resolution
rec-full-resolution-sub = On high-resolution (Retina) displays. Off looks the same at normal size and takes about a quarter of the space.
rec-microphone = Microphone
rec-microphone-default = System default
rec-microphone-device = Which microphone
rec-microphone-missing = Not connected (using the default)
rec-microphone-sub = Record your voice with the video
rec-quality = Quality
rec-quality-high = High
rec-quality-low = Low
rec-quality-medium = Medium
rec-quality-sub = Higher quality uses more disk space
rec-shortcut-hint = Start and stop recordings from the menu or with the shortcut set in Shortcuts.
rec-show-clicks = Show mouse clicks
rec-show-clicks-sub = A ring appears where you click, so viewers can follow along
rec-system-audio = System sound
rec-system-audio-sub = Record what your computer plays
rec-system-audio-unavailable = Needs macOS 13 or later
rec-microphone-denied = Microphone access is off for Rustshot

## Disk space

disk-change-folder = Change folder…
disk-continue = Continue anyway
disk-low-body =
    Only { $free } ({ $percent }%) is free on the disk where Rustshot saves screenshots and recordings:

    { $folder }

    Saving may fail when the disk fills up. Free up space, or choose a folder on another disk.
disk-low-title = Low disk space

## Notifications

toast-copied = Copied to clipboard
toast-no-text = No text found in the image
toast-text-copied = { $lines ->
    [one] Copied 1 line of text
   *[other] Copied { $lines } lines of text
}
toast-recording-discarded = Recording discarded
toast-recording-saved = Saved { $name } ({ $size })
toast-saved = Saved { $name }
toast-show-file = Click to show the file
toast-show-finder = Click to show in Finder

## Messages

notice-capture-failed = Capture failed
notice-disk-full-body = The disk was almost full, so Rustshot stopped recording and saved what it had. Free up space before recording again.
notice-disk-full-title = Recording stopped
notice-hotkeys-body =
    Some shortcuts could not be registered (another app may be using them):

    { $errors }

    Change them in Settings > Shortcuts.
notice-hotkeys-title = Shortcuts
notice-no-clipboard-image = There is no image on the clipboard.
notice-open-image = Open image
notice-recording-failed = Recording failed
notice-no-microphone-title = Recording without the microphone
notice-no-microphone-body = Rustshot doesn't have microphone access yet. Turn the Microphone switch off and on in Settings → Recording to be asked, or allow it in System Settings → Privacy & Security → Microphone.

## Screen Recording permission (macOS)

permission-body =
    Rustshot needs Screen Recording permission to capture other apps. Without it, macOS only lets it capture the desktop background.

    1. Click Open System Settings.
    2. Under Privacy & Security > Screen Recording, turn on Rustshot.
    3. Click Quit Rustshot, then open Rustshot again.

    Is Rustshot already turned on but captures still show no windows? Click Reset permission and answer the macOS prompt.

    Running Rustshot from a terminal? Give the permission to your terminal app instead.
permission-open-settings = Open System Settings
permission-reset = Reset permission
permission-reset-tip = Remove Rustshot's old Screen Recording entry and ask again
permission-title = Screen Recording permission needed

## About

about-credits = Inspired by Greenshot. Icons by Phosphor Icons.
about-view-github = View on GitHub
about-report-problem = Report a problem
about-release-notes = Release notes
about-reset-all = Reset all settings
about-reset-all-tip = Restore every setting to its default
about-settings-file = Settings file

## Window and file titles

title-clipboard = Clipboard
title-full-screen = Full screen
title-recording = Recording
title-screenshot = Screenshot

## Editor status bar

confirm-save-changes = Save changes to this screenshot before closing?
status-error = Error: { $error }
status-saved-to = Saved to { $path }

## File dialogs

filter-images = Images

## Buttons

button-cancel = Cancel
button-change = Change…
button-close = Close
button-open-settings = Open Settings
button-default = Default
button-ok = OK
button-show = Show
button-show-file = Show file
button-show-in-finder = Show in Finder

## Editor tools

tool-arrow = Arrow
tool-counter = Step counter
tool-crop = Crop
tool-ellipse = Ellipse
tool-freehand = Freehand
tool-highlight = Highlight
tool-line = Line
tool-obfuscate = Obfuscate
tool-rect = Rectangle
tool-select = Select
tool-speech = Speech bubble
tool-text = Text

## Editor toolbar

editor-apply-crop = Apply crop
editor-copy-tip = Copy image to clipboard ({ $keys })
editor-send-tip = Open in another app, print, email, share or copy the text
editor-crop-help = Drag to choose the area to keep, then press Enter.
editor-delete-tip = Delete selected (Delete)
editor-effects-tip = Effects: shadow, torn edges, border, greyscale, rotate, resize
editor-redo-tip = Redo ({ $keys })
editor-save-as-tip = Save as… ({ $keys })
editor-save-tip = Save to the output folder ({ $keys })
editor-select-help = Click an annotation to select it; drag to move, drag corners to resize.
editor-undo-tip = Undo ({ $keys })

## Editor effects

effect-border = Border
effect-drop-shadow = Drop shadow
effect-greyscale = Greyscale
effect-invert = Invert colours
effect-resize = Resize
effect-rotate-left = Rotate left
effect-rotate-right = Rotate right
effect-torn-edges = Torn edges

## Editor properties bar

highlight-grey = Grey out the rest
highlight-magnify = Magnify
highlight-marker = Marker
highlight-spotlight = Spotlight
prop-background = Background
prop-blur = Blur
prop-blur-radius = Blur radius
prop-colour = Colour
prop-fill = Fill
prop-line-colour = Line colour
prop-line-text-colour = Line and text colour
prop-mode = Mode
prop-no-fill = No fill
prop-pixel-size = Pixel size
prop-pixelate = Pixelate
prop-shadow = Shadow
prop-size = Size
prop-style = Style
prop-text-colour = Text colour
prop-text-size = Text size
prop-thickness = Thickness

## Editor zoom

zoom-fit = Fit
zoom-fit-window = Fit to window ({ $keys })
zoom-label = Zoom: { $zoom }

## Settings window

settings-about = About
settings-after-capture = After a capture
settings-always = Always:
settings-ask = Ask each time
settings-ask-sub = Show a quick menu: edit, copy or save
settings-capture = Capture
settings-copy-path = Copy the file path after saving
settings-delay = Delay before capturing
settings-delay-sub = Time to open a menu or hover something
settings-dest-copy = Copy
settings-dest-editor = Open in editor
settings-dest-folder = Save to folder
settings-open-with = Open in another app with
settings-open-with-sub = Used by “Open in another app”
settings-open-with-default = Default image app
settings-example = Example: { $name }
settings-file-name = File name
settings-folder = Folder
settings-format = Format
settings-free-space = Free space
settings-free-space-low = This disk is nearly full. Rustshot will warn before capturing; consider a folder on another disk.
settings-free-space-value = { $free } of { $total } ({ $percent }%)
settings-general = General
settings-icon-click = Clicking the icon
settings-jpeg-quality = JPEG quality
settings-language = Language
settings-language-system = System ({ $language })
settings-magnifier = Magnifier while selecting
settings-menubar-icon = Menu-bar icon
settings-notifications = Notifications
settings-notifications-sub = After saving or copying
settings-check-updates = Check for updates automatically
settings-check-updates-sub = At start-up and once a day; only asks GitHub for the latest version number
settings-off = Off
settings-open-at-login = Open at login
settings-otherwise-always = Otherwise, always:
settings-pointer = Include the mouse pointer
settings-recording = Recording
settings-right-click-hint = Right-click the icon for the menu.
settings-saving = Saving
settings-seconds = { $n } s
settings-shortcuts = Shortcuts
settings-title = Rustshot Settings
settings-tray-icon = Tray icon
token-date = date
token-time = time
token-title = window title
token-user = user

## Settings: shortcuts

shortcut-help = Click a shortcut, then press the new keys.
shortcut-help-recording = Press the new shortcut. Esc cancels; Delete clears it.
shortcut-need-modifier = Include a modifier key: Ctrl, Alt, Shift or Win.
shortcut-need-modifier-mac = Include a modifier key: Option, Control, Shift or Command.
shortcut-none = None
shortcut-press-keys = Press keys…
shortcut-record = Start or stop recording
shortcut-restore = Restore default shortcuts
shortcut-tip = Click, then press the new shortcut

## Settings: updates

update-available = Version { $version } is available.
update-check = Check for updates
update-checking = Checking GitHub…
update-download = Download
update-notice-title = Rustshot update available
update-notice-body = Rustshot { $version } is out. You have { $current }.
update-failed = Couldn't check for updates: { $error }
update-latest = You have the latest version.

## Uploads
dest-upload = Upload
dest-upload-to = Upload to { $name }
toast-uploaded = Uploaded; link copied: { $link }
settings-uploads = Uploads
upload-intro = Send captures to your own server or an image host, and copy the link it returns. Nothing is uploaded until you set this up and choose "Upload".
upload-import = Import ShareX uploader (.sxcu)…
upload-clear = Remove uploader
upload-name = Name
upload-name-sub = Shown in menus as "Upload to …"
upload-url = Upload address
upload-method = Method
upload-body = Send the image as
upload-body-form = Form field
upload-body-binary = File only
upload-file-field = Form field name
upload-link = Link in the reply
upload-link-sub = e.g. $json:data.link$; leave empty if the reply is the link
upload-headers = Headers (for example an API key)
upload-form = Extra form fields
upload-secrets-note = Header values, such as API keys, are saved in Rustshot's settings file on this computer.
upload-test = Test upload
upload-not-ready = Add a name and an address starting with https:// to use the uploader.
upload-remove = Remove
upload-add = Add

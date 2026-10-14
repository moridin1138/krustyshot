# Rustshot interface text: English (the reference language).
# Every other language file must define exactly these messages.

## Capture actions

capture-fullscreen = Capture full screen
capture-last-region = Capture last region
capture-region = Capture region
capture-window = Capture window

## Menu-bar / tray menu

menu-about = About Rustshot
menu-open-clipboard = Open image from clipboard
menu-open-file = Open image from file…
menu-open-folder = Open output folder
menu-quit = Quit Rustshot
menu-settings = Settings…
tray-click-menu = Show the menu

## After-capture destinations

dest-clipboard = Copy to clipboard
dest-editor = Open in image editor
dest-file = Save directly
dest-file-as = Save as…
dest-picker = Ask me (quick menu)
picker-heading = Screenshot { $width } × { $height }

## Capture overlay

overlay-help-region = Drag to select a region  ·  Click a window to capture it  ·  Space: window mode  ·  Esc: cancel
overlay-help-window = Click a window to capture it  ·  Space: region mode  ·  Esc: cancel

## Notifications

toast-copied = Copied to clipboard
toast-saved = Saved { $name }
toast-show-file = Click to show the file
toast-show-finder = Click to show in Finder

## Messages

notice-capture-failed = Capture failed
notice-hotkeys-body =
    Some shortcuts could not be registered (another app may be using them):

    { $errors }

    Change them in Settings > Shortcuts.
notice-hotkeys-title = Shortcuts
notice-no-clipboard-image = There is no image on the clipboard.
notice-open-image = Open image

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

about-body =
    Rustshot { $version }

    A cross-platform screenshot tool inspired by Greenshot, written in Rust with egui.

    Licensed under the PolyForm Shield License 1.0.0: free for any use, including commercial, except providing a product that competes with Rustshot.
about-credits = Inspired by Greenshot. Icons by Phosphor Icons.
about-project-page = Project page
about-reset-all = Reset all settings
about-reset-all-tip = Restore every setting to its default
about-settings-file = Settings file

## Window and file titles

title-clipboard = Clipboard
title-full-screen = Full screen
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
settings-example = Example: { $name }
settings-file-name = File name
settings-folder = Folder
settings-format = Format
settings-general = General
settings-icon-click = Clicking the icon
settings-jpeg-quality = JPEG quality
settings-language = Language
settings-language-system = System ({ $language })
settings-magnifier = Magnifier while selecting
settings-menubar-icon = Menu-bar icon
settings-notifications = Notifications
settings-notifications-sub = After saving or copying
settings-off = Off
settings-open-at-login = Open at login
settings-otherwise-always = Otherwise, always:
settings-pointer = Include the mouse pointer
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
shortcut-restore = Restore default shortcuts
shortcut-tip = Click, then press the new shortcut

## Settings: updates

update-available = Version { $version } is available.
update-check = Check for updates
update-checking = Checking GitHub…
update-download = Download
update-failed = Couldn't check for updates: { $error }
update-latest = You have the latest version.

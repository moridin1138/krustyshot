# Rustshot interface text: French.
# Machine-drafted translation (not yet reviewed by a native speaker).

## Capture actions

capture-fullscreen = Capturer tout l'écran
capture-last-region = Capturer la dernière zone
capture-region = Capturer une zone
capture-window = Capturer une fenêtre

## Menu-bar / tray menu

menu-about = À propos de Rustshot
menu-open-clipboard = Ouvrir l'image du presse-papiers
menu-open-file = Ouvrir une image…
menu-open-folder = Ouvrir le dossier de sortie
menu-quit = Quitter Rustshot
menu-settings = Réglages…
tray-click-menu = Afficher le menu

## After-capture destinations

dest-clipboard = Copier dans le presse-papiers
dest-editor = Ouvrir dans l'éditeur d'images
dest-file = Enregistrer directement
dest-file-as = Enregistrer sous…
dest-picker = Me demander (menu rapide)
picker-heading = Capture { $width } × { $height }

## Capture overlay

overlay-help-region = Faites glisser pour choisir une zone  ·  Cliquez sur une fenêtre pour la capturer  ·  Espace : mode fenêtre  ·  Échap : annuler
overlay-help-window = Cliquez sur une fenêtre pour la capturer  ·  Espace : mode zone  ·  Échap : annuler

## Notifications

toast-copied = Copié dans le presse-papiers
toast-saved = { $name } enregistré
toast-show-file = Cliquez pour afficher le fichier
toast-show-finder = Cliquez pour afficher dans le Finder

## Messages

notice-capture-failed = Échec de la capture
notice-hotkeys-body =
    Certains raccourcis n'ont pas pu être enregistrés (une autre app les utilise peut-être) :

    { $errors }

    Modifiez-les dans Réglages > Raccourcis.
notice-hotkeys-title = Raccourcis
notice-no-clipboard-image = Le presse-papiers ne contient aucune image.
notice-open-image = Ouvrir une image

## Screen Recording permission (macOS)

permission-body =
    Rustshot a besoin de l'autorisation Enregistrement de l'écran pour capturer les autres apps. Sans elle, macOS ne lui laisse capturer que le fond d'écran.

    1. Cliquez sur Ouvrir Réglages Système.
    2. Dans Confidentialité et sécurité > Enregistrement de l'écran, activez Rustshot.
    3. Cliquez sur Quitter Rustshot, puis rouvrez Rustshot.

    Rustshot est déjà activé mais les captures ne montrent toujours aucune fenêtre ? Cliquez sur Réinitialiser l'autorisation et répondez à la demande de macOS.

    Vous lancez Rustshot depuis un terminal ? Donnez plutôt l'autorisation à votre app de terminal.
permission-open-settings = Ouvrir Réglages Système
permission-reset = Réinitialiser l'autorisation
permission-reset-tip = Supprimer l'ancienne autorisation d'enregistrement de l'écran de Rustshot et la redemander
permission-title = Autorisation d'enregistrement de l'écran requise

## About

about-body =
    Rustshot { $version }

    Un outil de capture d'écran multiplateforme inspiré de Greenshot, écrit en Rust avec egui.

    Sous licence PolyForm Shield 1.0.0 : utilisation libre à toutes fins, y compris commerciales, sauf pour fournir un produit concurrent de Rustshot.
about-credits = Inspiré de Greenshot. Icônes de Phosphor Icons.
about-project-page = Page du projet
about-reset-all = Réinitialiser tous les réglages
about-reset-all-tip = Rétablir chaque réglage à sa valeur par défaut
about-settings-file = Fichier de réglages

## Window and file titles

title-clipboard = Presse-papiers
title-full-screen = Plein écran
title-screenshot = Capture d'écran

## Editor status bar

confirm-save-changes = Enregistrer les modifications de cette capture avant de fermer ?
status-error = Erreur : { $error }
status-saved-to = Enregistré dans { $path }

## File dialogs

filter-images = Images

## Buttons

button-cancel = Annuler
button-change = Modifier…
button-close = Fermer
button-default = Par défaut
button-ok = OK
button-show = Afficher
button-show-file = Afficher le fichier
button-show-in-finder = Afficher dans le Finder

## Editor tools

tool-arrow = Flèche
tool-counter = Compteur d'étapes
tool-crop = Recadrer
tool-ellipse = Ellipse
tool-freehand = Main levée
tool-highlight = Surligner
tool-line = Ligne
tool-obfuscate = Masquer
tool-rect = Rectangle
tool-select = Sélection
tool-speech = Bulle
tool-text = Texte

## Editor toolbar

editor-apply-crop = Appliquer le recadrage
editor-copy-tip = Copier l'image dans le presse-papiers ({ $keys })
editor-crop-help = Faites glisser pour choisir la zone à garder, puis appuyez sur Entrée.
editor-delete-tip = Supprimer la sélection (Suppr)
editor-effects-tip = Effets : ombre, bords déchirés, bordure, niveaux de gris, rotation, redimensionnement
editor-redo-tip = Rétablir ({ $keys })
editor-save-as-tip = Enregistrer sous… ({ $keys })
editor-save-tip = Enregistrer dans le dossier de sortie ({ $keys })
editor-select-help = Cliquez sur une annotation pour la sélectionner ; faites-la glisser pour la déplacer, tirez les coins pour la redimensionner.
editor-undo-tip = Annuler ({ $keys })

## Editor effects

effect-border = Bordure
effect-drop-shadow = Ombre portée
effect-greyscale = Niveaux de gris
effect-invert = Inverser les couleurs
effect-resize = Redimensionner
effect-rotate-left = Pivoter à gauche
effect-rotate-right = Pivoter à droite
effect-torn-edges = Bords déchirés

## Editor properties bar

highlight-grey = Griser le reste
highlight-magnify = Loupe
highlight-marker = Surligneur
highlight-spotlight = Projecteur
prop-background = Arrière-plan
prop-blur = Flou
prop-blur-radius = Rayon du flou
prop-colour = Couleur
prop-fill = Remplissage
prop-line-colour = Couleur du trait
prop-line-text-colour = Couleur du trait et du texte
prop-mode = Mode
prop-no-fill = Aucun remplissage
prop-pixel-size = Taille des pixels
prop-pixelate = Pixeliser
prop-shadow = Ombre
prop-size = Taille
prop-style = Style
prop-text-colour = Couleur du texte
prop-text-size = Taille du texte
prop-thickness = Épaisseur

## Editor zoom

zoom-fit = Ajusté
zoom-fit-window = Ajuster à la fenêtre ({ $keys })
zoom-label = Zoom : { $zoom }

## Settings window

settings-about = À propos
settings-after-capture = Après une capture
settings-always = Toujours :
settings-ask = Demander à chaque fois
settings-ask-sub = Afficher un menu rapide : modifier, copier ou enregistrer
settings-capture = Capture
settings-copy-path = Copier le chemin du fichier après l'enregistrement
settings-delay = Délai avant la capture
settings-delay-sub = Le temps d'ouvrir un menu ou de survoler un élément
settings-dest-copy = Copier
settings-dest-editor = Ouvrir dans l'éditeur
settings-dest-folder = Enregistrer dans le dossier
settings-example = Exemple : { $name }
settings-file-name = Nom du fichier
settings-folder = Dossier
settings-format = Format
settings-general = Général
settings-icon-click = Clic sur l'icône
settings-jpeg-quality = Qualité JPEG
settings-language = Langue
settings-language-system = Système ({ $language })
settings-magnifier = Loupe pendant la sélection
settings-menubar-icon = Icône de la barre des menus
settings-notifications = Notifications
settings-notifications-sub = Après un enregistrement ou une copie
settings-off = Non
settings-open-at-login = Ouvrir à la connexion
settings-otherwise-always = Sinon, toujours :
settings-pointer = Inclure le pointeur de la souris
settings-right-click-hint = Faites un clic droit sur l'icône pour le menu.
settings-saving = Enregistrement
settings-seconds = { $n } s
settings-shortcuts = Raccourcis
settings-title = Réglages de Rustshot
settings-tray-icon = Icône de la zone de notification
token-date = date
token-time = heure
token-title = titre de la fenêtre
token-user = utilisateur

## Settings: shortcuts

shortcut-help = Cliquez sur un raccourci, puis appuyez sur les nouvelles touches.
shortcut-help-recording = Appuyez sur le nouveau raccourci. Échap annule ; Suppr l'efface.
shortcut-need-modifier = Ajoutez une touche de modification : Ctrl, Alt, Maj ou Windows.
shortcut-need-modifier-mac = Ajoutez une touche de modification : Option, Contrôle, Maj ou Commande.
shortcut-none = Aucun
shortcut-press-keys = Appuyez sur les touches…
shortcut-restore = Rétablir les raccourcis par défaut
shortcut-tip = Cliquez, puis appuyez sur le nouveau raccourci

## Settings: updates

update-available = La version { $version } est disponible.
update-check = Rechercher des mises à jour
update-checking = Vérification sur GitHub…
update-download = Télécharger
update-failed = Impossible de vérifier les mises à jour : { $error }
update-latest = Vous avez la dernière version.

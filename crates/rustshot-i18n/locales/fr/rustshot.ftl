# Rustshot interface text: French.
# Machine-drafted translation (not yet reviewed by a native speaker).

## Capture actions

capture-fullscreen = Capturer tout l'écran
capture-last-region = Capturer la dernière zone
capture-region = Capturer une zone
capture-window = Capturer une fenêtre
record-fullscreen = Enregistrer tout l'écran
record-region = Enregistrer une zone…
record-stop = Arrêter l'enregistrement
record-window = Enregistrer une fenêtre…

## Menu-bar / tray menu

menu-about = À propos de Rustshot
menu-open-clipboard = Ouvrir l'image du presse-papiers
menu-open-file = Ouvrir une image…
menu-open-folder = Ouvrir le dossier de sortie
menu-quit = Quitter Rustshot
menu-update-available = Mise à jour disponible : { $version }…
menu-settings = Réglages…
tray-click-menu = Afficher le menu

## After-capture destinations

dest-clipboard = Copier dans le presse-papiers
dest-editor = Ouvrir dans l'éditeur d'images
dest-file = Enregistrer directement
dest-file-as = Enregistrer sous…
dest-picker = Me demander (menu rapide)
dest-open-with = Ouvrir dans une autre app
dest-print = Imprimer…
dest-email = Envoyer par e-mail
dest-share = Partager…
dest-copy-text = Copier le texte (OCR)
picker-heading = Capture { $width } × { $height }

## Capture overlay

overlay-help-region = Faites glisser pour choisir une zone  ·  Cliquez sur une fenêtre pour la capturer  ·  Espace : mode fenêtre  ·  Échap : annuler
overlay-help-window = Cliquez sur une fenêtre pour la capturer  ·  Espace : mode zone  ·  Échap : annuler
overlay-record-help-region = Enregistrer : faites glisser pour choisir une zone  ·  Cliquez sur une fenêtre ou le bureau  ·  Espace : mode fenêtre  ·  Échap : annuler
overlay-record-help-window = Enregistrer : cliquez sur une fenêtre  ·  Espace : mode zone  ·  Échap : annuler

## Recording

recording-bar-title = Enregistrement Rustshot
recording-frame-title = Zone d'enregistrement Rustshot
recording-discard = Supprimer
recording-discard-body = L'enregistrement va s'arrêter et la vidéo sera supprimée. Cette action est irréversible.
recording-discard-tip = Arrêter et supprimer cet enregistrement
recording-mute-microphone = Couper le microphone
recording-unmute-microphone = Réactiver le microphone
recording-mute-system = Couper le son du système
recording-unmute-system = Réactiver le son du système
recording-discard-title = Supprimer cet enregistrement ?
recording-keep = Continuer l'enregistrement
recording-no-display = Impossible de savoir quel écran enregistrer. Réessayez.
recording-stop = Arrêter

## Settings: recording

rec-estimate = Jusqu'à environ { $size } par minute pour un enregistrement plein écran
rec-estimate-sub = { $width } × { $height } à { $fps } i/s sur votre écran principal. Un écran calme ou une zone plus petite prennent beaucoup moins de place.
rec-format = Format
rec-format-gif-hint = GIF : se lit partout, y compris dans les messageries et les pages web, mais les fichiers grossissent vite. Idéal pour de courts extraits.
rec-format-gif-only-hint = L'enregistrement en MP4 arrive bientôt sur cette plateforme ; pour l'instant, les enregistrements sont en GIF, idéal pour de courts extraits.
rec-format-mp4-hint = MP4 : fichiers légers, vidéo fluide. Idéal pour tout ce qui dure plus de quelques secondes.
rec-fps = Images par seconde
rec-fps-value = { $fps } i/s
rec-full-resolution = Pleine résolution
rec-full-resolution-sub = Sur les écrans haute résolution (Retina). Désactivée, l'image paraît identique à taille normale et prend environ quatre fois moins de place.
rec-microphone = Microphone
rec-microphone-default = Par défaut du système
rec-microphone-device = Quel microphone
rec-microphone-missing = Non connecté (micro par défaut utilisé)
rec-microphone-sub = Enregistrer votre voix avec la vidéo
rec-quality = Qualité
rec-quality-high = Haute
rec-quality-low = Basse
rec-quality-medium = Moyenne
rec-quality-sub = Une meilleure qualité prend plus d'espace disque
rec-shortcut-hint = Démarrez et arrêtez les enregistrements depuis le menu ou avec le raccourci défini dans Raccourcis.
rec-show-clicks = Afficher les clics de souris
rec-show-clicks-sub = Un cercle apparaît là où vous cliquez, pour que le public suive
rec-system-audio = Son du système
rec-system-audio-sub = Enregistrer ce que joue votre ordinateur
rec-system-audio-unavailable = Nécessite macOS 13 ou plus récent
rec-microphone-denied = L'accès au microphone est désactivé pour Rustshot

## Disk space

disk-change-folder = Changer de dossier…
disk-continue = Continuer quand même
disk-low-body =
    Il ne reste que { $free } ({ $percent } %) d'espace libre sur le disque où Rustshot enregistre les captures et les enregistrements :

    { $folder }

    L'enregistrement peut échouer quand le disque sera plein. Libérez de l'espace ou choisissez un dossier sur un autre disque.
disk-low-title = Espace disque faible

## Notifications

toast-copied = Copié dans le presse-papiers
toast-no-text = Aucun texte trouvé dans l'image
toast-text-copied = { $lines ->
    [one] 1 ligne de texte copiée
   *[other] { $lines } lignes de texte copiées
}
toast-recording-discarded = Enregistrement supprimé
toast-recording-saved = { $name } enregistré ({ $size })
toast-saved = { $name } enregistré
toast-show-file = Cliquez pour afficher le fichier
toast-show-finder = Cliquez pour afficher dans le Finder

## Messages

notice-capture-failed = Échec de la capture
notice-disk-full-body = Le disque était presque plein : Rustshot a arrêté l'enregistrement et a conservé ce qui avait été enregistré. Libérez de l'espace avant d'enregistrer à nouveau.
notice-disk-full-title = Enregistrement arrêté
notice-hotkeys-body =
    Certains raccourcis n'ont pas pu être enregistrés (une autre app les utilise peut-être) :

    { $errors }

    Modifiez-les dans Réglages > Raccourcis.
notice-hotkeys-title = Raccourcis
notice-no-clipboard-image = Le presse-papiers ne contient aucune image.
notice-open-image = Ouvrir une image
notice-recording-failed = Échec de l'enregistrement
notice-no-microphone-title = Enregistrement sans le microphone
notice-no-microphone-body = Rustshot n'a pas encore accès au microphone. Désactivez puis réactivez l'option Microphone dans Réglages → Enregistrement pour que la demande s'affiche, ou autorisez-le dans Réglages Système → Confidentialité et sécurité → Microphone.

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

about-credits = Inspiré de Greenshot. Icônes de Phosphor Icons.
about-view-github = Voir sur GitHub
about-report-problem = Signaler un problème
about-release-notes = Notes de version
about-reset-all = Réinitialiser tous les réglages
about-reset-all-tip = Rétablir chaque réglage à sa valeur par défaut
about-settings-file = Fichier de réglages

## Window and file titles

title-clipboard = Presse-papiers
title-full-screen = Plein écran
title-recording = Enregistrement
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
button-open-settings = Ouvrir les réglages
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
editor-send-tip = Ouvrir dans une autre app, imprimer, envoyer par e-mail, partager ou copier le texte
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
settings-open-with = Ouvrir dans une autre app avec
settings-open-with-sub = Utilisé par « Ouvrir dans une autre app »
settings-open-with-default = App d'images par défaut
settings-example = Exemple : { $name }
settings-file-name = Nom du fichier
settings-folder = Dossier
settings-format = Format
settings-free-space = Espace libre
settings-free-space-low = Ce disque est presque plein. Rustshot vous avertira avant chaque capture ; pensez à choisir un dossier sur un autre disque.
settings-free-space-value = { $free } sur { $total } ({ $percent } %)
settings-general = Général
settings-icon-click = Clic sur l'icône
settings-jpeg-quality = Qualité JPEG
settings-language = Langue
settings-language-system = Système ({ $language })
settings-magnifier = Loupe pendant la sélection
settings-menubar-icon = Icône de la barre des menus
settings-notifications = Notifications
settings-notifications-sub = Après un enregistrement ou une copie
settings-check-updates = Rechercher les mises à jour automatiquement
settings-check-updates-sub = Au démarrage puis une fois par jour ; demande seulement à GitHub le dernier numéro de version
settings-off = Non
settings-open-at-login = Ouvrir à la connexion
settings-otherwise-always = Sinon, toujours :
settings-pointer = Inclure le pointeur de la souris
settings-recording = Enregistrement vidéo
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
shortcut-record = Démarrer ou arrêter l'enregistrement
shortcut-restore = Rétablir les raccourcis par défaut
shortcut-tip = Cliquez, puis appuyez sur le nouveau raccourci

## Settings: updates

update-available = La version { $version } est disponible.
update-check = Rechercher des mises à jour
update-checking = Vérification sur GitHub…
update-download = Télécharger
update-notice-title = Mise à jour de Rustshot disponible
update-notice-body = Rustshot { $version } est disponible. Vous avez la version { $current }.
update-failed = Impossible de vérifier les mises à jour : { $error }
update-latest = Vous avez la dernière version.

## Uploads
dest-upload = Envoyer en ligne
dest-upload-to = Envoyer vers { $name }
toast-uploaded = Envoyé ; lien copié : { $link }
settings-uploads = Envois
upload-intro = Envoyez vos captures vers votre propre serveur ou un hébergeur d'images, et copiez le lien renvoyé. Rien n'est envoyé tant que vous n'avez pas configuré ceci et choisi « Envoyer ».
upload-import = Importer un envoi ShareX (.sxcu)…
upload-clear = Supprimer l'envoi
upload-name = Nom
upload-name-sub = Affiché dans les menus : « Envoyer vers … »
upload-url = Adresse d'envoi
upload-method = Méthode
upload-body = Envoyer l'image comme
upload-body-form = Champ de formulaire
upload-body-binary = Fichier seul
upload-file-field = Nom du champ
upload-link = Lien dans la réponse
upload-link-sub = p. ex. $json:data.link$ ; laissez vide si la réponse est le lien
upload-headers = En-têtes (par exemple une clé d'API)
upload-form = Champs de formulaire supplémentaires
upload-secrets-note = Les valeurs des en-têtes, comme les clés d'API, sont enregistrées dans le fichier de réglages de Rustshot sur cet ordinateur.
upload-test = Tester l'envoi
upload-not-ready = Indiquez un nom et une adresse commençant par https:// pour utiliser l'envoi.
upload-remove = Supprimer
upload-add = Ajouter

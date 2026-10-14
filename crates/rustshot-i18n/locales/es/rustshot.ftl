# Rustshot interface text: Spanish.
# Machine-drafted translation (not yet reviewed by a native speaker).

## Capture actions

capture-fullscreen = Capturar pantalla completa
capture-last-region = Capturar la última zona
capture-region = Capturar una zona
capture-window = Capturar una ventana
record-fullscreen = Grabar pantalla completa
record-region = Grabar una zona…
record-stop = Detener grabación
record-window = Grabar una ventana…

## Menu-bar / tray menu

menu-about = Acerca de Rustshot
menu-open-clipboard = Abrir imagen del portapapeles
menu-open-file = Abrir imagen…
menu-open-folder = Abrir la carpeta de salida
menu-quit = Salir de Rustshot
menu-settings = Ajustes…
tray-click-menu = Mostrar el menú

## After-capture destinations

dest-clipboard = Copiar al portapapeles
dest-editor = Abrir en el editor de imágenes
dest-file = Guardar directamente
dest-file-as = Guardar como…
dest-picker = Preguntarme (menú rápido)
picker-heading = Captura { $width } × { $height }

## Capture overlay

overlay-help-region = Arrastra para elegir una zona  ·  Haz clic en una ventana para capturarla  ·  Espacio: modo ventana  ·  Esc: cancelar
overlay-help-window = Haz clic en una ventana para capturarla  ·  Espacio: modo zona  ·  Esc: cancelar
overlay-record-help-region = Grabar: arrastra para elegir una zona  ·  Haz clic en una ventana o en el escritorio  ·  Espacio: modo ventana  ·  Esc: cancelar
overlay-record-help-window = Grabar: haz clic en una ventana  ·  Espacio: modo zona  ·  Esc: cancelar

## Recording

recording-bar-title = Grabación de Rustshot
recording-discard = Descartar
recording-discard-body = La grabación se detendrá y el vídeo se eliminará. No se puede deshacer.
recording-discard-tip = Detener y descartar esta grabación
recording-discard-title = ¿Descartar esta grabación?
recording-keep = Seguir grabando
recording-no-display = No se pudo saber qué pantalla grabar. Inténtalo de nuevo.
recording-stop = Detener

## Settings: recording

rec-estimate = Hasta unos { $size } por minuto en una grabación de pantalla completa
rec-estimate-sub = { $width } × { $height } a { $fps } fps en tu pantalla principal. Las pantallas con poco movimiento y las zonas más pequeñas ocupan mucho menos.
rec-format = Formato
rec-format-gif-hint = GIF: se reproduce en cualquier sitio, incluidas apps de chat y páginas web, pero los archivos crecen rápido. Ideal para clips cortos.
rec-format-gif-only-hint = La grabación en MP4 llegará pronto a esta plataforma; por ahora las grabaciones son GIF, ideales para clips cortos.
rec-format-mp4-hint = MP4: archivos pequeños y vídeo fluido. Ideal para cualquier cosa de más de unos segundos.
rec-fps = Fotogramas por segundo
rec-fps-value = { $fps } fps
rec-full-resolution = Resolución completa
rec-full-resolution-sub = En pantallas de alta resolución (Retina). Desactivada se ve igual a tamaño normal y ocupa aproximadamente una cuarta parte.
rec-quality = Calidad
rec-quality-high = Alta
rec-quality-low = Baja
rec-quality-medium = Media
rec-quality-sub = Más calidad ocupa más espacio en disco
rec-shortcut-hint = Inicia y detén las grabaciones desde el menú o con el atajo definido en Atajos.

## Disk space

disk-change-folder = Cambiar carpeta…
disk-continue = Continuar de todos modos
disk-low-body =
    Solo quedan { $free } ({ $percent } %) libres en el disco donde Rustshot guarda capturas y grabaciones:

    { $folder }

    Puede que no se pueda guardar cuando el disco se llene. Libera espacio o elige una carpeta en otro disco.
disk-low-title = Poco espacio en disco

## Notifications

toast-copied = Copiado al portapapeles
toast-recording-discarded = Grabación descartada
toast-recording-saved = { $name } guardado ({ $size })
toast-saved = { $name } guardado
toast-show-file = Haz clic para mostrar el archivo
toast-show-finder = Haz clic para mostrarlo en el Finder

## Messages

notice-capture-failed = No se pudo capturar
notice-disk-full-body = El disco estaba casi lleno, así que Rustshot detuvo la grabación y guardó lo que tenía. Libera espacio antes de volver a grabar.
notice-disk-full-title = Grabación detenida
notice-hotkeys-body =
    No se pudieron registrar algunos atajos (puede que otra app los esté usando):

    { $errors }

    Cámbialos en Ajustes > Atajos.
notice-hotkeys-title = Atajos
notice-no-clipboard-image = No hay ninguna imagen en el portapapeles.
notice-open-image = Abrir imagen
notice-recording-failed = No se pudo grabar

## Screen Recording permission (macOS)

permission-body =
    Rustshot necesita el permiso de Grabación de pantalla para capturar otras apps. Sin él, macOS solo le deja capturar el fondo del escritorio.

    1. Haz clic en Abrir Ajustes del Sistema.
    2. En Privacidad y seguridad > Grabación de pantalla, activa Rustshot.
    3. Haz clic en Salir de Rustshot y vuelve a abrir Rustshot.

    ¿Rustshot ya está activado pero las capturas siguen sin mostrar ventanas? Haz clic en Restablecer permiso y responde a la solicitud de macOS.

    ¿Ejecutas Rustshot desde un terminal? Da el permiso a tu app de terminal.
permission-open-settings = Abrir Ajustes del Sistema
permission-reset = Restablecer permiso
permission-reset-tip = Eliminar la entrada antigua de Grabación de pantalla de Rustshot y volver a pedirla
permission-title = Se necesita el permiso de Grabación de pantalla

## About

about-body =
    Rustshot { $version }

    Una herramienta de capturas de pantalla multiplataforma inspirada en Greenshot, escrita en Rust con egui.

    Con licencia PolyForm Shield 1.0.0: uso libre para cualquier fin, incluido el comercial, salvo ofrecer un producto que compita con Rustshot.
about-credits = Inspirado en Greenshot. Iconos de Phosphor Icons.
about-project-page = Página del proyecto
about-reset-all = Restablecer todos los ajustes
about-reset-all-tip = Devolver cada ajuste a su valor predeterminado
about-settings-file = Archivo de ajustes

## Window and file titles

title-clipboard = Portapapeles
title-full-screen = Pantalla completa
title-recording = Grabación
title-screenshot = Captura de pantalla

## Editor status bar

confirm-save-changes = ¿Guardar los cambios de esta captura antes de cerrar?
status-error = Error: { $error }
status-saved-to = Guardado en { $path }

## File dialogs

filter-images = Imágenes

## Buttons

button-cancel = Cancelar
button-change = Cambiar…
button-close = Cerrar
button-default = Predeterminado
button-ok = Aceptar
button-show = Mostrar
button-show-file = Mostrar archivo
button-show-in-finder = Mostrar en el Finder

## Editor tools

tool-arrow = Flecha
tool-counter = Contador de pasos
tool-crop = Recortar
tool-ellipse = Elipse
tool-freehand = Mano alzada
tool-highlight = Resaltar
tool-line = Línea
tool-obfuscate = Ocultar
tool-rect = Rectángulo
tool-select = Seleccionar
tool-speech = Bocadillo
tool-text = Texto

## Editor toolbar

editor-apply-crop = Aplicar recorte
editor-copy-tip = Copiar la imagen al portapapeles ({ $keys })
editor-crop-help = Arrastra para elegir la zona que quieres conservar y pulsa Intro.
editor-delete-tip = Eliminar la selección (Supr)
editor-effects-tip = Efectos: sombra, bordes rasgados, borde, escala de grises, girar, cambiar tamaño
editor-redo-tip = Rehacer ({ $keys })
editor-save-as-tip = Guardar como… ({ $keys })
editor-save-tip = Guardar en la carpeta de salida ({ $keys })
editor-select-help = Haz clic en una anotación para seleccionarla; arrástrala para moverla y arrastra las esquinas para cambiar su tamaño.
editor-undo-tip = Deshacer ({ $keys })

## Editor effects

effect-border = Borde
effect-drop-shadow = Sombra paralela
effect-greyscale = Escala de grises
effect-invert = Invertir colores
effect-resize = Cambiar tamaño
effect-rotate-left = Girar a la izquierda
effect-rotate-right = Girar a la derecha
effect-torn-edges = Bordes rasgados

## Editor properties bar

highlight-grey = Atenuar el resto en gris
highlight-magnify = Lupa
highlight-marker = Rotulador
highlight-spotlight = Foco
prop-background = Fondo
prop-blur = Desenfocar
prop-blur-radius = Radio de desenfoque
prop-colour = Color
prop-fill = Relleno
prop-line-colour = Color de línea
prop-line-text-colour = Color de línea y texto
prop-mode = Modo
prop-no-fill = Sin relleno
prop-pixel-size = Tamaño de píxel
prop-pixelate = Pixelar
prop-shadow = Sombra
prop-size = Tamaño
prop-style = Estilo
prop-text-colour = Color del texto
prop-text-size = Tamaño del texto
prop-thickness = Grosor

## Editor zoom

zoom-fit = Ajustar
zoom-fit-window = Ajustar a la ventana ({ $keys })
zoom-label = Zoom: { $zoom }

## Settings window

settings-about = Acerca de
settings-after-capture = Después de una captura
settings-always = Siempre:
settings-ask = Preguntar cada vez
settings-ask-sub = Mostrar un menú rápido: editar, copiar o guardar
settings-capture = Captura
settings-copy-path = Copiar la ruta del archivo después de guardar
settings-delay = Retraso antes de capturar
settings-delay-sub = Tiempo para abrir un menú o pasar el puntero por algo
settings-dest-copy = Copiar
settings-dest-editor = Abrir en el editor
settings-dest-folder = Guardar en la carpeta
settings-example = Ejemplo: { $name }
settings-file-name = Nombre del archivo
settings-folder = Carpeta
settings-format = Formato
settings-free-space = Espacio libre
settings-free-space-low = Este disco está casi lleno. Rustshot avisará antes de capturar; plantéate usar una carpeta en otro disco.
settings-free-space-value = { $free } de { $total } ({ $percent } %)
settings-general = General
settings-icon-click = Al hacer clic en el icono
settings-jpeg-quality = Calidad JPEG
settings-language = Idioma
settings-language-system = Sistema ({ $language })
settings-magnifier = Lupa al seleccionar
settings-menubar-icon = Icono de la barra de menús
settings-notifications = Notificaciones
settings-notifications-sub = Después de guardar o copiar
settings-off = No
settings-open-at-login = Abrir al iniciar sesión
settings-otherwise-always = Si no, siempre:
settings-pointer = Incluir el puntero del ratón
settings-recording = Grabación
settings-right-click-hint = Haz clic derecho en el icono para ver el menú.
settings-saving = Guardado
settings-seconds = { $n } s
settings-shortcuts = Atajos
settings-title = Ajustes de Rustshot
settings-tray-icon = Icono de la bandeja
token-date = fecha
token-time = hora
token-title = título de la ventana
token-user = usuario

## Settings: shortcuts

shortcut-help = Haz clic en un atajo y pulsa las nuevas teclas.
shortcut-help-recording = Pulsa el nuevo atajo. Esc cancela; Supr lo borra.
shortcut-need-modifier = Incluye una tecla modificadora: Ctrl, Alt, Mayús o Windows.
shortcut-need-modifier-mac = Incluye una tecla modificadora: Opción, Control, Mayúsculas o Comando.
shortcut-none = Ninguno
shortcut-press-keys = Pulsa las teclas…
shortcut-record = Iniciar o detener la grabación
shortcut-restore = Restaurar los atajos predeterminados
shortcut-tip = Haz clic y pulsa el nuevo atajo

## Settings: updates

update-available = La versión { $version } está disponible.
update-check = Buscar actualizaciones
update-checking = Comprobando en GitHub…
update-download = Descargar
update-failed = No se pudieron buscar actualizaciones: { $error }
update-latest = Tienes la última versión.

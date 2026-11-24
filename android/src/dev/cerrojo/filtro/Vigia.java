package dev.cerrojo.filtro;

import android.accessibilityservice.AccessibilityService;
import android.accessibilityservice.GestureDescription;
import android.content.Context;
import android.content.SharedPreferences;
import android.content.res.Configuration;
import android.graphics.Path;
import android.graphics.PixelFormat;
import android.graphics.Rect;
import android.os.Build;
import android.os.Handler;
import android.os.Looper;
import android.os.SystemClock;
import android.view.Choreographer;
import android.view.Gravity;
import android.view.MotionEvent;
import android.view.View;
import android.view.WindowManager;
import android.view.accessibility.AccessibilityEvent;
import android.view.accessibility.AccessibilityNodeInfo;
import android.widget.TextView;
import android.widget.Toast;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;

/**
 * Deja usar YouTube e Instagram sin Shorts ni Reels. Es un servicio de
 * accesibilidad que enciende el PC por adb (ver src/device.rs). En esas apps:
 *  - si se abre el reproductor de Shorts o de Reels, lo cierra (Atrás y, si
 *    YouTube lo deja minimizado, cierra también el reproductor pequeño);
 *  - tapa su pestaña de la barra de abajo con un recuadro que no deja tocarla
 *    (la barra no se mueve, así que la tapa no va por detrás de nada);
 *  - en el feed, pulsa «No me interesa» en el bloque de Shorts para que
 *    YouTube lo quite él mismo.
 * Solo mira la pantalla de esas dos apps; no guarda ni envía nada.
 */
public class Vigia extends AccessibilityService {
    private static final String PREFS = "vigia";
    /** Cuántos nodos se miran como mucho al buscar en toda la pantalla. */
    private static final int MAX_NODOS = 2500;
    /** Tras salir de un Short, cuánto se vigila por si queda minimizado. */
    private static final long TRAS_SALIR = 5000;
    /**
     * Cuánto se adelanta la tapa a la barra cuando entra: lo que tarda en
     * llegar a la pantalla lo que mide el vigía (unos dos fotogramas).
     */
    private static final float ADELANTO = 40f;
    /** Tras un toque o un aviso, cuánto se mira la barra a cada fotograma. */
    private static final long MIRAR = 1000;
    /** Como mucho, una mirada a la pantalla cada tanto (los avisos pueden ser muchos). */
    private static final long CADA = 50;
    /** Cuánto se recuerda un Short que quedó minimizado en segundo plano. */
    private static final long PENDIENTE = 10 * 60 * 1000;

    /** Lo que se vigila en una app. Los textos van en minúsculas. */
    private static final class Regla {
        final String clave, pkg, nombre;
        /** Vistas del reproductor de videos cortos. */
        final String[] reproductor;
        /** La barra de pestañas de abajo. */
        final String[] barra;
        /** La pestaña: por id o por su texto o descripción. */
        final String[] pestana;
        /** Los textos de la pestaña (en minúsculas): el primero y los de otros idiomas. */
        String[] textos;
        /** Pantallas (por su nombre interno) que se cierran en cuanto se abren. */
        String[] actividades = {};
        /** ¿Puede dejar el reproductor minimizado al salir? (YouTube). */
        boolean minimiza;
        /** La pestaña de inicio cuyo feed se tapa entero (LinkedIn); vacío: ninguna. */
        String[] inicio = {};
        /** Lo que dicen las tapas grandes (feed o pantalla entera). */
        String aviso = "";
        /**
         * Una página de las que se pasan deslizando (Novedades de WhatsApp),
         * reconocida por un título suyo; y la pestaña a la que se vuelve.
         */
        String[] pagina = {};
        String[] volver = {};
        /** El botón de cerrar del reproductor minimizado (trozos de su descripción). */
        final String[] cerrar;
        /** Título de los bloques de videos cortos del feed (null: no se descartan). */
        final String bloque;
        /** El botón de menú del bloque y la opción que lo quita del feed. */
        final String[] menu, descartar;
        /** La etiqueta de los directos en el feed. */
        final String[] directos;
        /** Fondo de la app, para que la tapa no desentone: claro y oscuro. */
        final int claro, oscuro;

        Regla(String clave, String pkg, String nombre, String[] reproductor, String[] barra, String[] pestana,
              String texto, String[] cerrar, String bloque, String[] menu, String[] descartar,
              String[] directos, int claro, int oscuro) {
            this.clave = clave;
            this.pkg = pkg;
            this.nombre = nombre;
            this.reproductor = reproductor;
            this.barra = barra;
            this.pestana = pestana;
            this.textos = new String[]{texto};
            this.cerrar = cerrar;
            this.bloque = bloque;
            this.menu = menu;
            this.descartar = descartar;
            this.directos = directos;
            this.claro = claro;
            this.oscuro = oscuro;
        }

        Regla tambien(String... otros) {
            String[] n = Arrays.copyOf(textos, textos.length + otros.length);
            System.arraycopy(otros, 0, n, textos.length, otros.length);
            textos = n;
            return this;
        }

        Regla actividades(String... a) {
            actividades = a;
            return this;
        }

        Regla minimiza() {
            minimiza = true;
            return this;
        }

        Regla inicio(String... t) {
            inicio = t;
            return this;
        }

        Regla aviso(String a) {
            aviso = a;
            return this;
        }

        Regla pagina(String[] titulos, String... volverA) {
            pagina = titulos;
            volver = volverA;
            return this;
        }
    }

    // YouTube e Instagram cambian estos nombres de vez en cuando. Si deja de
    // funcionar, se miran con `adb shell uiautomator dump` y se ponen aquí.
    private static final Regla[] REGLAS = {
        new Regla("youtube", "com.google.android.youtube", "Shorts",
            new String[]{"reel_recycler", "reel_player_page_container", "reel_watch_player"},
            new String[]{"pivot_bar"},
            new String[]{},
            "shorts",
            new String[]{"cerrar", "close", "descartar", "dismiss"},
            "shorts",
            new String[]{"más opciones", "mas opciones", "more options", "menú de acciones", "action menu", "menú"},
            new String[]{"no me interesa", "not interested", "mostrar menos shorts", "show fewer shorts",
                "menos shorts", "fewer shorts"},
            new String[]{"en vivo", "en directo", "live", "directo"},
            0xFFFFFFFF, 0xFF0F0F0F).minimiza(),
        new Regla("instagram", "com.instagram.android", "Reels",
            new String[]{"clips_viewer_view_pager"},
            new String[]{"tab_bar"},
            new String[]{"clips_tab"},
            "reels",
            new String[]{"cerrar", "close"},
            null, new String[]{}, new String[]{}, new String[]{},
            0xFFFFFFFF, 0xFF000000),
        // WhatsApp: los Estados se ven en su propia pantalla (StatusPlaybackActivity),
        // así que basta con su nombre; la pestaña «Novedades» se tapa.
        new Regla("whatsapp", "com.whatsapp", "Estados",
            new String[]{}, new String[]{}, new String[]{},
            "novedades",
            new String[]{},
            null, new String[]{}, new String[]{}, new String[]{},
            0xFFFFFFFF, 0xFF0B141A)
            .tambien("updates", "estados", "status").actividades("StatusPlayback", "status.playback")
            .aviso("Estados bloqueados")
            .pagina(new String[]{"estados", "status"}, "chats"),
        new Regla("whatsapp", "com.whatsapp.w4b", "Estados",
            new String[]{}, new String[]{}, new String[]{},
            "novedades",
            new String[]{},
            null, new String[]{}, new String[]{}, new String[]{},
            0xFFFFFFFF, 0xFF0B141A)
            .tambien("updates", "estados", "status").actividades("StatusPlayback", "status.playback")
            .aviso("Estados bloqueados")
            .pagina(new String[]{"estados", "status"}, "chats"),
        // LinkedIn: el feed de Inicio se tapa entero (buscador, mensajes y las
        // demás pestañas quedan libres) y la pestaña «Video» también.
        new Regla("linkedin", "com.linkedin.android", "Videos",
            new String[]{}, new String[]{}, new String[]{},
            "video",
            new String[]{},
            null, new String[]{}, new String[]{}, new String[]{},
            0xFFFFFFFF, 0xFF1B1F23)
            .tambien("vídeo", "videos", "vídeos")
            .inicio("inicio", "home")
            .aviso("Feed bloqueado por Cerrojo\nLas demás pestañas siguen abiertas"),
    };

    private volatile Map<String, Regla> activas = new HashMap<>();
    private SharedPreferences.OnSharedPreferenceChangeListener oyente;

    private final List<TextView> vistas = new ArrayList<>();
    private final List<Rect> puestas = new ArrayList<>();
    /** Lo que se está tapando ahora y no se mueve: el reproductor minimizado. */
    private List<Rect> fijas = new ArrayList<>();
    /**
     * La tapa de la pestaña va aparte: la barra de abajo se esconde al
     * deslizar, así que esta tapa se quita en cuanto el dedo toca la pantalla
     * (antes de que nada se mueva) y vuelve cuando la barra está quieta.
     */
    private TextView tapaPestana;
    private Rect tapaPestanaEn;
    private boolean tapaPestanaVisible;
    private Rect pestanaAntes;
    /** Dónde descansa la pestaña cuando la barra está entera (para no pasarse al adelantar). */
    private Rect reposo;
    private long pestanaAntesEn, ultimoProceso;
    /**
     * Un punto invisible de 1 píxel en una esquina mientras YouTube está al
     * frente: Android le avisa al instante de cada toque en la pantalla (sin
     * decir dónde), y así el vigía empieza a mirar la barra antes de que se mueva.
     */
    private View vigilante;
    private boolean procesoPendiente;
    private AccessibilityNodeInfo nodoPestana;
    private Regla regla;
    private Rect pantallaActual;
    private final Handler manejador = new Handler(Looper.getMainLooper());
    private long seguirHasta;
    private boolean siguiendo;
    private long ultimoAtras, rachaDesde, ultimoAviso, salirHasta, ultimoCierre;
    /** Desde cuándo puede quedar un Short minimizado sin cerrar (0 = no). */
    private long pendienteDesde;
    /** El paquete de la última ventana al frente. */
    private String alFrente;
    private int racha, deslices;
    /**
     * «No me interesa»: menú abierto desde (0 = no), última vez, qué se está
     * descartando (0 bloque de Shorts, 1 directo) y fallos de cada uno.
     */
    private long menuDesde, ultimoBloque;
    private int menuTipo;
    private final int[] fallos = new int[2];
    /**
     * Una pantalla prohibida abierta (los Estados de WhatsApp): se tapa entera
     * y se pulsa Atrás hasta que se cierre de verdad.
     */
    private Regla bloqueando;
    private long bloqueoDesde;
    private int insistencias;
    private TextView tapaTotal;
    /** La página prohibida (Novedades): su nodo, su tapa y cómo se movía. */
    private AccessibilityNodeInfo nodoPagina;
    private TextView tapaPagina;
    private Rect tapaPaginaEn, paginaAntes;
    private long paginaAntesEn, paginaBuscada, ultimoVolver;
    /** El PC pidió quitar también los directos ("youtube-directos"). */
    private volatile boolean quitarDirectos;

    // ---- Lo que pide el PC ----

    /** Guarda qué apps vigilar: claves separadas por espacios ("youtube instagram"). */
    static void guardar(Context c, String claves) {
        c.getSharedPreferences(PREFS, MODE_PRIVATE).edit().putString("claves", claves == null ? "" : claves).commit();
    }

    /** Los paquetes que se vigilan ahora (para el guardián). */
    static Set<String> paquetes(Context c) {
        String claves = " " + c.getSharedPreferences(PREFS, MODE_PRIVATE).getString("claves", "") + " ";
        Set<String> s = new HashSet<>();
        for (Regla r : REGLAS) {
            if (claves.contains(" " + r.clave + " ")) s.add(r.pkg);
        }
        return s;
    }

    /** ¿Se vigila pkg ahora? No mientras dure un desbloqueo temporal (Guardia). */
    private boolean vigilada(Map<String, Regla> m, CharSequence pkg) {
        return pkg != null && m.containsKey(pkg.toString()) && !Guardia.abierto(this, pkg.toString());
    }

    private void cargar() {
        String claves = " " + getSharedPreferences(PREFS, MODE_PRIVATE).getString("claves", "") + " ";
        Map<String, Regla> m = new HashMap<>();
        for (Regla r : REGLAS) {
            if (claves.contains(" " + r.clave + " ")) m.put(r.pkg, r);
        }
        activas = m;
        quitarDirectos = claves.contains(" youtube-directos ");
        if (m.isEmpty()) quitarTapas();
    }

    @Override
    protected void onServiceConnected() {
        super.onServiceConnected();
        oyente = (p, k) -> cargar();
        getSharedPreferences(PREFS, MODE_PRIVATE).registerOnSharedPreferenceChangeListener(oyente);
        cargar();
    }

    @Override
    public void onInterrupt() {
    }

    @Override
    public void onDestroy() {
        quitarTapas();
        super.onDestroy();
    }

    // ---- Pantalla ----

    @Override
    public void onAccessibilityEvent(AccessibilityEvent e) {
        Map<String, Regla> m = activas;
        if (m.isEmpty()) return;
        CharSequence ep = e.getPackageName();
        boolean deVigilada = vigilada(m, ep);
        // Lo de otras apps solo importa para quitar las tapas al salir.
        if (!deVigilada && puestas.isEmpty() && tapaPestana == null && tapaTotal == null && tapaPagina == null
                && capaFeed.v == null && capaInicio.v == null) return;
        long ahora = SystemClock.uptimeMillis();
        if (e.getEventType() == AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED && e.getClassName() != null) {
            String pantallaNueva = e.getClassName().toString();
            Regla ra = deVigilada ? m.get(ep.toString()) : null;
            boolean prohibida = ra != null && esProhibida(ra, pantallaNueva);
            if (prohibida) {
                // Los Estados de WhatsApp: tapados ya y fuera.
                empezarBloqueo(ra, ahora);
                return;
            }
            if (bloqueando != null && ep != null && !ep.toString().equals(getPackageName())) {
                boolean cerrada;
                if (bloqueando.pkg.equals(ep.toString())) {
                    // Otra pantalla de WhatsApp de verdad (no un menú encima): se cerró.
                    cerrada = pantallaNueva.contains("Activity") || !pantallaNueva.startsWith("android.");
                } else {
                    // Otra app: solo cuenta si ya está al frente (no una notificación encima).
                    AccessibilityNodeInfo rt = getRootInActiveWindow();
                    CharSequence rtp = rt == null ? null : rt.getPackageName();
                    cerrada = rtp != null && !bloqueando.pkg.equals(rtp.toString());
                }
                if (cerrada) terminarBloqueo();
            }
        }
        if (deVigilada && (e.getEventType() == AccessibilityEvent.TYPE_VIEW_CLICKED
                || e.getEventType() == AccessibilityEvent.TYPE_VIEW_SELECTED)) {
            Regla rc = m.get(ep.toString());
            if (rc.inicio.length > 0 && pantallaActual != null) {
                // Estos avisos llegan al momento (no van con retraso como los demás).
                if (esInicio(e, rc)) {
                    inicioHasta = ahora + MARGEN_INICIO;
                    clicInicio = ahora;
                    Rect tb = capaInicio.en != null ? capaInicio.en
                            : nodoInicio != null ? limites(nodoInicio) : null;
                    if (tb != null) {
                        capaFeed.poner(rectFeed(tb, pantallaActual), rc);
                        capaInicio.quitar();
                    }
                } else if (e.getEventType() == AccessibilityEvent.TYPE_VIEW_CLICKED) {
                    // Otro botón: ya no se está yendo a Inicio.
                    inicioHasta = 0;
                    clicOtro = ahora;
                }
            }
            seguir(ahora);
        }
        if (deVigilada && e.getEventType() == AccessibilityEvent.TYPE_VIEW_SCROLLED) {
            // Deslizando: a mirar la barra a cada fotograma (lo que decide es
            // si ella se mueve, no el aviso, que llega tarde).
            seguir(ahora);
        }
        if (ahora - ultimoProceso < CADA) {
            if (!procesoPendiente) {
                procesoPendiente = true;
                manejador.postDelayed(procesarLuego, CADA - (ahora - ultimoProceso));
            }
            return;
        }
        procesar(deVigilada);
    }

    private final Runnable procesarLuego = () -> {
        procesoPendiente = false;
        procesar(true);
    };

    private void procesar(boolean deVigilada) {
        Map<String, Regla> m = activas;
        if (m.isEmpty()) return;
        ultimoProceso = SystemClock.uptimeMillis();
        AccessibilityNodeInfo root;
        try {
            root = getRootInActiveWindow();
        } catch (RuntimeException ex) {
            return;
        }
        CharSequence rp = root == null ? null : root.getPackageName();
        Regla r = vigilada(m, rp) ? m.get(rp.toString()) : null;
        long ahora = SystemClock.uptimeMillis();
        if (r == null) {
            // Otra app al frente (o el inicio): fuera la tapa.
            if (root != null || !deVigilada) quitarTapas();
            if (rp != null) alFrente = rp.toString();
            return;
        }
        // Se vuelve a la app con un Short que quedó minimizado (por ejemplo,
        // abierto desde una notificación: Atrás sale de YouTube y lo deja ahí).
        if (!r.pkg.equals(alFrente) && pendienteDesde != 0 && ahora - pendienteDesde < PENDIENTE) {
            salirHasta = ahora + TRAS_SALIR;
            deslices = 0;
        }
        alFrente = r.pkg;
        try {
            Rect pantalla = limites(root);
            if (pantalla.isEmpty()) return;

            // 1. El reproductor: grande, se sale; pequeño (minimizado), se cierra.
            List<Rect> pequenos = new ArrayList<>();
            boolean grande = reproductor(root, r, pantalla, pequenos);
            if (grande) {
                salir(r, ahora);
            } else if (!pequenos.isEmpty() || ahora < salirHasta) {
                cerrarMinimizado(root, r, pantalla, pequenos, ahora);
            }
            // 2. La pestaña de abajo. Mientras se cierra, el minimizado también
            // queda tapado para que no se pueda tocar.
            AccessibilityNodeInfo barra = barra(root, r);
            ponerVigilante();
            regla = r;
            pantallaActual = pantalla;
            fijas = pequenos;
            if (r.inicio.length > 0) revisarInicio(root, r, pantalla, true);
            if (bloqueando == null && reproductorDentro(root, r, ahora)) {
                bloqueoPorDentro = true;
                empezarBloqueo(r, ahora);
                return;
            }
            nodoPestana = pestana(root, barra, r, pantalla);
            if (r.pagina.length > 0) {
                if (nodoPagina == null || !nodoPagina.refresh()) nodoPagina = buscarPagina(root, r, pantalla, ahora);
                mostrarPagina(root, r);
            }
            mostrar();
            seguir(ahora);

            // 3. El bloque de Shorts del feed: que YouTube lo quite.
            if (!grande && pequenos.isEmpty() && ahora >= salirHasta) bloques(root, barra, r, pantalla, ahora);
        } catch (RuntimeException ignored) {
            // La pantalla cambió mientras la mirábamos: ya llegará otro evento.
        }
    }

    // ---- 1. Reproductor ----

    private static List<AccessibilityNodeInfo> porId(AccessibilityNodeInfo root, Regla r, String id) {
        List<AccessibilityNodeInfo> l = root.findAccessibilityNodeInfosByViewId(r.pkg + ":id/" + id);
        return l == null ? new ArrayList<>() : l;
    }

    /** ¿Está el reproductor a pantalla completa? Los minimizados van a pequenos. */
    private static boolean reproductor(AccessibilityNodeInfo root, Regla r, Rect pantalla, List<Rect> pequenos) {
        boolean grande = false;
        for (String id : r.reproductor) {
            for (AccessibilityNodeInfo n : porId(root, r, id)) {
                if (!n.isVisibleToUser()) continue;
                Rect b = limites(n);
                if (b.isEmpty()) continue;
                if (b.height() * 2 >= pantalla.height() && b.width() * 2 >= pantalla.width()) grande = true;
                else pequenos.add(b);
            }
        }
        return grande;
    }

    private void salir(Regla r, long ahora) {
        // Atrás cierra el Short o, si venía del feed, lo minimiza: eso lo
        // recoge cerrarMinimizado durante los próximos segundos.
        if (r.minimiza) {
            salirHasta = ahora + TRAS_SALIR;
            pendienteDesde = ahora;
        }
        if (ahora - ultimoAtras < 600) return; // dar tiempo a que se cierre
        deslices = 0;
        if (ahora - rachaDesde > 5000) {
            racha = 0;
            rachaDesde = ahora;
        }
        racha++;
        ultimoAtras = ahora;
        // Si Atrás no basta (se abrió desde un enlace, por ejemplo), al inicio.
        performGlobalAction(racha > 3 ? GLOBAL_ACTION_HOME : GLOBAL_ACTION_BACK);
        if (ahora - ultimoAviso > 4000) {
            ultimoAviso = ahora;
            Toast.makeText(this, "Cerrojo: " + r.nombre + " bloqueados", Toast.LENGTH_SHORT).show();
        }
    }

    /**
     * Cierra el reproductor minimizado: su botón de cerrar o, si no lo tiene a
     * la vista, deslizándolo fuera de la pantalla (como se haría con el dedo).
     */
    private void cerrarMinimizado(AccessibilityNodeInfo root, Regla r, Rect pantalla, List<Rect> pequenos, long ahora) {
        if (ahora - ultimoCierre < 500) return;
        // Dónde buscar: alrededor del reproductor pequeño o, si no se reconoce
        // (YouTube lo pasa a su reproductor de siempre), la parte de abajo.
        List<Rect> zonas = new ArrayList<>();
        for (Rect b : pequenos) {
            Rect z = new Rect(b);
            z.inset(-b.width() / 4 - 40, -b.height() / 4 - 40);
            zonas.add(z);
        }
        if (zonas.isEmpty()) {
            zonas.add(new Rect(pantalla.left, pantalla.top + pantalla.height() * 55 / 100, pantalla.right, pantalla.bottom));
        }
        for (AccessibilityNodeInfo n : recorrer(root)) {
            // Ojo: «close» también está en «closed captions» (subtítulos).
            String vid = n.getViewIdResourceName();
            boolean porId = vid != null && (vid.contains("close_button") || vid.endsWith("_close") || vid.endsWith("/close"));
            if (!porId && !igualAlguno(n.getContentDescription(), r.cerrar) && !igualAlguno(n.getText(), r.cerrar)) continue;
            // A veces la descripción está en el icono y el botón es su padre.
            AccessibilityNodeInfo boton = clicable(n);
            if (!boton.isClickable()) continue;
            Rect b = limites(boton);
            for (Rect z : zonas) {
                if (z.contains(b.centerX(), b.centerY())) {
                    boton.performAction(AccessibilityNodeInfo.ACTION_CLICK);
                    ultimoCierre = ahora;
                    pendienteDesde = 0;
                    return;
                }
            }
        }
        // Deslizar solo justo después de salir de un Short, y como mucho dos
        // veces: si no era un reproductor, no se queda moviendo el feed.
        if (!pequenos.isEmpty() && ahora < salirHasta && deslices < 2) {
            deslices++;
            Rect b = pequenos.get(0);
            Path camino = new Path();
            camino.moveTo(b.centerX(), b.centerY());
            camino.lineTo(b.centerX(), pantalla.bottom + b.height());
            dispatchGesture(new GestureDescription.Builder()
                    .addStroke(new GestureDescription.StrokeDescription(camino, 0, 180))
                    .build(), null, null);
            ultimoCierre = ahora;
        }
    }

    // ---- 2. Pestaña ----

    private static AccessibilityNodeInfo barra(AccessibilityNodeInfo root, Regla r) {
        for (String id : r.barra) {
            for (AccessibilityNodeInfo n : porId(root, r, id)) {
                if (n.isVisibleToUser() && !limites(n).isEmpty()) return n;
            }
        }
        return null;
    }

    /**
     * El botón de la pestaña de Shorts/Reels, o null si no hay. Dentro de la
     * barra se busca aunque YouTube la tenga fuera de la pantalla: así se ve
     * en cuanto empieza a volver.
     */
    private static AccessibilityNodeInfo pestana(AccessibilityNodeInfo root, AccessibilityNodeInfo barra, Regla r, Rect pantalla) {
        if (barra == null) {
            for (String id : r.barra) {
                List<AccessibilityNodeInfo> l = porId(root, r, id);
                if (!l.isEmpty()) {
                    barra = l.get(0);
                    break;
                }
            }
        }
        if (barra != null) {
            for (AccessibilityNodeInfo n : recorrerTodo(barra)) {
                boolean ok = false;
                String vid = n.getViewIdResourceName();
                for (String id : r.pestana) ok |= (r.pkg + ":id/" + id).equals(vid);
                ok |= igualAlguno(n.getText(), r.textos) || igualAlguno(n.getContentDescription(), r.textos);
                if (ok) return clicable(n);
            }
            return null;
        }
        // Sin barra reconocida: por id o texto, abajo del todo (o a la izquierda).
        List<AccessibilityNodeInfo> cand = new ArrayList<>();
        for (String id : r.pestana) cand.addAll(porId(root, r, id));
        for (String texto : r.textos) {
            List<AccessibilityNodeInfo> t = root.findAccessibilityNodeInfosByText(texto);
            if (t != null) cand.addAll(t);
        }
        for (AccessibilityNodeInfo n : cand) {
            if (n == null || !n.isVisibleToUser()) continue;
            boolean ok = false;
            String vid = n.getViewIdResourceName();
            for (String id : r.pestana) ok |= (r.pkg + ":id/" + id).equals(vid);
            // «Novedades, 3 nuevas» también cuenta.
            ok |= igualAlguno(n.getText(), r.textos) || igualAlguno(n.getContentDescription(), r.textos);
            if (!ok) continue;
            // La pestaña es el botón entero, no solo su texto.
            AccessibilityNodeInfo boton = clicable(n);
            Rect b = limites(boton);
            if (b.isEmpty()) continue;
            if (enBorde(b, pantalla)) return boton;
        }
        return null;
    }

    /**
     * Pone las tapas: el minimizado (si lo hay) y la pestaña, recortada al
     * borde de la pantalla para que baje con la barra cuando YouTube la esconde.
     */
    private void mostrar() {
        ponerTapas(fijas, regla);
        long ahora = SystemClock.uptimeMillis();
        AccessibilityNodeInfo n = nodoPestana;
        Rect b = n != null && pantallaActual != null && n.isVisibleToUser() ? limites(n) : null;
        if (b == null || b.isEmpty() || !Rect.intersects(b, pantallaActual)) {
            // Fuera de la pantalla (o no hay barra).
            pestanaAntes = null;
            ponerTapaPestana(null);
            return;
        }
        // Hacia dónde y qué tan rápido va la barra desde la última medida.
        int dy = 0;
        float v = 0;
        if (pestanaAntes != null) {
            dy = b.top - pestanaAntes.top;
            v = dy / (float) Math.max(1, ahora - pestanaAntesEn);
        }
        pestanaAntes = new Rect(b);
        pestanaAntesEn = ahora;
        if (dy != 0) seguirHasta = Math.max(seguirHasta, ahora + 300);

        if (dy > 0) {
            // Se va hacia abajo: fuera en el acto.
            ponerTapaPestana(null);
            return;
        }
        Rect c = new Rect(b);
        if (dy < 0) {
            // Entrando: tapar donde está y donde va a estar dentro de un
            // momento, sin pasar de donde descansa (ahí se para).
            int arriba = b.top + Math.round(v * ADELANTO);
            if (reposo != null) arriba = Math.max(arriba, Math.min(b.top, reposo.top));
            c.top = Math.min(c.top, arriba);
        } else if (pantallaActual.contains(b)) {
            reposo = new Rect(b);
        }
        if (c.intersect(pantallaActual) && c.height() > 2) ponerTapaPestana(c);
        else ponerTapaPestana(null);
    }

    /**
     * La barra se esconde y aparece con una animación, y durante ella Android
     * no avisa de nada: así que tras cada aviso se sigue la pestaña un rato,
     * a cada fotograma, y la tapa va pegada a ella.
     */
    private void seguir(long ahora) {
        seguirHasta = Math.max(seguirHasta, ahora + MIRAR);
        if (!siguiendo && regla != null) {
            siguiendo = true;
            Choreographer.getInstance().postFrameCallback(cuadro);
        }
    }

    /** Una medida por fotograma de la pantalla (al ritmo de la pantalla, no de un reloj). */
    private final Choreographer.FrameCallback cuadro = t -> paso();

    private void paso() {
        siguiendo = false;
        if (regla == null) return;
        try {
            // refresh() le pregunta a la app dónde está ahora; si la pestaña ya
            // no existe (YouTube rehízo la barra), se busca de nuevo.
            AccessibilityNodeInfo n = nodoPestana;
            if (n == null || !n.refresh()) nodoPestana = buscarPestana();
            mostrar();
            if (regla.inicio.length > 0 && pantallaActual != null) {
                // Cada fotograma: ¿se llegó a Inicio sin tocar el botón (con Atrás)?
                AccessibilityNodeInfo rt = getRootInActiveWindow();
                CharSequence rtp = rt == null ? null : rt.getPackageName();
                if (rtp != null && regla.pkg.equals(rtp.toString())) revisarInicio(rt, regla, pantallaActual, false);
            }
            if (regla.pagina.length > 0) {
                long ahora = SystemClock.uptimeMillis();
                AccessibilityNodeInfo pg = nodoPagina;
                if (pg == null || !pg.refresh()) {
                    AccessibilityNodeInfo root = getRootInActiveWindow();
                    nodoPagina = root == null || pantallaActual == null ? null : buscarPagina(root, regla, pantallaActual, ahora);
                }
                mostrarPagina(null, regla);
            }
        } catch (RuntimeException ignored) {
            // Se sigue en el siguiente fotograma.
        }
        if (SystemClock.uptimeMillis() < seguirHasta) {
            siguiendo = true;
            Choreographer.getInstance().postFrameCallback(cuadro);
        }
    }

    private AccessibilityNodeInfo buscarPestana() {
        Regla r = regla;
        AccessibilityNodeInfo root = getRootInActiveWindow();
        if (r == null || root == null || root.getPackageName() == null || !r.pkg.equals(root.getPackageName().toString())) {
            return null;
        }
        return pestana(root, null, r, limites(root));
    }

    /** Cada toque en la pantalla: a mirar la barra desde ya. */
    private void alTocar() {
        seguir(SystemClock.uptimeMillis());
    }

    private void ponerVigilante() {
        if (vigilante != null) return;
        View v = new View(this);
        v.setOnTouchListener((vista, ev) -> {
            if (ev.getActionMasked() == MotionEvent.ACTION_OUTSIDE) alTocar();
            return false;
        });
        WindowManager.LayoutParams lp = params(new Rect(0, 0, 1, 1), true);
        try {
            ((WindowManager) getSystemService(WINDOW_SERVICE)).addView(v, lp);
            vigilante = v;
        } catch (RuntimeException ignored) {
        }
    }

    private void quitarVigilante() {
        if (vigilante == null) return;
        try {
            ((WindowManager) getSystemService(WINDOW_SERVICE)).removeView(vigilante);
        } catch (RuntimeException ignored) {
        }
        vigilante = null;
    }

    // ---- 3. Bloque de Shorts del feed ----

    /**
     * Abre el menú del bloque «Shorts» y pulsa «No me interesa»: YouTube lo
     * quita del feed (durante un tiempo) sin que haya nada que tapar. Si el
     * menú no tiene esa opción, se cierra; tras tres fallos se deja de probar.
     */
    private void bloques(AccessibilityNodeInfo root, AccessibilityNodeInfo barra, Regla r, Rect pantalla, long ahora) {
        boolean directos = quitarDirectos && r.directos.length > 0;
        if (r.bloque == null && !directos) return;
        if (menuDesde != 0) {
            for (AccessibilityNodeInfo n : recorrer(root)) {
                if (igualAlguno(n.getText(), r.descartar) || igualAlguno(n.getContentDescription(), r.descartar)) {
                    clicable(n).performAction(AccessibilityNodeInfo.ACTION_CLICK);
                    menuDesde = 0;
                    fallos[menuTipo] = 0;
                    ultimoBloque = ahora;
                    return;
                }
            }
            if (ahora - menuDesde > 1500) {
                // Era otro menú: se cierra sin tocar nada.
                performGlobalAction(GLOBAL_ACTION_BACK);
                menuDesde = 0;
                fallos[menuTipo]++;
                ultimoBloque = ahora;
            }
            return;
        }
        if (ahora - ultimoBloque < 1200) return;
        ultimoBloque = ahora;
        Rect enBarra = barra == null ? null : limites(barra);
        List<AccessibilityNodeInfo> nodos = recorrer(root);
        List<AccessibilityNodeInfo> menus = new ArrayList<>();
        for (AccessibilityNodeInfo b : nodos) {
            if (b.isClickable() && contiene(b.getContentDescription(), r.menu) && limites(b).width() * 4 <= pantalla.width()) {
                menus.add(b);
            }
        }
        if (menus.isEmpty()) return;
        for (AccessibilityNodeInfo t : nodos) {
            Rect h = limites(t);
            // Nada de la barra de abajo.
            if (h.isEmpty() || (enBarra != null && Rect.intersects(enBarra, h)) || enBorde(h, pantalla)) continue;

            // El bloque «Shorts»: su menú está en la fila del título, a la derecha.
            if (r.bloque != null && fallos[0] < 3 && (igual(t.getText(), r.bloque) || igual(t.getContentDescription(), r.bloque))) {
                // Arriba del todo suele ser el filtro «Shorts» de la búsqueda: ese no.
                if (h.top < pantalla.top + pantalla.height() * 0.12) continue;
                for (AccessibilityNodeInfo b : menus) {
                    Rect mb = limites(b);
                    boolean fila = mb.centerY() >= h.top - h.height() && mb.centerY() <= h.bottom + h.height();
                    if (fila && mb.left >= h.right) {
                        abrirMenu(b, 0, ahora);
                        return;
                    }
                }
                continue;
            }

            // Un directo: la etiqueta «EN VIVO» sobre la miniatura. Solo en los
            // feeds (con la barra de abajo a la vista), no viendo un video.
            if (directos && enBarra != null && fallos[1] < 3
                    && (exacto(t.getText(), r.directos) || exacto(t.getContentDescription(), r.directos))) {
                Rect tarjeta = limites(tarjeta(t, pantalla));
                for (AccessibilityNodeInfo b : menus) {
                    Rect mb = limites(b);
                    if (tarjeta.contains(mb.centerX(), mb.centerY())) {
                        abrirMenu(b, 1, ahora);
                        return;
                    }
                }
            }
        }
    }

    private void abrirMenu(AccessibilityNodeInfo boton, int tipo, long ahora) {
        boton.performAction(AccessibilityNodeInfo.ACTION_CLICK);
        menuDesde = ahora;
        menuTipo = tipo;
    }

    /** La tarjeta del feed que contiene n: el primer ancestro casi tan ancho como la pantalla. */
    private static AccessibilityNodeInfo tarjeta(AccessibilityNodeInfo n, Rect pantalla) {
        AccessibilityNodeInfo m = n;
        for (int i = 0; i < 10; i++) {
            AccessibilityNodeInfo p = m.getParent();
            if (p == null) break;
            Rect b = limites(p);
            if (b.height() * 10 > pantalla.height() * 7) break; // ya es el feed entero
            m = p;
            if (b.width() * 100 >= pantalla.width() * 85 && b.height() * 4 >= pantalla.width()) break;
        }
        return m;
    }

    // ---- Utilidades ----

    /** Todos los nodos bajo n, se vean o no (para la barra, que puede estar fuera). */
    private static List<AccessibilityNodeInfo> recorrerTodo(AccessibilityNodeInfo n0) {
        List<AccessibilityNodeInfo> out = new ArrayList<>();
        ArrayDeque<AccessibilityNodeInfo> pila = new ArrayDeque<>();
        pila.push(n0);
        while (!pila.isEmpty() && out.size() < 300) {
            AccessibilityNodeInfo n = pila.pop();
            out.add(n);
            for (int i = n.getChildCount() - 1; i >= 0; i--) {
                AccessibilityNodeInfo c = n.getChild(i);
                if (c != null) pila.push(c);
            }
        }
        return out;
    }

    /** Los nodos visibles bajo n (hasta MAX_NODOS). */
    private static List<AccessibilityNodeInfo> recorrer(AccessibilityNodeInfo n0) {
        List<AccessibilityNodeInfo> out = new ArrayList<>();
        ArrayDeque<AccessibilityNodeInfo> pila = new ArrayDeque<>();
        pila.push(n0);
        while (!pila.isEmpty() && out.size() < MAX_NODOS) {
            AccessibilityNodeInfo n = pila.pop();
            if (!n.isVisibleToUser()) continue;
            out.add(n);
            for (int i = n.getChildCount() - 1; i >= 0; i--) {
                AccessibilityNodeInfo c = n.getChild(i);
                if (c != null) pila.push(c);
            }
        }
        return out;
    }

    /** El botón que contiene a n (o n mismo). */
    private static AccessibilityNodeInfo clicable(AccessibilityNodeInfo n) {
        AccessibilityNodeInfo m = n;
        for (int i = 0; i < 4 && m != null && !m.isClickable(); i++) m = m.getParent();
        return m == null ? n : m;
    }

    private static Rect limites(AccessibilityNodeInfo n) {
        Rect b = new Rect();
        n.getBoundsInScreen(b);
        return b;
    }

    private static String minus(CharSequence s) {
        return s == null ? null : s.toString().trim().toLowerCase(Locale.ROOT);
    }

    private static boolean igual(CharSequence s, String objetivo) {
        String t = minus(s);
        return t != null && t.equals(objetivo);
    }

    private static boolean igualAlguno(CharSequence s, String[] objetivos) {
        String t = minus(s);
        if (t == null) return false;
        for (String x : objetivos) {
            if (t.equals(x) || t.startsWith(x + " ") || t.startsWith(x + ",")) return true;
        }
        return false;
    }

    /** Igual a alguno, sin más (una etiqueta, no un título que empiece así). */
    private static boolean exacto(CharSequence s, String[] objetivos) {
        for (String x : objetivos) {
            if (igual(s, x)) return true;
        }
        return false;
    }

    private static boolean contiene(CharSequence s, String[] trozos) {
        String t = minus(s);
        if (t == null) return false;
        for (String x : trozos) {
            if (t.contains(x)) return true;
        }
        return false;
    }

    /** Sin barra reconocible: abajo del todo (celular) o a la izquierda (tablet). */
    private static boolean enBorde(Rect b, Rect p) {
        boolean pequena = b.height() * 5 <= p.height() && b.width() * 2 <= p.width();
        boolean abajo = b.top >= p.top + p.height() * 0.75;
        boolean izquierda = b.right <= p.left + p.width() * 0.15;
        return pequena && (abajo || izquierda);
    }

    // ---- Las tapas (solo cosas que no se mueven: la pestaña y el minimizado) ----

    private void ponerTapas(List<Rect> tapas, Regla r) {
        WindowManager wm = (WindowManager) getSystemService(WINDOW_SERVICE);
        boolean oscuro = (getResources().getConfiguration().uiMode & Configuration.UI_MODE_NIGHT_MASK)
                == Configuration.UI_MODE_NIGHT_YES;
        int fondo = r == null ? 0 : (oscuro ? r.oscuro : r.claro);
        for (int i = 0; i < tapas.size(); i++) {
            Rect t = tapas.get(i);
            if (i < puestas.size()) {
                if (puestas.get(i).equals(t)) continue;
                try {
                    TextView v = vistas.get(i);
                    v.setBackgroundColor(fondo);
                    textoTapa(v, t, r);
                    wm.updateViewLayout(v, params(t, true));
                    puestas.set(i, t);
                } catch (RuntimeException ignored) {
                }
                continue;
            }
            try {
                TextView v = nuevaTapa(fondo);
                textoTapa(v, t, r);
                wm.addView(v, params(t, true));
                vistas.add(v);
                puestas.add(t);
            } catch (RuntimeException ignored) {
                return;
            }
        }
        while (puestas.size() > tapas.size()) {
            int last = puestas.size() - 1;
            try {
                wm.removeView(vistas.get(last));
            } catch (RuntimeException ignored) {
            }
            vistas.remove(last);
            puestas.remove(last);
        }
    }

    /** Las tapas grandes (el feed) dicen qué tapan; las pequeñas, solo el cerebro. */
    private void textoTapa(TextView v, Rect t, Regla r) {
        Rect p = pantallaActual;
        boolean grande = p != null && t.height() * 4 > p.height() && r != null && !r.aviso.isEmpty();
        v.setText(grande ? "\uD83E\uDDE0\n" + r.aviso : "\uD83E\uDDE0");
        v.setTextSize(grande ? 18 : 16);
        v.setTextColor(0xFF8A8F98);
    }

    // ---- Pantallas prohibidas (Estados de WhatsApp) ----

    private static final String[] REPRODUCTOR = {"playback", "viewer", "player"};

    /**
     * ¿Es una pantalla de Estados? Por su nombre exacto o, si WhatsApp usa otra
     * (desde el perfil, por ejemplo), por uno que junte «status» con
     * «playback», «viewer» o «player».
     */
    private static boolean esProhibida(Regla r, String clase) {
        if (r.actividades.length == 0) return false;
        for (String a : r.actividades) {
            if (clase.contains(a)) return true;
        }
        String c = clase.toLowerCase(Locale.ROOT);
        return c.contains("status") && contiene(c, REPRODUCTOR);
    }

    private long reproductorMirado;
    /** El bloqueo empezó por el reproductor dentro de otra pantalla (no por su nombre). */
    private boolean bloqueoPorDentro;

    /**
     * El reproductor de Estados metido dentro de otra pantalla (el perfil de un
     * contacto): se reconoce por los nombres internos de sus piezas.
     */
    private boolean reproductorDentro(AccessibilityNodeInfo root, Regla r, long ahora) {
        if (r.actividades.length == 0 || ahora - reproductorMirado < 120) return false;
        return hayReproductor(root, r, ahora);
    }

    private boolean hayReproductor(AccessibilityNodeInfo root, Regla r, long ahora) {
        if (root == null) return false;
        reproductorMirado = ahora;
        String pre = r.pkg + ":id/";
        for (AccessibilityNodeInfo n : recorrer(root)) {
            String vid = n.getViewIdResourceName();
            if (vid == null || !vid.startsWith(pre)) continue;
            String id = vid.substring(pre.length()).toLowerCase(Locale.ROOT);
            if (id.contains("status") && contiene(id, REPRODUCTOR)) return true;
            if (id.startsWith("playback") || id.contains("status_view_pager") || id.contains("statuses_view_pager")) return true;
        }
        return false;
    }

    private void empezarBloqueo(Regla r, long ahora) {
        if (bloqueando == null) {
            bloqueoDesde = ahora;
            insistencias = 0;
        }
        bloqueando = r;
        ponerTapaTotal(r);
        manejador.removeCallbacks(insistir);
        insistir.run();
    }

    /**
     * Atrás hasta que la pantalla se cierre: mientras se está abriendo,
     * WhatsApp ignora el primer Atrás. Si a la tercera sigue ahí, al inicio.
     */
    private final Runnable insistir = new Runnable() {
        @Override
        public void run() {
            Regla r = bloqueando;
            if (r == null) return;
            long ahora = SystemClock.uptimeMillis();
            if (ahora - bloqueoDesde > 8000) {
                terminarBloqueo();
                return;
            }
            if (bloqueoPorDentro && insistencias > 0) {
                // Cerrarlo no cambia de pantalla: se mira si sigue ahí antes de
                // otro Atrás (si no, se saldría también del perfil).
                if (!hayReproductor(getRootInActiveWindow(), r, ahora)) {
                    terminarBloqueo();
                    return;
                }
            }
            insistencias++;
            performGlobalAction(insistencias > 3 ? GLOBAL_ACTION_HOME : GLOBAL_ACTION_BACK);
            if (insistencias == 1 && ahora - ultimoAviso > 4000) {
                ultimoAviso = ahora;
                Toast.makeText(Vigia.this, "Cerrojo: " + r.nombre + " bloqueados", Toast.LENGTH_SHORT).show();
            }
            manejador.postDelayed(this, 350);
        }
    };

    private void terminarBloqueo() {
        bloqueando = null;
        bloqueoPorDentro = false;
        manejador.removeCallbacks(insistir);
        if (tapaTotal != null) {
            try {
                ((WindowManager) getSystemService(WINDOW_SERVICE)).removeView(tapaTotal);
            } catch (RuntimeException ignored) {
            }
            tapaTotal = null;
        }
    }

    /** Toda la pantalla tapada: no se llega a ver nada mientras se cierra. */
    private void ponerTapaTotal(Regla r) {
        if (tapaTotal != null) return;
        TextView v = nuevaTapa(fondo(r));
        v.setText("\uD83E\uDDE0\n" + (r.aviso.isEmpty() ? r.nombre + " bloqueados" : r.aviso));
        v.setTextSize(20);
        v.setTextColor(0xFF8A8F98);
        WindowManager.LayoutParams lp = new WindowManager.LayoutParams(
                WindowManager.LayoutParams.MATCH_PARENT, WindowManager.LayoutParams.MATCH_PARENT,
                WindowManager.LayoutParams.TYPE_ACCESSIBILITY_OVERLAY,
                WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE
                        | WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN
                        | WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS,
                PixelFormat.OPAQUE);
        if (Build.VERSION.SDK_INT >= 28) {
            lp.layoutInDisplayCutoutMode = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES;
        }
        try {
            ((WindowManager) getSystemService(WINDOW_SERVICE)).addView(v, lp);
            tapaTotal = v;
        } catch (RuntimeException ignored) {
        }
    }

    // ---- Página que se pasa deslizando (Novedades de WhatsApp) ----

    /**
     * La página entera que contiene uno de los títulos (aunque esté fuera de la
     * pantalla, esperando al lado): el ancestro que es una página del
     * deslizador (su padre se desplaza y ocupa casi todo el ancho).
     */
    private AccessibilityNodeInfo buscarPagina(AccessibilityNodeInfo root, Regla r, Rect pantalla, long ahora) {
        if (ahora - paginaBuscada < 100) return null;
        paginaBuscada = ahora;
        for (String t : r.pagina) {
            List<AccessibilityNodeInfo> l = root.findAccessibilityNodeInfosByText(t);
            if (l == null) continue;
            for (AccessibilityNodeInfo n : l) {
                if (!igual(n.getText(), t)) continue;
                AccessibilityNodeInfo m = n;
                for (int i = 0; i < 14 && m != null; i++) {
                    AccessibilityNodeInfo padre = m.getParent();
                    if (padre == null) break;
                    if (padre.isScrollable() && limites(padre).width() * 10 >= pantalla.width() * 9) {
                        // ¿Es una página (alta) y no una fila de una lista?
                        Rect b = limites(m);
                        boolean alta = m.getChildCount() > 0 && (b.isEmpty() || b.height() * 2 >= pantalla.height()
                                || limites(padre).height() * 2 >= pantalla.height());
                        if (alta && padre.getChildCount() > 1) return m;
                    }
                    m = padre;
                }
            }
        }
        return null;
    }

    /**
     * La tapa de la página sigue a la página mientras entra deslizando, un
     * poco por delante; si se queda en pantalla, se vuelve a la pestaña de chats.
     */
    private void mostrarPagina(AccessibilityNodeInfo root, Regla r) {
        long ahora = SystemClock.uptimeMillis();
        AccessibilityNodeInfo n = nodoPagina;
        Rect p = pantallaActual;
        Rect b = n == null || p == null ? null : limites(n);
        if (b == null || b.isEmpty() || !Rect.intersects(b, p)) {
            paginaAntes = null;
            ponerTapaPagina(null, r);
            return;
        }
        // Entra por la derecha (su borde izquierdo avanza) o por la izquierda
        // (avanza el derecho): la tapa se adelanta por ese borde.
        int dl = 0, dr = 0;
        float dt = 1;
        if (paginaAntes != null) {
            dl = b.left - paginaAntes.left;
            dr = b.right - paginaAntes.right;
            dt = Math.max(1, ahora - paginaAntesEn);
        }
        boolean moviendose = dl != 0 || dr != 0;
        paginaAntes = new Rect(b);
        paginaAntesEn = ahora;
        Rect c = new Rect(b);
        c.left += Math.min(0, Math.round(dl / dt * ADELANTO));
        c.right += Math.max(0, Math.round(dr / dt * ADELANTO));
        if (!c.intersect(p)) {
            ponerTapaPagina(null, r);
            return;
        }
        ponerTapaPagina(c, r);
        seguirHasta = Math.max(seguirHasta, ahora + 300);
        // Ya quieta y casi entera: de vuelta a Chats.
        if (!moviendose && c.width() * 10 >= p.width() * 8 && ahora - ultimoVolver > 700) {
            ultimoVolver = ahora;
            AccessibilityNodeInfo rt = root != null ? root : getRootInActiveWindow();
            if (rt != null) volver(rt, r, p);
        }
    }

    private static void volver(AccessibilityNodeInfo root, Regla r, Rect pantalla) {
        for (String t : r.volver) {
            List<AccessibilityNodeInfo> l = root.findAccessibilityNodeInfosByText(t);
            if (l == null) continue;
            for (AccessibilityNodeInfo n : l) {
                if (!n.isVisibleToUser()) continue;
                if (!igualAlguno(n.getText(), r.volver) && !igualAlguno(n.getContentDescription(), r.volver)) continue;
                AccessibilityNodeInfo b = clicable(n);
                if (enBorde(limites(b), pantalla) && b.performAction(AccessibilityNodeInfo.ACTION_CLICK)) return;
            }
        }
    }

    private void ponerTapaPagina(Rect b, Regla r) {
        WindowManager wm = (WindowManager) getSystemService(WINDOW_SERVICE);
        try {
            if (b == null) {
                if (tapaPagina != null) {
                    wm.removeView(tapaPagina);
                    tapaPagina = null;
                    tapaPaginaEn = null;
                }
                return;
            }
            if (tapaPagina == null) {
                TextView v = nuevaTapa(fondo(r));
                v.setText("\uD83E\uDDE0\n" + (r == null ? "" : r.aviso));
                v.setTextSize(18);
                v.setTextColor(0xFF8A8F98);
                wm.addView(v, params(b, true));
                tapaPagina = v;
            } else if (!b.equals(tapaPaginaEn)) {
                wm.updateViewLayout(tapaPagina, params(b, true));
            }
            tapaPaginaEn = new Rect(b);
        } catch (RuntimeException ignored) {
            tapaPagina = null;
            tapaPaginaEn = null;
        }
    }

    // ---- Feed de inicio (LinkedIn) ----
    //
    // Con «Inicio» elegida, una tapa fija sobre el feed (buscador, mensajes y
    // las demás pestañas quedan libres). Para que no haya ni un fotograma de
    // feed al pulsar «Inicio» desde otra pestaña, encima de ese botón hay una
    // capa transparente: el toque llega primero aquí, se pone la tapa y solo
    // entonces se le pasa el toque a LinkedIn.

    private static final String[] SELECCIONADA = {"seleccionad", "selected"};

    /** Una ventana de tapa suelta (el feed, o la capa del botón). */
    private final class Capa {
        private final boolean transparente;
        TextView v;
        Rect en;

        Capa(boolean transparente) {
            this.transparente = transparente;
        }

        void poner(Rect b, Regla r) {
            WindowManager wm = (WindowManager) getSystemService(WINDOW_SERVICE);
            try {
                if (v == null) {
                    v = nuevaTapa(transparente ? 0 : fondo(r));
                    if (transparente) {
                        v.setText("");
                        v.setOnClickListener(x -> pulsarInicio());
                    } else {
                        v.setText("\uD83E\uDDE0\n" + (r == null ? "" : r.aviso));
                        v.setTextSize(18);
                        v.setTextColor(0xFF8A8F98);
                    }
                    wm.addView(v, params(b, true));
                } else if (!b.equals(en)) {
                    wm.updateViewLayout(v, params(b, true));
                }
                en = new Rect(b);
            } catch (RuntimeException ignored) {
                quitar();
            }
        }

        void quitar() {
            if (v != null) {
                try {
                    ((WindowManager) getSystemService(WINDOW_SERVICE)).removeView(v);
                } catch (RuntimeException ignored) {
                }
            }
            v = null;
            en = null;
        }
    }

    private final Capa capaFeed = new Capa(false);
    private final Capa capaInicio = new Capa(true);
    /** El botón «Inicio» de abajo, y dónde quedó el feed la última vez. */
    private AccessibilityNodeInfo nodoInicio;
    private Rect feedUltimo;
    private long feedMedido;
    /** Tras pulsar «Inicio», hasta cuándo se da por hecho que se está en Inicio. */
    private long inicioHasta;
    private static final long MARGEN_INICIO = 800;
    /** El orden de los últimos toques: «Inicio» y cualquier otro botón. */
    private long clicInicio, clicOtro;

    /** El botón «Inicio» de la barra de abajo, o null. */
    private static AccessibilityNodeInfo botonInicio(AccessibilityNodeInfo root, Regla r, Rect pantalla) {
        for (String t : r.inicio) {
            List<AccessibilityNodeInfo> l = root.findAccessibilityNodeInfosByText(t);
            if (l == null) continue;
            for (AccessibilityNodeInfo n : l) {
                if (!n.isVisibleToUser()) continue;
                if (!igualAlguno(n.getText(), r.inicio) && !igualAlguno(n.getContentDescription(), r.inicio)) continue;
                AccessibilityNodeInfo b = clicable(n);
                if (enBorde(limites(b), pantalla)) return b;
            }
        }
        return null;
    }

    private static boolean elegida(AccessibilityNodeInfo b) {
        if (b.isSelected() || contiene(b.getContentDescription(), SELECCIONADA)) return true;
        for (int i = 0; i < b.getChildCount(); i++) {
            AccessibilityNodeInfo c = b.getChild(i);
            if (c != null && (c.isSelected() || contiene(c.getContentDescription(), SELECCIONADA))) return true;
        }
        return false;
    }

    /** Dónde tapar el feed: lo último medido o, sin medida, de arriba a la barra. */
    private Rect rectFeed(Rect inicio, Rect pantalla) {
        if (feedUltimo != null) {
            Rect f = new Rect(feedUltimo);
            f.bottom = Math.min(f.bottom, inicio.top);
            if (f.height() > pantalla.height() / 5) return f;
        }
        return new Rect(pantalla.left, pantalla.top + pantalla.height() * 12 / 100, pantalla.right, inicio.top);
    }

    /** La lista más grande por encima de la barra (el feed), para ajustar la tapa. */
    private void medirFeed(AccessibilityNodeInfo root, Rect inicio, Rect pantalla) {
        Rect lista = null;
        for (AccessibilityNodeInfo n : recorrer(root)) {
            if (!n.isScrollable()) continue;
            Rect b = limites(n);
            if (b.width() * 2 < pantalla.width() || b.top >= inicio.top) continue;
            if (lista == null || (long) b.width() * b.height() > (long) lista.width() * lista.height()) lista = b;
        }
        if (lista != null && Math.min(lista.bottom, inicio.top) - lista.top > pantalla.height() / 5) {
            feedUltimo = new Rect(pantalla.left, lista.top, pantalla.right, Math.min(lista.bottom, inicio.top));
        }
    }

    /**
     * Pone o quita la tapa del feed según la pestaña elegida. Barato (una
     * búsqueda por texto): se puede llamar en cada fotograma.
     */
    private void revisarInicio(AccessibilityNodeInfo root, Regla r, Rect pantalla, boolean medir) {
        AccessibilityNodeInfo b = root == null ? null : botonInicio(root, r, pantalla);
        nodoInicio = b;
        if (b == null) {
            capaFeed.quitar();
            capaInicio.quitar();
            return;
        }
        Rect tb = limites(b);
        // Ante la duda, tapado: solo se destapa si LinkedIn dice que hay OTRA
        // pestaña elegida (con toques rápidos, la marca va por detrás).
        boolean otra = false;
        AccessibilityNodeInfo nav = b.getParent();
        if (nav != null) {
            for (int i = 0; i < nav.getChildCount(); i++) {
                AccessibilityNodeInfo c = nav.getChild(i);
                if (c != null && !c.equals(b) && elegida(c)) otra = true;
            }
        }
        // Si lo último que se tocó fue otro botón (después de «Inicio»), también
        // vale como confirmación, por si LinkedIn no marca las demás pestañas.
        boolean salio = otra || clicOtro > clicInicio;
        boolean tapar = elegida(b) || SystemClock.uptimeMillis() < inicioHasta || (!salio && capaFeed.v != null);
        if (tapar) {
            capaInicio.quitar();
            long ahora = SystemClock.uptimeMillis();
            if (medir && (feedUltimo == null || ahora - feedMedido > 1500)) {
                feedMedido = ahora;
                medirFeed(root, tb, pantalla);
            }
            capaFeed.poner(rectFeed(tb, pantalla), r);
        } else {
            capaFeed.quitar();
            capaInicio.poner(tb, r);
        }
    }

    /** ¿El aviso es del botón «Inicio»? (por su texto o descripción) */
    private static boolean esInicio(AccessibilityEvent e, Regla r) {
        if (igualAlguno(e.getContentDescription(), r.inicio)) return true;
        for (CharSequence t : e.getText()) {
            if (igualAlguno(t, r.inicio)) return true;
        }
        return false;
    }

    /** Tocaste «Inicio»: la tapa ya, y después el toque a LinkedIn. */
    private void pulsarInicio() {
        Regla r = regla;
        Rect p = pantallaActual;
        AccessibilityNodeInfo b = nodoInicio;
        if (r == null || p == null || b == null) return;
        Rect tb = capaInicio.en != null ? capaInicio.en : limites(b);
        inicioHasta = SystemClock.uptimeMillis() + MARGEN_INICIO;
        clicInicio = SystemClock.uptimeMillis();
        capaFeed.poner(rectFeed(tb, p), r);
        capaInicio.quitar();
        b.performAction(AccessibilityNodeInfo.ACTION_CLICK);
        seguir(SystemClock.uptimeMillis());
    }

    private TextView nuevaTapa(int fondo) {
        TextView v = new TextView(this);
        v.setGravity(Gravity.CENTER);
        v.setText("\uD83E\uDDE0"); // cerebro
        v.setTextSize(16);
        // Se come los toques: lo de debajo no se puede pulsar.
        v.setClickable(true);
        v.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO);
        v.setBackgroundColor(fondo);
        return v;
    }

    private int fondo(Regla r) {
        if (r == null) return 0;
        boolean oscuro = (getResources().getConfiguration().uiMode & Configuration.UI_MODE_NIGHT_MASK)
                == Configuration.UI_MODE_NIGHT_YES;
        return oscuro ? r.oscuro : r.claro;
    }

    /**
     * La tapa de la pestaña (null: esconderla). Esconderla es al instante
     * (la vista se vuelve invisible en el siguiente fotograma) y además deja de
     * comerse los toques, para no tapar el feed cuando la barra ya no está.
     */
    private void ponerTapaPestana(Rect b) {
        WindowManager wm = (WindowManager) getSystemService(WINDOW_SERVICE);
        if (b == null) {
            if (tapaPestana != null && tapaPestanaVisible) {
                tapaPestana.setVisibility(View.INVISIBLE);
                tapaPestanaVisible = false;
                try {
                    wm.updateViewLayout(tapaPestana, params(tapaPestanaEn, false));
                } catch (RuntimeException ignored) {
                }
            }
            return;
        }
        try {
            if (tapaPestana == null) {
                TextView v = nuevaTapa(fondo(regla));
                // También avisa de los toques (por si el vigilante no está).
                v.setOnTouchListener((vista, ev) -> {
                    if (ev.getActionMasked() == MotionEvent.ACTION_OUTSIDE) alTocar();
                    return false;
                });
                wm.addView(v, params(b, true));
                tapaPestana = v;
            } else if (!tapaPestanaVisible || !b.equals(tapaPestanaEn)) {
                tapaPestana.setBackgroundColor(fondo(regla));
                wm.updateViewLayout(tapaPestana, params(b, true));
                tapaPestana.setVisibility(View.VISIBLE);
            }
            tapaPestanaEn = new Rect(b);
            tapaPestanaVisible = true;
        } catch (RuntimeException ignored) {
            quitarTapaPestana();
        }
    }

    private void quitarTapaPestana() {
        if (tapaPestana != null) {
            try {
                ((WindowManager) getSystemService(WINDOW_SERVICE)).removeView(tapaPestana);
            } catch (RuntimeException ignored) {
            }
        }
        tapaPestana = null;
        tapaPestanaEn = null;
        tapaPestanaVisible = false;
        pestanaAntes = null;
        reposo = null;
    }

    /** tocable: se come los toques y avisa de los de fuera; si no, deja pasar el dedo. */
    private static WindowManager.LayoutParams params(Rect t, boolean tocable) {
        int flags = WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE
                | WindowManager.LayoutParams.FLAG_NOT_TOUCH_MODAL
                | WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN
                | WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS;
        flags |= tocable ? WindowManager.LayoutParams.FLAG_WATCH_OUTSIDE_TOUCH
                : WindowManager.LayoutParams.FLAG_NOT_TOUCHABLE;
        WindowManager.LayoutParams lp = new WindowManager.LayoutParams(
                t.width(), t.height(),
                WindowManager.LayoutParams.TYPE_ACCESSIBILITY_OVERLAY, flags,
                PixelFormat.TRANSLUCENT);
        lp.gravity = Gravity.TOP | Gravity.START;
        lp.x = t.left;
        lp.y = t.top;
        if (Build.VERSION.SDK_INT >= 28) {
            lp.layoutInDisplayCutoutMode = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES;
        }
        return lp;
    }

    private void quitarTapas() {
        nodoPestana = null;
        regla = null;
        fijas = new ArrayList<>();
        Choreographer.getInstance().removeFrameCallback(cuadro);
        siguiendo = false;
        ponerTapas(new ArrayList<>(), null);
        quitarTapaPestana();
        quitarVigilante();
        capaFeed.quitar();
        capaInicio.quitar();
        nodoInicio = null;
        terminarBloqueo();
        nodoPagina = null;
        paginaAntes = null;
        ponerTapaPagina(null, null);
    }
}

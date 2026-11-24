package dev.cerrojo.filtro;

import android.accessibilityservice.AccessibilityService;
import android.accessibilityservice.AccessibilityServiceInfo;
import android.os.Handler;
import android.os.Looper;
import android.os.SystemClock;
import android.view.accessibility.AccessibilityEvent;
import android.view.accessibility.AccessibilityNodeInfo;

import java.util.List;
import java.util.Map;

/**
 * Te saca del video vertical (Shorts, Reels…) sin bloquear la app entera:
 * si en la pantalla se ve una de las vistas que lo delatan, pulsa "atrás";
 * si sigue ahí (entraste por un enlace), "inicio".
 * Qué vistas son lo dice el PC (Guardia.reglas, del catálogo de Cerrojo).
 */
public class Vertical extends AccessibilityService {
    private static Vertical activo;

    private final Handler h = new Handler(Looper.getMainLooper());
    private volatile Map<String, String[]> reglas;
    private String pendiente;
    private long ultimaSalida;

    static void recargar() {
        Vertical v = activo;
        if (v != null) v.h.post(v::configurar);
    }

    @Override
    protected void onServiceConnected() {
        activo = this;
        configurar();
    }

    @Override
    public void onDestroy() {
        if (activo == this) activo = null;
        super.onDestroy();
    }

    private void configurar() {
        reglas = Guardia.reglas(this);
        AccessibilityServiceInfo info = getServiceInfo();
        if (info == null) return;
        // Sin paquetes, Android mandaría los eventos de todas las apps.
        info.packageNames = reglas.isEmpty() ? new String[] {"dev.cerrojo.ninguno"} : reglas.keySet().toArray(new String[0]);
        setServiceInfo(info);
    }

    @Override
    public void onAccessibilityEvent(AccessibilityEvent e) {
        CharSequence p = e.getPackageName();
        if (p == null || !reglas.containsKey(p.toString())) return;
        // Llegan muchos eventos seguidos: se mira una vez cuando se calman.
        String pkg = p.toString();
        if (pkg.equals(pendiente)) return;
        pendiente = pkg;
        h.postDelayed(() -> {
            pendiente = null;
            mirar(pkg, false);
        }, 150);
    }

    private void mirar(String pkg, boolean segunda) {
        if (!enVertical(pkg)) return;
        long ahora = SystemClock.elapsedRealtime();
        if (!segunda && ahora - ultimaSalida < 400) return;
        ultimaSalida = ahora;
        performGlobalAction(segunda ? GLOBAL_ACTION_HOME : GLOBAL_ACTION_BACK);
        if (!segunda) h.postDelayed(() -> mirar(pkg, true), 700);
    }

    private boolean enVertical(String pkg) {
        String[] ids = reglas.get(pkg);
        if (ids == null || Guardia.abierto(this, pkg)) return false;
        AccessibilityNodeInfo raiz = getRootInActiveWindow();
        if (raiz == null || raiz.getPackageName() == null || !pkg.contentEquals(raiz.getPackageName())) return false;
        for (String id : ids) {
            List<AccessibilityNodeInfo> l = raiz.findAccessibilityNodeInfosByViewId(id.contains(":") ? id : pkg + ":id/" + id);
            for (AccessibilityNodeInfo n : l) {
                if (n.isVisibleToUser()) return true;
            }
        }
        return false;
    }

    @Override
    public void onInterrupt() {
    }
}

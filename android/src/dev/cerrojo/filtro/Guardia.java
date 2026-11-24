package dev.cerrojo.filtro;

import android.app.AlarmManager;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.Service;
import android.app.admin.DevicePolicyManager;
import android.content.BroadcastReceiver;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.SharedPreferences;
import android.database.ContentObserver;
import android.os.Build;
import android.os.Handler;
import android.os.IBinder;
import android.os.Looper;
import android.os.SystemClock;
import android.os.UserManager;
import android.provider.Settings;

import java.io.FileDescriptor;
import java.io.PrintWriter;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;

/**
 * El guardián: corre siempre en primer plano mientras haya algo que vigilar.
 * - Si apagas el servicio de "sin vertical" en Ajustes, lo vuelve a encender
 *   (permiso WRITE_SECURE_SETTINGS, que da el PC por adb).
 * - Con protección máxima (propietario del equipo): suspende las apps
 *   bloqueadas en cuanto aparecen, no deja desinstalarlas ni desinstalar el
 *   filtro, ni tocar la VPN, y cierra solo los desbloqueos temporales.
 *
 * Estado para el PC: adb shell dumpsys activity service dev.cerrojo.filtro/.Guardia
 */
public class Guardia extends Service {
    private static final String PREFS = "guardia";
    private static final String REVISAR = "dev.cerrojo.filtro.REVISAR";
    private static final String[] RESTRICCIONES = {
        UserManager.DISALLOW_CONFIG_VPN, "no_config_private_dns", UserManager.DISALLOW_SAFE_BOOT,
    };

    private final Handler h = new Handler(Looper.getMainLooper());
    private ContentObserver observador;
    private BroadcastReceiver receptor;

    // ---- Estado guardado ----

    static SharedPreferences prefs(Context c) {
        return c.getSharedPreferences(PREFS, MODE_PRIVATE);
    }

    /** "pkg:id1,id2;pkg2:id3" → paquete → vistas. */
    static Map<String, String[]> reglas(Context c) {
        Map<String, String[]> m = new HashMap<>();
        for (String r : prefs(c).getString("vertical", "").split(";")) {
            int i = r.indexOf(':');
            if (i <= 0) continue;
            String[] ids = r.substring(i + 1).split(",");
            if (ids.length > 0 && !ids[0].isEmpty()) m.put(r.substring(0, i).trim(), ids);
        }
        return m;
    }

    static Set<String> bloqueadas(Context c) {
        Set<String> s = new HashSet<>();
        for (String p : prefs(c).getString("bloquear", "").split("\\s+")) if (!p.isEmpty()) s.add(p);
        return s;
    }

    /** Nueva lista de bloqueadas: las que salen de ella se liberan (con protección máxima). */
    static void cambiarBloqueadas(Context c, String lista) {
        Set<String> antes = bloqueadas(c);
        prefs(c).edit().putString("bloquear", lista == null ? "" : lista.trim()).commit();
        antes.removeAll(bloqueadas(c));
        if (antes.isEmpty() || !dueno(c)) return;
        DevicePolicyManager dpm = c.getSystemService(DevicePolicyManager.class);
        ComponentName a = new ComponentName(c, Admin.class);
        for (String p : antes) {
            try {
                dpm.setUninstallBlocked(a, p, false);
            } catch (RuntimeException ignored) {
            }
        }
        try {
            dpm.setPackagesSuspended(a, antes.toArray(new String[0]), false);
        } catch (RuntimeException ignored) {
        }
    }

    private static int arranque(Context c) {
        return Settings.Global.getInt(c.getContentResolver(), Settings.Global.BOOT_COUNT, 0);
    }

    /** Milisegundos que le quedan al desbloqueo temporal de pkg (0 si no hay). */
    static long queda(Context c, String pkg) {
        SharedPreferences p = prefs(c);
        // Un reinicio cierra todo: el reloj monótono vuelve a cero.
        if (p.getInt("abierto_arranque", -1) != arranque(c)) return 0;
        long hasta = p.getLong("abierto_" + pkg, 0);
        return Math.max(0, hasta - SystemClock.elapsedRealtime());
    }

    static boolean abierto(Context c, String pkg) {
        return queda(c, pkg) > 0;
    }

    static void abrir(Context c, String[] pkgs, int minutos) {
        SharedPreferences p = prefs(c);
        SharedPreferences.Editor e = p.edit();
        if (p.getInt("abierto_arranque", -1) != arranque(c)) {
            for (String k : p.getAll().keySet()) if (k.startsWith("abierto_")) e.remove(k);
            e.putInt("abierto_arranque", arranque(c));
        }
        long hasta = SystemClock.elapsedRealtime() + minutos * 60_000L;
        for (String pkg : pkgs) if (!pkg.isEmpty()) e.putLong("abierto_" + pkg, hasta);
        e.commit();
    }

    static void cerrarTodo(Context c) {
        SharedPreferences.Editor e = prefs(c).edit();
        for (String k : prefs(c).getAll().keySet()) if (k.startsWith("abierto_") && !k.equals("abierto_arranque")) e.remove(k);
        e.commit();
    }

    static boolean dueno(Context c) {
        DevicePolicyManager dpm = c.getSystemService(DevicePolicyManager.class);
        return dpm != null && dpm.isDeviceOwnerApp(c.getPackageName());
    }

    static boolean hayTrabajo(Context c) {
        return dueno(c) || !reglas(c).isEmpty();
    }

    static void encender(Context c) {
        Intent i = new Intent(c, Guardia.class);
        try {
            if (Build.VERSION.SDK_INT >= 26) c.startForegroundService(i);
            else c.startService(i);
        } catch (RuntimeException ignored) {
            // Sin permiso para arrancar en segundo plano: ya lo hará el PC.
        }
    }

    // ---- Servicio ----

    @Override
    public void onCreate() {
        super.onCreate();
        observador = new ContentObserver(h) {
            @Override
            public void onChange(boolean self) {
                asegurarVertical();
            }
        };
        getContentResolver().registerContentObserver(Settings.Secure.getUriFor(Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES), false, observador);
        getContentResolver().registerContentObserver(Settings.Secure.getUriFor(Settings.Secure.ACCESSIBILITY_ENABLED), false, observador);
        receptor = new BroadcastReceiver() {
            @Override
            public void onReceive(Context c, Intent i) {
                aplicar();
            }
        };
        IntentFilter f = new IntentFilter();
        f.addAction(Intent.ACTION_PACKAGE_ADDED);
        f.addAction(Intent.ACTION_PACKAGE_REPLACED);
        f.addAction(Intent.ACTION_PACKAGE_CHANGED);
        f.addDataScheme("package");
        registerReceiver(receptor, f);
        registerReceiver(receptor, new IntentFilter(Intent.ACTION_SCREEN_ON));
    }

    @Override
    public void onDestroy() {
        getContentResolver().unregisterContentObserver(observador);
        unregisterReceiver(receptor);
        super.onDestroy();
    }

    @Override
    public int onStartCommand(Intent in, int flags, int id) {
        primerPlano();
        if (!hayTrabajo(this)) {
            stopForeground(true);
            stopSelf();
            return START_NOT_STICKY;
        }
        asegurarVertical();
        aplicar();
        return START_STICKY;
    }

    @Override
    public IBinder onBind(Intent i) {
        return null;
    }

    private void primerPlano() {
        if (Build.VERSION.SDK_INT < 26) return;
        NotificationManager nm = getSystemService(NotificationManager.class);
        nm.createNotificationChannel(new NotificationChannel("guardia", "Cerrojo", NotificationManager.IMPORTANCE_MIN));
        Notification n = new Notification.Builder(this, "guardia")
                .setSmallIcon(android.R.drawable.ic_lock_lock)
                .setContentTitle("Cerrojo")
                .setOngoing(true)
                .build();
        startForeground(2, n);
    }

    /** Que el servicio de accesibilidad siga encendido si hay reglas. */
    private void asegurarVertical() {
        if (reglas(this).isEmpty()) return;
        String yo = new ComponentName(this, Vertical.class).flattenToString();
        try {
            String l = Settings.Secure.getString(getContentResolver(), Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES);
            boolean esta = false;
            if (l != null) for (String s : l.split(":")) esta |= s.equalsIgnoreCase(yo);
            if (!esta) {
                Settings.Secure.putString(getContentResolver(), Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES,
                        l == null || l.isEmpty() ? yo : l + ":" + yo);
            }
            if (Settings.Secure.getInt(getContentResolver(), Settings.Secure.ACCESSIBILITY_ENABLED, 0) != 1) {
                Settings.Secure.putInt(getContentResolver(), Settings.Secure.ACCESSIBILITY_ENABLED, 1);
            }
        } catch (SecurityException ignored) {
            // Falta el permiso: el PC lo vuelve a dar al conectarse.
        }
    }

    /** Con protección máxima: deja las apps como deben estar ahora mismo. */
    private void aplicar() {
        if (!dueno(this)) return;
        DevicePolicyManager dpm = getSystemService(DevicePolicyManager.class);
        ComponentName a = new ComponentName(this, Admin.class);
        try {
            dpm.setUninstallBlocked(a, getPackageName(), true);
            for (String r : RESTRICCIONES) dpm.addUserRestriction(a, r);
        } catch (RuntimeException ignored) {
        }
        List<String> cerrar = new ArrayList<>(), abrir = new ArrayList<>();
        long proxima = Long.MAX_VALUE;
        for (String p : bloqueadas(this)) {
            long q = queda(this, p);
            if (q > 0) {
                abrir.add(p);
                proxima = Math.min(proxima, q);
            } else {
                cerrar.add(p);
            }
            try {
                dpm.setUninstallBlocked(a, p, true);
            } catch (RuntimeException ignored) {
                // No instalada todavía: se hará al aparecer (PACKAGE_ADDED).
            }
        }
        try {
            if (!cerrar.isEmpty()) dpm.setPackagesSuspended(a, cerrar.toArray(new String[0]), true);
            if (!abrir.isEmpty()) dpm.setPackagesSuspended(a, abrir.toArray(new String[0]), false);
        } catch (RuntimeException ignored) {
        }
        for (String p : reglas(this).keySet()) {
            long q = queda(this, p);
            if (q > 0) proxima = Math.min(proxima, q);
        }
        programar(proxima);
    }

    /** Despertar cuando acabe el próximo desbloqueo, y cada 15 min por si acaso. */
    private void programar(long dentro) {
        AlarmManager am = getSystemService(AlarmManager.class);
        PendingIntent pi = PendingIntent.getService(this, 0, new Intent(this, Guardia.class).setAction(REVISAR),
                PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
        long cuando = SystemClock.elapsedRealtime() + Math.min(dentro + 1000, 15 * 60_000L);
        try {
            if (Build.VERSION.SDK_INT >= 31 && !am.canScheduleExactAlarms()) {
                am.setAndAllowWhileIdle(AlarmManager.ELAPSED_REALTIME_WAKEUP, cuando, pi);
            } else {
                am.setExactAndAllowWhileIdle(AlarmManager.ELAPSED_REALTIME_WAKEUP, cuando, pi);
            }
        } catch (RuntimeException ignored) {
        }
        h.removeCallbacksAndMessages(null);
        h.postDelayed(this::aplicar, Math.min(dentro + 1000, 15 * 60_000L));
    }

    /** Quitar la protección máxima (lo pide el PC: Orden --ez soltar true). */
    static void soltar(Context c) {
        if (!dueno(c)) return;
        DevicePolicyManager dpm = c.getSystemService(DevicePolicyManager.class);
        ComponentName a = new ComponentName(c, Admin.class);
        try {
            Set<String> b = bloqueadas(c);
            for (String p : b) {
                try {
                    dpm.setUninstallBlocked(a, p, false);
                } catch (RuntimeException ignored) {
                }
            }
            if (!b.isEmpty()) dpm.setPackagesSuspended(a, b.toArray(new String[0]), false);
            dpm.setUninstallBlocked(a, c.getPackageName(), false);
            for (String r : RESTRICCIONES) dpm.clearUserRestriction(a, r);
        } catch (RuntimeException ignored) {
        }
        dpm.clearDeviceOwnerApp(c.getPackageName());
    }

    @Override
    protected void dump(FileDescriptor fd, PrintWriter w, String[] args) {
        w.println("dueno=" + dueno(this));
        w.println("vertical=" + prefs(this).getString("vertical", ""));
        w.println("bloquear=" + prefs(this).getString("bloquear", ""));
        Set<String> todos = new HashSet<>(bloqueadas(this));
        todos.addAll(reglas(this).keySet());
        for (String p : todos) {
            long q = queda(this, p);
            if (q > 0) w.println("abierto " + p + " " + (q / 1000));
        }
    }
}

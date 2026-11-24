package dev.cerrojo.filtro;

import android.app.Activity;
import android.content.Intent;
import android.os.Bundle;

/**
 * Órdenes del PC (adb shell am start -n dev.cerrojo.filtro/.Orden ...):
 *   --es dominios "a.com b.com"      guarda la lista y enciende el filtro web
 *   --ez apagar true                 apaga el filtro web
 *   --es vigia "youtube instagram"   qué apps vigila el vigía ("" = ninguna)
 *   --es bloquear "pkg1 pkg2"        apps que la protección máxima mantiene suspendidas
 *   --es abrir "pkg1 pkg2" --ei minutos N   desbloqueo temporal
 *   --ez cerrar true                 acaba ya los desbloqueos temporales
 *   --ez soltar true                 quita la protección máxima
 * Solo el shell puede abrirla (pide el permiso DUMP).
 */
public class Orden extends Activity {
    @Override
    protected void onCreate(Bundle b) {
        super.onCreate(b);
        Intent in = getIntent();
        if (in.getBooleanExtra("apagar", false)) {
            Filtro.guardar(this, "", false);
            startService(new Intent(this, Filtro.class).setAction(Filtro.APAGAR));
        } else if (in.hasExtra("dominios")) {
            Filtro.guardar(this, in.getStringExtra("dominios"), true);
            Filtro.encender(this);
        }
        boolean guardia = false;
        if (in.hasExtra("vigia")) {
            Vigia.guardar(this, in.getStringExtra("vigia"));
            guardia = true;
        }
        if (in.hasExtra("bloquear")) {
            Guardia.cambiarBloqueadas(this, in.getStringExtra("bloquear"));
            guardia = true;
        }
        if (in.getBooleanExtra("cerrar", false)) {
            Guardia.cerrarTodo(this);
            guardia = true;
        }
        String abrir = in.getStringExtra("abrir");
        if (abrir != null) {
            Guardia.abrir(this, abrir.trim().split("\\s+"), in.getIntExtra("minutos", 15));
            guardia = true;
        }
        if (in.getBooleanExtra("soltar", false)) {
            Guardia.soltar(this);
            guardia = true;
        }
        if (guardia) Guardia.encender(this);
        finish();
    }
}

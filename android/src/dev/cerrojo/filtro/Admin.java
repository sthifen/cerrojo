package dev.cerrojo.filtro;

import android.app.admin.DeviceAdminReceiver;
import android.content.Context;
import android.content.Intent;

/**
 * Lo que hace del filtro el "propietario del equipo" (protección máxima).
 * Se pone solo por adb: dpm set-device-owner dev.cerrojo.filtro/.Admin
 */
public class Admin extends DeviceAdminReceiver {
    @Override
    public void onEnabled(Context c, Intent i) {
        Guardia.encender(c);
    }
}

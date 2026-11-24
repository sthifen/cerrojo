package dev.cerrojo.filtro;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;

/** Al encender el celular (o al actualizar el filtro) vuelve a ponerlo todo. */
public class Arranque extends BroadcastReceiver {
    @Override
    public void onReceive(Context c, Intent i) {
        if (Filtro.activo(c)) Filtro.encender(c);
        if (Guardia.hayTrabajo(c)) Guardia.encender(c);
    }
}

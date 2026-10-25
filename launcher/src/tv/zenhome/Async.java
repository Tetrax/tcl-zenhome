package tv.zenhome;

import android.os.Handler;

import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/**
 * Un seul thread d'arriere-plan ; les resultats reviennent par `ui` (`h` est vide dans onPause,
 * ce qui perdrait les resultats en attente). Rien n'est lance hors premier plan.
 */
class Async {
    static final long RETRY_MS = 5 * 60_000L;   // delai avant de retenter une requete en echec / identique

    final Handler h = new Handler();                  // ticks, vide dans onPause
    final Handler ui = new Handler(android.os.Looper.getMainLooper());   // resultats des taches d'arriere-plan
    ExecutorService io;
    volatile boolean fg = false, destroyed = false;   // premier plan / activite detruite

    void start() { io = Executors.newSingleThreadExecutor(); }

    void bg(Runnable r) {
        if (destroyed || io == null) return;
        try { io.execute(r); } catch (Throwable ignored) { }
    }

    void onPause() {
        fg = false;
        h.removeCallbacksAndMessages(null); // rien ne tourne pendant qu'une appli est ouverte
    }

    void onDestroy() {
        destroyed = true; fg = false;
        if (io != null) io.shutdownNow();
        ui.removeCallbacksAndMessages(null);
    }
}

package tv.zenhome;

import android.accessibilityservice.AccessibilityService;
import android.content.Intent;
import android.os.Build;
import android.view.KeyEvent;
import android.view.accessibility.AccessibilityEvent;

/**
 * Boutons d'applis de la telecommande (Netflix, YouTube, Prime Video) sur les TV TCL.
 * Ces touches envoient des codes propres a TCL, que seul l'accueil Google TV traitait :
 * une fois cet accueil desactive par le kit, elles ne faisaient plus rien.
 * Le kit n'active ce service que dans ce cas. Seules ces touches sont interceptees (et Source / TV quand l'accueil est
 * au premier plan, et Reglages, voir KeyRules), toutes les autres passent sans modification.
 */
public class KeyService extends AccessibilityService {

    /** Instance connectee (meme processus que l'accueil) ; null si le service n'est pas actif. */
    static volatile KeyService active;

    /**
     * Met la TV en veille (minuteur). Seul point a changer pour une autre methode. Ici : GLOBAL_ACTION_LOCK_SCREEN (API 28+),
     * sans shell. Renvoie faux si le service n'est pas actif ou si l'action est refusee.
     */
    static boolean sleepNow() {
        KeyService s = active;
        if (s == null || Build.VERSION.SDK_INT < 28) return false;
        try { return s.performGlobalAction(GLOBAL_ACTION_LOCK_SCREEN); } catch (Throwable e) { return false; }
    }

    @Override protected void onServiceConnected() { active = this; }
    @Override public boolean onUnbind(Intent i) { if (active == this) active = null; return super.onUnbind(i); }
    @Override public void onDestroy() { if (active == this) active = null; super.onDestroy(); }

    /** Codes de touche TCL -> appli a ouvrir. */
    static final int[] TCL_KEYS = {4062, 4065, 4099};
    static final String[] TCL_APPS = {"com.netflix.ninja", "com.google.android.youtube.tv", "com.amazon.amazonvideo.livingroom"};

    static boolean isTcl() {
        String b = (Build.BRAND + " " + Build.MANUFACTURER).toUpperCase(java.util.Locale.ROOT);
        return b.contains("TCL");
    }

    /** Appli associee a ce code de touche sur cette TV, ou null. */
    static String appForKey(int code) {
        if (!isTcl()) return null;
        for (int i = 0; i < TCL_KEYS.length; i++) if (TCL_KEYS[i] == code) return TCL_APPS[i];
        return null;
    }

    boolean settingsHeld;   // la touche Reglages a ete prise sur son appui : on prend aussi sa repetition et son relachement

    @Override
    protected boolean onKeyEvent(KeyEvent e) {
        // Reglages : sans l'accueil Google TV, la touche ne fait plus rien. Menu Reglages de Zen Home s'il est au premier plan,
        // sinon panneau rapide Image de TCL, partout ; si rien n'est possible la touche suit son chemin
        if (KeyRules.isSettingsKey(e.getKeyCode())) {
            if (e.getAction() == KeyEvent.ACTION_DOWN && e.getRepeatCount() == 0) {
                final Home hm = Home.front;
                int a = KeyRules.settingsAction(e.getKeyCode(), hm != null, Apps.tclPanelAvailable(this));
                if (a == KeyRules.HOME_MENU) { hm.runOnUiThread(new Runnable() { public void run() { hm.onSettingsKey(); } }); settingsHeld = true; }
                else settingsHeld = a == KeyRules.TCL_PANEL && Apps.startTclPanel(this, "picture");
            }
            boolean held = settingsHeld;
            if (e.getAction() == KeyEvent.ACTION_UP) settingsHeld = false;
            return held;
        }
        // Source / TV : le panneau des entrees de l'accueil, mais seulement quand l'accueil est au premier plan
        final Home h = Home.front;
        if (KeyRules.shouldHandleSource(e.getKeyCode(), h != null)) {
            if (e.getAction() == KeyEvent.ACTION_DOWN && e.getRepeatCount() == 0)
                h.runOnUiThread(new Runnable() { public void run() { h.onSourceKey(); } });
            return true;   // DOWN et UP consommes
        }
        String pkg = appForKey(e.getKeyCode());
        if (pkg == null) return false;
        Intent i = Apps.launchIntent(getPackageManager(), pkg);
        if (i == null) return false;                       // appli absente : la touche suit son chemin normal
        if (e.getAction() == KeyEvent.ACTION_UP) {
            i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            try { startActivity(i); } catch (Exception ignored) { }
        }
        return true;
    }

    @Override public void onAccessibilityEvent(AccessibilityEvent event) { }
    @Override public void onInterrupt() { }
}

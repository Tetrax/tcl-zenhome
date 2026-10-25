package tv.zenhome;

/** Decisions sur les touches interceptees par KeyService (sans android.*). */
final class KeyRules {
    private KeyRules() { }

    static final int KEYCODE_TV = 170, KEYCODE_TV_INPUT = 178;   // KeyEvent.KEYCODE_*
    static final int[] SOURCE_KEYS = {KEYCODE_TV_INPUT, KEYCODE_TV};

    /**
     * Source / TV : on la prend (et on ouvre le panneau des entrees) seulement quand l'accueil Zen Home est au premier plan ;
     * dans les autres applis la touche suit son chemin normal (menu TCL).
     */
    static boolean shouldHandleSource(int code, boolean homeResumed) {
        if (!homeResumed) return false;
        for (int k : SOURCE_KEYS) if (k == code) return true;
        return false;
    }
}

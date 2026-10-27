package tv.zenhome;

/** Decisions sur les touches interceptees par KeyService (sans android.*). */
final class KeyRules {
    private KeyRules() { }

    static final int KEYCODE_TV = 170, KEYCODE_TV_INPUT = 178;   // KeyEvent.KEYCODE_*
    static final int[] SOURCE_KEYS = {KEYCODE_TV_INPUT, KEYCODE_TV};
    static final int KEYCODE_SETTINGS = 176;                       // KeyEvent.KEYCODE_SETTINGS
    static final int KEYCODE_NOTIFICATION = 83;                    // touche engrenage des telecommandes TCL
    static final int NONE = 0, HOME_MENU = 1, TCL_PANEL = 2;       // suite donnee a la touche Reglages

    static boolean isSettingsKey(int code) { return code == KEYCODE_SETTINGS || code == KEYCODE_NOTIFICATION; }

    /**
     * Touche Reglages de la telecommande (qui ne fait plus rien sans l'accueil Google TV) : menu « Reglages » de Zen Home quand il est au
     * premier plan ; sinon, partout ailleurs, le panneau rapide Image de TCL s'il est disponible ; sinon NONE (la touche suit son chemin).
     */
    static int settingsAction(int code, boolean homeResumed, boolean tclPanel) {
        if (!isSettingsKey(code)) return NONE;
        if (code == KEYCODE_NOTIFICATION && !tclPanel) return NONE;   // hors TCL, 83 reste la touche Notifications
        return homeResumed ? HOME_MENU : (tclPanel ? TCL_PANEL : NONE);
    }

    /**
     * Source / TV : on la prend (et on ouvre le panneau des entrees) seulement quand l'accueil Zen Home est au premier plan ;
     * dans les autres applis la touche suit son chemin normal (menu TCL). Sans entree physique (Mi Box...) elle n'est jamais prise.
     */
    static boolean shouldHandleSource(int code, boolean homeResumed, boolean hasInputs) {
        if (!homeResumed || !hasInputs) return false;
        for (int k : SOURCE_KEYS) if (k == code) return true;
        return false;
    }
}

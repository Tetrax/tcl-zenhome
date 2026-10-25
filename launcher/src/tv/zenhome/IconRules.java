package tv.zenhome;

/** Icones Phosphor (sous-ensembles de police) : nom -> codepoint, meteo -> icone (sans android.*). Les 7 premieres suivent Icons.LineIcon. */
final class IconRules {
    private IconRules() { }

    static final String[] NAMES = {"television-simple", "squares-four", "gear-six", "pencil-simple", "image", "moon", "google-play-logo",
            "sun", "cloud-sun", "cloud-moon", "cloud", "cloud-fog", "cloud-rain", "cloud-lightning", "cloud-snow", "drop",
            "plug", "plugs", "speaker-hifi", "plus", "caret-down", "check"};
    static final int[] CODES = {0xEAE6, 0xE464, 0xE272, 0xE3B4, 0xE2CA, 0xE330, 0xE294,
            0xE472, 0xE540, 0xE53E, 0xE1AA, 0xE53C, 0xE1B4, 0xE1B2, 0xE1B8, 0xE210,
            0xE946, 0xEB56, 0xEA08, 0xE3D4, 0xE136, 0xE182};

    /** Codepoint du glyphe, ou -1 si le nom est inconnu. */
    static int codepoint(String name) {
        for (int i = 0; i < NAMES.length; i++) if (NAMES[i].equals(name)) return CODES[i];
        return -1;
    }

    /** Glyphe (texte) du nom, vide s'il est inconnu. */
    static String glyph(String name) {
        int c = codepoint(name);
        return c < 0 ? "" : new String(Character.toChars(c));
    }

    /** Glyphe de l'icone numero kind (LineIcon.INPUT...), vide si kind est hors table. */
    static String glyph(int kind) {
        return kind < 0 || kind >= CODES.length ? "" : new String(Character.toChars(CODES[kind]));
    }

    /** Code WMO -> nom d'icone (clair : soleil ou lune ; peu nuageux : nuage avec soleil ou lune ; couvert ; brouillard ; pluie ; neige ; orage). */
    static String weatherIcon(int c, boolean night) {
        if (c == 0) return night ? "moon" : "sun";
        if (c >= 1 && c <= 2) return night ? "cloud-moon" : "cloud-sun";
        if (c == 45 || c == 48) return "cloud-fog";
        if ((c >= 51 && c <= 57) || (c >= 61 && c <= 67) || (c >= 80 && c <= 82)) return "cloud-rain";
        if ((c >= 71 && c <= 77) || c == 85 || c == 86) return "cloud-snow";
        if (c >= 95) return "cloud-lightning";
        return "cloud";
    }
}

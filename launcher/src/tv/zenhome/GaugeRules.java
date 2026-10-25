package tv.zenhome;

/** Jauges RAM / disque : pourcentages, texte et angle de l'arc (sans android.*). */
final class GaugeRules {
    private GaugeRules() { }

    static final int WARN_PCT = 85;   // au-dela : arc orange

    /** used / total en % arrondi (0..100), ou -1 si les valeurs sont invalides (total <= 0, negatif, NaN, used > total). */
    static int pct(double used, double total) {
        if (Double.isNaN(used) || Double.isNaN(total) || Double.isInfinite(total) || total <= 0 || used < 0 || used > total) return -1;
        return (int) Math.round(used * 100.0 / total);
    }

    /** « 73 % » (espace fine insecable) en francais, « 73% » en anglais ; vide si invalide. */
    static String text(int pct, boolean fr) {
        if (pct < 0 || pct > 100) return "";
        return pct + (fr ? " %" : "%");
    }

    /** Angle de l'arc de progression en degres (depart en haut, sens horaire) ; 0 si invalide. */
    static float sweep(int pct) { return pct < 0 ? 0f : Math.min(pct, 100) * 3.6f; }

    static boolean warn(int pct) { return pct > WARN_PCT; }
}

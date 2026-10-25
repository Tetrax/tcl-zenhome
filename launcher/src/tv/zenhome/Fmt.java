package tv.zenhome;

/** Formats et petits calculs purs (sans android.*). */
final class Fmt {
    private Fmt() { }

    /** Valeur en ko d'une ligne /proc/meminfo, ou -1 si la ligne est malformee. */
    static long kb(String line) {
        String[] p = line.trim().split("\\s+");
        try { return Long.parseLong(p[1]); } catch (Exception e) { return -1; }
    }

    /** Prefixes de ligne de menu : coche sur le choix courant, meme largeur sinon (remplaces par un glyphe a l'affichage). */
    static final String CHECK = "✓  ", NO_CHECK = "     ";

    static String deg(double v) { return Math.round(v) + "°"; }

    static boolean stale(long ts, long now, long ttl) { return ts <= 0 || ts > now || now - ts > ttl; }
}

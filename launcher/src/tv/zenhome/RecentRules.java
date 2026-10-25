package tv.zenhome;

import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.List;
import java.util.Locale;
import java.util.Map;

/** Applis ouvertes : paquets jamais proposes, selection des applis recentes, textes (sans android.*). */
final class RecentRules {
    private RecentRules() { }

    static final long WINDOW_MS = 3 * 3_600_000L;   // « recente » = utilisee dans les 3 dernieres heures
    static final int MAX = 10;
    static final long MIN_FREED_MB = 20;            // en dessous : « Memoire deja libre »
    static final String SELF = "tv.zenhome";

    /** Zen Home et les paquets systeme critiques ne sont jamais fermes ni proposes. */
    static boolean protectedPkg(String p) {
        if (p == null || p.isEmpty()) return true;
        return p.equals(SELF) || p.equals("android") || p.startsWith("com.android.") || p.startsWith("com.mediatek.")
                || p.equals("com.google.android.gms") || p.startsWith("com.google.android.gms.")
                || p.startsWith("com.tcl.") || p.startsWith("com.google.android.apps.tv.");
    }

    static boolean canClose(String p) { return !protectedPkg(p); }

    /** Applis recentes : paquet -> dernier usage (ms) ; hors paquets proteges, hors fenetre, du plus recent au plus ancien, au plus max. */
    static List<String> select(Map<String, Long> lastUsed, long now, long windowMs, int max) {
        List<String> l = new ArrayList<>();
        for (Map.Entry<String, Long> e : lastUsed.entrySet()) {
            long t = e.getValue() == null ? 0 : e.getValue();
            if (protectedPkg(e.getKey()) || t <= 0 || now - t > windowMs || t > now + 60_000L) continue;   // horloge en avance : ignore
            l.add(e.getKey());
        }
        final Map<String, Long> m = lastUsed;
        Collections.sort(l, new Comparator<String>() {
            public int compare(String a, String b) {
                int c = Long.compare(m.get(b), m.get(a));
                return c != 0 ? c : a.compareTo(b);
            }
        });
        return l.size() > max ? new ArrayList<>(l.subList(0, max)) : l;
    }

    /** « il y a 5 min » / « il y a 2 h » / « il y a 3 j » ; « a l'instant » sous une minute. */
    static String agoText(long now, long last, boolean fr) {
        long s = Math.max(0, (now - last) / 1000);
        if (s < 60) return fr ? "à l'instant" : "just now";
        long m = s / 60;
        if (m < 60) return fr ? "il y a " + m + " min" : m + " min ago";
        long h = m / 60;
        if (h < 24) return fr ? "il y a " + h + " h" : h + " h ago";
        return fr ? "il y a " + h / 24 + " j" : h / 24 + " d ago";
    }

    /** Memoire gagnee en Mo (jamais negative) a partir de MemAvailable avant / apres (ko). */
    static long freedMb(long beforeKb, long afterKb) { return Math.max(0, (afterKb - beforeKb) / 1024); }

    /**
     * Resultat de « Tout liberer » : « 3 applis fermees » (+ « · 312 Mo liberes » si le gain atteint 20 Mo : Android avait souvent deja mis ces
     * applis en cache) ; « Rien a fermer » si aucune appli n'etait listee.
     */
    static String freeAllText(int closed, long beforeKb, long afterKb, boolean fr) {
        if (closed <= 0) return fr ? "Rien à fermer" : "Nothing to close";
        String s = fr ? closed + (closed == 1 ? " appli fermée" : " applis fermées") : closed + (closed == 1 ? " app closed" : " apps closed");
        long mb = freedMb(beforeKb, afterKb);
        return mb < MIN_FREED_MB ? s : s + "  ·  " + mb + (fr ? " Mo libérés" : " MB freed");
    }

    /** « RAM libre 0,8 Go » (une decimale sous 10 Go) ; vide si la valeur est inconnue. */
    static String ramText(long availKb, boolean fr) {
        if (availKb < 0) return "";
        double g = availKb / 1048576.0;
        String v = String.format(fr ? Locale.FRANCE : Locale.US, g >= 10 ? "%.0f" : "%.1f", g);
        return fr ? "RAM libre " + v + " Go" : "Free RAM " + v + " GB";
    }

    /** Message apres fermeture : recent = true (dans la liste des applis recentes), false (absente), null (inconnu : pas d'acces aux stats). */
    static String closedText(String label, Boolean recent, boolean fr) {
        if (recent == null) return fr ? "Fermeture demandée : " + label : "Close requested: " + label;
        if (recent) return fr ? label + " fermée" : label + " closed";
        return fr ? label + " est déjà fermée" : label + " is already closed";
    }
}

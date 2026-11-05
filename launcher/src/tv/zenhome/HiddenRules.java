package tv.zenhome;

import java.util.ArrayList;
import java.util.List;

/** Applis masquees (pref « hidden », paquets separes par des virgules) : logique pure, sans android.*. */
final class HiddenRules {
    private HiddenRules() { }

    /** Chaine pref -> paquets (separateurs , ; espaces), sans vides ni doublons ; null = liste vide. */
    static List<String> parse(String csv) {
        List<String> out = new ArrayList<>();
        if (csv == null) return out;
        for (String p : ConfigRules.csv(csv)) { if (!out.contains(p)) out.add(p); }
        return out;
    }

    static String serialize(List<String> pkgs) {
        StringBuilder b = new StringBuilder();
        for (String p : pkgs) { if (b.length() > 0) b.append(','); b.append(p); }
        return b.toString();
    }

    static boolean contains(List<String> hidden, String pkg) { return hidden.contains(pkg); }

    /** Nouvelle liste avec pkg en plus (inchangee s'il y est deja ou s'il est vide). */
    static List<String> add(List<String> hidden, String pkg) {
        List<String> out = new ArrayList<>(hidden);
        if (pkg != null && pkg.length() > 0 && !out.contains(pkg)) out.add(pkg);
        return out;
    }

    static List<String> remove(List<String> hidden, String pkg) {
        List<String> out = new ArrayList<>(hidden);
        out.remove(pkg);
        return out;
    }

    /** Paquets de pkgs qui ne sont pas masques, dans l'ordre. */
    static List<String> filter(List<String> pkgs, List<String> hidden) {
        List<String> out = new ArrayList<>();
        for (String p : pkgs) { if (!hidden.contains(p)) out.add(p); }
        return out;
    }
}

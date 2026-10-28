package tv.zenhome;

import java.util.ArrayList;
import java.util.List;

/** Validation pure de la configuration (jeton, coordonnees, ville, dock). */
final class ConfigRules {
    private ConfigRules() { }

    /** Jeton TMDB nettoye, ou null s'il est invalide (jamais de saut de ligne dans un en-tete). */
    static String token(String tok) {
        if (tok == null) return null;
        tok = tok.trim();
        return tok.matches("[A-Za-z0-9._~+/=-]{8,4096}") ? tok : null;
    }

    static boolean valid(String s, double max) {
        if (s == null) return false;
        s = s.trim();
        if (!s.matches("-?\\d{1,3}(\\.\\d{1,8})?")) return false;
        double v = Double.parseDouble(s);
        return v >= -max && v <= max;
    }

    /** Coordonnees {lat, lon} lues dans les prefs, ou null si absentes / hors bornes. */
    static double[] coords(String lat, String lon) {
        try {
            double la = Double.parseDouble(lat), lo = Double.parseDouble(lon);
            if (Math.abs(la) <= 90 && Math.abs(lo) <= 180) return new double[]{la, lo};
        } catch (Exception ignored) { }
        return null;
    }

    /** Ville sans caracteres de controle, 48 caracteres max ; vide = a effacer. */
    static String city(String city) {
        city = city.replaceAll("\\p{Cntrl}", "").trim();
        if (city.length() > 48) city = city.substring(0, 48);
        return city;
    }

    /**
     * Ligne « paquet<TAB>libelle » pour la sortie de am broadcast, ou null si le paquet est invalide.
     * Libelle sans controle / format (bidi) / separateurs de ligne ni guillemets, 60 caracteres max ; vide = nom du paquet.
     */
    static String labelLine(String pkg, CharSequence label) {
        if (pkg == null || !pkg.matches("[A-Za-z][A-Za-z0-9_]*(\\.[A-Za-z][A-Za-z0-9_]*)+")) return null;
        String l = label == null ? "" : label.toString().replaceAll("[\\p{Cntrl}\\p{Cf}\\p{Zl}\\p{Zp}\"]", "").trim();
        if (l.length() > 60) {
            int n = Character.isHighSurrogate(l.charAt(59)) ? 59 : 60;   // ne coupe pas une paire de substitution
            l = l.substring(0, n).trim();
        }
        return pkg + "\t" + (l.isEmpty() ? pkg : l);
    }

    /** Liste CSV de paquets (separateurs , ; espaces) -> elements non vides, tels quels. */
    static List<String> csv(String csv) {
        List<String> out = new ArrayList<>();
        for (String p : csv.split("[,;\\s]+")) { p = p.trim(); if (p.length() > 0) out.add(p); }
        return out;
    }

    /** Dock affiche different de la liste recalculee (appli desinstallee, desactivee ou reactivee) : il faut le reconstruire. */
    static boolean dockStale(List<String> shown, List<String> fresh) { return !shown.equals(fresh); }

    /** Nombre d'applis du dock : 2 a 10, sinon 4. */
    static int layout(String s) {
        int n;
        try { n = Integer.parseInt(s); } catch (Exception ex) { n = 4; }
        if (n < 2 || n > 10) n = 4;
        return n;
    }
}

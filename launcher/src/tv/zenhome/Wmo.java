package tv.zenhome;

/** Codes meteo WMO (Open-Meteo) -> pictogramme, libelle et phrase d'ambiance. */
final class Wmo {
    private Wmo() { }

    private static String t(boolean fr, String frTxt, String enTxt) { return fr ? frTxt : enTxt; }

    private static String[] w(boolean fr, String icon, String frTxt, String enTxt) { return new String[]{icon, t(fr, frTxt, enTxt)}; }

    /** Code WMO -> {pictogramme, libelle}. */
    static String[] of(int c, boolean fr) {
        if (c == 0) return w(fr, "☀", "Ensoleillé", "Clear");
        if (c <= 2) return w(fr, "⛅", "Peu nuageux", "Partly cloudy");
        if (c == 3) return w(fr, "☁", "Couvert", "Overcast");
        if (c == 45 || c == 48) return w(fr, "≋", "Brouillard", "Fog");
        if (c >= 51 && c <= 57) return w(fr, "☂", "Bruine", "Drizzle");
        if (c >= 61 && c <= 67) return w(fr, "☂", "Pluie", "Rain");
        if (c >= 71 && c <= 77) return w(fr, "❄", "Neige", "Snow");
        if (c >= 80 && c <= 82) return w(fr, "☂", "Averses", "Showers");
        if (c == 85 || c == 86) return w(fr, "❄", "Averses de neige", "Snow showers");
        if (c >= 95) return w(fr, "⚡", "Orage", "Thunderstorm");
        return w(fr, "☁", "Variable", "Variable");
    }

    /** Libelle du code ; la nuit, le ciel clair n'est plus « Ensoleillé » mais « Dégagé » / « Clear » (les autres libelles ne changent pas). */
    static String label(int c, boolean fr, boolean night) {
        return c == 0 && night ? t(fr, "Dégagé", "Clear") : of(c, fr)[1];
    }

    /** Phrase meteo pour le code WMO courant (slot = tranche horaire 0..3 de Greeting). */
    static String phrase(int c, int slot, String city, boolean fr) {
        if (c == 0) {
            if (slot <= 1) {
                String w = t(fr, "Grand soleil sur ", "Bright sunshine in ") + city;
                return city.isEmpty() || w.length() > 40 ? t(fr, "Grand soleil aujourd'hui", "Bright sunshine today") : w;
            }
            return slot == 2 ? t(fr, "Ciel dégagé ce soir", "Clear sky tonight") : t(fr, "Ciel dégagé cette nuit", "Clear skies overnight");
        }
        if (c <= 2) return t(fr, "Entre nuages et éclaircies", "Clouds and sunny spells");
        if (c == 3) return t(fr, "Ciel gris, ambiance série", "Grey sky, series mood");
        if (c == 45 || c == 48) return slot == 0 ? t(fr, "Brouillard ce matin, reste au chaud", "Foggy morning, stay warm")
                : t(fr, "Brouillard dehors, reste au chaud", "Foggy outside, stay warm");
        if (c >= 71 && c <= 77 || c == 85 || c == 86) return t(fr, "Il neige : plaid et film ?", "Snowing: blanket and a movie?");
        if (c >= 95) return t(fr, "Orage dehors, reste au chaud", "Stormy outside, stay in");
        return t(fr, "Il pleut : parfait pour un film", "Rainy out: perfect for a movie");   // bruine, pluie, averses
    }
}

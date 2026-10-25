package tv.zenhome;

/**
 * Previsions horaires et pluie imminente (sans android.*). Les heures sont des « heures locales du lieu » en ms, lues comme si elles
 * etaient UTC (Open-Meteo, timezone=auto, donne des heures locales + utc_offset_seconds) : aucun fuseau de la TV n'intervient.
 */
final class WeatherRules {
    private WeatherRules() { }

    static final long HOUR_MS = 3_600_000L, QUARTER_MS = 900_000L, WINDOW_MS = HOUR_MS;
    static final double POP_MIN = 50, MM_MIN = 0.1, POP_SHOWN = 20;   // pluie annoncee si proba >= 50 % ou > 0,1 mm ; % affiche des 20 %
    static final long NEAR_MS = 30 * 60_000L;                         // debut dans moins de 30 min : « dans l'heure », sinon « vers 21 h »

    /** « 2026-10-25T14:00 » -> ms (lu comme UTC), -1 si illisible. */
    static long local(String iso) {
        if (iso == null || iso.length() < 16 || iso.charAt(10) != 'T') return -1;
        try {
            int y = Integer.parseInt(iso.substring(0, 4)), mo = Integer.parseInt(iso.substring(5, 7)), d = Integer.parseInt(iso.substring(8, 10));
            int h = Integer.parseInt(iso.substring(11, 13)), mi = Integer.parseInt(iso.substring(14, 16));
            if (mo < 1 || mo > 12 || d < 1 || d > 31 || h > 23 || mi > 59 || h < 0 || mi < 0) return -1;
            return (days(y, mo, d) * 24 + h) * 60 * 60_000L + mi * 60_000L;
        } catch (NumberFormatException e) { return -1; }
    }

    /** Jours depuis 1970-01-01 (calendrier gregorien). */
    private static long days(int y, int m, int d) {
        if (m <= 2) y--;
        long era = (y >= 0 ? y : y - 399) / 400, yoe = y - era * 400;
        long doy = (153 * (m + (m > 2 ? -3 : 9)) + 2) / 5 + d - 1, doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        return era * 146097 + doe - 719468;
    }

    /** Maintenant, en heure locale du lieu. */
    static long nowLocal(long utcMs, int offsetSeconds) { return utcMs + offsetSeconds * 1000L; }

    /** Indices des `count` prochaines heures (times croissants) : a partir de la premiere heure pleine >= now. */
    static int[] nextHours(long[] times, long now, int count) {
        int from = 0;
        while (from < times.length && times[from] < now) from++;
        int n = Math.max(0, Math.min(count, times.length - from));
        int[] r = new int[n];
        for (int i = 0; i < n; i++) r[i] = from + i;
        return r;
    }

    /**
     * Debut (ms, heure locale) de la pluie dans les 60 prochaines minutes, ou -1. Open-Meteo donne probabilite et cumul pour la periode
     * PRECEDENT l'heure indiquee : la valeur datee t couvre ]t - duree, t]. Le debut est borne a now (pluie deja en cours).
     * hourly : probabilite en % (negatif = inconnu) ; quarter : cumul en mm par quart d'heure. Tableaux null = pas de donnees.
     */
    static long rainStart(long[] hT, double[] pop, long[] qT, double[] mm, long now) {
        long best = -1;
        if (hT != null && pop != null) for (int i = 0; i < hT.length && i < pop.length; i++)
            if (pop[i] >= POP_MIN) best = earliest(best, hT[i], HOUR_MS, now);
        if (qT != null && mm != null) for (int i = 0; i < qT.length && i < mm.length; i++)
            if (mm[i] > MM_MIN) best = earliest(best, qT[i], QUARTER_MS, now);
        return best;
    }

    private static long earliest(long best, long t, long span, long now) {
        if (t <= now || t - span >= now + WINDOW_MS) return best;   // periode hors de la fenetre [now, now + 60 min]
        long s = Math.max(now, t - span);
        return best < 0 || s < best ? s : best;
    }

    /** « Pluie dans l'heure » (debut dans moins de 30 min) ou « Pluie vers 21 h » ; vide s'il n'y a pas de pluie (start < 0). */
    static String rainText(long start, long now, boolean fr) {
        if (start < 0) return "";
        if (start - now < NEAR_MS) return fr ? "Pluie dans l'heure" : "Rain within the hour";
        long mi = start / 60_000L % 60;
        String h = String.valueOf(start / HOUR_MS % 24);
        if (fr) return "Pluie vers " + h + " h" + (mi == 0 ? "" : " " + mi);
        return "Rain around " + h + ":" + (mi < 10 ? "0" : "") + mi;
    }

    static final long BRIEF_TTL_MS = 3 * HOUR_MS;   // anciennete maximale du cache pour l'ecran de veille
    static final long DAY_MS = 24 * HOUR_MS;

    /**
     * Nuit a l'heure locale t ? Avec lever / coucher du jour de t (tableaux Open-Meteo, heures locales) : avant le lever ou apres
     * le coucher ; sinon (tableaux absents ou jour introuvable) : avant 7 h ou a partir de 19 h.
     */
    static boolean isNight(long t, long[] sunrise, long[] sunset) {
        long day = Math.floorDiv(t, DAY_MS);
        if (sunrise != null && sunset != null)
            for (int i = 0; i < sunrise.length && i < sunset.length; i++)
                if (sunrise[i] >= 0 && sunset[i] >= 0 && Math.floorDiv(sunrise[i], DAY_MS) == day) return t < sunrise[i] || t >= sunset[i];
        long h = Math.floorMod(t, DAY_MS) / HOUR_MS;
        return h < 7 || h >= 19;
    }

    /** « 15 h » / « 15:00 ». */
    static String hourLabel(long t, boolean fr) {
        long h = t / HOUR_MS % 24;
        return fr ? h + " h" : (h < 10 ? "0" : "") + h + ":00";
    }

    /** « 60% » des 20 %, vide sinon (ou si inconnu). */
    static String popText(double pop) { return pop >= POP_SHOWN ? Math.round(pop) + "%" : ""; }
}

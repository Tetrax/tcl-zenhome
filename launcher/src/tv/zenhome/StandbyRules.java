package tv.zenhome;

import java.util.List;

/** Textes de l'ecran de veille (ambiance et economiseur) : meteo detaillee, prochaine sortie de Ma liste (sans android.*). */
final class StandbyRules {
    private StandbyRules() { }

    static final String[] MONTHS_FR = {"janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"};
    static final String[] MONTHS_EN = {"January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"};
    static final int NEAR_DAYS = 7;   // au-dela : la date

    /** Le cache meteo est-il assez recent (3 h au plus) pour etre affiche ? */
    static boolean fresh(long ts, long now) { return !Fmt.stale(ts, now, WeatherRules.BRIEF_TTL_MS); }

    /** « 17° ☀ Ensoleillé · 10° / 21° » (pic = picto ou marqueur Ui.PIC) ; sans min / max (NaN) : « 17° ☀ Ensoleillé » ; vide sans temperature. */
    static String weatherLine(double temp, int code, double min, double max, boolean fr, String pic, boolean night) {
        if (Double.isNaN(temp)) return "";
        String s = Fmt.deg(temp) + " " + pic + " " + Wmo.label(code, fr, night);
        if (!Double.isNaN(min) && !Double.isNaN(max)) s += "  ·  " + Fmt.deg(min) + " / " + Fmt.deg(max);
        return s;
    }

    /** « Demain : 19° ☁ » (pic = picto ou marqueur) ; vide sans temperature. */
    static String tomorrowLine(double max, int code, boolean fr, String pic) {
        if (Double.isNaN(max)) return "";
        return (fr ? "Demain : " : "Tomorrow: ") + Fmt.deg(max) + " " + pic;
    }

    /** Jours entre deux dates yyyy-MM-dd, ou Long.MIN_VALUE si l'une est illisible. */
    static long daysBetween(String from, String to) {
        long a = WeatherRules.local(from + "T00:00"), b = WeatherRules.local(to + "T00:00");
        return a < 0 || b < 0 ? Long.MIN_VALUE : (b - a) / 86_400_000L;
    }

    /** Film de la liste dont la sortie (>= today) est la plus proche ; le premier de la liste en cas d'egalite ; null s'il n'y en a pas. */
    static Movie nextRelease(List<Movie> l, String today) {
        Movie best = null;
        for (Movie m : l) {
            if (m.date.length() != 10 || m.title.isEmpty() || daysBetween(today, m.date) == Long.MIN_VALUE || m.date.compareTo(today) < 0) continue;
            if (best == null || m.date.compareTo(best.date) < 0) best = m;
        }
        return best;
    }

    /** « X sort aujourd'hui / demain / dans 3 jours / le 4 novembre » ; vide si la date est passee ou illisible. */
    static String releaseText(String title, String date, String today, boolean fr) {
        long d = daysBetween(today, date);
        if (d == Long.MIN_VALUE || d < 0) return "";
        String when;
        if (d == 0) when = fr ? "aujourd'hui" : "today";
        else if (d == 1) when = fr ? "demain" : "tomorrow";
        else if (d <= NEAR_DAYS) when = fr ? "dans " + d + " jours" : "in " + d + " days";
        else {
            int mo = Integer.parseInt(date.substring(5, 7)), day = Integer.parseInt(date.substring(8, 10));
            when = fr ? "le " + (day == 1 ? "1er" : String.valueOf(day)) + " " + MONTHS_FR[mo - 1] : "on " + MONTHS_EN[mo - 1] + " " + day;
        }
        return title + (fr ? " sort " : " is out ") + when;
    }
}

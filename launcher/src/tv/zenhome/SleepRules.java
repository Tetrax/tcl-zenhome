package tv.zenhome;

/** Minuteur de mise en veille : echeance absolue (ms epoch), textes, et decisions du receiver (sans android.*). */
final class SleepRules {
    private SleepRules() { }

    static final int[] CHOICES = {0, 15, 30, 60, 90, 120, 180, 240};   // minutes ; 0 = desactive
    static final long MIN_MS = 60_000L, WARN_MS = 60_000L;   // avertissement 1 minute avant
    static final long EARLY_MS = 5_000L;                     // tolerance : une alarme ne doit pas etre prise en compte plus de 5 s avant l'echeance

    /** Echeance pour « minutes » a partir de now ; 0 si desactive. */
    static long deadline(long now, int minutes) { return minutes <= 0 ? 0 : now + minutes * MIN_MS; }

    static boolean active(long deadline, long now) { return deadline > 0 && deadline > now; }

    /** Instant de l'avertissement, ou 0 s'il est deja passe (echeance dans moins d'une minute). */
    static long warnAt(long deadline, long now) {
        long w = deadline - WARN_MS;
        return deadline > 0 && w > now ? w : 0;
    }

    /** Minutes restantes arrondies au-dessus (au moins 1 tant que le minuteur est actif), 0 sinon. */
    static int minutesLeft(long deadline, long now) {
        if (!active(deadline, now)) return 0;
        return (int) ((deadline - now + MIN_MS - 1) / MIN_MS);
    }

    /** Duree courte : « 42 min » jusqu'a 60 min, « 1 h 05 » au-dela ; vide si le minuteur n'est pas actif. */
    static String shortLeft(long deadline, long now) {
        int m = minutesLeft(deadline, now);
        if (m == 0) return "";
        if (m <= 60) return m + " min";
        return m / 60 + " h " + (m % 60 < 10 ? "0" : "") + m % 60;
    }

    /** « Veille dans 42 min » / « Veille dans 3 h 20 », ou vide si le minuteur n'est pas actif. */
    static String leftText(long deadline, long now, boolean fr) {
        String s = shortLeft(deadline, now);
        return s.isEmpty() ? "" : (fr ? "Veille dans " : "Sleep in ") + s;
    }

    static String choiceLabel(int minutes, boolean fr) {
        return minutes <= 0 ? (fr ? "Désactivé" : "Off") : minutes + " min";
    }

    static int indexOf(int minutes) {
        for (int i = 0; i < CHOICES.length; i++) if (CHOICES[i] == minutes) return i;
        return 0;
    }

    /** L'alarme de veille doit-elle agir ? Meme echeance que celle enregistree (sinon elle a ete annulee / remplacee) et arrivee a terme. */
    static boolean due(long saved, long fired, long now) { return saved > 0 && saved == fired && now >= saved - EARLY_MS; }

    /** L'alarme d'avertissement doit-elle afficher le message ? Meme echeance, pas encore echue. */
    static boolean warnDue(long saved, long fired, long now) { return saved > 0 && saved == fired && now < saved; }

    static String warnText(boolean fr) { return fr ? "Mise en veille dans 1 minute" : "Going to sleep in 1 minute"; }

    static String failText(boolean fr) {
        return fr ? "Mise en veille impossible (service des touches inactif)" : "Cannot sleep (key service inactive)";
    }

    static String offText(boolean fr) { return fr ? "Minuteur désactivé" : "Sleep timer off"; }
}

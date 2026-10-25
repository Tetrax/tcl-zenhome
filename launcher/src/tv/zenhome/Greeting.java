package tv.zenhome;

import java.util.Calendar;

/** Salutation et phrase d'ambiance (deterministes : graine = jour de l'annee et tranche horaire). */
final class Greeting {
    private Greeting() { }

    /** Soir et nuit (22h a 6h) : l'ambiance est assombrie. */
    static boolean night(int hour) { return hour >= 22 || hour < 6; }

    static final String[][] GREET = {
            {"Bonjour", "Good morning"}, {"Bon après-midi", "Good afternoon"}, {"Bonsoir", "Good evening"}, {"Bonne nuit", "Good night"}};
    static final String[][][] LINES = {
            {{"Café et nouveautés ?", "Belle journée à toi", "Une bonne journée t'attend", "Bien dormi ?", "On commence en douceur ?", "Prêt pour la journée ?"},
             {"Coffee and something new?", "Have a lovely day", "A good day is waiting for you", "Sleep well?", "Let's start gently?", "Ready for the day?"}},
            {{"Une petite pause ?", "Un petit film pour souffler ?", "Bon après-midi à toi", "Et si on faisait une pause ?", "Du thé et un bon épisode ?", "Prends un moment pour toi"},
             {"A little break?", "A short film to unwind?", "Enjoy your afternoon", "How about a pause?", "Tea and a good episode?", "Take a moment for yourself"}},
            {{"Une soirée film ?", "Le canapé t'attend", "Pop-corn prêt ?", "Détends-toi, c'est le soir", "Que regarde-t-on ce soir ?", "Ta série t'attend", "Plaid, canapé, écran"},
             {"Movie night?", "The couch is waiting", "Popcorn ready?", "Relax, it's evening", "What are we watching tonight?", "Your show is waiting", "Blanket, couch, screen"}},
            {{"Un dernier épisode ?", "Pense à dormir un peu", "Il se fait tard", "Encore debout ?", "Fais de beaux rêves", "Un film tranquille pour finir ?"},
             {"One last episode?", "Remember to get some sleep", "It's getting late", "Still up?", "Sweet dreams", "A quiet film to end the night?"}}};

    static int slotOf(int hour) { return hour >= 5 && hour < 12 ? 0 : (hour >= 12 && hour < 18 ? 1 : (hour >= 18 && hour < 22 ? 2 : 3)); }

    private static String t(boolean fr, String frTxt, String enTxt) { return fr ? frTxt : enTxt; }

    /**
     * Phrase d'ambiance. dow = Calendar.DAY_OF_WEEK ; wxCode = code WMO du cache ou -1 s'il n'y a pas de meteo.
     */
    static String phrase(int doy, int dow, int slot, int wxCode, String city, boolean fr) {
        if ((doy + slot) % 2 == 0) {   // jour special : une fois sur deux seulement
            if (dow == Calendar.FRIDAY && slot == 2) return t(fr, "Enfin le week-end", "Weekend at last");
            if (dow == Calendar.SATURDAY && slot < 3) return t(fr, "Bon samedi", "Happy Saturday");
            if (dow == Calendar.SUNDAY && slot < 3) return t(fr, "Bon dimanche", "Happy Sunday");
            if (dow == Calendar.MONDAY && slot == 0) return t(fr, "Courage pour ce lundi", "Hang in there, it's Monday");
        }
        if (wxCode >= 0) return Wmo.phrase(wxCode, slot, city, fr);
        String[] l = LINES[slot][fr ? 0 : 1];
        return l[(doy * 4 + slot) % l.length];
    }
}

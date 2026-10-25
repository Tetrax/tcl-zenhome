package tv.zenhome;

/** Machine d'etats de l'accueil en niveaux : etat + touche + drapeaux -> etat suivant. Aucune dependance Android. */
final class Nav {
    private Nav() { }

    /** REST = dock ; APPS = carrousel sous le dock ; FILMS = affiches ; TABS = onglets ; TOP = barre de raccourcis ; WEATHER = ligne meteo ; AMBIENT = UI effacee (photo + heure), atteint par IDLE. */
    enum State { REST, APPS, FILMS, TABS, TOP, WEATHER, AMBIENT }

    /** LEFT_EDGE / RIGHT_EDGE : Gauche / Droite sur le premier / dernier element de la rangee (sans voisin dans la rangee). */
    enum Key { UP, DOWN, LEFT_EDGE, RIGHT_EDGE, BACK, IDLE }

    static final class Flags {
        final boolean films, weather, edit, firstOfRow, idleOk;
        /** films : jeton TMDB present ; weather : meteo affichee ; edit : mode edition ; firstOfRow : en TOP, le focus est sur le 1er raccourci. */
        Flags(boolean films, boolean weather, boolean edit, boolean firstOfRow) { this(films, weather, edit, firstOfRow, true); }
        /** idleOk : l'ambiance est permise (premier plan, aucun overlay, pas de deplacement en cours). */
        Flags(boolean films, boolean weather, boolean edit, boolean firstOfRow, boolean idleOk) {
            this.films = films; this.weather = weather; this.edit = edit; this.firstOfRow = firstOfRow; this.idleOk = idleOk;
        }
        boolean filmsOn() { return films && !edit; }
    }

    /** Niveau qui reste atteignable avec ces drapeaux (sinon REST). */
    static State available(State s, Flags f) {
        if ((s == State.FILMS || s == State.TABS) && !f.filmsOn()) return State.REST;
        if (s == State.WEATHER && !f.weather) return State.REST;
        return s;
    }

    /** Sortie d'ambiance : on retrouve le niveau d'avant s'il existe encore, sinon le repos. */
    static State wake(State before, Flags f) {
        return before == State.AMBIENT ? State.REST : available(before, f);
    }

    static State next(State s, Key k, Flags f) {
        if (available(s, f) != s) return State.REST;   // niveau devenu indisponible
        if (k == Key.IDLE) return f.idleOk && !f.edit ? State.AMBIENT : s;   // ambiance : jamais en edition
        if (s == State.AMBIENT) return k == Key.BACK ? State.REST : s;       // la touche reveille (Levels), elle n'agit pas
        if (k == Key.BACK) return State.REST;
        switch (s) {
            case REST:
                if (k == Key.UP) return f.filmsOn() ? State.FILMS : State.TOP;
                if (k == Key.DOWN) return State.APPS;
                return s;
            case APPS:
                return k == Key.UP ? State.REST : s;
            case FILMS:
                if (k == Key.UP) return State.TABS;
                if (k == Key.DOWN) return State.REST;
                return s;
            case TABS:
                if (k == Key.UP) return State.TOP;
                if (k == Key.DOWN) return State.FILMS;
                return s;
            case TOP:
                if (k == Key.DOWN) return f.filmsOn() ? State.TABS : State.REST;
                if (k == Key.LEFT_EDGE && f.firstOfRow && f.weather) return State.WEATHER;
                return s;
            default:   // WEATHER
                if (k == Key.RIGHT_EDGE) return State.TOP;
                if (k == Key.DOWN) return f.filmsOn() ? State.TABS : State.REST;
                return s;
        }
    }
}

package tv.zenhome;

/** Film TMDB reduit aux champs affiches. */
final class Movie {
    String title = "", overview = "", poster = "", backdrop = "", date = "";
    double vote;
    int id;   // id TMDB (0 si inconnu : cache d'avant la version 2)
}

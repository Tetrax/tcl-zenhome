package tv.zenhome;

/** Mentions de sources de donnees affichees dans l'appli (conditions TMDB et Open-Meteo), sans android.*. */
final class Attribution {
    private Attribution() { }

    static String tmdb(boolean fr) {
        return fr ? "Données et images : TMDB. Ce produit utilise l'API TMDB mais n'est ni approuvé ni certifié par TMDB."
                : "Data and images: TMDB. This product uses the TMDB API but is not endorsed or certified by TMDB.";
    }

    static String weather(boolean fr) { return fr ? "Météo : Open-Meteo (CC BY 4.0)" : "Weather: Open-Meteo (CC BY 4.0)"; }
}

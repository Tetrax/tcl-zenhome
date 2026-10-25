package tv.zenhome;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.net.URL;

import javax.net.ssl.HttpsURLConnection;

/**
 * Acces reseau minimal : HTTPS uniquement, hotes en liste blanche, tailles plafonnees,
 * aucune redirection suivie. A appeler depuis un thread d'arriere-plan (jamais le thread UI).
 * Ne journalise rien : ni URL, ni en-tetes (le jeton TMDB passe par ici).
 */
final class Net {
    static final String[] HOSTS = {"api.open-meteo.com", "api.themoviedb.org", "image.tmdb.org"};
    static final int JSON_MAX = 512 * 1024, IMAGE_MAX = 1024 * 1024;
    static final int CONNECT_MS = 8000, READ_MS = 10000;
    static final long DEADLINE_MS = 20000;   // duree maximale d'une requete

    private Net() { }

    static boolean allowed(URL u) {
        if (!"https".equals(u.getProtocol()) || u.getUserInfo() != null) return false;
        if (u.getPort() != -1 && u.getPort() != 443) return false;
        String host = u.getHost();
        for (String h : HOSTS) if (h.equals(host)) return true;
        return false;
    }

    /** Reponse JSON (UTF-8, 512 Ko max). bearer peut etre null. */
    static String json(String url, String bearer) throws IOException {
        return new String(get(url, bearer, "application/json", JSON_MAX), "UTF-8");
    }

    /** Image brute (1 Mo max). */
    static byte[] image(String url) throws IOException {
        return get(url, null, "image/*", IMAGE_MAX);
    }

    private static byte[] get(String url, String bearer, String accept, int max) throws IOException {
        long end = System.currentTimeMillis() + DEADLINE_MS;
        URL u = new URL(url);
        if (!allowed(u)) throw new IOException("hote ou schema refuse");
        HttpsURLConnection c = (HttpsURLConnection) u.openConnection();
        try {
            c.setInstanceFollowRedirects(false);   // une redirection (3xx) est traitee comme un echec
            c.setConnectTimeout(CONNECT_MS);
            c.setReadTimeout(READ_MS);
            c.setRequestMethod("GET");
            c.setRequestProperty("User-Agent", "ZenHome/1.0");
            c.setRequestProperty("Accept", accept);
            if (bearer != null) c.setRequestProperty("Authorization", "Bearer " + bearer);
            if (c.getResponseCode() != 200) throw new IOException("HTTP " + c.getResponseCode());
            if (c.getContentLength() > max) throw new IOException("reponse trop grande");
            InputStream in = c.getInputStream();
            try {
                ByteArrayOutputStream out = new ByteArrayOutputStream();
                byte[] buf = new byte[8192];
                int n, total = 0;
                while ((n = in.read(buf)) > 0) {
                    total += n;
                    if (System.currentTimeMillis() > end) throw new IOException("delai depasse");
                    if (total > max) throw new IOException("reponse trop grande");
                    out.write(buf, 0, n);
                }
                return out.toByteArray();
            } finally { in.close(); }
        } finally { c.disconnect(); }
    }
}

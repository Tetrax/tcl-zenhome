package tv.zenhome;

/** Geometrie du dock (unites de la maquette 1280 de large) pour n applis, 2 a 10. */
final class DockMetrics {
    private DockMetrics() { }

    static final int SCREEN_M = 56;   // marge ecran (comme le titre)
    static final int DOCK_PAD = 20;   // marge interne du dock

    static int bigW(int n) { return Math.min(176, (int) ((1280 - 2 * SCREEN_M - 2 * DOCK_PAD) / (n * 1.1f))); }   // tuile + 2 x 5 % d'espacement
    static int bigH(int n) { return Math.round(bigW(n) * 9f / 16f); }   // ratio banniere
    static int bigM(int n) { return Math.max(2, Math.round(bigW(n) * 0.05f)); }
    static boolean compact(int n) { return bigW(n) < 110; }          // trop etroit pour une banniere : icone + libelle
    static int layoutHint(int n) { return 46 + bigH(n) + 36 + 19; }       // marge basse du dock + dock + espace : base de l'indication

    // Accueil en niveaux (memes unites, mesurees depuis le bas de l'ecran)
    static final int DOCK_BOTTOM = 46;   // marge basse du dock
    static final int CAR_H = 88;         // hauteur du carrousel d'applis
    static final int CAR_GAP = 6;        // espace entre le bas du dock et le haut du carrousel
    static final int CAR_MARGIN = 20;    // marge basse du carrousel une fois le bloc du bas remonte
    static final int FILMS_H = 190;      // onglets + rangee d'affiches
    static final int FILMS_GAP = 14;     // espace entre le haut du dock et le bas des onglets/affiches
    static final int SCREEN_H = 720;
    static final int TOP_ZONE = 200;     // haut de l'ecran reserve (salutation, meteo, mise en avant du film)

    static int dockH(int n) { return bigH(n) + 36; }
    static int carBase() { return DOCK_BOTTOM - CAR_GAP - CAR_H; }   // marge basse du carrousel au repos : juste sous le dock (negative : sous l'ecran)
    static int appsShift() { return CAR_MARGIN - carBase(); }       // montee du bloc du bas pour reveler le carrousel
    static int nxBase(int n) { return DOCK_BOTTOM + dockH(n) + FILMS_GAP; }   // marge basse de la rangee de films (au-dessus du dock)
}

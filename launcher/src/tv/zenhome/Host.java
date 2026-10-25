package tv.zenhome;

import android.view.View;
import android.widget.LinearLayout;

/** Ce que les modules Android remontent a Home (seul point de contact : ils ne se connaissent pas entre eux). */
interface Host {
    /** Clic sur une tuile cablee par Ui.wire : Home decide (menu, lancement differe, execution directe). */
    void onTileClick(View v, Runnable r);

    /** Focus gagne / perdu par une tuile cablee par Ui.wire (bulle des raccourcis, mise en avant du film). */
    void onTileFocus(View v, boolean has);

    /** Un deplacement de tuile est en cours. */
    boolean moving();

    /** Le mode edition est actif. */
    boolean editing();

    /** Vue qui a le focus (null s'il n'y en a pas). */
    View currentFocus();

    /** Rend le focus au dock (repli quand l'origine d'un overlay a disparu). */
    void focusDock();

    /** Niveau courant de l'accueil (celui de la vue qui a le focus). */
    Nav.State level();

    /** Rend le focus au niveau donne (repos s'il n'est plus disponible). */
    void focusLevel(Nav.State s);

    /** Un overlay vient d'etre retire de la pile (avant la mise a jour du blocage de focus). */
    void onOverlayClosed(View ov);

    /** Appui long sur une appli de la liste « Toutes les applis ». */
    void onAppLongPress(String pkg, View ov);

    /** Rafraichit meteo, films et affiches (appele toutes les 15 s par le tick, et au changement d'onglet). */
    void dataTick();

    /** Code WMO courant du cache meteo, ou -1. */
    int weatherCode();

    /** Le dock a ete enregistre et le carrousel suit le dock : le recalculer et le reconstruire. */
    void carouselRefresh();

    /** Repositionne les rangees (carrousel, films) selon la hauteur du dock. */
    void placeRails();

    /** Apres la reconstruction asynchrone d'une rangee : corrige le focus qui aurait migre vers elle. */
    void fixRowFocus(View before, int idx, LinearLayout row);

    /** La mise en avant d'un film apparait (true) ou disparait (false) : la meteo s'efface / revient. */
    void onSpot(boolean on);

    /** Fond = backdrop du film focalise : liberer la couche cachee avant le decodage / afficher le bitmap / revenir a la photo. */
    void backdropPrepare();
    void backdropReady(android.graphics.Bitmap b);
    void backdropClear();

    /** « Ma liste » : l'affiche y est-elle / l'y mettre ou l'en retirer. */
    boolean inMyList(Movie m);
    void toggleMyList(Movie m);

    /** OK sur une affiche : ouvrir le detail du film. */
    void onMovieClick(Movie m);

    /** Le dock vient d'etre reconstruit : repositionner l'indication et les rangees. */
    void onDockBuilt();

    /** Menu de la tuile du dock (index) ou menu general (-1). */
    void showTileMenu(int index);

    /** Menu de la tuile du carrousel (index) ou menu general (-1). */
    void showCarMenu(int index);

    /** Selecteur d'appli pour le dock / pour le carrousel. */
    void pickAppToAdd();
    void pickCarouselApp();

    /** Panneau des entrees TV (raccourci et touche Source). */
    void showInputs();

    /** Un panneau plein ecran (entrees) cache (true) ou rend (false) le bloc du bas et les films. */
    void cover(boolean on);

    /** Menu lateral « Reglages » (Image, Son, tous les parametres). */
    void showSettings();

    /** Menu lateral « Applis ouvertes ». */
    void showRecents();

    /** Menu du minuteur de mise en veille. */
    void showSleep();

    /** Raccourcis : liste de toutes les applis / mode edition / photo de fond suivante. */
    void showAllApps();
    void editHome();
    void nextWall();

    /** Le titre / mode de salutation a change : rafraichir la salutation. */
    void applyTitle();
}

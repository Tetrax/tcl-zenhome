package tv.zenhome;

import android.app.Activity;
import android.content.Intent;
import android.content.SharedPreferences;
import android.graphics.Color;
import android.graphics.drawable.GradientDrawable;
import android.os.Bundle;
import android.provider.Settings;
import android.view.Gravity;
import android.view.KeyEvent;
import android.view.View;
import android.widget.FrameLayout;
import android.widget.LinearLayout;

import java.util.List;

/**
 * Zen Home : ecran d'accueil minimaliste pour Android TV, facon Apple TV.
 * Photo plein ecran qui change + horloge + dock d'applis personnalisable.
 *
 * Personnalisation directement sur la TV : appui long sur OK (ou touche Menu) sur une appli du dock
 *   -> deplacer, remplacer, retirer, ajouter, disposition 4/6/8 applis, titre.
 *
 * Reglages envoyes par le kit (adb), pris en compte quand zenhome_rev change :
 *   settings put global zenhome_rev         <horodatage>
 *   settings put global zenhome_title_mode  device | none | custom
 *   settings put global zenhome_title_b64   <titre en base64 UTF-8>   (si custom)
 *   settings put global zenhome_layout      2..10 (nombre d'applis du dock, une seule rangee)
 *   settings put global zenhome_dock        com.netflix.ninja,com.google.android.youtube.tv,...
 *   settings put global zenhome_interval    60   (secondes entre deux photos)
 *   settings put global zenhome_theme       glass | night | sand | forest | auto   (couleur du verre ; auto = teinte de la photo)
 * Un nouveau zenhome_rev reapplique uniquement les cles zenhome_* presentes (title_mode, title_b64, layout, dock) :
 * les choix faits sur la TV pour les cles absentes sont conserves.
 * Photos perso : fichiers .jpg/.png dans /sdcard/ZenHome/ (remplacent les photos fournies).
 *   Permission accordee par adb (pas de dialogue) : pm grant tv.zenhome android.permission.READ_MEDIA_IMAGES (Android 13+)
 *   ou android.permission.READ_EXTERNAL_STORAGE (Android 12 et moins).
 *
 * Meteo (Open-Meteo) et « Nouveau sur Netflix » (TMDB) : configuration privee par adb, voir ConfigReceiver
 * (coordonnees, ville, jeton TMDB dans les preferences privees ; jamais dans Settings.Global).
 */
public class Home extends Activity implements Host {

    SharedPreferences prefs;
    FrameLayout root, content;   // content = tout le fond (bloque au clavier quand un overlay est ouvert)
    final Async async = new Async();
    Ui ui;
    HomeConfig config;
    Wallpaper wall;
    Header header;
    Weather weather;
    Movies movies;
    Dock dock;
    Carousel carousel;
    Shortcuts shortcuts;
    Editor editor;
    MovieDetail detail;
    Apps apps;
    Overlays ovs;
    AppsGrid grid;
    Levels levels;
    Ambient ambient;
    Inputs inputs;
    SettingsMenu settingsMenu;
    Sleep sleep;
    Banner banner;

    /** Accueil au premier plan (entre onResume et onPause), lu par KeyService (meme processus) ; null sinon. */
    static volatile Home front;

    boolean dockOnResume = true;     // vrai au lancement et apres la touche Accueil : focus sur le dock
    boolean needDockFocus = false;   // le prochain gain de focus fenetre / la prochaine mise en page doit viser le dock

    // =====================================================================
    @Override
    protected void onCreate(Bundle b) {
        super.onCreate(b);
        prefs = getSharedPreferences("zenhome", MODE_PRIVATE);
        ui = new Ui(this, this);
        apps = new Apps(this, ui);
        config = new HomeConfig(this, prefs, apps, this);
        config.loadInterval();
        config.load();
        ui.theme.set(prefs.getString("theme", ThemeRules.GLASS));
        dock = new Dock(this, ui, apps, config, this);
        carousel = new Carousel(this, ui, apps, prefs, config, this);
        carousel.load();
        async.start();
        wall = new Wallpaper(this, async, config.rotationMs);
        wall.theme = ui.theme;
        header = new Header(this, ui, prefs, async, config, this);
        weather = new Weather(this, ui, prefs, async);

        root = new FrameLayout(this);
        root.setBackgroundColor(Color.BLACK);
        root.setClipChildren(false);
        content = new FrameLayout(this);
        content.setClipChildren(false);
        ovs = new Overlays(this, ui, root, content, this);
        grid = new AppsGrid(this, ui, apps, ovs, this);
        inputs = new Inputs(this, ui, apps, ovs, this);
        settingsMenu = new SettingsMenu(this, ui, ovs);
        sleep = new Sleep(this, ui, prefs, ovs, new Runnable() { public void run() { shortcuts.sleepBadge(sleep.badgeText()); } });
        sleep.restore();
        shortcuts = new Shortcuts(this, ui, apps, root, this);
        detail = new MovieDetail(this, ui, prefs, async, ovs, apps, this);
        movies = new Movies(this, ui, prefs, async, ovs, this);
        editor = new Editor(this, ui, apps, grid, ovs, config, dock, carousel, this);
        levels = new Levels(ui, dock, carousel, movies, weather, shortcuts, editor, ovs, content, this);
        root.addView(content, new FrameLayout.LayoutParams(-1, -1));
        wall.build(content);
        FrameLayout uiLayer = new FrameLayout(this);   // toute l'interface au-dessus de la photo (s'efface en ambiance)
        uiLayer.setClipChildren(false);
        content.addView(uiLayer, new FrameLayout.LayoutParams(-1, -1));

        View top = new View(this);
        top.setBackground(new GradientDrawable(GradientDrawable.Orientation.TOP_BOTTOM, new int[]{0x8C000000, 0x00000000}));
        uiLayer.addView(top, new FrameLayout.LayoutParams(-1, ui.dp(200), Gravity.TOP));
        View bot = new View(this);
        bot.setBackground(new GradientDrawable(GradientDrawable.Orientation.BOTTOM_TOP, new int[]{0x99000000, 0x00000000}));
        uiLayer.addView(bot, new FrameLayout.LayoutParams(-1, ui.dp(340), Gravity.BOTTOM));

        header.addGreeting(uiLayer);

        weather.build(uiLayer);

        banner = new Banner(this, ui, prefs, async, weather, this);
        banner.build(uiLayer);

        movies.buildSpot(uiLayer);

        // Horloge + date
        header.addClock(uiLayer);
        shortcuts.buildRow(header.panel);

        editor.buildHint(uiLayer);

        dock.build(uiLayer);

        levels.build(uiLayer);

        movies.buildRail(uiLayer);

        carousel.build(uiLayer);

        shortcuts.buildBubble(uiLayer);

        ambient = new Ambient(ui, async, levels, ovs, editor, uiLayer, prefs);
        ambient.build(content);

        levels.init();
        dock.rebuild(0);

        setContentView(root);
        root.getViewTreeObserver().addOnGlobalFocusChangeListener(new android.view.ViewTreeObserver.OnGlobalFocusChangeListener() {
            public void onGlobalFocusChanged(View oldFocus, View newFocus) { levels.onFocus(newFocus); } });
        wall.loadWalls();
        wall.showWall(true);
    }

    // ---------- Host (appele par Ui.wire)
    public void onTileClick(View x, Runnable r) {
        if (editor.moveIndex >= 0) return;
        boolean car = carousel.contains(x), sc = x.getParent() == shortcuts.row, poster = x.getParent() == movies.nxRow;
        if (editor.editMode && sc) { editor.showTileMenu(-1); return; }   // raccourcis : menu general
        if (editor.editMode && car) { editor.showCarMenu(carousel.tiles.indexOf(x)); return; }   // la case « + » du carrousel : menu général
        if (editor.editMode && dock.contains(x)) { editor.showTileMenu(dock.bigTiles.indexOf(x)); return; }   // la case « + » et les petites tuiles : menu général
        if (dock.contains(x) || car || sc || poster) {
            // dock, affiches : on attend un peu, une rafale de OK (telecommandes Sony) vaut un appui long
            if (pendingClick != null) async.h.removeCallbacks(pendingClick);
            final Runnable run = r;
            pendingClick = new Runnable() { public void run() { pendingClick = null; run.run(); } };   // un clic execute n'est plus « en attente »
            async.h.postDelayed(pendingClick, BURST_GAP_MS);
        } else r.run();
    }

    public void onTileFocus(View x, boolean has) {
        if (x.getParent() == shortcuts.row) shortcuts.showBubble(x, has);
        if (x.getParent() == movies.nxRow) movies.scheduleSpot();
    }

    public int weatherCode() { return weather.currentCode(); }

    public void onDockBuilt() {
        editor.placeHint();
        placeRails();
    }

    public void applyTitle() { header.applyTitle(); }

    public void showAllApps() { grid.showApps(null); }

    public void showInputs() { inputs.show(); }

    public void cover(boolean on) { levels.cover(on); }

    public void showSleep() { sleep.show(); }

    public void showSettings() { settingsMenu.show(); }

    public void editHome() { editor.startEdit(); }

    public void nextWall() { wall.nextWall(); }

    public void backdropPrepare() { wall.prepare(); }
    public void backdropReady(android.graphics.Bitmap b) { wall.override(b); }
    public void backdropClear() { wall.clear(); }

    public void onSpot(boolean on) { weather.fade(on ? 0f : 1f); }

    public void onMovieClick(Movie m) { detail.show(m, movies.nxTab); }

    public boolean inMyList(Movie m) { return movies.inMyList(m); }
    public void toggleMyList(Movie m) { movies.toggleMyList(m); }

    public void carouselRefresh() { carousel.load(); carousel.rebuild(-1); }

    public boolean moving() { return editor.moveIndex >= 0; }
    public boolean editing() { return editor.editMode; }

    public void showTileMenu(int index) { editor.showTileMenu(index); }
    public void showCarMenu(int index) { editor.showCarMenu(index); }
    public void pickAppToAdd() { editor.pickAppToAdd(); }
    public void pickCarouselApp() { editor.pickCarouselApp(); }
    public void onAppLongPress(String pkg, View ov) { editor.onAppLongPress(pkg, ov); }

    public View currentFocus() { return getCurrentFocus(); }

    public Nav.State level() { return levels.level(); }

    public void focusLevel(Nav.State s) { levels.enter(s); }

    public void onOverlayClosed(View ov) { detail.onOverlayClosed(ov); if (ov == inputs.panel) { inputs.panel = null; levels.cover(false); } }

    /** Tuiles qui ouvrent un menu par appui long / rafale de OK : dock, carrousel, raccourcis. */
    boolean isMenuZone(View f) { return dock.contains(f) || carousel.contains(f) || f.getParent() == shortcuts.row || f.getParent() == movies.nxRow; }

    /** Appui long / rafale de OK : affiche -> « Ma liste » ; tuile -> menu contextuel. */
    void openMenu(View f) {
        if (f.getParent() == movies.nxRow) {
            int i = movies.nxRow.indexOfChild(f);
            if (i >= 0 && i < movies.movies.size()) movies.toggleMyList(movies.movies.get(i));
        } else editor.openTileMenu(f);
    }

    boolean swallowOkUp = false;

    // Certaines telecommandes (Sony a infrarouge) envoient un appui maintenu comme une rafale
    // d'appuis courts : BURST_PRESSES appuis espaces de moins de BURST_GAP_MS = appui long.
    static final long BURST_GAP_MS = 250;
    static final int BURST_PRESSES = 3;
    Runnable pendingClick;            // clic du dock en attente (annule si une rafale commence)
    View burstView;
    int okBurst = 0;
    long lastOkUp = 0, lastOkEvent = 0;
    boolean burstSwallow = false;     // rafale deja transformee en appui long : on avale la suite

    static boolean isOk(int c) {
        return c == KeyEvent.KEYCODE_DPAD_CENTER || c == KeyEvent.KEYCODE_ENTER || c == KeyEvent.KEYCODE_NUMPAD_ENTER;
    }

    @Override
    public boolean dispatchKeyEvent(KeyEvent e) {
        if (ambient.onKey(e)) return true;   // ambiance : la premiere touche reveille sans agir
        int kc = e.getKeyCode();
        // appui long sur OK, aussi pour les telecommandes qui envoient la touche en repetition
        if (isOk(kc) && editor.moveIndex < 0) {
            long t = e.getEventTime();
            if (burstSwallow) {
                if (t - lastOkEvent < BURST_GAP_MS) { lastOkEvent = t; return true; }
                burstSwallow = false;
            }
            lastOkEvent = t;
            if (e.getAction() == KeyEvent.ACTION_DOWN && e.getRepeatCount() == 0 && ovs.isEmpty()) {
                View f = getCurrentFocus();
                if (f != null && isMenuZone(f)) {
                    okBurst = (f == burstView && t - lastOkUp < BURST_GAP_MS) ? okBurst + 1 : 1;
                    burstView = f;
                    if (okBurst >= 2 && pendingClick != null) { async.h.removeCallbacks(pendingClick); pendingClick = null; }
                    if (okBurst >= BURST_PRESSES) {
                        okBurst = 0;
                        burstSwallow = true;
                        f.cancelLongPress(); f.setPressed(false);
                        openMenu(f);
                        return true;
                    }
                }
            }
            if (e.getAction() == KeyEvent.ACTION_UP) lastOkUp = t;
            if (e.getAction() == KeyEvent.ACTION_UP && swallowOkUp) { swallowOkUp = false; return true; }
            if (e.getAction() == KeyEvent.ACTION_DOWN && ovs.isEmpty() && (e.getRepeatCount() >= 1 || e.isLongPress())) {
                View f = getCurrentFocus();
                if (f != null && isMenuZone(f)) {
                    swallowOkUp = true;
                    f.cancelLongPress(); f.setPressed(false);
                    openMenu(f);
                    return true;
                }
            }
            if (swallowOkUp && e.getAction() == KeyEvent.ACTION_DOWN) return true;
        }
        if (editor.moveIndex >= 0 && e.getAction() == KeyEvent.ACTION_DOWN) return editor.onMoveKey(e);
        if (editor.moveIndex >= 0) return true; // ignore les relachements pendant le deplacement
        // Fleches : niveaux de l'accueil (Haut / Bas, et Gauche / Droite aux bords d'une rangee) ; les bords sont bloques
        if (e.getAction() == KeyEvent.ACTION_DOWN && ovs.isEmpty() && levels.key(e)) return true;
        // ◀ ▶ sur une pastille deja focalisee : seul cas ou le focus change d'onglet
        View ft = getCurrentFocus();
        boolean tabKey = ft != null && ft.getParent() == movies.nxTabsRow && e.getAction() == KeyEvent.ACTION_DOWN
                && (kc == KeyEvent.KEYCODE_DPAD_LEFT || kc == KeyEvent.KEYCODE_DPAD_RIGHT);
        if (tabKey) {
            // le focus ne bouge qu'apres dispatchKeyEvent (ViewRootImpl) : on le deplace nous-memes, drapeau leve pendant l'appel
            int cur = -1;
            for (int i = 0; i < movies.nxTabs.length; i++) if (movies.nxTabs[i] == ft) cur = i;
            int to = MovieRules.tabStep(cur, kc == KeyEvent.KEYCODE_DPAD_LEFT ? -1 : 1);
            if (to != cur) {
                movies.tabNav = true;
                try { movies.nxTabs[to].requestFocus(); } finally { movies.tabNav = false; }
            }
            return true;
        }
        return super.dispatchKeyEvent(e);
    }

    /** Touche Source / TV recue par KeyService : ouvre le panneau des entrees (thread UI). */
    void onSourceKey() {
        if (front != this || !ovs.isEmpty() || editor.moveIndex >= 0) return;
        ambient.reset();
        inputs.show();
    }

    /** Touche Reglages recue par KeyService : menu « Reglages » (thread UI). */
    void onSettingsKey() {
        if (front != this || !ovs.isEmpty() || editor.moveIndex >= 0) return;
        ambient.reset();
        settingsMenu.show();
    }

    public void focusDock() {
        levels.enter(Nav.State.REST);
    }

    /** Focus initial sur le dock ; si la fenetre n'a pas encore le focus, on re-essaie quand elle l'aura. */
    void requestDockFocus() {
        needDockFocus = true;
        dock.view.post(new Runnable() { public void run() {
            if (ovs.isEmpty() && editor.moveIndex < 0) focusDock();
            if (hasWindowFocus()) needDockFocus = false;
        } });
    }

    @Override
    public void onWindowFocusChanged(boolean has) {
        super.onWindowFocusChanged(has);
        if (has && needDockFocus) {
            needDockFocus = false;
            dock.view.post(new Runnable() { public void run() { if (ovs.isEmpty() && editor.moveIndex < 0) focusDock(); } });
        }
    }

    /**
     * Apres la reconstruction asynchrone d'une rangee (affiches, carrousel) : un view ajouté peut recevoir le
     * focus tout seul (aucune vue focalisee). Si le focus a migre vers une rangee alors que l'utilisateur n'y
     * etait pas -> dock ; s'il y etait (idx >= 0) -> meme position dans la rangee.
     */
    public void fixRowFocus(View before, int idx, LinearLayout row) {
        View now = getCurrentFocus();
        if (now == before || !ovs.isEmpty()) return;
        if (now == null) { levels.check(true); return; }   // focus perdu avec la rangee reconstruite : on le rend au niveau courant
        if (!(now.getParent() == movies.nxRow || now.getParent() == movies.nxTabsRow || carousel.contains(now))) return;
        if (idx >= 0 && row.getChildCount() > 0) row.getChildAt(Math.min(idx, row.getChildCount() - 1)).requestFocus();
        else focusDock();
    }

    @Override
    public boolean onKeyDown(int code, KeyEvent e) {
        if (code == KeyEvent.KEYCODE_BACK) {   // l'accueil ne se ferme jamais
            if (!ovs.isEmpty()) ovs.closeTopOverlay(); else if (editor.editMode) editor.endEdit(); else levels.back();
            return true;
        }
        // boutons d'applis de la telecommande TCL (si le service KeyService n'est pas actif)
        String keyApp = KeyService.appForKey(code);
        if (keyApp != null && ovs.isEmpty() && editor.moveIndex < 0 && apps.launchIntent(keyApp) != null) {
            if (e.getRepeatCount() == 0) apps.launch(keyApp);
            return true;
        }
        // touche Source / TV : sur certaines TV, seul l'accueil d'origine la gerait
        if ((code == KeyEvent.KEYCODE_TV_INPUT || code == KeyEvent.KEYCODE_TV) && ovs.isEmpty() && editor.moveIndex < 0) {
            inputs.show();
            return true;
        }
        if (code == KeyEvent.KEYCODE_MENU) {
            if (!ovs.isEmpty()) return true;
            View f = getCurrentFocus();
            if (f != null && carousel.contains(f)) editor.showCarMenu(carousel.tiles.indexOf(f));        // tuile : menu contextuel
            else if (f != null && dock.contains(f)) editor.showTileMenu(dock.bigTiles.indexOf(f));
            else if (f != null && f.getParent() == movies.nxRow) openMenu(f);                            // affiche : Ma liste
            else if (editor.editMode) editor.showTileMenu(-1);                                           // deja en edition : menu general
            else editor.startEdit();                                                                      // ailleurs : mode edition
            return true;
        }
        return super.onKeyDown(code, e);
    }

    @Override
    public void onBackPressed() { if (!ovs.isEmpty()) ovs.closeTopOverlay(); else if (editor.editMode) editor.endEdit(); else levels.back(); }

    @Override
    protected void onNewIntent(Intent i) {
        super.onNewIntent(i);
        // touche Accueil : ferme tout ce qui est ouvert
        ambient.reset();
        if (editor.moveIndex >= 0) editor.endMove();
        while (!ovs.isEmpty()) ovs.closeOverlay(ovs.top(), false);
        if (editor.editMode) editor.endEdit();
        levels.reset();   // repos, dock index 0
        requestDockFocus();
        dockOnResume = true;   // touche Accueil : toujours le dock
    }

    @Override
    protected void onResume() {
        super.onResume();
        front = this;
        async.fg = true;
        // la configuration a pu changer via le kit pendant que l'accueil etait en fond
        if (config.revChanged() && editor.moveIndex < 0 && ovs.isEmpty()) {
            config.load(); ui.theme.set(prefs.getString("theme", ThemeRules.GLASS)); header.applyTitle(); dock.rebuild(0);
        }
        async.h.removeCallbacksAndMessages(null);
        header.start();
        wall.start();
        ambient.reset();
        movies.scheduleSpot();   // le vidage de async.h ci-dessus a pu effacer un spotRun en attente (focus change avant onResume)
        if (dockOnResume && ovs.isEmpty() && editor.moveIndex < 0) requestDockFocus();   // lancement / touche Accueil : 1re tuile du dock
        dockOnResume = false;   // simple retour d'une appli ou d'un overlay : on garde la position
        // coordonnees / jeton / caches ont pu changer par adb : on relit les preferences, cache d'abord
        List<String> cp = carousel.compute();
        if (!cp.equals(carousel.pkgs) && editor.moveIndex < 0 && ovs.isEmpty()) { carousel.pkgs = cp; carousel.rebuild(-1); }
        if (!ui.theme.name.equals(ThemeRules.normalize(prefs.getString("theme", ThemeRules.GLASS)))) ui.theme.set(prefs.getString("theme", ThemeRules.GLASS));
        weather.render(); movies.render();
        detail.reloadBackdrop();
        banner.check(movies.myList());
    }

    @Override
    protected void onPause() {
        super.onPause();
        if (front == this) front = null;
        banner.stop(false);
        ambient.stop();
        async.onPause();
    }

    @Override
    protected void onStop() {
        super.onStop();
        movies.dropBackdrop();
        movies.hideSpot();   // plus de mise en avant en arriere-plan : la meteo est retablie au retour
        wall.release();   // libere la memoire des photos pendant qu'on regarde une video
        movies.releasePosters(); detail.dropBackdrop();
    }

    @Override
    protected void onStart() {
        super.onStart();
        if (wall.restore()) movies.loadPosters();
    }

    @Override
    protected void onDestroy() {
        if (front == this) front = null;
        async.onDestroy();
        detail.freeDetail();
        super.onDestroy();
    }

    /** Positionne le carrousel (sous le dock) et les films (au-dessus), puis verifie que le niveau courant existe encore. */
    public void placeRails() {
        if (carousel.view == null) return;
        carousel.place();
        movies.place(config.layout);
        if (levels != null) levels.check(false);
    }

    // =====================================================================
    // Reseau
    /** Appele toutes les 15 s par tick (au premier plan) : relance les requetes dont le cache est trop vieux. */
    public void dataTick() {
        if (!async.fg || async.destroyed) return;
        weather.render(); movies.render(); movies.loadPosters();
        long now = System.currentTimeMillis();
        shortcuts.sleepBadge(sleep.badgeText());
        weather.refresh(now);
        movies.refresh(now);
    }
}

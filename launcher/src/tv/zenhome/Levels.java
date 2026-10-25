package tv.zenhome;

import android.view.Gravity;
import android.view.KeyEvent;
import android.view.View;
import android.view.ViewGroup;
import android.view.animation.DecelerateInterpolator;
import android.widget.FrameLayout;
import android.widget.ImageView;

import java.util.HashMap;

/**
 * Accueil en niveaux : applique l'etat de Nav (focus + animations visuelles). L'etat est deduit de la vue qui a le focus
 * (onFocus), la table des transitions est dans Nav. Les conteneurs caches (films, carrousel) sont bloques au focus
 * (FOCUS_BLOCK_DESCENDANTS) puis INVISIBLE a la fin de l'animation ; pour entrer dans un niveau on debloque, on donne le
 * focus, puis on anime : le focus change tout de suite, l'animation n'est que visuelle.
 * Levels connait les modules qu'il pilote (comme Editor) : sinon Host aurait une dizaine de methodes de plus.
 */
class Levels {
    static final long MS = 200;

    final Ui ui;
    final Dock dock;
    final Carousel carousel;
    final Movies movies;
    final Weather weather;
    final Shortcuts shortcuts;
    final Editor editor;
    final Overlays ovs;
    final Host host;
    final View content;
    View chevron;   // « ⌄ » sous le dock : il y a des applis en dessous

    Nav.State cur = Nav.State.REST;
    // dernieres positions visitees (index dans la rangee)
    int lastDock, lastCar, lastPoster, lastShortcut;
    // etat visuel voulu de chaque vue animee : {translationY, alpha, cache ? 1 : 0}
    final HashMap<View, float[]> goal = new HashMap<>();

    Levels(Ui ui, Dock dock, Carousel carousel, Movies movies, Weather weather, Shortcuts shortcuts, Editor editor,
           Overlays ovs, View content, Host host) {
        this.ui = ui; this.dock = dock; this.carousel = carousel; this.movies = movies; this.weather = weather;
        this.shortcuts = shortcuts; this.editor = editor; this.ovs = ovs; this.content = content; this.host = host;
    }

    void build(ViewGroup parent) {
        FrameLayout.LayoutParams lp;
        if (ui.pReg != null) {
            chevron = ui.icon("caret-down", "⌄", 22, 0xFFFFFFFF);
            lp = new FrameLayout.LayoutParams(-2, -2, Gravity.BOTTOM | Gravity.CENTER_HORIZONTAL);
            lp.bottomMargin = ui.dp(12);
        } else {   // repli sans police
            ImageView iv = new ImageView(ui.ctx);
            iv.setImageDrawable(new Icons.ChevronIcon());
            chevron = iv;
            lp = new FrameLayout.LayoutParams(ui.dp(26), ui.dp(14), Gravity.BOTTOM | Gravity.CENTER_HORIZONTAL);
            lp.bottomMargin = ui.dp(16);
        }
        parent.addView(chevron, lp);
    }

    /** Panneau des entrees ouvert (on) ou ferme : le bloc du bas et les films disparaissent en fondu, puis reviennent dans l'etat du niveau. */
    void cover(boolean on) {
        for (View v : new View[]{dock.view, carousel.view, chevron, movies.nxBox, editor.hintView}) {
            if (v == null) continue;
            float[] g = goal.get(v);
            v.animate().alpha(on ? 0f : (g != null ? g[1] : 1f)).setDuration(MS).start();
        }
    }

    /** Etat initial (repos), sans animation : a appeler quand toutes les vues sont construites. */
    void init() { apply(Nav.State.REST, false); }

    // ---------- Etat <-> vue

    boolean inContent(View v) {
        while (v != null) { if (v == content) return true; v = v.getParent() instanceof View ? (View) v.getParent() : null; }
        return false;
    }

    /** Niveau de la vue, ou null si elle n'appartient a aucun niveau. */
    Nav.State stateOf(View f) {
        if (f.getParent() == shortcuts.row) return Nav.State.TOP;
        if (f == weather.line) return Nav.State.WEATHER;
        if (f.getParent() == movies.nxTabsRow) return Nav.State.TABS;
        if (f.getParent() == movies.nxRow) return Nav.State.FILMS;
        if (carousel.contains(f)) return Nav.State.APPS;
        if (dock.contains(f)) return Nav.State.REST;
        return null;
    }

    static View child(ViewGroup g, int i) {
        return g.getChildCount() == 0 ? null : g.getChildAt(Math.max(0, Math.min(i, g.getChildCount() - 1)));
    }

    View target(Nav.State s) {
        switch (s) {
            case REST: return dock.tileAt(lastDock);
            case APPS: return child(carousel.row, lastCar);
            case FILMS: return child(movies.nxRow, lastPoster);
            case TABS: return movies.nxTabs[movies.nxTab];
            case TOP: return child(shortcuts.row, lastShortcut);
            default: return weather.line;
        }
    }

    void remember(Nav.State s, View f) {
        switch (s) {
            case REST: lastDock = Math.max(0, dock.indexOf(f)); break;
            case APPS: lastCar = carousel.row.indexOfChild(f); break;
            case FILMS: lastPoster = movies.nxRow.indexOfChild(f); break;
            case TOP: lastShortcut = shortcuts.row.indexOfChild(f); break;
            default: break;
        }
    }

    Nav.Flags flags(View f) {
        return new Nav.Flags(movies.token().length() > 0, weather.available(), editor.editMode,
                f != null && shortcuts.row.getChildCount() > 0 && f == shortcuts.row.getChildAt(0));
    }

    /** Niveau courant : celui de la vue qui a le focus, sinon le dernier applique. */
    Nav.State level() {
        View f = host.currentFocus();
        Nav.State s = f != null ? stateOf(f) : null;
        return s != null ? s : cur;
    }

    // ---------- Entrees

    /** Le focus vient de changer (listener global) : on suit la vue focalisee. */
    void onFocus(View f) {
        if (f == null || !ovs.isEmpty() || !inContent(f)) return;
        Nav.State s = stateOf(f);
        if (s == null) return;
        remember(s, f);
        apply(s, true);
    }

    /**
     * Fleche directionnelle (ACTION_DOWN, aucun overlay) : renvoie vrai si Levels la consomme. Haut / Bas passent toujours
     * par Nav ; Gauche / Droite seulement aux bords d'une rangee (sinon : recherche de focus normale). Les bords sont
     * bloques : le focus ne saute jamais d'une rangee a l'autre.
     */
    boolean key(KeyEvent e) {
        int kc = e.getKeyCode();
        if (kc != KeyEvent.KEYCODE_DPAD_UP && kc != KeyEvent.KEYCODE_DPAD_DOWN
                && kc != KeyEvent.KEYCODE_DPAD_LEFT && kc != KeyEvent.KEYCODE_DPAD_RIGHT) return false;
        View f = host.currentFocus();
        if (f == null) { enter(cur); return true; }
        if (!inContent(f)) return false;
        Nav.State s = stateOf(f);
        if (s == null) return false;
        Nav.Key k;
        if (kc == KeyEvent.KEYCODE_DPAD_UP) k = Nav.Key.UP;
        else if (kc == KeyEvent.KEYCODE_DPAD_DOWN) k = Nav.Key.DOWN;
        else {
            boolean left = kc == KeyEvent.KEYCODE_DPAD_LEFT;
            View n = f.focusSearch(left ? View.FOCUS_LEFT : View.FOCUS_RIGHT);
            if (n != null && n != f && n.getParent() == f.getParent()) return false;   // voisin dans la rangee
            k = left ? Nav.Key.LEFT_EDGE : Nav.Key.RIGHT_EDGE;
        }
        if (e.getRepeatCount() > 0) return true;   // une touche maintenue ne traverse pas plusieurs niveaux
        Nav.State n = Nav.next(s, k, flags(f));
        if (n != s) enter(n);
        return true;
    }

    /** Retour : redescend au repos. */
    void back() {
        if (level() != Nav.State.REST) enter(Nav.State.REST);
    }

    /** Touche Accueil : repos, premiere tuile du dock, memoires effacees. */
    void reset() {
        lastDock = lastCar = lastPoster = lastShortcut = 0;
        enter(Nav.State.REST);
    }

    /** Va au niveau s : debloque ses conteneurs, donne le focus, puis anime. Niveau indisponible -> repos. */
    void enter(Nav.State s) {
        s = Nav.available(s, flags(null));
        if (s == Nav.State.FILMS && movies.movies.isEmpty()) s = Nav.State.TABS;   // rangee pas encore chargee
        View t = target(s);
        if (t == null) { s = Nav.State.REST; t = target(s); }
        unblock(s);
        if (t != null) t.requestFocus();
        apply(s, true);
    }

    /**
     * Apres un rendu (dock, films, meteo) : un niveau devenu indisponible ramene au repos (liste de films vide en FILMS,
     * plus de meteo en WEATHER...). regain : le focus a ete perdu avec une rangee reconstruite, on le rend au niveau.
     */
    void check(boolean regain) {
        if (!ovs.isEmpty() || editor.moveIndex >= 0) return;
        View f = host.currentFocus();
        Nav.State s = f != null ? stateOf(f) : null;
        if (s == null) s = cur;
        Nav.State r = Nav.available(s, flags(null));
        if (r == Nav.State.FILMS && movies.movies.isEmpty()) r = Nav.State.REST;
        if (r != s || (regain && f == null)) enter(r); else apply(s, true);
    }

    // ---------- Rendu

    void unblock(Nav.State s) {
        if (filmsShown(s)) show(movies.nxBox);
        if (s == Nav.State.APPS) show(carousel.view);
    }

    static void show(View v) {
        v.setVisibility(View.VISIBLE);
        if (v instanceof ViewGroup) ((ViewGroup) v).setDescendantFocusability(ViewGroup.FOCUS_AFTER_DESCENDANTS);
    }

    /** Les films (onglets + affiches) restent affiches tant qu'on est dans la partie haute : on les traverse en montant. */
    boolean filmsShown(Nav.State s) {
        return movies.token().length() > 0 && !editor.editMode
                && (s == Nav.State.FILMS || s == Nav.State.TABS || s == Nav.State.TOP || s == Nav.State.WEATHER);
    }

    void apply(Nav.State s, boolean anim) {
        cur = s;
        boolean films = filmsShown(s), apps = s == Nav.State.APPS;
        float shift = apps ? -ui.dp(DockMetrics.appsShift()) : 0;
        go(movies.nxBox, films ? 0 : ui.dp(40), films ? 1f : 0f, true, !films, anim);
        go(carousel.view, apps ? shift : ui.dp(60), apps ? 1f : 0f, true, !apps, anim);   // hors ecran tant qu'il n'est pas revele
        go(dock.view, shift, films ? 0.55f : 1f, false, false, anim);
        go(editor.hintView, shift, 1f, false, false, anim);
        go(chevron, 0, !apps && !films ? 0.5f : 0f, false, false, anim);
        go(weather.ext, 0, s == Nav.State.WEATHER ? 1f : 0f, false, false, anim);
    }

    /**
     * Amene v a (ty, alpha). gated : conteneur de focus (films, carrousel) debloque + visible tout de suite s'il apparait ;
     * s'il disparait, bloque tout de suite puis INVISIBLE a la fin de l'animation. Calque materiel pendant l'animation seulement.
     */
    void go(final View v, float ty, float a, final boolean gated, boolean hidden, boolean anim) {
        float[] g = goal.get(v);
        if (g != null && g[0] == ty && g[1] == a && g[2] == (hidden ? 1f : 0f)) return;
        goal.put(v, new float[]{ty, a, hidden ? 1f : 0f});
        v.animate().cancel();
        if (gated) {
            if (!hidden) show(v);
            else if (v instanceof ViewGroup) ((ViewGroup) v).setDescendantFocusability(ViewGroup.FOCUS_BLOCK_DESCENDANTS);
        }
        if (!anim) {
            v.setTranslationY(ty); v.setAlpha(a);
            if (gated && hidden) v.setVisibility(View.INVISIBLE);
            return;
        }
        v.animate().translationY(ty).alpha(a).setDuration(MS).setInterpolator(new DecelerateInterpolator()).withLayer()
                .withEndAction(new Runnable() { public void run() {
                    float[] c = goal.get(v);
                    if (gated && c != null && c[2] == 1f) v.setVisibility(View.INVISIBLE);
                } }).start();
    }
}

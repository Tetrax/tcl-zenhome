package tv.zenhome;

import android.content.SharedPreferences;
import android.view.Gravity;
import android.view.KeyEvent;
import android.view.View;
import android.view.ViewGroup;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.TextView;

import java.util.Calendar;
import java.util.Date;

/**
 * Mode ambiance : apres 3 min sans touche (premier plan, aucun overlay, pas d'edition), l'UI s'efface en fondu et il reste
 * la photo, l'heure et la date en grand (voile sombre le soir). La premiere touche reveille et est consommee.
 * Le minuteur est dans async.h (vide dans onPause) ; aucune requete reseau en plus.
 */
class Ambient {
    static final long IDLE_MS = 3 * 60_000L, FADE_MS = 400, CLOCK_MS = 15_000L;
    static final float NIGHT_VEIL = 0.35f;

    final Ui ui;
    final Async async;
    final Levels levels;
    final Overlays ovs;
    final Editor editor;
    final View uiLayer;   // tout l'interface (la photo est dessous)
    FrameLayout layer;    // voile + heure en grand
    View veil;
    TextView bigClock, bigDate;
    Standby standby;   // meteo, prochaine sortie, minuteur, phrase du jour
    final SharedPreferences prefs;
    boolean active = false;
    Nav.State before = Nav.State.REST;
    int wakeKey = -1;     // touche qui a reveille : son relachement est avale

    Ambient(Ui ui, Async async, Levels levels, Overlays ovs, Editor editor, View uiLayer, SharedPreferences prefs) {
        this.prefs = prefs; this.ui = ui; this.async = async; this.levels = levels; this.ovs = ovs; this.editor = editor; this.uiLayer = uiLayer;
    }

    void build(ViewGroup parent) {
        layer = new FrameLayout(ui.ctx);
        layer.setVisibility(View.INVISIBLE);
        layer.setAlpha(0f);
        veil = new View(ui.ctx);
        veil.setBackgroundColor(0xFF000000);
        layer.addView(veil, new FrameLayout.LayoutParams(-1, -1));
        bigClock = ui.text("", 120, ui.fLight, 0xFFFFFFFF);
        bigDate = ui.text("", 24, ui.fReg, 0xE6FFFFFF);
        LinearLayout col = new LinearLayout(ui.ctx);
        col.setOrientation(LinearLayout.VERTICAL);
        col.addView(bigClock); col.addView(bigDate);
        standby = new Standby(ui.ctx, ui, prefs, 16);
        col.addView(standby.box);
        FrameLayout.LayoutParams lp = new FrameLayout.LayoutParams(-2, -2, Gravity.BOTTOM | Gravity.START);
        lp.setMargins(ui.dp(80), 0, 0, ui.dp(70));
        layer.addView(col, lp);
        parent.addView(layer, new FrameLayout.LayoutParams(-1, -1));
    }

    /** Touche (appelee en tete de dispatchKeyEvent) : vrai si elle est consommee. */
    boolean onKey(KeyEvent e) {
        if (active) {
            if (e.getAction() == KeyEvent.ACTION_DOWN) { wakeKey = e.getKeyCode(); wake(true); }
            return true;
        }
        if (e.getAction() == KeyEvent.ACTION_UP && e.getKeyCode() == wakeKey) { wakeKey = -1; return true; }
        touch();
        return false;
    }

    /** (Re)arme le minuteur d'inactivite ; rien ne tourne hors du premier plan. */
    void touch() {
        async.h.removeCallbacks(idle);
        if (async.fg) async.h.postDelayed(idle, IDLE_MS);
    }

    final Runnable idle = new Runnable() { public void run() {
        Nav.Flags f = levels.flags(null);
        boolean ok = async.fg && ovs.isEmpty() && editor.moveIndex < 0;
        if (Nav.next(levels.level(), Nav.Key.IDLE, new Nav.Flags(f.films, f.weather, f.edit, false, ok)) == Nav.State.AMBIENT) enter();
        else touch();
    } };

    final Runnable clockTick = new Runnable() { public void run() {
        if (!active) return;
        fillClock();
        async.h.postDelayed(this, CLOCK_MS);
    } };

    void fillClock() {
        Date n = new Date();
        bigClock.setText(Header.time(ui.ctx, n));
        bigDate.setText(Header.dateText(ui, n));
        standby.update(false);   // a la minute
    }

    void enter() {
        active = true;
        before = levels.level();
        standby.update(true);
        fillClock();
        veil.setAlpha(Greeting.night(Calendar.getInstance().get(Calendar.HOUR_OF_DAY)) ? NIGHT_VEIL : 0f);
        layer.animate().cancel(); uiLayer.animate().cancel();
        layer.setVisibility(View.VISIBLE);
        layer.animate().alpha(1f).setDuration(FADE_MS).withLayer().start();
        uiLayer.animate().alpha(0f).setDuration(FADE_MS).withLayer().start();
        async.h.postDelayed(clockTick, CLOCK_MS);
    }

    /** Reveil : l'UI revient en fondu, on retrouve le niveau d'avant (repos s'il n'existe plus). */
    void wake(boolean anim) {
        boolean was = active;
        active = false;
        async.h.removeCallbacks(clockTick);
        layer.animate().cancel(); uiLayer.animate().cancel();
        if (anim) {
            layer.animate().alpha(0f).setDuration(FADE_MS).withLayer().withEndAction(new Runnable() { public void run() {
                if (!active) layer.setVisibility(View.INVISIBLE); } }).start();
            uiLayer.animate().alpha(1f).setDuration(FADE_MS).withLayer().start();
        } else {
            layer.setAlpha(0f); layer.setVisibility(View.INVISIBLE); uiLayer.setAlpha(1f);
        }
        if (was) {
            Nav.State r = Nav.wake(before, levels.flags(null));
            if (r != before) levels.enter(r);
        }
        touch();
    }

    /** onResume / touche Accueil : sortie immediate. */
    void reset() { wakeKey = -1; wake(false); }

    /** onPause : plus de minuteur (async.h est vide), UI restauree. */
    void stop() {
        active = false; wakeKey = -1;
        async.h.removeCallbacks(idle); async.h.removeCallbacks(clockTick);
        layer.animate().cancel(); uiLayer.animate().cancel();
        layer.setAlpha(0f); layer.setVisibility(View.INVISIBLE); uiLayer.setAlpha(1f);
    }
}

package tv.zenhome;

import android.content.SharedPreferences;
import android.service.dreams.DreamService;
import android.view.Gravity;
import android.view.View;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.TextView;

import java.util.Date;
import java.util.Random;

/**
 * Economiseur d'ecran : la photo de fond (Wallpaper, rotation lente, fondu) et l'horloge, qui change de place
 * toutes les ~60 s pour ne pas marquer l'ecran. Aucun reseau. Rien n'est active par l'application : voir les reglages
 * Settings.Secure (screensaver_components) envoyes par adb.
 */
public class ZenDream extends DreamService {
    static final long ROTATE_MS = 2 * 60_000L, MOVE_MS = 60_000L, CLOCK_MS = 15_000L, DRIFT_MS = 4000L;

    final Async async = new Async();
    final Random rnd = new Random();
    Ui ui;
    Wallpaper wall;
    LinearLayout col;
    TextView clock, date;
    Standby standby;
    SharedPreferences prefs;

    @Override
    public void onAttachedToWindow() {
        super.onAttachedToWindow();
        setInteractive(false);   // une touche quelconque termine l'economiseur
        setFullscreen(true);
        ui = new Ui(this, null);
        prefs = getSharedPreferences("zenhome", MODE_PRIVATE);
        wall = new Wallpaper(this, async, ROTATE_MS);
        FrameLayout root = new FrameLayout(this);
        root.setBackgroundColor(0xFF000000);
        wall.build(root);
        View veil = new View(this);
        veil.setBackgroundColor(0x40000000);   // photo un peu plus sombre qu'a l'accueil
        root.addView(veil, new FrameLayout.LayoutParams(-1, -1));
        clock = ui.text("", 90, ui.fLight, 0xFFFFFFFF);
        date = ui.text("", 22, ui.fReg, 0xE6FFFFFF);
        col = new LinearLayout(this);
        col.setOrientation(LinearLayout.VERTICAL);
        col.addView(clock); col.addView(date);
        standby = new Standby(this, ui, prefs, 14);
        col.addView(standby.box);
        root.addView(col, new FrameLayout.LayoutParams(-2, -2, Gravity.TOP | Gravity.START));
        setContentView(root);
    }

    @Override
    public void onDreamingStarted() {
        super.onDreamingStarted();
        async.fg = true;
        wall.loadWalls();
        wall.showWall(true);
        wall.start();
        async.h.post(clockTick);
        col.post(new Runnable() { public void run() { move(false); } });
    }

    final Runnable clockTick = new Runnable() { public void run() {
        Date n = new Date();
        clock.setText(Header.time(ZenDream.this, n));
        date.setText(Header.dateText(ui, n));
        standby.update(false);   // a la minute
        async.h.postDelayed(this, CLOCK_MS);
    } };

    final Runnable mover = new Runnable() { public void run() { move(true); } };

    /** Nouvelle position aleatoire (marges de 60 unites), atteinte en glissant lentement. */
    void move(boolean anim) {
        FrameLayout p = (FrameLayout) col.getParent();
        int maxX = Math.max(1, p.getWidth() - col.getWidth() - ui.dp(120)), maxY = Math.max(1, p.getHeight() - col.getHeight() - ui.dp(120));
        float x = ui.dp(60) + rnd.nextInt(maxX), y = ui.dp(60) + rnd.nextInt(maxY);
        if (anim) col.animate().translationX(x).translationY(y).setDuration(DRIFT_MS).start();
        else { col.setTranslationX(x); col.setTranslationY(y); }
        async.h.postDelayed(mover, MOVE_MS);
    }

    @Override
    public void onDreamingStopped() {
        async.fg = false;
        async.h.removeCallbacksAndMessages(null);
        super.onDreamingStopped();
    }

    @Override
    public void onDetachedFromWindow() {
        async.h.removeCallbacksAndMessages(null);
        if (col != null) col.animate().cancel();
        if (wall != null) wall.release();   // libere les bitmaps
        super.onDetachedFromWindow();
    }
}

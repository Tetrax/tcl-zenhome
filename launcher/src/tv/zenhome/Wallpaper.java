package tv.zenhome;

import android.content.Context;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.graphics.drawable.BitmapDrawable;
import android.graphics.drawable.Drawable;
import android.os.Environment;
import android.view.ViewGroup;
import android.widget.FrameLayout;
import android.widget.ImageView;

import java.io.File;
import java.io.InputStream;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Locale;

/** Photo de fond : liste (fichiers ou assets), decodage, fondu entre deux photos, rotation, liberation / reprise. */
class Wallpaper {
    static final long FADE_MS = 2500L, OVERRIDE_FADE_MS = 700L;

    final Context ctx;
    final Async async;
    final long rotationMs;
    Theme theme;   // teinte « auto » : echantillonne chaque photo / backdrop affiche (null : economiseur)
    FrameLayout wallLayer;
    ImageView wallA, wallB;
    boolean aOnTop = true, released = false;
    boolean overridden = false;   // un backdrop de film remplace la photo (rotation suspendue)
    long fadeUntil = 0;
    List<String> walls = new ArrayList<>();
    boolean wallsFromFiles = false;
    int wallIdx = 0;

    Wallpaper(Context ctx, Async async, long rotationMs) {
        this.ctx = ctx; this.async = async; this.rotationMs = rotationMs;
    }

    void build(ViewGroup parent) {
        wallA = newWall(); wallB = newWall();
        wallLayer = new FrameLayout(ctx);
        wallLayer.addView(wallB); wallLayer.addView(wallA);
        parent.addView(wallLayer, new FrameLayout.LayoutParams(-1, -1));
    }

    ImageView newWall() {
        ImageView v = new ImageView(ctx);
        v.setScaleType(ImageView.ScaleType.CENTER_CROP);
        v.setLayoutParams(new FrameLayout.LayoutParams(-1, -1));
        return v;
    }

    // =====================================================================
    // Fonds d'ecran
    void loadWalls() {
        walls.clear();
        wallsFromFiles = false;
        try {
            File dir = new File(Environment.getExternalStorageDirectory(), "ZenHome");
            File[] fs = dir.listFiles();
            if (fs != null) {
                Arrays.sort(fs);
                for (File f : fs) {
                    String n = f.getName().toLowerCase(Locale.ROOT);
                    if (f.isFile() && (n.endsWith(".jpg") || n.endsWith(".jpeg") || n.endsWith(".png") || n.endsWith(".webp")))
                        walls.add(f.getAbsolutePath());
                }
            }
        } catch (Throwable ignored) { }
        if (!walls.isEmpty()) { wallsFromFiles = true; }
        else {
            try {
                String[] a = ctx.getAssets().list("walls");
                if (a != null) { Arrays.sort(a); for (String n : a) { String l = n.toLowerCase(Locale.ROOT); if (l.endsWith(".jpg") || l.endsWith(".png")) walls.add(n); } }
            } catch (Exception ignored) { }
        }
        wallIdx = walls.isEmpty() ? 0 : (int) ((System.currentTimeMillis() / rotationMs) % walls.size());
    }

    Bitmap decode(String name) {
        try {
            BitmapFactory.Options o = new BitmapFactory.Options();
            int maxW = Math.max(ctx.getResources().getDisplayMetrics().widthPixels, 1280);
            if (wallsFromFiles) {
                o.inJustDecodeBounds = true;
                BitmapFactory.decodeFile(name, o);
                int s = 1;
                while (o.outWidth / (s * 2) >= maxW) s *= 2;
                o = new BitmapFactory.Options();
                o.inSampleSize = s;
                o.inPreferredConfig = Bitmap.Config.RGB_565;
                return BitmapFactory.decodeFile(name, o);
            }
            o.inPreferredConfig = Bitmap.Config.RGB_565;
            InputStream is = ctx.getAssets().open("walls/" + name);
            Bitmap b = BitmapFactory.decodeStream(is, null, o);
            is.close();
            return b;
        } catch (Throwable t) { return null; }
    }

    void showWall(boolean instant) {
        if (walls.isEmpty()) return;
        Bitmap bm = decode(walls.get(wallIdx));
        if (bm == null) return;
        swap(bm, instant, FADE_MS);
    }

    /** Met bm sur la couche cachee et la fait apparaitre par-dessus (fondu fadeMs) ; l'ancien contenu de cette couche est recycle. */
    void swap(Bitmap bm, boolean instant, long fadeMs) {
        if (theme != null) theme.onPhoto(bm);
        final ImageView next = aOnTop ? wallB : wallA;
        final ImageView cur = aOnTop ? wallA : wallB;
        fadeUntil = System.currentTimeMillis() + (instant ? 0 : fadeMs);
        Drawable old = next.getDrawable();
        next.setImageBitmap(bm);
        if (old instanceof BitmapDrawable) {
            Bitmap ob = ((BitmapDrawable) old).getBitmap();
            if (ob != null && ob != bm) ob.recycle();
        }
        next.bringToFront(); // uniquement dans le calque des photos
        if (instant) { next.setAlpha(1f); cur.setAlpha(0f); }
        else {
            next.setAlpha(0f);
            android.animation.ObjectAnimator.ofFloat(next, "alpha", 0f, 1f).setDuration(fadeMs).start();
        }
        aOnTop = !aOnTop;
    }

    final Runnable rotate = new Runnable() {
        public void run() {
            if (walls.size() > 1 && !overridden) { wallIdx = (wallIdx + 1) % walls.size(); showWall(false); }
            async.h.postDelayed(this, rotationMs);
        }
    };

    /** Lance la rotation (onResume, apres le vidage de async.h). */
    void start() { async.h.postDelayed(rotate, rotationMs); }

    // Bouton « Fond » : photo suivante tout de suite, minuteur de rotation relance
    void nextWall() {
        if (walls.size() < 2) return;
        wallIdx = (wallIdx + 1) % walls.size();
        if (!overridden) showWall(false);
        async.h.removeCallbacks(rotate);
        async.h.postDelayed(rotate, rotationMs);
    }

    /**
     * Avant de decoder un backdrop : libere la couche cachee pour ne jamais garder plus de 2 bitmaps plein ecran
     * (sauf pendant un fondu de photo, ou la couche cachee est encore partiellement visible).
     */
    void prepare() {
        if (System.currentTimeMillis() < fadeUntil) return;
        ImageView hidden = aOnTop ? wallB : wallA;
        Drawable d = hidden.getDrawable(); hidden.setImageDrawable(null);
        if (d instanceof BitmapDrawable) { Bitmap b = ((BitmapDrawable) d).getBitmap(); if (b != null) b.recycle(); }
    }

    /** Backdrop du film focalise : remplace la photo en fondu et suspend la rotation. */
    void override(Bitmap bm) {
        if (released) { bm.recycle(); return; }
        overridden = true;
        swap(bm, false, OVERRIDE_FADE_MS);
    }

    /** Retour a la photo courante en fondu. */
    void clear() {
        if (!overridden) return;
        overridden = false;
        prepare();
        showWall(false);
    }

    /** onStop : libere la memoire des photos pendant qu'on regarde une video. */
    void release() {
        for (ImageView v : new ImageView[]{wallA, wallB}) {
            v.animate().cancel();
            Drawable d = v.getDrawable(); v.setImageDrawable(null);
            if (d instanceof BitmapDrawable) { Bitmap b = ((BitmapDrawable) d).getBitmap(); if (b != null) b.recycle(); }
        }
        released = true; overridden = false;
    }

    /** onStart : recharge les photos si elles avaient ete liberees ; renvoie vrai dans ce cas. */
    boolean restore() {
        if (!released) return false;
        released = false; loadWalls(); showWall(true);
        return true;
    }
}

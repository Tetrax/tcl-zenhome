package tv.zenhome;

import android.content.Context;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.util.TypedValue;
import android.view.View;
import android.view.animation.DecelerateInterpolator;
import android.widget.FrameLayout;
import android.widget.HorizontalScrollView;
import android.widget.LinearLayout;
import android.widget.TextView;
import android.widget.Toast;

import java.util.Locale;

/** Densite, polices, langue et fabriques de vues communes. Le clic / focus des tuiles remonte par Host. */
class Ui {
    static final int ACCENT = 0xFF7CC4FF;
    static final String PIC = "\uE000";   // marqueur de texte : remplace par un glyphe Phosphor (picText)

    final Context ctx;
    final Host host;
    final Theme theme = new Theme();   // couleurs de verre (themes) : jamais codees en dur dans les modules
    Typeface fReg, fMed, fLight;
    Typeface pReg, pFill;   // icones Phosphor (Regular / Fill) ; null si absentes : Icons.LineIcon prend le relais
    float density;
    boolean fr;

    Ui(Context ctx, Host host) {
        this.ctx = ctx;
        this.host = host;
        fr = "fr".equals(Locale.getDefault().getLanguage());
        android.util.DisplayMetrics dm = ctx.getResources().getDisplayMetrics();
        density = Math.max(dm.widthPixels, dm.heightPixels) / 1280f; // maquette sur 1280 unites de large
        try {
            fReg = Typeface.createFromAsset(ctx.getAssets(), "Poppins-Regular.ttf");
            fMed = Typeface.createFromAsset(ctx.getAssets(), "Poppins-Medium.ttf");
            fLight = Typeface.createFromAsset(ctx.getAssets(), "Poppins-Light.ttf");
        } catch (Exception e) { fReg = fMed = fLight = Typeface.SANS_SERIF; }
        try {
            pReg = Typeface.createFromAsset(ctx.getAssets(), "Phosphor-Regular-subset.ttf");
            pFill = Typeface.createFromAsset(ctx.getAssets(), "Phosphor-Fill-subset.ttf");
        } catch (Exception e) { pReg = pFill = null; }
    }

    int dp(float v) { return Math.round(v * density); }
    String t(String frTxt, String enTxt) { return fr ? frTxt : enTxt; }

    /** Glyphe Phosphor seul dans un TextView ; repli : le texte fallback en police normale si la police manque. */
    TextView icon(String name, String fallback, float size, int color) {
        TextView tv = text(pReg != null ? IconRules.glyph(name) : fallback, size, pReg != null ? pReg : fLight, color);
        tv.setGravity(android.view.Gravity.CENTER);
        return tv;
    }

    /** Remplace le marqueur PIC de s par le glyphe Phosphor (taille scale x texte), ou par fallback si la police manque. */
    CharSequence picText(String s, String name, String fallback, float scale) {
        int i = s.indexOf(PIC);
        if (i < 0) return s;
        if (pReg == null) return s.replace(PIC, fallback);
        android.text.SpannableStringBuilder b = new android.text.SpannableStringBuilder(s);
        String g = IconRules.glyph(name);
        b.replace(i, i + PIC.length(), g);
        b.setSpan(new GlyphSpan(pReg, scale), i, i + g.length(), android.text.Spanned.SPAN_EXCLUSIVE_EXCLUSIVE);
        return b;
    }

    /** Police Phosphor (et taille) appliquee a un morceau de texte. */
    static class GlyphSpan extends android.text.style.MetricAffectingSpan {
        final Typeface tf;
        final float scale;
        GlyphSpan(Typeface tf, float scale) { this.tf = tf; this.scale = scale; }
        void apply(android.text.TextPaint p) { p.setTypeface(tf); p.setTextSize(p.getTextSize() * scale); }
        @Override public void updateMeasureState(android.text.TextPaint p) { apply(p); }
        @Override public void updateDrawState(android.text.TextPaint p) { apply(p); }
    }

    void toast(String s) { Toast.makeText(ctx, s, Toast.LENGTH_SHORT).show(); }

    TextView text(String s, float size, Typeface tf, int color) {
        TextView tv = new TextView(ctx);
        tv.setText(s); tv.setTextColor(color); tv.setTypeface(tf);
        tv.setTextSize(TypedValue.COMPLEX_UNIT_PX, size * density);
        tv.setShadowLayer(dp(6), 0, dp(1), 0x80000000);
        tv.setIncludeFontPadding(false);
        return tv;
    }

    FrameLayout roundedFrame(final int radius, int color) {
        FrameLayout f = new FrameLayout(ctx);
        GradientDrawable bg = new GradientDrawable(); bg.setCornerRadius(dp(radius)); bg.setColor(color);
        f.setBackground(bg);
        f.setClipToOutline(true);
        f.setOutlineProvider(new android.view.ViewOutlineProvider() {
            public void getOutline(View v, android.graphics.Outline o) { o.setRoundRect(0, 0, v.getWidth(), v.getHeight(), dp(radius)); } });
        return f;
    }

    void wire(final View v, final Runnable r, final float scale) {
        v.setFocusable(true);
        v.setClickable(true);
        v.setOnClickListener(new View.OnClickListener() { public void onClick(View x) { host.onTileClick(x, r); } });
        v.setOnFocusChangeListener(new View.OnFocusChangeListener() {
            public void onFocusChange(View x, boolean has) {
                x.animate().scaleX(has ? scale : 1f).scaleY(has ? scale : 1f)
                        .translationY(has ? -dp(6) : 0).translationZ(has ? dp(12) : 0)
                        .setDuration(180).setInterpolator(new DecelerateInterpolator()).start();
                host.onTileFocus(x, has);
                int r = x.getTag() instanceof Integer ? (Integer) x.getTag() : 14;
                x.setForeground(has ? ring(r, host.moving() || host.editing() ? ACCENT : 0xF2FFFFFF) : null);
            }
        });
    }

    GradientDrawable ring(int r, int color) {
        GradientDrawable g = new GradientDrawable();
        g.setCornerRadius(dp(r)); g.setColor(0x00000000); g.setStroke(dp(host.moving() ? 3 : 2), color);
        return g;
    }

    // Rangees defilantes (carrousel, Netflix) : sans barre, bords en fondu
    HorizontalScrollView rail(LinearLayout row) {
        HorizontalScrollView s = new HorizontalScrollView(ctx);
        s.setHorizontalScrollBarEnabled(false);
        s.setOverScrollMode(View.OVER_SCROLL_NEVER);
        s.setHorizontalFadingEdgeEnabled(true);
        s.setFadingEdgeLength(dp(40));
        s.setClipChildren(false); s.setClipToPadding(false);
        s.addView(row, new FrameLayout.LayoutParams(-2, -2));
        return s;
    }
}

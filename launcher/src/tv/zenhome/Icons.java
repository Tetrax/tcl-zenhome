package tv.zenhome;

import android.graphics.drawable.Drawable;

/** Icones dessinees des raccourcis. */
class Icons {
    /** Jauge en anneau : fond, arc de progression (depart en haut, sens horaire, extremites arrondies), orange au-dela de 85 %. */
    static class RingGauge extends Drawable {
        final android.graphics.Paint p = new android.graphics.Paint(android.graphics.Paint.ANTI_ALIAS_FLAG);
        final android.graphics.RectF r = new android.graphics.RectF();
        int pct = -1;
        RingGauge() { p.setStyle(android.graphics.Paint.Style.STROKE); p.setStrokeCap(android.graphics.Paint.Cap.ROUND); }
        void set(int pct) { if (pct != this.pct) { this.pct = pct; invalidateSelf(); } }
        @Override public void draw(android.graphics.Canvas c) {
            android.graphics.Rect b = getBounds();
            float s = Math.min(b.width(), b.height()), sw = s * 0.14f;   // 4 unites pour 28
            p.setStrokeWidth(sw);
            r.set(b.exactCenterX() - s / 2f + sw / 2, b.exactCenterY() - s / 2f + sw / 2, b.exactCenterX() + s / 2f - sw / 2, b.exactCenterY() + s / 2f - sw / 2);
            p.setColor(0x40FFFFFF);
            c.drawArc(r, 0, 360, false, p);
            float sweep = GaugeRules.sweep(pct);
            if (sweep > 0) {
                p.setColor(GaugeRules.warn(pct) ? 0xFFFFB020 : 0xF2FFFFFF);
                c.drawArc(r, -90, sweep, false, p);
            }
        }
        @Override public void setAlpha(int a) { p.setAlpha(a); }
        @Override public void setColorFilter(android.graphics.ColorFilter f) { p.setColorFilter(f); }
        @Override public int getOpacity() { return android.graphics.PixelFormat.TRANSLUCENT; }
    }

    /** Glyphe Phosphor centre : Regular au repos, Fill sombre au focus. */
    static class GlyphIcon extends Drawable {
        final android.graphics.Paint p = new android.graphics.Paint(android.graphics.Paint.ANTI_ALIAS_FLAG);
        final android.graphics.Typeface reg, fill;
        final String g;
        GlyphIcon(android.graphics.Typeface reg, android.graphics.Typeface fill, String g) {
            this.reg = reg; this.fill = fill; this.g = g;
            p.setTextAlign(android.graphics.Paint.Align.CENTER);
            focus(false);
        }
        /** null si les polices ou le glyphe manquent (l'appelant se rabat sur LineIcon). */
        static GlyphIcon make(android.graphics.Typeface reg, android.graphics.Typeface fill, String g) {
            if (reg == null || fill == null || g.isEmpty()) return null;
            if (android.os.Build.VERSION.SDK_INT >= 23) {
                android.graphics.Paint t = new android.graphics.Paint();
                t.setTypeface(reg); if (!t.hasGlyph(g)) return null;
                t.setTypeface(fill); if (!t.hasGlyph(g)) return null;
            }
            return new GlyphIcon(reg, fill, g);
        }
        void focus(boolean has) { p.setTypeface(has ? fill : reg); p.setColor(has ? 0xFF111111 : 0xF2FFFFFF); invalidateSelf(); }
        @Override public void draw(android.graphics.Canvas c) {
            android.graphics.Rect b = getBounds();
            p.setTextSize(b.height());
            c.drawText(g, b.exactCenterX(), b.exactCenterY() - (p.ascent() + p.descent()) / 2f, p);
        }
        @Override public void setAlpha(int a) { p.setAlpha(a); }
        @Override public void setColorFilter(android.graphics.ColorFilter f) { p.setColorFilter(f); }
        @Override public int getOpacity() { return android.graphics.PixelFormat.TRANSLUCENT; }
    }

    /** Repli des icones Phosphor : trait fin blanc a extremites arrondies (dessinees dans un carre). */
    static class LineIcon extends Drawable {
        static final int INPUT = 0, GRID = 1, GEAR = 2, PENCIL = 3, IMAGE = 4, MOON = 5, PLAY = 6;
        final android.graphics.Paint p = new android.graphics.Paint(android.graphics.Paint.ANTI_ALIAS_FLAG);
        final android.graphics.Path path = new android.graphics.Path(), cut = new android.graphics.Path();
        final int kind;
        float x0, y0, s;
        LineIcon(int kind) {
            this.kind = kind;
            p.setColor(0xFFFFFFFF); p.setStrokeCap(android.graphics.Paint.Cap.ROUND); p.setStrokeJoin(android.graphics.Paint.Join.ROUND);
        }
        void setColor(int c) { p.setColor(c); invalidateSelf(); }
        float X(float f) { return x0 + s * f; }
        float Y(float f) { return y0 + s * f; }
        void line(android.graphics.Canvas c, float ax, float ay, float bx, float by) { c.drawLine(X(ax), Y(ay), X(bx), Y(by), p); }
        @Override public void draw(android.graphics.Canvas c) {
            android.graphics.Rect b = getBounds();
            s = Math.min(b.width(), b.height()); x0 = b.exactCenterX() - s / 2f; y0 = b.exactCenterY() - s / 2f;
            p.setStrokeWidth(s * 0.09f);   // ~2 unites pour 22
            p.setStyle(android.graphics.Paint.Style.STROKE);
            switch (kind) {
                case INPUT:   // fleche qui entre dans une porte
                    path.reset(); path.moveTo(X(0.58f), Y(0.18f)); path.lineTo(X(0.84f), Y(0.18f)); path.lineTo(X(0.84f), Y(0.82f)); path.lineTo(X(0.58f), Y(0.82f));
                    c.drawPath(path, p);
                    line(c, 0.14f, 0.5f, 0.60f, 0.5f);
                    path.reset(); path.moveTo(X(0.42f), Y(0.32f)); path.lineTo(X(0.60f), Y(0.5f)); path.lineTo(X(0.42f), Y(0.68f));
                    c.drawPath(path, p);
                    break;
                case GRID:   // 3 x 3 points
                    p.setStyle(android.graphics.Paint.Style.FILL);
                    for (int i = 0; i < 3; i++) for (int j = 0; j < 3; j++) c.drawCircle(X(0.2f + 0.3f * j), Y(0.2f + 0.3f * i), s * 0.07f, p);
                    break;
                case GEAR:   // couronne, moyeu et 8 dents
                    c.drawCircle(X(0.5f), Y(0.5f), s * 0.25f, p);
                    c.drawCircle(X(0.5f), Y(0.5f), s * 0.09f, p);
                    for (int k = 0; k < 8; k++) {
                        double a = k * Math.PI / 4;
                        float cx = (float) Math.cos(a), cy = (float) Math.sin(a);
                        line(c, 0.5f + 0.25f * cx, 0.5f + 0.25f * cy, 0.5f + 0.38f * cx, 0.5f + 0.38f * cy);
                    }
                    break;
                case PENCIL:   // crayon incline : corps, bague et pointe
                    c.save(); c.rotate(45, b.exactCenterX(), b.exactCenterY());
                    path.reset(); path.moveTo(X(0.40f), Y(0.10f)); path.lineTo(X(0.60f), Y(0.10f)); path.lineTo(X(0.60f), Y(0.68f));
                    path.lineTo(X(0.50f), Y(0.88f)); path.lineTo(X(0.40f), Y(0.68f)); path.close();
                    c.drawPath(path, p);
                    line(c, 0.40f, 0.26f, 0.60f, 0.26f);
                    c.restore();
                    break;
                case IMAGE:   // cadre, montagne et soleil
                    c.drawRoundRect(X(0.12f), Y(0.2f), X(0.88f), Y(0.8f), s * 0.1f, s * 0.1f, p);
                    path.reset(); path.moveTo(X(0.18f), Y(0.72f)); path.lineTo(X(0.40f), Y(0.46f)); path.lineTo(X(0.55f), Y(0.63f));
                    path.lineTo(X(0.65f), Y(0.53f)); path.lineTo(X(0.82f), Y(0.72f));
                    c.drawPath(path, p);
                    p.setStyle(android.graphics.Paint.Style.FILL);
                    c.drawCircle(X(0.68f), Y(0.36f), s * 0.05f, p);
                    break;
                case MOON: {   // croissant en contour
                    float r = s * 0.34f, cx = X(0.47f), cy = Y(0.5f);
                    path.reset(); path.addCircle(cx, cy, r, android.graphics.Path.Direction.CW);
                    cut.reset(); cut.addCircle(cx + r * 0.55f, cy - r * 0.35f, r * 0.82f, android.graphics.Path.Direction.CW);
                    path.op(cut, android.graphics.Path.Op.DIFFERENCE);
                    c.drawPath(path, p);
                    break;
                }
                default:   // PLAY : triangle en contour
                    path.reset(); path.moveTo(X(0.32f), Y(0.2f)); path.lineTo(X(0.8f), Y(0.5f)); path.lineTo(X(0.32f), Y(0.8f)); path.close();
                    c.drawPath(path, p);
            }
        }
        @Override public void setAlpha(int a) { p.setAlpha(a); }
        @Override public void setColorFilter(android.graphics.ColorFilter f) { p.setColorFilter(f); }
        @Override public int getOpacity() { return android.graphics.PixelFormat.TRANSLUCENT; }
    }

    /** Icone "toutes les applis" : grille 3x3 centree. */
    static class GridIcon extends Drawable {
        final android.graphics.Paint p = new android.graphics.Paint(android.graphics.Paint.ANTI_ALIAS_FLAG);
        GridIcon() { p.setColor(0xFFFFFFFF); }
        @Override public void draw(android.graphics.Canvas c) {
            android.graphics.Rect b = getBounds();
            float size = Math.min(b.width(), b.height()) * 0.74f;
            float cell = size / 3f, sq = cell * 0.66f, r = sq * 0.3f;
            float x0 = b.exactCenterX() - size / 2f, y0 = b.exactCenterY() - size / 2f;
            for (int i = 0; i < 3; i++) for (int j = 0; j < 3; j++) {
                float x = x0 + j * cell + (cell - sq) / 2f, y = y0 + i * cell + (cell - sq) / 2f;
                c.drawRoundRect(x, y, x + sq, y + sq, r, r, p);
            }
        }
        @Override public void setAlpha(int a) { p.setAlpha(a); }
        @Override public void setColorFilter(android.graphics.ColorFilter f) { p.setColorFilter(f); }
        @Override public int getOpacity() { return android.graphics.PixelFormat.TRANSLUCENT; }
    }

    /** Petit chevron « ⌄ » (dessine : aucune dependance a la police). */
    static class ChevronIcon extends Drawable {
        final android.graphics.Paint p = new android.graphics.Paint(android.graphics.Paint.ANTI_ALIAS_FLAG);
        final android.graphics.Path v = new android.graphics.Path();
        ChevronIcon() {
            p.setColor(0xFFFFFFFF); p.setStyle(android.graphics.Paint.Style.STROKE);
            p.setStrokeCap(android.graphics.Paint.Cap.ROUND); p.setStrokeJoin(android.graphics.Paint.Join.ROUND);
        }
        @Override public void draw(android.graphics.Canvas c) {
            android.graphics.Rect b = getBounds();
            float w = b.width() * 0.8f, h = b.height() * 0.5f, cx = b.exactCenterX(), cy = b.exactCenterY();
            p.setStrokeWidth(b.height() * 0.16f);
            v.reset();
            v.moveTo(cx - w / 2, cy - h / 2); v.lineTo(cx, cy + h / 2); v.lineTo(cx + w / 2, cy - h / 2);
            c.drawPath(v, p);
        }
        @Override public void setAlpha(int a) { p.setAlpha(a); }
        @Override public void setColorFilter(android.graphics.ColorFilter f) { p.setColorFilter(f); }
        @Override public int getOpacity() { return android.graphics.PixelFormat.TRANSLUCENT; }
    }
}

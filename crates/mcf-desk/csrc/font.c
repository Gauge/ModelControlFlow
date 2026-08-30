/* The whole of MCF's text stack.
 *
 * stb_truetype is a single public-domain header that turns an outline font
 * into coverage bitmaps. It is compiled here, once, with its implementation
 * switched on; `build.rs` hands the object to the linker.
 *
 * Two functions cross into Rust, and stb's own are not among them. That is
 * deliberate: `stbtt_fontinfo` is 160-odd bytes whose layout is stb's
 * business, and a mirror of it on the Rust side would be a copy of somebody
 * else's private struct that nothing checks and that breaks silently when it
 * changes. Keeping it here costs a shim and buys a boundary that is two
 * functions wide.
 *
 * stb ships a baker of its own, `stbtt_BakeFontBitmap`, and it is not used:
 * it takes a *contiguous* run of codepoints, and MCF's interface needs
 * degrees, middots and multiplication signs without also carrying every
 * accented letter between them. The packer below takes a list.
 *
 * Licence: MIT or public domain, at the recipient's choice. The text is in
 * stb_truetype.LICENSE beside this file, and doc/vendored.md carries what was
 * checked and against what.
 */

#define STB_TRUETYPE_IMPLEMENTATION
#include "stb_truetype.h"

/* Where one glyph landed in the atlas, and what it does to the pen.
 *
 * Mirrored in Rust as `font::Glyph`, and a test compares the two sizes — a
 * struct that crosses a C boundary has its layout checked rather than
 * assumed.
 */
typedef struct {
    short x0, y0, x1, y1;      /* the glyph's box within the atlas       */
    float x_off, y_off;        /* where to put that box, pen-relative    */
    float advance;             /* how far the pen then moves             */
} mcf_glyph;

/* Vertical metrics, scaled to a pixel height.
 *
 * Returns 0 if the bytes are not a font MCF can read — a real outcome, not a
 * crash: the file came off somebody's machine and nothing promised it was a
 * font (A2).
 */
int mcf_font_metrics(const unsigned char *ttf, int index, float pixels,
                     float *ascent, float *descent, float *line_gap)
{
    stbtt_fontinfo info;
    int offset = stbtt_GetFontOffsetForIndex(ttf, index);
    if (offset < 0) return 0;
    if (!stbtt_InitFont(&info, ttf, offset)) return 0;

    int a = 0, d = 0, g = 0;
    stbtt_GetFontVMetrics(&info, &a, &d, &g);
    float scale = stbtt_ScaleForPixelHeight(&info, pixels);

    *ascent   = (float)a * scale;
    *descent  = (float)d * scale;   /* negative, as stb reports it */
    *line_gap = (float)g * scale;
    return 1;
}

/* Rasterises `count` codepoints into `atlas` and says where each one went.
 *
 * Shelf packing: glyphs go left to right along a row, and a full row starts a
 * new one. It is not the tightest packing there is, and it does not need to
 * be — this runs a handful of times at startup over a few hundred glyphs.
 *
 * Returns 1 when every glyph fit. Returns 0 when the font is unreadable or
 * the atlas ran out of room, and writes nothing more; the caller then tries a
 * larger atlas rather than drawing text with holes in it (A2, A7).
 */
int mcf_font_bake(const unsigned char *ttf, int index, float pixels,
                  const unsigned int *codepoints, int count,
                  unsigned char *atlas, int width, int height,
                  mcf_glyph *out)
{
    stbtt_fontinfo info;
    int offset = stbtt_GetFontOffsetForIndex(ttf, index);
    if (offset < 0) return 0;
    if (!stbtt_InitFont(&info, ttf, offset)) return 0;

    float scale = stbtt_ScaleForPixelHeight(&info, pixels);
    for (int i = 0; i < width * height; ++i) atlas[i] = 0;

    /* One transparent pixel is left at the origin. Drawing a rectangle is
     * then the same operation as drawing a glyph with a different source
     * box, which is how the panels and the text end up in one pass. */
    int pen_x = 1, pen_y = 1, row_height = 1;

    for (int i = 0; i < count; ++i) {
        int x0 = 0, y0 = 0, x1 = 0, y1 = 0;
        stbtt_GetCodepointBitmapBox(&info, (int)codepoints[i], scale, scale,
                                    &x0, &y0, &x1, &y1);
        int w = x1 - x0, h = y1 - y0;

        if (pen_x + w + 1 > width) {      /* next shelf */
            pen_x = 1;
            pen_y += row_height + 1;
            row_height = 0;
        }
        if (pen_y + h + 1 > height) return 0;   /* no room: say so */

        if (w > 0 && h > 0) {
            stbtt_MakeCodepointBitmap(&info, atlas + pen_y * width + pen_x,
                                      w, h, width, scale, scale,
                                      (int)codepoints[i]);
        }

        int advance = 0, bearing = 0;
        stbtt_GetCodepointHMetrics(&info, (int)codepoints[i], &advance, &bearing);

        out[i].x0 = (short)pen_x;
        out[i].y0 = (short)pen_y;
        out[i].x1 = (short)(pen_x + w);
        out[i].y1 = (short)(pen_y + h);
        out[i].x_off = (float)x0;
        out[i].y_off = (float)y0;
        out[i].advance = (float)advance * scale;

        pen_x += w + 1;
        if (h > row_height) row_height = h;
    }
    return 1;
}

/* The size of the struct above, so Rust can check its mirror rather than
 * trust it. */
int mcf_glyph_bytes(void) { return (int)sizeof(mcf_glyph); }

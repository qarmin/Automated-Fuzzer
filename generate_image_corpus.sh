#!/bin/bash
# Generate a tiny-but-broad image fuzzing corpus.
#
# Goal: cover as many image *formats*, *bit depths*, *colour spaces*,
# *encodings/compressions* and *channel layouts* as possible while keeping
# every file tiny (base canvas is 16x16) so the whole set stays well under
# 100 MB. Byte-identical outputs are de-duplicated at the end.
#
# Everything is best-effort: combinations the local ImageMagick can't produce
# are silently skipped, so the script is safe to re-run as delegates change.
#
# Usage: bash generate_image_corpus.sh [OUTPUT_DIR]
set -u

DIR="${1:-AA_IMAGE_FUZZ_CORPUS}"
rm -rf "$DIR"
mkdir -p "$DIR"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# ── Master images ────────────────────────────────────────────────────────
# A 16x16 RGBA image: red→blue colour gradient with a black→white alpha
# gradient. The smooth gradients exercise bit-depth quantisation; the colour
# range exercises colour-space conversion; the alpha exercises channel layout.
M="$WORK/master_rgba.png"
magick -size 16x16 gradient:red-blue \
  \( -size 16x16 gradient:black-white \) \
  -alpha off -compose CopyOpacity -composite "$M"

# Opaque RGB master (no alpha) — many formats don't support alpha.
MRGB="$WORK/master_rgb.png"
magick "$M" -alpha remove -alpha off "$MRGB"

# Grayscale master.
MGRAY="$WORK/master_gray.png"
magick "$M" -alpha remove -colorspace Gray "$MGRAY"

count() { find "$DIR" -type f | wc -l; }

# g <name> <magick-args...>   (output path is appended automatically)
# Keeps the file only if magick succeeded AND produced non-empty output.
g() {
  local out="$DIR/$1"; shift
  magick "$@" "$out" 2>/dev/null
  [ -s "$out" ] || rm -f "$out"
}

echo "Generating into $DIR ..."

# ── PNG: depths × channel layouts × interlace ─────────────────────────────
for depth in 1 2 4 8 16; do
  for type in Bilevel Grayscale GrayscaleAlpha Palette PaletteAlpha TrueColor TrueColorAlpha; do
    for il in None PNG; do
      g "png__d${depth}__${type}__il-${il}.png" "$M" -depth "$depth" -type "$type" -interlace "$il"
    done
  done
done
# Explicit PNG coder variants (palette/16-bit forced by the coder, not heuristics)
for coder in PNG8 PNG24 PNG32 PNG48 PNG64; do
  out="$DIR/png__coder-${coder}.png"; magick "$M" "${coder}:$out" 2>/dev/null; [ -s "$out" ] || rm -f "$out"
done

# ── JPEG: baseline/progressive × chroma subsampling × quality × colour ────
for q in 5 25 60 95 100; do
  for samp in 4:4:4 4:2:2 4:2:0; do
    for il in None JPEG; do
      g "jpeg__q${q}__s${samp//:/-}__il-${il}.jpg" "$MRGB" -quality "$q" -sampling-factor "$samp" -interlace "$il"
    done
  done
done
g "jpeg__grayscale.jpg"   "$MGRAY" -quality 80
g "jpeg__cmyk.jpg"        "$MRGB"  -colorspace CMYK -quality 80
g "jpeg__q100_444.jpg"    "$MRGB"  -quality 100 -sampling-factor 4:4:4

# ── TIFF: compression × depth × colour space × planar config ──────────────
for comp in None LZW Zip JPEG PackBits RLE LZMA Zstd; do
  for depth in 1 8 16; do
    g "tiff__c-${comp}__d${depth}.tif" "$M" -compress "$comp" -depth "$depth"
  done
done
g "tiff__group4_bilevel.tif" "$MGRAY" -monochrome -compress Group4
g "tiff__cmyk.tif"           "$MRGB"  -colorspace CMYK -compress LZW
g "tiff__planar.tif"         "$M"     -define tiff:planar-config=separate -compress LZW
g "tiff__bigendian.tif"      "$M"     -define tiff:endian=msb -compress None
g "tiff__littleendian.tif"   "$M"     -define tiff:endian=lsb -compress None
g "tiff__float.tif"          "$M"     -depth 32 -define quantum:format=floating-point

# ── BMP: header versions × depth ──────────────────────────────────────────
for coder in BMP2 BMP3 BMP; do
  for depth in 1 4 8 16 24; do
    out="$DIR/bmp__${coder}__d${depth}.bmp"
    magick "$M" -depth "$depth" "${coder}:$out" 2>/dev/null; [ -s "$out" ] || rm -f "$out"
  done
done
g "bmp__rgba32.bmp" "$M" -type TrueColorAlpha

# ── GIF: interlace × palette size ─────────────────────────────────────────
for colors in 2 16 256; do
  g "gif__pal${colors}.gif"        "$M" -colors "$colors"
  g "gif__pal${colors}__il.gif"    "$M" -colors "$colors" -interlace GIF
done

# ── WebP: lossless / lossy ────────────────────────────────────────────────
g "webp__lossless.webp"   "$M"   -define webp:lossless=true
g "webp__lossy_q20.webp"  "$MRGB" -quality 20
g "webp__lossy_q80.webp"  "$M"    -quality 80
g "webp__exact.webp"      "$M"    -define webp:exact=true -define webp:lossless=true

# ── PNM family: ASCII vs binary, all sub-formats ──────────────────────────
# -compress None => ASCII (P1/P2/P3); default => binary (P4/P5/P6).
g "pbm__ascii.pbm"  "$MGRAY" -monochrome -compress None
g "pbm__binary.pbm" "$MGRAY" -monochrome
g "pgm__ascii.pgm"  "$MGRAY" -compress None
g "pgm__binary.pgm" "$MGRAY"
g "pgm__d16.pgm"    "$MGRAY" -depth 16
g "ppm__ascii.ppm"  "$MRGB"  -compress None
g "ppm__binary.ppm" "$MRGB"
g "ppm__d16.ppm"    "$MRGB"  -depth 16
g "pnm__binary.pnm" "$MRGB"
g "pam__rgba.pam"   "$M"
g "pfm__float.pfm"  "$MRGB"

# ── HDR / floating-point / scientific ─────────────────────────────────────
g "hdr__rgbe.hdr"   "$MRGB"
g "exr__half.exr"   "$M"
g "exr__cmyk.exr"   "$MRGB" -colorspace CMYK
g "fits__gray.fits" "$MGRAY"
g "dpx__rgb.dpx"    "$MRGB"

# ── JPEG 2000 / JNG ───────────────────────────────────────────────────────
g "jp2__q50.jp2"    "$MRGB" -quality 50
g "jp2__lossless.jp2" "$M"  -quality 100
g "j2k__codestream.j2k" "$MRGB"
g "jng__alpha.jng"  "$M"

# ── Modern codecs (HEIC / AVIF / JXL) ─────────────────────────────────────
g "heic__q50.heic"  "$MRGB" -quality 50
g "avif__lossy.avif" "$MRGB" -quality 50
g "avif__lossless.avif" "$M" -define heic:lossless=true
g "jxl__lossy.jxl"  "$MRGB" -quality 80
g "jxl__lossless.jxl" "$M"  -quality 100

# ── zune-specific niche formats ───────────────────────────────────────────
g "qoi__rgba.qoi"        "$M"
g "qoi__rgb.qoi"         "$MRGB"
g "farbfeld__rgba16.ff"  "$M"
g "psd__rgba.psd"        "$M"
g "psd__cmyk.psd"        "$MRGB" -colorspace CMYK
g "psd__gray.psd"        "$MGRAY"

# ── Other / legacy formats (one or two variants each) ─────────────────────
g "tga__norle.tga"   "$M" -compress None
g "tga__rle.tga"     "$M" -compress RLE
g "sgi__rgb.sgi"     "$MRGB"
g "sun__rast.ras"    "$MRGB"
g "palm__pal.palm"   "$M" -colors 16
g "pcx__d8.pcx"      "$M" -depth 8
g "xpm__pal.xpm"     "$M" -colors 16
g "xbm__bilevel.xbm" "$MGRAY" -monochrome
g "wbmp__bilevel.wbmp" "$MGRAY" -monochrome
g "mng__anim.mng"    "$M"
g "otb__bilevel.otb" "$MGRAY" -monochrome
g "dds__dxt.dds"     "$M"
g "ico__multi.ico"   "$M" -define icon:auto-resize=16
g "cur__cursor.cur"  "$M"
g "miff__native.miff" "$M"
g "dcx__multi.dcx"   "$M"

# ── Colour-space sweep on a stable carrier (TIFF, lossless) ───────────────
for cs in Gray sRGB RGB CMYK YCbCr Lab HSL CMY OHTA YIQ YUV XYZ; do
  g "colorspace__${cs}.tif" "$MRGB" -colorspace "$cs" -compress LZW
done

# ── Dimension edge cases (1px strips, single pixel) — a few key formats ────
for fmt in png bmp ppm tiff jpg gif qoi webp; do
  magick -size 1x1   gradient:red-blue "$DIR/edge__1x1.$fmt"   2>/dev/null || true
  magick -size 1x16  gradient:red-blue "$DIR/edge__1x16.$fmt"  2>/dev/null || true
  magick -size 16x1  gradient:red-blue "$DIR/edge__16x1.$fmt"  2>/dev/null || true
done
for f in "$DIR"/edge__*; do [ -s "$f" ] || rm -f "$f"; done

# ── Every additional REAL image format ImageMagick can write ──────────────
# One small file per coder so the corpus covers formats the current target
# doesn't support yet (other libraries / future expansion). This is an
# *allow-list* of genuine, self-describing raster image formats only — it
# deliberately excludes things no image decoder reads: documents/vector
# (pdf, ps, eps, ai, pcl, wpg), headerless raw dumps (rgb, bgr, cmyk, gray,
# yuv, ycbcr, bayer, uyvy), engine-internal/meta (miff, mpc, vips, mat, map,
# pal, json/yaml/txt, histogram, null), video containers, and braille/sixel
# text encodings. Best-effort: coders that fail or yield empty output drop.
ALLOW='^(aai|apng|art|avci|avif|avs|bie|bmp|bmp2|bmp3|cal|cals|cin|cip|cur|dcx|dds|dpx|dxt1|dxt5|farbfeld|fax|ff|fits|fts|g3|g4|gif|gif87|group4|hdr|heic|hrz|icb|ico|ipl|j2c|j2k|jbg|jbig|jng|jp2|jpc|jpe|jpeg|jpg|jpm|jps|jpt|jxl|mng|mtv|otb|palm|pam|pbm|pcd|pcds|pct|pcx|pdb|pfm|pgm|pgx|phm|picon|pict|pjpeg|png|png00|png24|png32|png48|png64|png8|pnm|ppm|psb|psd|ptif|qoi|ras|rgf|sf3|sgi|sun|tga|tiff|tiff64|vda|vicar|viff|vst|wbmp|webp|xbm|xpm|xwd)$'
magick -list format 2>/dev/null \
  | awk 'NR>2 && NF>=4 { fmt=$1; mode=$3; gsub(/\*/,"",fmt); if (substr(mode,2,1)=="w") print tolower(fmt) }' \
  | sort -u \
  | while read -r coder; do
      [[ "$coder" =~ $ALLOW ]] || continue
      out="$DIR/all__${coder}.${coder}"
      magick "$M" "${coder}:$out" 2>/dev/null
      [ -s "$out" ] || rm -f "$out"
    done

# ── ffmpeg-only still-image formats not covered by ImageMagick ────────────
if command -v ffmpeg >/dev/null 2>&1; then
  ff() { ffmpeg -loglevel error -y -i "$M" "$DIR/$1" >/dev/null 2>&1; [ -s "$DIR/$1" ] || rm -f "$DIR/$1"; }
  ff "ffmpeg__jpegls.jls"   # JPEG-LS — a real codec ImageMagick lacks
  ff "ffmpeg__ljpeg.ljpg"   # lossless JPEG
fi

echo "Generated $(count) files (before dedup)."

# ── De-duplicate byte-identical outputs to keep the set minimal ───────────
if [ -f dedup_files.py ]; then
  python3 dedup_files.py "$DIR" >/dev/null 2>&1 || true
fi

TOTAL_KB=$(du -sk "$DIR" | awk '{print $1}')
echo "Final: $(count) files, $((TOTAL_KB)) KB total in $DIR"

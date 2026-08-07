#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PROJECT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
SOURCE_IMAGE="$PROJECT_DIR/assets/ngneon_icon.png"
OUTPUT_DIR="$PROJECT_DIR/assets/intro"
OUTPUT_VIDEO="$OUTPUT_DIR/ngneon-intro.mp4"
FONT_FILE=/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf

command -v ffmpeg >/dev/null 2>&1 || {
    echo "Error: ffmpeg no esta instalado." >&2
    exit 1
}

test -f "$SOURCE_IMAGE" || {
    echo "Error: no existe $SOURCE_IMAGE" >&2
    exit 1
}

test -f "$FONT_FILE" || {
    echo "Error: no existe $FONT_FILE" >&2
    exit 1
}

mkdir -p "$OUTPUT_DIR"

ffmpeg -hide_banner -loglevel warning -y \
    -f lavfi -i "color=c=0x020208:s=1280x720:r=60:d=6.2" \
    -loop 1 -framerate 60 -i "$SOURCE_IMAGE" \
    -f lavfi -i "sine=frequency=392:sample_rate=48000:duration=0.75" \
    -f lavfi -i "sine=frequency=523.25:sample_rate=48000:duration=0.9" \
    -f lavfi -i "sine=frequency=783.99:sample_rate=48000:duration=1.2" \
    -filter_complex "
        [0:v]drawgrid=w=iw:h=4:t=1:c=0x182040@0.16[background];
        [1:v]format=rgba,
            scale=w='trunc((80+500*min(max((t-0.45)/1.15,0),1))/2)*2':h='trunc((80+500*min(max((t-0.45)/1.15,0),1))/2)*2':eval=frame,
            fade=t=in:st=0.40:d=0.45:alpha=1,
            fade=t=out:st=5.05:d=0.55:alpha=1,
            split=2[logo][glow];
        [glow]gblur=sigma=28,colorchannelmixer=aa=0.55[logo_glow];
        [background][logo_glow]overlay=x='(W-w)/2':y='(H-h)/2-35':format=auto[with_glow];
        [with_glow][logo]overlay=x='(W-w)/2':y='(H-h)/2-35':format=auto,
            drawtext=fontfile='$FONT_FILE':text='NGNEON-EMU':fontsize=48:fontcolor=white:borderw=2:bordercolor=0x00cfff:x=(w-text_w)/2:y=610:alpha='if(lt(t,1.75),0,if(lt(t,2.45),(t-1.75)/0.70,if(lt(t,5.0),1,max(0,(5.60-t)/0.60))))',
            drawtext=fontfile='$FONT_FILE':text='NEO GEO EMULATION SYSTEM':fontsize=21:fontcolor=0xff42d0:x=(w-text_w)/2:y=670:alpha='if(lt(t,2.25),0,if(lt(t,2.85),(t-2.25)/0.60,if(lt(t,5.0),0.92,max(0,(5.60-t)/0.60))))',
            fade=t=out:st=5.60:d=0.60[video];
        [2:a]afade=t=out:st=0.30:d=0.45,adelay=700:all=1[note1];
        [3:a]afade=t=out:st=0.40:d=0.50,adelay=1050:all=1[note2];
        [4:a]afade=t=out:st=0.55:d=0.65,adelay=1400:all=1[note3];
        [note1][note2][note3]amix=inputs=3:duration=longest:normalize=0,
            volume=0.22,apad=pad_dur=6.2[audio]
    " \
    -map "[video]" -map "[audio]" -t 6.2 \
    -c:v libx264 -preset medium -crf 18 -pix_fmt yuv420p \
    -movflags +faststart -c:a aac -b:a 160k "$OUTPUT_VIDEO"

echo "Intro creada: $OUTPUT_VIDEO"


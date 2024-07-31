_default:
    just --list

# yes i know these require you to have an nvidia gpu but idrc yknow its not like
# anyone's ever gonna actually use this

test_frames:
    ffmpeg -f image2 -r 60 -pattern_type glob -i "frames/*.png" -vf "scale=-2:720:flags=neighbor" -pix_fmt yuv420p -c:v h264_nvenc -qp 27 -f matroska - | mpv -loop -

output_frames:
    ffmpeg -f image2 -r 60 -pattern_type glob -i "frames/*.png" -vf "scale=-2:720:flags=neighbor" -pix_fmt yuv420p -c:v h264_nvenc -qp 27 -y testington.mp4

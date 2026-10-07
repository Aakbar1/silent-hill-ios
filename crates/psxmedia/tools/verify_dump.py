"""Optional independent FFmpeg oracle. Requires Pillow and imageio-ffmpeg.

Install those only in a local private venv; this script never installs anything.
All input slices, decoded game data and comparison reports stay in private/work/media.
The native Rust library does not use this script or FFmpeg.
"""

import array
import csv
import json
import subprocess
import wave
from pathlib import Path

import imageio_ffmpeg
from PIL import Image, ImageDraw

PRIVATE = Path("C:/Claude Projects/Silent Hill iOS/private")
OUTPUT = PRIVATE / "work/media"
LBA = 133127


def main():
    with (PRIVATE / "disc/Silent Hill (USA).bin").open("rb") as disc:
        disc.seek(LBA * 2352)
        (OUTPUT / "intro300.str").write_bytes(disc.read(3000 * 2352))
    binary = imageio_ffmpeg.get_ffmpeg_exe()
    for args in [
        ["-map", "0:v:0", "-frames:v", "300", "-f", "rawvideo", "-pix_fmt", "rgba", str(OUTPUT / "reference.rgba")],
        ["-map", "0:a:0", "-c:a", "pcm_s16le", str(OUTPUT / "reference.wav")],
    ]:
        subprocess.run([binary, "-v", "error", "-y", "-i", str(OUTPUT / "intro300.str"), *args], check=True)

    audio = []
    for name in ["intro-20s.wav", "reference.wav"]:
        with wave.open(str(OUTPUT / name)) as wav:
            assert (wav.getnchannels(), wav.getsampwidth(), wav.getframerate(), wav.getnframes()) == (2, 2, 37800, 756000)
            audio.append(array.array("h", wav.readframes(wav.getnframes())))
    assert audio[0] == audio[1], "native XA differs from the independent decoder"

    reference = (OUTPUT / "reference.rgba").read_bytes()
    frame_bytes = 320 * 208 * 4
    assert len(reference) == frame_bytes * 300
    total_error = total = within_two = maximum = 0
    per_frame = []
    for index in range(300):
        native = Image.open(OUTPUT / "frames" / f"{index+1:04}.png").tobytes()
        expected = reference[index * frame_bytes : (index + 1) * frame_bytes]
        assert len(native) == len(expected)
        errors = [abs(a-b) for i, (a, b) in enumerate(zip(native, expected)) if i % 4 != 3]
        total_error += sum(errors)
        total += len(errors)
        maximum = max(maximum, max(errors))
        within_two += sum(e <= 2 for e in errors)
        per_frame.append(sum(errors) / len(errors))
    # These guard against frame/chroma/block-order faults, while allowing the
    # documented PSX/FFmpeg differences in integer IDCT and colour rounding.
    assert total_error / total < 1 and maximum <= 8

    with (OUTPUT / "timeline.csv").open() as timeline:
        rows = list(csv.DictReader(timeline))
    video = [r for r in rows if r["kind"] == "video"]
    xa = [r for r in rows if r["kind"] == "audio"]
    assert len(video) == 300 and len(xa) == 375
    residuals = [abs(float(r["pts_seconds"]) - (int(r["sector"]) - origin) / 150)
                 for track, origin in [(video, LBA), (xa, LBA+7)] for r in track]
    assert max(residuals) < 1e-8
    assert all(abs(float(t[-1]["pts_seconds"]) + float(t[-1]["duration_seconds"]) - 20) < 1e-8 for t in [video, xa])

    canvas = Image.new("RGB", (1280, 696))
    draw = ImageDraw.Draw(canvas)
    for index, frame in enumerate([1,30,60,90,120,150,180,210,240,260,280,300]):
        x, y = (index % 4) * 320, (index // 4) * 232
        canvas.paste(Image.open(OUTPUT / "frames" / f"{frame:04}.png").convert("RGB"), (x,y))
        draw.text((x+4,y+210), f"Frame {frame}, {(frame-1)/15:.2f}s", fill="white")
    canvas.save(OUTPUT / "contact-sheet.png")
    summary = {"reference": "FFmpeg 7.1 via local imageio-ffmpeg", "audio_samples_exact": len(audio[0]),
               "audio_max_error": 0, "video_rgb_mae": total_error / total, "video_max_error": maximum,
               "video_within_two_fraction": within_two / total, "max_timestamp_residual_seconds": max(residuals),
               "per_frame_video_mae": per_frame}
    (OUTPUT / "comparison-results.json").write_text(json.dumps(summary, indent=2))
    print(json.dumps({k:v for k,v in summary.items() if k != "per_frame_video_mae"}, indent=2))


if __name__ == "__main__":
    main()

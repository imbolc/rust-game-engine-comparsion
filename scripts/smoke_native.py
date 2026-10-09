#!/usr/bin/env python3
"""Check the ALSA default device and native game startup on a Linux desktop."""

import argparse
import ctypes
import os
from pathlib import Path
import signal
import subprocess
import time


ROOT = Path(__file__).resolve().parent.parent
PULSE_CONFIG = Path("/etc/alsa/conf.d/99-pulseaudio-default.conf")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("engines", nargs="*", help="fyrox or macroquad; defaults to both")
    parser.add_argument("--configure-alsa", action="store_true", help="Enable the installed PulseAudio default configuration with sudo")
    args = parser.parse_args()
    engines = args.engines or ["fyrox", "macroquad"]
    if set(engines) - {"fyrox", "macroquad"}:
        parser.error("supported engines: fyrox, macroquad")
    if args.configure_alsa and not (PULSE_CONFIG.exists() or PULSE_CONFIG.is_symlink()):
        subprocess.run([
            "sudo", "-n", "ln", "-s", str(PULSE_CONFIG) + ".example", str(PULSE_CONFIG),
        ], check=True)

    alsa = ctypes.CDLL("libasound.so.2")
    alsa.snd_pcm_open.argtypes = [ctypes.POINTER(ctypes.c_void_p), ctypes.c_char_p, ctypes.c_int, ctypes.c_int]
    alsa.snd_pcm_close.argtypes = [ctypes.c_void_p]
    alsa.snd_strerror.restype = ctypes.c_char_p
    handle = ctypes.c_void_p()
    result = alsa.snd_pcm_open(ctypes.byref(handle), b"default", 0, 0)
    if result < 0:
        raise SystemExit("ALSA default device failed: " + alsa.snd_strerror(result).decode())
    alsa.snd_pcm_close(handle)
    print("ALSA default playback device opened successfully", flush=True)

    output = ROOT / "target/native-smoke"
    output.mkdir(parents=True, exist_ok=True)
    for engine in engines:
        binary = ROOT / f"tetris-{engine}/target/debug/tetris-{engine}"
        log_path = output / f"{engine}.log"
        with log_path.open("w") as log:
            process = subprocess.Popen([str(binary)], cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                time.sleep(5)
                exited = process.poll()
            finally:
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGTERM)
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        os.killpg(process.pid, signal.SIGKILL)
                        process.wait()
        text = log_path.read_text()
        failures = [line for line in text.splitlines() if any(error in line for error in (
            "panicked at", "Audio thread died", "Unable to initialize audio output", "Broken pipe", "ALSA lib", "[ERROR]",
        ))]
        if exited is not None or failures:
            print("\n".join(failures[-20:]))
            raise SystemExit(f"{engine}: native startup failed (exit {exited}); full log: {log_path}")
        print(f"{engine}: native startup stayed running for 5 seconds with no audio/display errors; log: {log_path}", flush=True)


if __name__ == "__main__":
    main()

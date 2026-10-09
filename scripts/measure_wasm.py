#!/usr/bin/env python3
"""Build browser WASM with one size profile and print size/production Rust LOC."""

import argparse
import gzip
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tomllib


ROOT = Path(__file__).resolve().parent.parent
ENGINES = {"bevy": "bevy", "fyrox": "fyrox", "macroquad": "macroquad"}
PROFILE = {
    "opt-level": "z",
    "lto": "true",
    "codegen-units": "1",
    "panic": "abort",
    "strip": "true",
}
TOOLS = {}
WASM_FLAGS = [
    "-Oz",
    "--strip-debug",
    "--strip-producers",
    "--enable-bulk-memory",
    "--enable-sign-ext",
    "--enable-nontrapping-float-to-int",
    "--enable-mutable-globals",
    "--enable-reference-types",
    "--enable-multivalue",
    "--enable-simd",
]
RAW_STRING = re.compile(r'(?:br|cr|r)(#*)"')
CHAR = re.compile(r"'(?:\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.)|[^'\\\n])'")
TEST_MODULE = re.compile(
    r"(?m)^[ \t]*#\[cfg\(test\)\][ \t]*\n"
    r"(?:[ \t]*#\[[^\n]*\][ \t]*\n)*"
    r"[ \t]*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{"
)


def rust_loc(source):
    """Count physical Rust lines with code, excluding cfg(test) modules."""
    code, structure = list(source), list(source)

    def erase(chars, start, end):
        chars[start:end] = ["\n" if c == "\n" else " " for c in chars[start:end]]

    i = 0
    while i < len(source):
        start = i
        if source.startswith("//", i):
            end = source.find("\n", i)
            i = len(source) if end == -1 else end
            erase(code, start, i)
        elif source.startswith("/*", i):
            i, depth = i + 2, 1
            while depth and i < len(source):
                if source.startswith("/*", i):
                    depth += 1
                    i += 2
                elif source.startswith("*/", i):
                    depth -= 1
                    i += 2
                else:
                    i += 1
            erase(code, start, i)
        elif raw := RAW_STRING.match(source, i):
            closing = '"' + raw[1]
            end = source.find(closing, raw.end())
            i = len(source) if end == -1 else end + len(closing)
        elif source[i] == '"':
            i += 1
            while i < len(source):
                if source[i] == "\\":
                    i += 2
                elif source[i] == '"':
                    i += 1
                    break
                else:
                    i += 1
        elif char := CHAR.match(source, i):
            i = char.end()
        else:
            i += 1
            continue
        erase(structure, start, i)

    masked = "".join(structure)
    for module in TEST_MODULE.finditer(masked):
        end, depth = module.end(), 1
        while depth and end < len(masked):
            depth += (masked[end] == "{") - (masked[end] == "}")
            end += 1
        erase(code, module.start(), end)
    return sum(bool(line.strip()) for line in "".join(code).splitlines())


def run(command, env=None):
    print("+ " + " ".join(map(str, command)), file=sys.stderr, flush=True)
    log_path = ROOT / "target/wasm-size/commands.log"
    with log_path.open("a+", encoding="utf-8") as log:
        log.write("\nCommand: " + json.dumps(list(map(str, command))) + "\n")
        if env is not None:
            log.write("Release profile: " + json.dumps(PROFILE) + "\n")
        log.flush()
        start = log.tell()
        result = subprocess.run(command, cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT)
        if result.returncode:
            log.seek(start)
            lines = log.read().splitlines()
            excerpt = lines if len(lines) <= 40 else lines[:20] + ["... omitted; full output in log ..."] + lines[-20:]
            print("\n".join(excerpt), file=sys.stderr)
            print(f"Full command output: {log_path}", file=sys.stderr)
            raise subprocess.CalledProcessError(result.returncode, command)
    print(f"  completed: {Path(command[0]).name}", file=sys.stderr, flush=True)


def version(tool):
    return subprocess.check_output([TOOLS[tool], "--version"], text=True).strip()


def measure(engine, skip_build):
    crate = ROOT / f"tetris-{engine}"
    manifest = tomllib.loads((crate / "Cargo.toml").read_text())
    package = manifest["package"]["name"]
    if engine == "fyrox":
        package = manifest.get("lib", {}).get("name", package.replace("-", "_"))
    artifact = crate / "target/wasm32-unknown-unknown/release" / f"{package}.wasm"
    output = ROOT / "target/wasm-size" / engine
    output.mkdir(parents=True, exist_ok=True)
    if not skip_build:
        env = os.environ.copy()
        for key, value in PROFILE.items():
            env["CARGO_PROFILE_RELEASE_" + key.replace("-", "_").upper()] = value
        command = [
            TOOLS["cargo"], "build", "-q", "--locked", "--release",
            "--target", "wasm32-unknown-unknown", "--target-dir", str(crate / "target"),
            "--manifest-path", str(crate / "Cargo.toml"),
        ]
        if engine == "fyrox":
            command.append("--lib")
        run(command, env)

    if engine == "macroquad":
        browser_wasm = artifact
        destination = output / "app.wasm"
    else:
        run([
            TOOLS["wasm-bindgen"], str(artifact), "--target", "web", "--no-typescript",
            "--out-name", "app", "--out-dir", str(output),
        ])
        browser_wasm = output / "app_bg.wasm"
        destination = browser_wasm

    before = browser_wasm.stat().st_size
    optimized = output / "optimized.wasm"
    run([TOOLS["wasm-opt"], str(browser_wasm), *WASM_FLAGS, "-o", str(optimized)])
    optimized.replace(destination)
    data = destination.read_bytes()
    compressed = gzip.compress(data, compresslevel=9, mtime=0)
    # Normalize the header across Python versions and operating systems.
    compressed = compressed[:9] + b"\xff" + compressed[10:]
    destination.with_suffix(".wasm.gz").write_bytes(compressed)

    html = (crate / "index.html").read_text()
    if engine == "macroquad":
        html = html.replace("tetris-macroquad.wasm", "app.wasm")
        shutil.copyfile(crate / "mq_js_bundle.js", output / "mq_js_bundle.js")
    elif engine == "fyrox":
        html = html.replace("./pkg/tetris_fyrox.js", "./app.js")
    else:
        html = re.sub(
            r'<link data-trunk rel="rust" data-bin="tetris-bevy">',
            '<script type="module">import init from "./app.js"; init().catch(console.error);</script>',
            html,
        )
    (output / "index.html").write_text(html)

    lock = tomllib.loads((crate / "Cargo.lock").read_text())
    engine_version = next(p["version"] for p in lock["package"] if p["name"] == ENGINES[engine])
    rust_files = sorted((crate / "src").rglob("*.rs"))
    if (crate / "build.rs").is_file():
        rust_files.append(crate / "build.rs")
    result = {
        "engine": engine,
        "engine_version": engine_version,
        "raw_release_bytes": artifact.stat().st_size,
        "wasm_before_minification_bytes": before,
        "wasm_bytes": len(data),
        "gzip_bytes": len(compressed),
        "rust_loc": sum(rust_loc(path.read_text()) for path in rust_files),
        "wasm_file": str(destination.relative_to(ROOT)),
    }
    (output / "metrics.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("engines", nargs="*", metavar="ENGINE", help="bevy, fyrox or macroquad; defaults to all")
    parser.add_argument("--skip-build", action="store_true", help="Measure existing release artifacts built with the documented profile")
    parser.add_argument("--tool-dir", type=Path, default=ROOT / "target/tools/bin", help="Prefer tools installed here, then search PATH")
    args = parser.parse_args()
    args.engines = args.engines or list(ENGINES)
    if unknown := set(args.engines) - ENGINES.keys():
        parser.error("unknown engines: " + ", ".join(sorted(unknown)))
    for tool in ["cargo", "rustc", "wasm-opt"] + (["wasm-bindgen"] if any(e != "macroquad" for e in args.engines) else []):
        local = args.tool_dir / tool
        TOOLS[tool] = str(local) if local.is_file() else shutil.which(tool)
        if not TOOLS[tool]:
            parser.error(f"{tool} is required in {args.tool_dir} or on PATH")

    try:
        results = [measure(engine, args.skip_build) for engine in args.engines]
    except subprocess.CalledProcessError as error:
        parser.exit(error.returncode, "Build/minification failed; see target/wasm-size/commands.log\n")
    metadata = {
        "rustc": version("rustc"),
        "wasm_opt": version("wasm-opt"),
        "wasm_bindgen": version("wasm-bindgen") if any(e != "macroquad" for e in args.engines) else None,
        "release_profile": PROFILE,
        "wasm_opt_flags": WASM_FLAGS,
        "gzip": "level 9, mtime 0, no filename",
        "loc": "Physical Rust src/ and build.rs lines containing code, excluding comments, blank lines and cfg(test) modules; includes copied game logic",
        "engines": results,
    }
    output = ROOT / "target/wasm-size/metrics.json"
    output.write_text(json.dumps(metadata, indent=2) + "\n")
    print("| Engine | Release WASM (bytes) | Minified WASM (bytes) | Gzip (bytes) | Rust LOC |")
    print("| --- | ---: | ---: | ---: | ---: |")
    for result in results:
        print(f'| {result["engine"].capitalize()} {result["engine_version"]} | {result["wasm_before_minification_bytes"]:,} | {result["wasm_bytes"]:,} | {result["gzip_bytes"]:,} | {result["rust_loc"]:,} |')
    print(f"\nMetadata: {output.relative_to(ROOT)}")


if __name__ == "__main__":
    main()

"""Package a verified portable runtime. Standard library only; never installs MSI.

This is a new x64 path. The historical windows/build.py remains unchanged.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import struct
import subprocess
import sys
import time
import uuid
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parent.parent
NS = "http://schemas.microsoft.com/wix/2006/wi"
ET.register_namespace("", NS)
NAMESPACE = uuid.UUID("89d7c4d0-038f-48d9-af03-5e1d980c8034")
LAUNCHER = "quick-switch-launcher.exe"
EXCLUDE = {
    "launch.cmd", "runtime-manifest.json", "RUNTIME.txt",
    "README.md", "MODIFICATIONS.md", "quick-switching.md",
    "windows-installer.md", "INSTALLER.txt",
    "lib/gdk-pixbuf-2.0/2.10.0/loaders.cache",
}


def write_distribution_notes(stage):
    """Ship usage and required notices, not developer documentation or local history."""
    (stage / "README.md").write_text(
        "# Keyboard Configurator Quick Switch\n\n"
        "Start from the Start menu or quick-switch-launcher.exe.\n"
        "Add exported JSON profiles in Quick Switch Settings, then Save.\n"
        "Close other keyboard tools while switching; keep the USB cable connected.\n\n"
        "Unofficial, unsigned build. See MODIFICATIONS.md and LICENSE.\n",
        encoding="utf-8")
    shutil.copy2(ROOT / "MODIFICATIONS.md", stage / "MODIFICATIONS.md")


def source_path_flags():
    """Remove machine-specific prefixes from launcher compiler output."""
    flags = []
    for prefix, replacement in ((Path.home(), "/build/user"), (ROOT, "/src/keyboard-configurator")):
        for spelling in sorted({str(prefix), prefix.as_posix()}):
            flags.append(f"--remap-path-prefix={spelling}={replacement}")
    return flags


def verify_distribution(stage):
    """Reject accidental distribution of this machine's paths or developer notes."""
    prefixes = {str(ROOT), ROOT.as_posix(), str(Path.home()), Path.home().as_posix()}
    needles = {text.encode(encoding).lower() for text in prefixes for encoding in ("utf-8", "utf-16-le")}
    for path in stage.rglob("*"):
        if not path.is_file():
            continue
        contents = path.read_bytes().lower()
        if any(needle in contents for needle in needles):
            raise ValueError(f"Local build path leaked into distribution: {path.relative_to(stage)}")
    for name in ("quick-switching.md", "windows-installer.md", "INSTALLER.txt"):
        if (stage / name).exists():
            raise ValueError(f"Developer documentation must not be installed: {name}")
REQUIRED = {
    "system76-keyboard-configurator.exe", "gdk-pixbuf-query-loaders.exe",
    "share/glib-2.0/schemas/gschemas.compiled", "etc/fonts/fonts.conf",
    "lib/gdk-pixbuf-2.0/2.10.0/loaders/pixbufloader_svg.dll",
    "LICENSE", "MODIFICATIONS.md", "quick-switching.md",
}


def sha256(path):
    with path.open("rb") as stream:
        digest = hashlib.sha256()
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
        return digest.hexdigest()


def safe_relative(value):
    if not isinstance(value, str) or not value or "\\" in value:
        raise ValueError(f"Invalid manifest path: {value!r}")
    path = PurePosixPath(value)
    if (path.is_absolute() or path.as_posix() != value
            or any(part in (".", "..") or ":" in part or part.endswith((".", " "))
                   for part in path.parts)):
        raise ValueError(f"Unsafe manifest path: {value!r}")
    return path


def verify_bundle(bundle):
    """Fail closed: hashes, full inventory, case collisions, links and PE architecture."""
    paths = [bundle, *bundle.rglob("*")]
    for path in paths:
        if path.is_symlink() or getattr(path.stat(), "st_file_attributes", 0) & 0x400:
            raise ValueError(f"Reparse points are not allowed: {path}")
    manifest = json.loads((bundle / "runtime-manifest.json").read_text(encoding="utf-8"))
    if not isinstance(manifest, list) or not manifest:
        raise ValueError("Runtime manifest must be a nonempty list")
    records = {}
    folded = set()
    for record in manifest:
        relative = safe_relative(record["path"]).as_posix()
        if (PurePosixPath(relative).name.casefold() in ("profiles.json", "quick-switch.json")
                or "backups" in (part.casefold() for part in PurePosixPath(relative).parts)):
            raise ValueError(f"User settings/backups must not be packaged: {relative}")
        if relative.casefold() in folded:
            raise ValueError(f"Duplicate/case-colliding manifest path: {relative}")
        folded.add(relative.casefold())
        expected = record["sha256"]
        if not isinstance(expected, str) or not re.fullmatch(r"[0-9a-f]{64}", expected):
            raise ValueError(f"Invalid SHA256: {relative}")
        path = bundle / relative
        if not path.is_file() or sha256(path) != expected:
            raise ValueError(f"Runtime hash mismatch or missing file: {relative}")
        records[relative] = expected
    actual = set()
    for path in bundle.rglob("*"):
        if path.is_file():
            actual.add(path.relative_to(bundle).as_posix())
    if actual != set(records) | {"runtime-manifest.json"}:
        raise ValueError(f"Unmanifested/missing files: {actual ^ (set(records) | {'runtime-manifest.json'})}")
    if not REQUIRED.issubset(records):
        raise ValueError(f"Missing runtime requirements: {REQUIRED - set(records)}")
    for relative in records:
        if Path(relative).suffix.lower() in (".exe", ".dll"):
            verify_pe(bundle / relative)
    return records


def verify_pe(path):
    with path.open("rb") as stream:
        if stream.read(2) != b"MZ":
            raise ValueError(f"Not a PE binary: {path}")
        stream.seek(0x3C)
        offset = struct.unpack("<I", stream.read(4))[0]
        stream.seek(offset)
        if stream.read(6) != b"PE\0\0\x64\x86":
            raise ValueError(f"Not an x64 PE binary: {path}")


def validate_version(value):
    if not re.fullmatch(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", value):
        raise ValueError("MSI version must have exactly three numeric fields, e.g. 1.3.13")
    parts = [int(part) for part in value.split(".")]
    if any(part > limit for part, limit in zip(parts, (255, 255, 65535))):
        raise ValueError("MSI version exceeds 255.255.65535")
    return value


def identifier(prefix, relative):
    return prefix + hashlib.sha256(relative.casefold().encode("utf-8")).hexdigest()[:32]


def element(parent, name, **attributes):
    return ET.SubElement(parent, f"{{{NS}}}{name}", attributes)


def generate_payload(stage, destination):
    document = ET.Element(f"{{{NS}}}Wix")
    fragment = element(document, "Fragment")
    reference = element(fragment, "DirectoryRef", Id="INSTALLFOLDER")
    directories = {"": reference}
    group = element(fragment, "ComponentGroup", Id="QuickSwitchPayload")
    files = sorted(path for path in stage.rglob("*") if path.is_file())
    for path in files:
        relative = path.relative_to(stage)
        current = ""
        for part in relative.parts[:-1]:
            parent = current
            current = f"{current}/{part}".lstrip("/")
            if current not in directories:
                directories[current] = element(directories[parent], "Directory",
                                               Id=identifier("D", current), Name=part)
        key = relative.as_posix()
        component_id = identifier("C", key)
        component = element(directories[current], "Component", Id=component_id,
                            Guid=str(uuid.uuid5(NAMESPACE, key.casefold())).upper(), Win64="yes")
        file_id = "QuickSwitchLauncher" if key == LAUNCHER else identifier("F", key)
        file = element(component, "File", Id=file_id, Name=relative.name, Source=str(path), KeyPath="yes")
        if key == LAUNCHER:
            element(file, "Shortcut", Id="QuickSwitchStartMenu", Directory="QuickSwitchMenu",
                    Name="Keyboard Configurator Quick Switch", WorkingDirectory="INSTALLFOLDER",
                    Advertise="yes", Description="Unofficial System76 Keyboard Configurator with Quick Switch")
            element(component, "RemoveFolder", Id="RemoveQuickSwitchMenu", Directory="QuickSwitchMenu", On="uninstall")
        element(group, "ComponentRef", Id=component_id)
    ET.indent(document, space="  ")
    ET.ElementTree(document).write(destination, encoding="utf-8", xml_declaration=True)
    return len(files)


def run(command, log, env=None):
    with log.open("wb") as output:
        result = subprocess.run([str(arg) for arg in command], stdout=output,
                                stderr=subprocess.STDOUT, env=env, check=False)
    if result.returncode:
        raise RuntimeError(f"Command failed ({result.returncode}): {command[0]}. See {log}")


def package(args):
    version = args.version
    if version is None:
        match = re.search(r'^version\s*=\s*"([^"]+)"', (ROOT / "Cargo.toml").read_text(), re.MULTILINE)
        version = match.group(1)
    version = validate_version(version)
    bundle = args.bundle_root.absolute()
    if bundle.is_symlink() or getattr(bundle.stat(), "st_file_attributes", 0) & 0x400:
        raise ValueError("Bundle root must not be a reparse point")
    bundle = bundle.resolve(strict=True)
    records = verify_bundle(bundle)
    release = ROOT / "target/x86_64-pc-windows-gnu/release/system76-keyboard-configurator.exe"
    if sha256(release) != records[release.name]:
        raise ValueError("Bundle EXE does not match current Release build; refresh and verify it first")
    tools = [args.wix_bin / name for name in ("candle.exe", "light.exe")]
    tools += [args.rust_bin / "rustc.exe", args.gtk_root / "bin/gcc.exe"]
    for tool in tools:
        if not tool.is_file():
            raise ValueError(f"Required packaging tool not found: {tool}")
    for tool in tools[:2]:
        info = subprocess.run([str(tool), "-?"], capture_output=True, text=True, check=True)
        if "3.14.1." not in info.stdout:
            raise ValueError(f"This compatibility packaging path requires WiX3.14.1: {tool}")
    output = args.output_directory.resolve()
    if output == bundle or bundle in output.parents:
        raise ValueError("Output directory must not be inside the input bundle")
    output.mkdir(parents=True, exist_ok=True)
    final = output / f"keyboard-configurator-quick-switch-{version}-x64.msi"
    if final.exists():
        raise ValueError(f"Refusing to overwrite existing MSI: {final}; use a fresh output directory")
    work = output / f"build-{time.time_ns()}"
    stage = work / "payload"
    stage.mkdir(parents=True)
    # Copy only verified records. Recheck after copy to guard against stale/racing inputs.
    for relative, digest in records.items():
        if relative in EXCLUDE:
            continue
        target = stage / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(bundle / relative, target)
        if sha256(target) != digest:
            raise ValueError(f"Input changed while packaging: {relative}")
    write_distribution_notes(stage)
    environment = dict(os.environ)
    environment["PATH"] = str(args.gtk_root / "bin") + os.pathsep + environment["PATH"]
    compile_args = [args.rust_bin / "rustc.exe", "--edition=2021", "--target=x86_64-pc-windows-gnu",
                    "-C", f"linker={args.gtk_root / 'bin/gcc.exe'}", *source_path_flags(), ROOT / "windows/installer-launcher.rs"]
    run(compile_args + ["--test", "-o", work / "launcher-tests.exe"], work / "launcher-test-build.log", environment)
    run([work / "launcher-tests.exe"], work / "launcher-tests.log", environment)
    run(compile_args + ["-C", "opt-level=2", "-C", "strip=debuginfo", "-o", stage / LAUNCHER],
        work / "launcher-build.log", environment)
    verify_pe(stage / LAUNCHER)
    verify_distribution(stage)
    payload_manifest = [dict(path=p.relative_to(stage).as_posix(), sha256=sha256(p))
                        for p in sorted(stage.rglob("*")) if p.is_file()]
    (stage / "installer-manifest.json").write_text(json.dumps(payload_manifest, indent=2) + "\n", encoding="utf-8")
    count = generate_payload(stage, work / "payload.wxs")
    candle, light = tools[:2]
    run([candle, "-nologo", "-wx", "-arch", "x64", f"-dProductVersion={version}",
         f"-dLicenseRtf={ROOT / 'windows/gpl.rtf'}", "-out", str(work) + os.sep,
         ROOT / "windows/quick-switch-x64.wxs", work / "payload.wxs"], work / "candle.log")
    # Leave ICE validation enabled. No suppressions; never execute the MSI.
    built = work / final.name
    run([light, "-nologo", "-wx", "-ext", "WixUIExtension", "-cultures:ja-jp",
         "-loc", ROOT / "windows/i18n/ja-jp.wxl", "-out", built,
         work / "quick-switch-x64.wixobj", work / "payload.wixobj"], work / "light.log")
    shutil.copy2(built, final)
    (output / (final.name + ".sha256")).write_text(f"{sha256(final)}  {final.name}\n", encoding="ascii")
    sources = ["windows/package-msi.py", "windows/package-msi.ps1", "windows/quick-switch-x64.wxs",
               "windows/installer-launcher.rs", "windows/i18n/ja-jp.wxl",
               "docs/windows-installer.md", "MODIFICATIONS.md"]
    result = dict(status="BUILT", msi=str(final), sha256=sha256(final), version=version,
                  payload_files=count, work_directory=str(work), source_bundle=str(bundle),
                  source_manifest_sha256=sha256(bundle / "runtime-manifest.json"),
                  wix_candle_sha256=sha256(candle), wix_light_sha256=sha256(light),
                  source_sha256={relative: sha256(ROOT / relative) for relative in sources},
                  installed=False, hardware_access=False, signed=False)
    (work / "build-result.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(f"Created {final.name} ({final.stat().st_size:,} bytes)")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundle-root", type=Path, default=ROOT / "target/portable/system76-keyboard-configurator")
    parser.add_argument("--wix-bin", type=Path, required=True, help="WiX3.14.1 directory containing candle.exe/light.exe")
    parser.add_argument("--rust-bin", type=Path, required=True)
    parser.add_argument("--gtk-root", type=Path, required=True, help="Matching x64 GNU prefix containing bin/gcc.exe")
    parser.add_argument("--version", help="Separate MSI version; defaults to Cargo.toml. Increase on each released update.")
    parser.add_argument("--output-directory", type=Path, default=ROOT / "target/installer")
    args = parser.parse_args()
    try:
        package(args)
    except (ValueError, OSError, RuntimeError, KeyError) as error:
        print(f"MSI packaging failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
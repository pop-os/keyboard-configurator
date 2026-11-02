"""Exercise the extracted MSI launcher with isolated dummy devices only.

No MSI installation. Optional --read-only temporarily denies this user's writes
on the disposable extracted payload and restores the ACL in a finally block.
Never use this script against an installed app or a personal portable directory.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent.parent


def hashes(root):
    return {p.relative_to(root).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in root.rglob("*") if p.is_file()}


def user_sid():
    result = subprocess.run([str(Path(os.environ["SystemRoot"]) / "System32/whoami.exe"), "/user", "/fo", "csv", "/nh"],
                            capture_output=True, text=True, check=True)
    import csv
    return next(csv.reader([result.stdout.strip()]))[1]


def run(args):
    payload = args.payload.resolve(strict=True)
    verification_path = payload.parent / "verification.json"
    if not verification_path.is_file():
        raise ValueError("Use only the disposable payload produced by verify-msi.py")
    verification = json.loads(verification_path.read_text())
    if verification.get("status") != "PASS" or Path(verification["payload"]) != payload:
        raise ValueError("Extraction verification is missing or does not match this payload")
    args.work_directory.mkdir(parents=True, exist_ok=False)
    work = args.work_directory.resolve()
    if work == payload or payload in work.parents:
        raise ValueError("Smoke data must be outside the payload")
    before = hashes(payload)
    env = dict(os.environ)
    # The launcher must replace these intentionally broken inherited paths.
    env.update(PATH=str(Path(os.environ["SystemRoot"]) / "System32"),
               GTK_PATH=str(work / "not-a-gtk-runtime"),
               GDK_PIXBUF_MODULE_FILE=str(work / "nonexistent-loaders.cache"),
               GSETTINGS_SCHEMA_DIR=str(work / "nonexistent-schemas"),
               APPDATA=str(work / "appdata"), LOCALAPPDATA=str(work / "localappdata"),
               LAUNCH_HEAVY_FAKE_ONLY="1", CONFIGURATOR_PROFILE_DIR=str(work / "settings"),
               CONFIGURATOR_LAUNCHER_NO_DIALOG="1", CONFIGURATOR_SMOKE_LANGUAGE="ja",
               LAUNCH_HEAVY_DUMMY_IO_DELAY_MS="1")
    for variable in ("LAUNCH_HEAVY_UI_REPORT", "LAUNCH_HEAVY_UI_IMAGE", "LAUNCH_HEAVY_PROGRESS_IMAGE",
                     "CONFIGURATOR_SETTINGS_IMAGE", "LAUNCH_HEAVY_SMOKE_SOURCE"):
        env.pop(variable, None)
    export = json.loads((ROOT / "layouts/system76/launch_heavy_1/default.json").read_text())
    for layer in export["layers"]:
        layer["brightness"] = 50  # Dummy reports max=100; public defaults use 255.
    source = work / "public-export.json"
    source.write_text(json.dumps(export), encoding="utf-8")
    launcher = payload / "quick-switch-launcher.exe"
    icacls = Path(os.environ["SystemRoot"]) / "System32/icacls.exe"
    sid = user_sid()
    denied = False
    try:
        if args.read_only:
            # Deny file creation/data append/write only, not attributes needed to execute
            # a Windows image. Disposable extracted payload; never host/profile ACLs.
            subprocess.run([str(icacls), str(payload), "/deny", f"*{sid}:(OI)(CI)(WD,AD)"], check=True,
                           stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            denied = True
            try:
                (payload / "write-must-fail.txt").write_text("must fail")
            except PermissionError:
                pass
            else:
                raise AssertionError("Read-only ACL did not deny writes")
        # No GUI/device initialization: help only, but exercise path/default-profile selection,
        # cache lifecycle, argument/exit propagation and parallel launcher independence.
        help_env = dict(env)
        help_env.pop("CONFIGURATOR_PROFILE_DIR", None)
        children = [subprocess.Popen([str(launcher), "--help-gtk"], env=help_env,
                                     stdout=subprocess.PIPE, stderr=subprocess.PIPE) for _ in range(2)]
        for index, child in enumerate(children):
            stdout, stderr = child.communicate(timeout=30)
            (work / f"help-{index}.stdout.log").write_bytes(stdout)
            (work / f"help-{index}.stderr.log").write_bytes(stderr)
            assert child.returncode == 0 and b"GTK+ Options" in stdout
        error = subprocess.run([str(launcher), "--not-a-configurator-option"], env=env,
                               capture_output=True, timeout=30)
        assert error.returncode != 0, "Launcher must propagate failed child exit status"
        env.update(LAUNCH_HEAVY_SMOKE_SOURCE=str(source),
                   LAUNCH_HEAVY_UI_REPORT=str(work / "report.json"),
                   LAUNCH_HEAVY_UI_IMAGE=str(work / "overview.png"),
                   LAUNCH_HEAVY_PROGRESS_IMAGE=str(work / "progress.png"),
                   CONFIGURATOR_SETTINGS_IMAGE=str(work / "settings.png"))
        with (work / "stdout.log").open("wb") as stdout, (work / "stderr.log").open("wb") as stderr:
            result = subprocess.run([str(launcher), "--fake-keyboard=system76/launch_heavy_1"],
                                    env=env, stdout=stdout, stderr=stderr, timeout=100)
        assert result.returncode == 0, (result.returncode, work / "stderr.log")
        report = json.loads((work / "report.json").read_text(encoding="utf-8"))
        settings = json.loads((work / "settings/quick-switch.json").read_text(encoding="utf-8"))
        assert [slot["name"] for slot in settings["buttons"]] == ["Office", "Study"]
        assert all(set(slot) == {"name", "source", "json"} for slot in settings["buttons"])
        samples = [record for record in report if "progress_sample" in record]
        assert len(samples) > 5
        assert all(s["modal"] and .90 < s["opacity"] < 1 and not s["parent_sensitive"] for s in samples)
        assert any(0 < s["progress_sample"] < 1 for s in samples)
        assert len({s["elapsed"] for s in samples}) > 2
        assert any("/" in s["detail"] for s in samples)
        translations = (ROOT / "i18n/ja/system76_keyboard_configurator.ftl").read_text(encoding="utf-8")
        title = next(line.split("=", 1)[1].strip() for line in translations.splitlines()
                     if line.startswith("quick-switch-switching ="))
        assert all(s["title"] == title for s in samples)
        assert any(s.get("cancel_unchanged") and s.get("progress_hidden_after_completion") for s in report)
        widths = set()
        for record in report:
            widgets = record.get("widgets", [])
            geometry = [widget for widget in widgets if "keys" in widget]
            assert all(g["fits"] and g["hits_match"] and g["keys"] == 105 for g in geometry)
            area = next((w for w in widgets if w.get("name") == "quick-switch-area"), None)
            buttons = next((w for w in widgets if w.get("name") == "quick-switch-buttons"), None)
            if buttons and buttons["width"] > 1:
                widths.add(record["window_width"])
                assert abs(buttons["origin"][0] + buttons["width"] / 2 - area["origin"][0] - area["width"] / 2) <= 2
                assert abs(buttons["origin"][1] + buttons["height"] / 2 - area["origin"][1] - area["height"] / 2) <= 2
        assert {900, 1280, 1500}.issubset(widths)
        editor = next(r for r in report if r.get("step") == 10)
        assert not any(w.get("name") in ("quick-switch-area", "quick-switch-buttons", "quick-switch-keycap")
                       for w in editor["widgets"])
        backups = list((work / "settings/backups").glob("*.json"))
        assert len(backups) == 2
        snapshots = {json.dumps(r["snapshot"], sort_keys=True) for r in report if "snapshot" in r}
        assert {json.dumps(json.loads(p.read_text()), sort_keys=True) for p in backups} == snapshots
        cache = work / "localappdata/KeyboardConfiguratorQuickSwitch/cache"
        assert not list(cache.glob("loaders-*")), "Launcher left its temporary loader cache behind"
        assert before == hashes(payload), "Installed payload was changed at runtime"
        result = dict(status="PASS", payload_files=len(before), read_only_acl=args.read_only,
                      parallel_launchers=True, child_exit_propagated=True, cache_cleaned=True,
                      settings_isolated=True, japanese_samples=len(samples), backups=2,
                      widths=sorted(widths), installed=False, physical_device_access=False)
        (work / "smoke-result.json").write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps(result, indent=2))
    finally:
        if denied:
            subprocess.run([str(icacls), str(payload), "/remove:d", f"*{sid}", "/T", "/C"], check=True,
                           stdout=subprocess.PIPE, stderr=subprocess.STDOUT)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--payload", required=True, type=Path)
    parser.add_argument("--work-directory", required=True, type=Path)
    parser.add_argument("--read-only", action="store_true")
    run(parser.parse_args())
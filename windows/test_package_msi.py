"""Hardware-free packaging regressions: python -m unittest discover -s windows -p test_*.py"""
import importlib.util
import json
from pathlib import Path
import re
import struct
import tempfile
import unittest
import xml.etree.ElementTree as ET

SPEC = importlib.util.spec_from_file_location("package_msi", Path(__file__).with_name("package-msi.py"))
package = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(package)


class PackagingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def bundle(self):
        for relative in package.REQUIRED:
            path = self.root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b"runtime fixture")
            if path.suffix in (".exe", ".dll"):
                path.write_bytes(b"MZ" + bytes(58) + struct.pack("<I", 64) + b"PE\0\0\x64\x86")
        self.manifest()

    def manifest(self):
        records = [dict(path=p.relative_to(self.root).as_posix(), sha256=package.sha256(p))
                   for p in self.root.rglob("*") if p.is_file() and p.name != "runtime-manifest.json"]
        (self.root / "runtime-manifest.json").write_text(json.dumps(records))
        return records

    def test_rejects_traversal_and_noncanonical_paths(self):
        for path in ("../private", "/absolute", "C:/private", "a\\b", "a/../b", "./a", "a//b", "a.", "a ", ""):
            with self.subTest(path=path), self.assertRaises(ValueError):
                package.safe_relative(path)
        self.assertEqual(package.safe_relative("share/Unicode Ω/file.txt").as_posix(), "share/Unicode Ω/file.txt")

    def test_versions_follow_msi_three_field_limits(self):
        for value in ("1.3.13", "255.255.65535", "0.0.1"):
            self.assertEqual(package.validate_version(value), value)
        for value in ("1.2", "1.2.3.4", "1.2.beta", "01.2.3", "256.0.0", "1.1.65536"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                package.validate_version(value)

    def test_accepts_complete_x64_bundle(self):
        self.bundle()
        self.assertEqual(set(package.verify_bundle(self.root)), package.REQUIRED)

    def test_rejects_extra_private_settings(self):
        self.bundle()
        (self.root / "profiles.json").write_text("private")
        with self.assertRaisesRegex(ValueError, "Unmanifested"):
            package.verify_bundle(self.root)

    def test_rejects_tampered_input(self):
        self.bundle()
        (self.root / "LICENSE").write_text("changed")
        with self.assertRaisesRegex(ValueError, "hash mismatch"):
            package.verify_bundle(self.root)

    def test_rejects_even_manifested_user_data(self):
        self.bundle()
        (self.root / "quick-switch.json").write_text("private")
        self.manifest()
        with self.assertRaisesRegex(ValueError, "User settings"):
            package.verify_bundle(self.root)

    def test_rejects_missing_runtime_resource(self):
        self.bundle()
        (self.root / "etc/fonts/fonts.conf").unlink()
        self.manifest()
        with self.assertRaisesRegex(ValueError, "Missing runtime"):
            package.verify_bundle(self.root)

    def test_rejects_invalid_digest(self):
        self.bundle()
        records = self.manifest()
        records[0]["sha256"] = "not-a-digest"
        (self.root / "runtime-manifest.json").write_text(json.dumps(records))
        with self.assertRaisesRegex(ValueError, "Invalid SHA256"):
            package.verify_bundle(self.root)

    def test_rejects_duplicate_case_paths(self):
        self.bundle()
        records = self.manifest()
        records.append(dict(path="license", sha256=package.sha256(self.root / "LICENSE")))
        (self.root / "runtime-manifest.json").write_text(json.dumps(records))
        with self.assertRaisesRegex(ValueError, "Duplicate"):
            package.verify_bundle(self.root)

    def test_rejects_x86_pe(self):
        self.bundle()
        path = self.root / "system76-keyboard-configurator.exe"
        path.write_bytes(path.read_bytes()[:-2] + b"\x4c\x01")
        self.manifest()
        with self.assertRaisesRegex(ValueError, "Not an x64"):
            package.verify_bundle(self.root)

    def test_generated_components_stable_when_file_added(self):
        stage = self.root / "payload"
        (stage / "share/Unicode Ω").mkdir(parents=True)
        (stage / package.LAUNCHER).write_text("fixture")
        (stage / "share/Unicode Ω/a.txt").write_text("fixture")
        wxs = self.root / "payload.wxs"
        package.generate_payload(stage, wxs)
        ns = {"w": package.NS}
        def ids():
            root = ET.parse(wxs)
            return {c.attrib["Id"]: c.attrib["Guid"] for c in root.findall(".//w:Component", ns)}
        before = ids()
        (stage / "share/new.txt").write_text("new")
        package.generate_payload(stage, wxs)
        self.assertTrue(before.items() <= ids().items())
        root = ET.parse(wxs)
        self.assertTrue(all(c.attrib["Win64"] == "yes" for c in root.findall(".//w:Component", ns)))
        self.assertEqual(len(root.findall(".//w:File[@Id='QuickSwitchLauncher']", ns)), 1)
        launcher = root.find(".//w:File[@Id='QuickSwitchLauncher']", ns)
        self.assertEqual(launcher.find("w:Shortcut", ns).attrib["Advertise"], "yes")

    def test_installer_never_removes_user_data_or_runs_application(self):
        tree = ET.parse(Path(__file__).with_name("quick-switch-x64.wxs"))
        ns = {"w": package.NS}
        product = tree.find("w:Product", ns)
        self.assertNotEqual(product.attrib["UpgradeCode"].lower(), "548e3064-0ff9-421a-91dc-1f1bd0afe9b6")
        self.assertEqual(product.find("w:Package", ns).attrib["Platform"], "x64")
        for name in ("CustomAction", "RemoveFile", "Environment", "ServiceInstall"):
            self.assertEqual(tree.findall(f".//w:{name}", ns), [])
        self.assertTrue(all(r.attrib["Root"] == "HKLM" for r in tree.findall(".//w:RegistryValue", ns)))

    def test_installer_message_references_resolve_in_translation_resource(self):
        source = Path(__file__).with_name("quick-switch-x64.wxs").read_text(encoding="utf-8")
        references = set(re.findall(r"!\(loc\.([A-Za-z0-9_]+)\)", source))
        resource = ET.parse(Path(__file__).parent / "i18n/ja-jp.wxl").getroot()
        self.assertEqual(resource.attrib["Culture"], "ja-jp")
        strings = {item.attrib["Id"]: item.text for item in resource}
        self.assertEqual(references, set(strings))
        self.assertEqual(len(references), 3)
        self.assertTrue(all(strings.values()))

    def test_distribution_notes_are_minimal_and_machine_independent(self):
        package.write_distribution_notes(self.root)
        self.assertEqual({p.name for p in self.root.iterdir()}, {"README.md", "MODIFICATIONS.md"})
        readme = (self.root / "README.md").read_text(encoding="utf-8")
        self.assertLess(len(readme), 500)
        self.assertTrue(readme.isascii())
        for path in self.root.iterdir():
            text = path.read_text(encoding="utf-8")
            self.assertNotIn(str(package.ROOT), text)
            self.assertNotIn(str(Path.home()), text)
        self.assertTrue({"quick-switching.md", "windows-installer.md", "INSTALLER.txt"}.issubset(package.EXCLUDE))

    def test_repository_documentation_has_no_japanese_text(self):
        extensions = {".md", ".txt", ".rst", ".rtf", ".adoc"}
        paths = list(package.ROOT.iterdir())
        for directory in ("docs", "windows", "macos", "layouts"):
            paths.extend((package.ROOT / directory).rglob("*"))
        japanese = re.compile(r"[\u3040-\u30ff\u3400-\u9fff\uff66-\uff9f]")
        for path in sorted(paths):
            if not path.is_file() or path.suffix.lower() not in extensions:
                continue
            with self.subTest(path=path.relative_to(package.ROOT).as_posix()):
                text = path.read_text(encoding="utf-8")
                if path.suffix.lower() == ".rtf":
                    text = re.sub(r"\\u(-?\d+)\??", lambda match: chr(int(match[1]) % 65536), text)
                self.assertIsNone(japanese.search(text), "Documentation must be English-only")

    def test_launcher_compilation_remaps_local_paths(self):
        flags = package.source_path_flags()
        self.assertIn(f"--remap-path-prefix={package.ROOT}=/src/keyboard-configurator", flags)
        self.assertIn(f"--remap-path-prefix={Path.home()}=/build/user", flags)

    def test_distribution_rejects_embedded_local_path(self):
        for encoding in ("utf-8", "utf-16-le"):
            with self.subTest(encoding=encoding):
                (self.root / "binary.exe").write_bytes(str(package.ROOT).encode(encoding))
                with self.assertRaisesRegex(ValueError, "Local build path"):
                    package.verify_distribution(self.root)
        (self.root / "binary.exe").write_bytes(b"/src/keyboard-configurator")
        package.verify_distribution(self.root)


if __name__ == "__main__":
    unittest.main()
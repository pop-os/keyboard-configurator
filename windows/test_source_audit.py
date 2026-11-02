"""Source-publication hygiene tests using isolated fixtures and Git indexes."""
import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("audit_source", Path(__file__).with_name("audit-source.py"))
audit = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(audit)


class SourceAuditTests(unittest.TestCase):
    def test_japanese_is_only_allowed_in_explicit_translation_resources(self):
        text = "\u65e5\u672c\u8a9e".encode()
        for path in ("src/example.rs", "docs/example.md", "i18n/ja/unexpected.rs"):
            self.assertTrue(audit.inspect(path, text))
        for path in audit.TRANSLATIONS:
            self.assertEqual(audit.inspect(path, text), [])

    def test_rtf_unicode_escapes_cannot_hide_document_text(self):
        self.assertTrue(audit.inspect("docs/example.rtf", br"\u26085?"))

    def test_machine_paths_include_escaped_json(self):
        for value in (str(audit.ROOT), str(Path.home()), str(audit.ROOT).replace("\\", "\\\\")):
            self.assertTrue(any(item["rule"] == "personal-machine-path"
                                for item in audit.inspect("config.json", value.encode())))

    def test_pattern_findings_do_not_echo_secrets(self):
        token = "gh" + "p_" + "a" * 36
        findings = audit.inspect("config.txt", token.encode())
        self.assertTrue(any(item["rule"] == "possible-secret" for item in findings))
        self.assertNotIn(token, repr(findings))

    def test_public_urls_copyright_and_relative_paths_are_allowed(self):
        text = b'license = "GPL-3.0-or-later"\npath = "../profile-engine"\nhttps://github.com/pop-os/keyboard-configurator\n'
        self.assertEqual(audit.inspect("Cargo.toml", text), [])

    def test_private_and_binary_names_are_rejected(self):
        for name in (".env", "quick-switch.json", "release.exe", "memory-bank/notes.md"):
            self.assertTrue(audit.inspect(name, b"fixture"))

    def test_audit_rules_do_not_flag_their_own_source(self):
        path = Path(__file__).with_name("audit-source.py")
        self.assertEqual(audit.inspect("windows/audit-source.py", path.read_bytes()), [])

    def test_staged_content_is_checked_even_when_working_copy_is_clean(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            # Tests must never inherit an index/worktree override from the caller.
            env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
            def git(*args):
                subprocess.run(["git", "-C", str(root), *args], env=env, check=True, capture_output=True)
            git("init", "--quiet")
            (root / "note.md").write_text("\u65e5\u672c\u8a9e", encoding="utf-8")
            git("add", "note.md")
            (root / "note.md").write_text("English only", encoding="utf-8")
            (root / "new.rs").write_text("// English", encoding="utf-8")
            # The audit itself is read-only and must preserve the original index.
            index = (root / ".git/index").read_bytes()
            from unittest.mock import patch
            with patch.dict(os.environ, env, clear=True):
                result = audit.audit(root)
            self.assertEqual(result["status"], "FAIL")
            self.assertTrue(any(item["view"] == "staged" and item["rule"] == "japanese-outside-translation"
                                for item in result["findings"]))
            self.assertFalse(any(item["view"] == "working" for item in result["findings"]))
            self.assertEqual((root / ".git/index").read_bytes(), index)


if __name__ == "__main__":
    unittest.main()
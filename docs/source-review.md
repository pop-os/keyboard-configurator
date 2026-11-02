# Pre-commit source review

Run from the repository root:

```sh
python windows/audit-source.py
python -m unittest discover -s windows -p 'test_*.py'
```

The audit reads changed tracked files, untracked non-ignored files, and staged
blobs separately. A clean working copy does not override a finding in the index.
It does not stage, unstage, commit, or change any source files.

- Documentation, code, and comments are English. Japanese text belongs only in
  the three `i18n/ja/*.ftl` catalogs and `windows/i18n/ja-jp.wxl` listed by the audit.
  Keep UI translations and interpolation variables intact.
- Supply local tool paths through arguments or environment variables. Do not
  commit user directories, checkout paths, machine-specific settings, or logs.
- Keep credentials, personal profiles, build outputs, and private working notes
  outside the source change. Pattern matches require human review; no scan proves
  the absence of all secrets or security defects.
- Preserve upstream authors, copyright notices, licenses, and source provenance.
  Compatibility identifiers, standard environment variables, and synthetic invalid
  paths in path-validation tests are not personal machine dependencies.

Private working notes, reference material, and machine-specific editor settings
are excluded by the shared `.gitignore`, not only by local Git configuration.
Ignored files and historical commits are outside this check. Do not force-add
local build directories, editor settings, private notes, or binary artifacts.
Before a source release, build from a clean checkout and verify the declared Rust
toolchain and supported platforms. The Windows-tested compiler and known gaps are
documented in `windows/README.md`.

## Binary publication is a separate gate

Source hygiene is not MSI lifecycle validation or a complete dependency security
audit. Before distributing a binary, review dependency/runtime licenses and provide
the corresponding source and build scripts, retain required notices, validate
install/repair/upgrade/uninstall and coexistence in a disposable VM, and complete
the signing and supported-installer-tooling review. Do not direct users to an
upstream release page as if it contained this modified binary's source.
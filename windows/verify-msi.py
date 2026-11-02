"""Read MSI database and embedded cabinet without installation or custom actions.

Use --msi <file> --work-directory <fresh directory>. Windows only, stdlib only.
Does not call msiexec, administrative installation, or Windows Installer actions.
"""
import argparse
import ctypes
from ctypes import wintypes
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

OFFICIAL_UPGRADE_CODE = "{548E3064-0FF9-421A-91DC-1F1BD0AFE9B6}"
UPGRADE_CODE = "{89D7C4D0-038F-48D9-AF03-5E1D980C8034}"


def checksum(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class Database:
    def __init__(self, path):
        self.api = ctypes.WinDLL("msi")
        self.api.MsiOpenDatabaseW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR, ctypes.POINTER(wintypes.UINT)]
        self.api.MsiDatabaseOpenViewW.argtypes = [wintypes.UINT, wintypes.LPCWSTR, ctypes.POINTER(wintypes.UINT)]
        self.api.MsiViewExecute.argtypes = [wintypes.UINT, wintypes.UINT]
        self.api.MsiViewFetch.argtypes = [wintypes.UINT, ctypes.POINTER(wintypes.UINT)]
        self.api.MsiRecordGetStringW.argtypes = [wintypes.UINT, wintypes.UINT, wintypes.LPWSTR, ctypes.POINTER(wintypes.DWORD)]
        self.api.MsiRecordReadStream.argtypes = [wintypes.UINT, wintypes.UINT, ctypes.c_void_p, ctypes.POINTER(wintypes.DWORD)]
        self.api.MsiCloseHandle.argtypes = [wintypes.UINT]
        self.api.MsiGetSummaryInformationW.argtypes = [wintypes.UINT, wintypes.LPCWSTR, wintypes.UINT,
                                                      ctypes.POINTER(wintypes.UINT)]
        self.api.MsiSummaryInfoGetPropertyW.argtypes = [wintypes.UINT, wintypes.UINT,
            ctypes.POINTER(wintypes.UINT), ctypes.POINTER(ctypes.c_int),
            ctypes.POINTER(wintypes.FILETIME), wintypes.LPWSTR, ctypes.POINTER(wintypes.DWORD)]
        self.handle = wintypes.UINT()
        self.check(self.api.MsiOpenDatabaseW(str(path), None, ctypes.byref(self.handle)))

    @staticmethod
    def check(code):
        if code:
            raise OSError(f"Windows Installer database API error {code}")

    def close(self):
        self.api.MsiCloseHandle(self.handle)

    def template(self):
        summary = wintypes.UINT()
        self.check(self.api.MsiGetSummaryInformationW(self.handle, None, 0, ctypes.byref(summary)))
        try:
            kind, number, timestamp = wintypes.UINT(), ctypes.c_int(), wintypes.FILETIME()
            size = wintypes.DWORD(1024)
            buffer = ctypes.create_unicode_buffer(size.value)
            self.check(self.api.MsiSummaryInfoGetPropertyW(summary, 7, ctypes.byref(kind),
                       ctypes.byref(number), ctypes.byref(timestamp), buffer, ctypes.byref(size)))
            return buffer.value
        finally:
            self.api.MsiCloseHandle(summary)

    def records(self, sql):
        view = wintypes.UINT()
        self.check(self.api.MsiDatabaseOpenViewW(self.handle, sql, ctypes.byref(view)))
        try:
            self.check(self.api.MsiViewExecute(view, 0))
            while True:
                record = wintypes.UINT()
                status = self.api.MsiViewFetch(view, ctypes.byref(record))
                if status == 259:
                    break
                self.check(status)
                try:
                    yield record
                finally:
                    self.api.MsiCloseHandle(record)
        finally:
            self.api.MsiCloseHandle(view)

    def text(self, record, field):
        size = wintypes.DWORD(0)
        status = self.api.MsiRecordGetStringW(record, field, None, ctypes.byref(size))
        if status not in (0, 234):
            self.check(status)
        size.value += 1
        buffer = ctypes.create_unicode_buffer(size.value)
        self.check(self.api.MsiRecordGetStringW(record, field, buffer, ctypes.byref(size)))
        return buffer.value

    def rows(self, table, columns):
        fields = ", ".join(f"`{column}`" for column in columns)
        return [tuple(self.text(record, i + 1) for i in range(len(columns)))
                for record in self.records(f"SELECT {fields} FROM `{table}`")]

    def extract_stream(self, name, destination):
        if "'" in name:
            raise ValueError("Invalid cabinet stream name")
        found = False
        for record in self.records(f"SELECT `Data` FROM `_Streams` WHERE `Name` = '{name}'"):
            found = True
            with destination.open("wb") as output:
                while True:
                    size = wintypes.DWORD(1024 * 1024)
                    buffer = ctypes.create_string_buffer(size.value)
                    self.check(self.api.MsiRecordReadStream(record, 1, buffer, ctypes.byref(size)))
                    if size.value == 0:
                        break
                    output.write(buffer.raw[:size.value])
        if not found:
            raise ValueError(f"Embedded cabinet not found: {name}")


def verify(msi, work):
    work.mkdir(parents=True, exist_ok=False)
    extracted = work / "cabinet"
    extracted.mkdir()
    database = Database(msi)
    try:
        tables = {row[0] for row in database.rows("_Tables", ["Name"])}
        properties = dict(database.rows("Property", ["Property", "Value"]))
        if database.template() != "x64;1041":
            raise ValueError("Unexpected MSI summary architecture/language")
        if properties["UpgradeCode"].upper() != UPGRADE_CODE or properties["UpgradeCode"].upper() == OFFICIAL_UPGRADE_CODE:
            raise ValueError("Unexpected or official MSI identity")
        if properties["ProductLanguage"] != "1041" or "Unofficial" not in properties["ProductName"]:
            raise ValueError("MSI must clearly identify the localized unofficial build")
        if properties.get("ALLUSERS") != "1":
            raise ValueError("Expected per-machine installation")
        directories = {row[0]: row[1:] for row in database.rows("Directory", ["Directory", "Directory_Parent", "DefaultDir"])}
        if directories["INSTALLFOLDER"] != ("ProgramFiles64Folder", "Keyboard Configurator Quick Switch"):
            # WiX can emit short|long names; compare the long name below instead.
            if (directories["INSTALLFOLDER"][0] != "ProgramFiles64Folder"
                    or directories["INSTALLFOLDER"][1].split("|")[-1] != "Keyboard Configurator Quick Switch"):
                raise ValueError("Unexpected installation location")
        components = {r[0]: r for r in database.rows("Component", ["Component", "ComponentId", "Directory_", "Attributes", "KeyPath"])}
        if not all(int(row[3]) & 256 for row in components.values()):
            raise ValueError("Non-x64 component found")
        for table in ("Environment", "ServiceInstall"):
            if table in tables:
                raise ValueError(f"Unexpected global integration table: {table}")
        if "Registry" in tables and database.rows("Registry", ["Registry"]):
            raise ValueError("Unexpected application registry entries")
        for row in database.rows("RemoveFile", ["FileKey", "FileName", "DirProperty"]):
            if row[1] or row[2] != "QuickSwitchMenu":
                raise ValueError(f"Unsafe removal action: {row}")
        if "CustomAction" in tables:
            # WixUI_Minimal uses UI-only standard WiX property/navigation actions.
            allowed = {"WixUIValidatePath", "WixUIPrintEula", "WixUI_Bmp_Dialog", "WixUI_Bmp_Banner"}
            for row in database.rows("CustomAction", ["Action", "Type", "Source", "Target"]):
                if not (row[0] in allowed or int(row[1]) == 51):
                    raise ValueError(f"Unexpected custom action: {row}")
        upgrades = database.rows("Upgrade", ["UpgradeCode", "VersionMin", "VersionMax", "Attributes", "ActionProperty"])
        if any(row[0].upper() != UPGRADE_CODE for row in upgrades):
            raise ValueError("MSI would detect/replace another product")
        version = properties["ProductVersion"]
        if not any(row[1] == version and row[4] == "SAME_OR_NEWER_FOUND" and int(row[3]) & 2 for row in upgrades):
            raise ValueError("Missing equal/newer version guard")
        shortcuts = database.rows("Shortcut", ["Shortcut", "Directory_", "Target", "WkDir"])
        if shortcuts != [("QuickSwitchStartMenu", "QuickSwitchMenu", "Complete", "INSTALLFOLDER")]:
            raise ValueError(f"Unexpected shortcut: {shortcuts}")
        sequence = dict(database.rows("InstallExecuteSequence", ["Action", "Sequence"]))
        if not (int(sequence["InstallInitialize"]) < int(sequence["RemoveExistingProducts"])
                < int(sequence["InstallFinalize"])):
            raise ValueError("Major upgrade removal must run inside the rollback transaction")
        for cabinet, in database.rows("Media", ["Cabinet"]):
            if not cabinet.startswith("#"):
                raise ValueError("External cabinet is not supported")
            cab = work / cabinet[1:]
            database.extract_stream(cabinet[1:], cab)
            expand = Path(os.environ["SystemRoot"]) / "System32/expand.exe"
            result = subprocess.run([str(expand), str(cab), "-F:*", str(extracted)], capture_output=True)
            (work / f"{cab.stem}-expand.log").write_bytes(result.stdout + result.stderr)
            if result.returncode:
                raise RuntimeError("Cabinet extraction failed")
        files = database.rows("File", ["File", "Component_", "FileName", "FileSize"])
        payload = work / "payload"
        payload.mkdir()
        for file_id, component_id, name, size in files:
            directory = components[component_id][2]
            parts = [name.split("|")[-1]]
            while directory != "INSTALLFOLDER":
                parent, folder = directories[directory]
                parts.insert(0, folder.split("|")[-1])
                directory = parent
            if any(part in ("", ".", "..") or "/" in part or "\\" in part or ":" in part for part in parts):
                raise ValueError("Unsafe payload path")
            source = extracted / file_id
            if source.stat().st_size != int(size):
                raise ValueError(f"Cabinet file size mismatch: {file_id}")
            target = payload.joinpath(*parts)
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(source.read_bytes())
        manifest = json.loads((payload / "installer-manifest.json").read_text())
        expected = {r["path"] for r in manifest} | {"installer-manifest.json"}
        actual = {p.relative_to(payload).as_posix() for p in payload.rglob("*") if p.is_file()}
        if actual != expected:
            raise ValueError("MSI payload inventory does not match its manifest")
        for record in manifest:
            if checksum(payload / record["path"]) != record["sha256"]:
                raise ValueError(f"MSI payload hash mismatch: {record['path']}")
        result = dict(status="PASS", msi=str(msi), sha256=checksum(msi),
                      version=version, files=len(files), components=len(components),
                      template=database.template(),
                      upgrade_code=properties["UpgradeCode"], payload=str(payload),
                      installed=False, installer_actions_executed=False, hardware_access=False)
        (work / "verification.json").write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps(result, indent=2))
    finally:
        database.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--msi", required=True, type=Path)
    parser.add_argument("--work-directory", required=True, type=Path)
    args = parser.parse_args()
    verify(args.msi.resolve(strict=True), args.work_directory.resolve())


if __name__ == "__main__":
    main()
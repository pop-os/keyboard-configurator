//! Dependency-free Windows launcher for the separately installed Quick Switch build.
//! Compile with the same x86_64-pc-windows-gnu Rust toolchain as the application.
#![windows_subsystem = "windows"]

use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    os::windows::{ffi::OsStrExt, process::CommandExt},
    path::{Path, PathBuf},
    process::{self, Command},
    time::{SystemTime, UNIX_EPOCH},
};

const APP: &str = "KeyboardConfiguratorQuickSwitch";
const LOADERS: &str = "lib/gdk-pixbuf-2.0/2.10.0/loaders";

struct Cache(PathBuf);

impl Drop for Cache {
    fn drop(&mut self) {
        // Only the unique cache created by this invocation; never user settings.
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn profile_directory(roaming: &Path, explicit: Option<OsString>) -> PathBuf {
    explicit
        .map(PathBuf::from)
        .unwrap_or_else(|| roaming.join(APP))
}

fn base_command(root: &Path, program: &str) -> Result<Command, Box<dyn std::error::Error>> {
    let mut command = Command::new(root.join(program));
    // Do not inherit development GTK paths or modules from a different installation.
    for (name, _) in env::vars_os() {
        let name_upper = name.to_string_lossy().to_ascii_uppercase();
        if ["GTK_", "GDK_", "GIO_", "GSETTINGS_", "XDG_", "FONTCONFIG_"]
            .iter()
            .any(|prefix| name_upper.starts_with(prefix))
        {
            command.env_remove(name);
        }
    }
    let windows = PathBuf::from(env::var_os("SystemRoot").ok_or("SystemRoot is not set")?);
    let path = env::join_paths([root, &windows.join("System32"), &windows])?;
    command
        .current_dir(root)
        .env("PATH", path)
        .env("XDG_DATA_DIRS", root.join("share"))
        .env("GSETTINGS_SCHEMA_DIR", root.join("share/glib-2.0/schemas"))
        .env("GDK_PIXBUF_MODULEDIR", root.join(LOADERS))
        .env("FONTCONFIG_PATH", root.join("etc/fonts"))
        .env("FONTCONFIG_FILE", root.join("etc/fonts/fonts.conf"));
    Ok(command)
}

fn run() -> Result<i32, Box<dyn std::error::Error>> {
    let exe = env::current_exe()?;
    let root = exe.parent().ok_or("Launcher has no parent directory")?;
    let local = PathBuf::from(env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA is not set")?);
    let roaming = PathBuf::from(env::var_os("APPDATA").ok_or("APPDATA is not set")?);
    for relative in [
        "system76-keyboard-configurator.exe",
        "gdk-pixbuf-query-loaders.exe",
        "share/glib-2.0/schemas/gschemas.compiled",
        "etc/fonts/fonts.conf",
    ] {
        if !root.join(relative).is_file() {
            return Err(format!(
                "Required runtime file is missing: {relative}. Repair the MSI installation."
            )
            .into());
        }
    }
    let cache_root = local.join(APP).join("cache");
    fs::create_dir_all(&cache_root)?;
    let unique = format!(
        "loaders-{}-{}",
        process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    );
    let cache = Cache(cache_root.join(unique));
    fs::create_dir(&cache.0)?;
    let module_file = cache.0.join("loaders.cache");
    let mut query = base_command(root, "gdk-pixbuf-query-loaders.exe")?;
    // With no arguments the helper writes cache contents to stdout, not INSTALLDIR.
    query
        .env("GDK_PIXBUF_MODULE_FILE", &module_file)
        .creation_flags(0x08000000); // CREATE_NO_WINDOW for the console helper only.
    let output = query.output()?;
    if !output.status.success() || output.stdout.is_empty() {
        return Err(format!(
            "Image-loader discovery failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    if !String::from_utf8_lossy(&output.stdout).contains("pixbufloader_svg.dll") {
        return Err("GTK SVG image loader is missing; repair the MSI installation".into());
    }
    fs::write(&module_file, output.stdout)?;
    let status = base_command(root, "system76-keyboard-configurator.exe")?
        .env("GDK_PIXBUF_MODULE_FILE", module_file)
        .env("XDG_CACHE_HOME", &cache_root)
        .env(
            "CONFIGURATOR_PROFILE_DIR",
            profile_directory(&roaming, env::var_os("CONFIGURATOR_PROFILE_DIR")),
        )
        .args(env::args_os().skip(1))
        .status()?;
    // Cache stays alive until the app/daemon exits; parallel launches never share it.
    drop(cache);
    Ok(status.code().unwrap_or(1))
}

fn show_error(message: &str) {
    eprintln!("Quick Switch launcher: {message}");
    if env::var_os("CONFIGURATOR_LAUNCHER_NO_DIALOG").is_some() {
        return;
    }
    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(
            window: *mut std::ffi::c_void,
            text: *const u16,
            caption: *const u16,
            kind: u32,
        ) -> i32;
    }
    let wide = |text: &OsStr| text.encode_wide().chain(Some(0)).collect::<Vec<_>>();
    let text = wide(OsStr::new(message));
    let title = wide(OsStr::new(
        "Keyboard Configurator Quick Switch - Startup error",
    ));
    unsafe {
        MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), 0x10);
    }
}

fn main() {
    match run() {
        Ok(code) => process::exit(code),
        Err(error) => {
            show_error(&error.to_string());
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_are_separate() {
        let roaming = env::temp_dir().join("profile-fixture").join("Roaming");
        assert_eq!(profile_directory(&roaming, None), roaming.join(APP));
    }

    #[test]
    fn explicit_isolation_is_preserved() {
        let isolated = env::temp_dir().join("isolated smoke").join("Unicode Ω");
        assert_eq!(
            profile_directory(Path::new("unused"), Some(isolated.clone().into_os_string())),
            isolated
        );
    }

    #[test]
    fn command_uses_current_windows_directory_and_local_runtime() {
        let root = env::temp_dir().join("launcher-runtime-fixture");
        let windows = PathBuf::from(env::var_os("SystemRoot").unwrap());
        let command = base_command(&root, "fixture.exe").unwrap();
        assert_eq!(command.get_program(), root.join("fixture.exe"));
        assert_eq!(command.get_current_dir(), Some(root.as_path()));
        let path = command
            .get_envs()
            .find(|(key, _)| *key == OsStr::new("PATH"))
            .and_then(|(_, value)| value)
            .unwrap();
        assert_eq!(
            env::split_paths(path).collect::<Vec<_>>(),
            vec![root, windows.join("System32"), windows]
        );
    }

    #[test]
    fn cache_guard_removes_only_its_own_directory() {
        let root = env::temp_dir().join(format!("configurator-cache-test-{}", process::id()));
        fs::create_dir_all(root.join("settings")).unwrap();
        fs::write(root.join("settings/keep"), "keep").unwrap();
        fs::create_dir(root.join("ephemeral")).unwrap();
        fs::write(root.join("ephemeral/loaders.cache"), "cache").unwrap();
        drop(Cache(root.join("ephemeral")));
        assert!(!root.join("ephemeral").exists());
        assert!(root.join("settings/keep").is_file());
        fs::remove_dir_all(root).unwrap();
    }
}

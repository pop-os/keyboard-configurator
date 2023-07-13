use std::{
    env,
    fs::{self, File},
    io::Write,
    path::Path,
};

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let mut outfile = File::create(Path::new(&out_dir).join("keyboards.rs")).unwrap();

    writeln!(outfile, "keyboards![").unwrap();

    println!("cargo:rerun-if-changed=../layouts/system76");
    for i in fs::read_dir("../layouts/system76").unwrap() {
        let i = i.unwrap();

        let path = i.path().join("meta.json");
        let meta = serde_json::from_reader::<_, serde_json::Map<_, _>>(File::open(&path).unwrap())
            .unwrap();
        let file_name = i.file_name().into_string().unwrap();
        let keyboard = meta
            .get("keyboard")
            .and_then(|x| x.as_str())
            .unwrap_or_else(|| {
                panic!("'keyboard:' not found in {}", path.display());
            });
        let is_qmk = meta
            .get("is_qmk")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        writeln!(
            outfile,
            "    (\"system76/{}\", \"{}\", {}),",
            file_name, keyboard, is_qmk
        )
        .unwrap();

        println!("cargo:rerun-if-changed=../layouts/system76/{}", file_name);
        println!("cargo:rerun-if-changed=../layouts/keyboards/{}", keyboard);
    }

    writeln!(outfile, "];").unwrap();
}

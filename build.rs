use std::path::Path;

fn main() {
    // En Windows, el icono dentro de cerrojo.exe (Explorador y barra de tareas).
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("packaging/cerrojo.ico");
        res.set("FileDescription", "Cerrojo");
        res.set("ProductName", "Cerrojo");
        let _ = res.compile();
    }

    // Los recursos sueltos del ayudante (assets/filtro/res/…, los deja
    // android/build.sh): la lista va a src/apk.rs como FILTER_RES.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/filtro");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = Vec::new();
    collect(&root, &root.join("res"), &mut files);
    files.sort();
    if files.is_empty() {
        println!("cargo:warning=assets/filtro no tiene el vigía: compílalo con android/build.sh");
    }
    let mut out = String::from("pub const FILTER_RES: &[(&str, &[u8])] = &[\n");
    for (name, path) in files {
        out.push_str(&format!("    ({name:?}, include_bytes!({path:?})),\n"));
    }
    out.push_str("];\n");
    let dest = Path::new(&std::env::var("OUT_DIR").unwrap()).join("filtro_res.rs");
    std::fs::write(dest, out).unwrap();
}

fn collect(root: &Path, dir: &Path, files: &mut Vec<(String, String)>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let path = e.path();
        println!("cargo:rerun-if-changed={}", path.display());
        if path.is_dir() {
            collect(root, &path, files);
        } else {
            let name = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            files.push((name, path.to_string_lossy().into_owned()));
        }
    }
}

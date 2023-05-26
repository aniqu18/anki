// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

use std::collections::HashSet;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use anyhow::Context;
use anyhow::Result;
use camino::Utf8Path;
use once_cell::unsync::Lazy;
use walkdir::WalkDir;

const NONSTANDARD_HEADER: &[&str] = &[
    "./pylib/anki/_vendor/stringcase.py",
    "./pylib/anki/importing/pauker.py",
    "./pylib/anki/importing/supermemo_xml.py",
    "./pylib/anki/statsbg.py",
    "./pylib/tools/protoc-gen-mypy.py",
    "./python/pyqt/install.py",
    "./python/write_wheel.py",
    "./qt/aqt/mpv.py",
    "./qt/aqt/winpaths.py",
    "./qt/bundle/build.rs",
    "./qt/bundle/src/main.rs",
];

const IGNORED_FOLDERS: &[&str] = &[
    "./out",
    "./node_modules",
    "./qt/aqt/forms",
    "./tools/workspace-hack",
    "./qt/bundle/PyOxidizer",
];

fn main() -> Result<()> {
    let want_fix = env::args().nth(1) == Some("fix".to_string());
    let mut ctx = LintContext::new(want_fix);
    ctx.walk_folders(Path::new("."))?;
    if ctx.found_problems {
        std::process::exit(1);
    }
    Ok(())
}

struct LintContext {
    want_fix: bool,
    unstaged_changes: Lazy<()>,
    found_problems: bool,
    nonstandard_headers: HashSet<&'static Utf8Path>,
}

impl LintContext {
    pub fn new(want_fix: bool) -> Self {
        Self {
            want_fix,
            unstaged_changes: Lazy::new(check_for_unstaged_changes),
            found_problems: false,
            nonstandard_headers: NONSTANDARD_HEADER.iter().map(Utf8Path::new).collect(),
        }
    }

    pub fn walk_folders(&mut self, root: &Path) -> Result<()> {
        let ignored_folders: HashSet<_> = IGNORED_FOLDERS.iter().map(Utf8Path::new).collect();
        let walker = WalkDir::new(root).into_iter();
        for entry in walker.filter_entry(|e| {
            !ignored_folders.contains(&Utf8Path::from_path(e.path()).expect("utf8"))
        }) {
            let entry = entry.unwrap();
            let path = Utf8Path::from_path(entry.path()).context("utf8")?;

            let exts: HashSet<_> = ["py", "ts", "rs", "svelte", "mjs"]
                .into_iter()
                .map(Some)
                .collect();
            if exts.contains(&path.extension()) {
                self.check_copyright(path)?;
            }
        }
        Ok(())
    }

    fn check_copyright(&mut self, path: &Utf8Path) -> Result<()> {
        if path.file_name().unwrap().ends_with(".d.ts") {
            return Ok(());
        }
        let head = head_of_file(path)?;
        if head.is_empty() {
            return Ok(());
        }
        if self.nonstandard_headers.contains(&path) {
            return Ok(());
        }
        let missing = !head.contains("Ankitects Pty Ltd and contributors");
        if missing {
            if self.want_fix {
                Lazy::force(&self.unstaged_changes);
                fix_copyright(path)?;
            } else {
                println!("missing standard copyright header: {:?}", path);
                self.found_problems = true;
            }
        }
        Ok(())
    }
}

fn head_of_file(path: &Utf8Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut buffer = vec![0; 256];
    let size = file.read(&mut buffer)?;
    buffer.truncate(size);
    Ok(String::from_utf8(buffer).unwrap_or_default())
}

fn fix_copyright(path: &Utf8Path) -> Result<()> {
    let header = match path.extension().unwrap() {
        "py" => {
            r#"# Copyright: Ankitects Pty Ltd and contributors
# License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html
"#
        }
        "ts" | "rs" | "mjs" => {
            r#"// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html
"#
        }
        "svelte" => {
            r#"<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html
-->
"#
        }
        _ => unreachable!(),
    };

    let data = fs::read_to_string(path).with_context(|| format!("reading {path}"))?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .open(path)
        .with_context(|| format!("opening {path}"))?;
    write!(file, "{}{}", header, data).with_context(|| format!("writing {path}"))?;
    Ok(())
}

fn check_for_unstaged_changes() {
    let output = Command::new("git").arg("diff").output().unwrap();
    if !output.stdout.is_empty() {
        println!("stage any changes first");
        std::process::exit(1);
    }
}

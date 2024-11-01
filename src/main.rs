use std::fs;
use std::process::exit;
use std::process::Command;
use std::{env, path::Path};

use serde::Deserialize;
use tabled::{settings::Style, Table, Tabled};

#[allow(dead_code)]
#[derive(Deserialize)]
struct Jvm {
    #[serde(rename = "JVMArch")]
    arch: String,
    #[serde(rename = "JVMBundleID")]
    bundle_id: String,
    #[serde(rename = "JVMEnabled")]
    enabled: bool,
    #[serde(rename = "JVMHomePath")]
    home_path: String,
    #[serde(rename = "JVMName")]
    name: String,
    #[serde(rename = "JVMPlatformVersion")]
    platform_version: String,
    #[serde(rename = "JVMVendor")]
    vendor: String,
    #[serde(rename = "JVMVersion")]
    version: String,
}

impl Jvm {
    fn major_version(&self) -> u16 {
        let version = self.version.clone();
        let (major_version, rest) = version.split_once('.').unwrap_or_else(|| {
            exit_with_err(
                &format!(
                "Version number {} of jvm {} should contain at least one period!",
                self.version, self.home_path
            ),
                false,
            )
        });

        let major_version = match major_version {
            "1" => rest.split_once('.').unwrap_or_else(|| {
                exit_with_err(&format!(
                    "Version number {} of jvm {} should contain at least two periods when 1-prefixed!",
                    self.version, self.home_path
                ), false)
            }).0,
            otherwise => otherwise,
        };

        major_version.parse::<u16>().unwrap_or_else(|_| {
            exit_with_err(
                &format!(
                    "Major version number {} of JVM {} should be numeric!",
                    major_version, self.home_path
                ),
                false,
            )
        })
    }

    fn to_display(&self) -> DisplayJvm {
        DisplayJvm {
            arch: self.arch.clone(),
            name: self.name.clone(),
            version: self.version.clone(),
        }
    }
}

#[derive(Tabled)]
struct DisplayJvm {
    version: String,
    name: String,
    arch: String,
}

fn list_all(jvms: &[Jvm]) {
    let table = jvms
        .iter()
        .map(|jvm| jvm.to_display())
        .collect::<Vec<DisplayJvm>>();

    let table = Table::new(table).with(Style::rounded()).to_string();

    println!("{}", table);
}

#[derive(Debug)]
struct V {
    number: u16,
    distro: Option<String>,
}

fn get_distro(spec: &str) -> Option<String> {
    let dspec: String = spec.chars().take_while(|c| c.is_alphabetic()).collect();
    if dspec.is_empty() {
        None
    } else {
        Some(dspec)
    }
}

fn get_version_from_input(spec: &str) -> Option<V> {
    let distro = get_distro(spec);
    let number = match spec
        .chars()
        .skip_while(|c| c.is_alphabetic() || *c == '-')
        .collect::<String>()
        .split_once('.')
    {
        Some(("1", ver)) => ver.parse::<u16>().ok(),
        Some((ver, _)) => ver.parse::<u16>().ok(),
        _ => spec.parse::<u16>().ok(),
    };
    number.map(|n| V { distro, number: n })
}

fn distro_matches(v: &V, jvm: &Jvm) -> bool {
    match &v.distro {
        None => true,
        Some(distro) => {
            jvm.bundle_id.contains(distro) || jvm.home_path.contains(distro)
        }
    }
}

fn switch_to(spec: &str, jvms: &[Jvm], quiet: bool) {
    let old_java_home = env::var("JAVA_HOME").ok();
    if let Some(v) = get_version_from_input(spec) {
        let selection = jvms
            .iter()
            .find(|jvm| jvm.major_version() == v.number && distro_matches(&v, jvm))
            .unwrap_or_else(|| {
                panic!(
                    "You requested a JVM of version {:?}, but no such JVM is installed!",
                    v
                )
            });

        println!("{}", selection.home_path);
        if !quiet
            || (old_java_home.is_none()
                || old_java_home != Some(selection.home_path.clone()))
        {
            eprintln!("Activating Java {}", selection.name);
        }
    } else if !quiet {
        panic!("Did not understand version spec {}", spec);
    } else {
        exit(0)
    }
}

fn find_version_string_from_tool_versions(path: &Path) -> Option<String> {
    let contents = fs::read_to_string(path).ok()?;
    let java_line = contents
        .lines()
        .map(|l| l.trim())
        .find(|l| l.starts_with("java"))?;

    Some(java_line.replace("java ", ""))
}

fn find_version_string_from_file(dir: &Path, quiet: bool) -> String {
    let java_version_file = dir.join(".java-version");
    let tool_version_file = dir.join(".tool-versions");
    if fs::exists(java_version_file.clone()).unwrap() {
        let contents = fs::read_to_string(java_version_file).unwrap();
        contents.trim().to_string()
    } else if let Some(spec) =
        find_version_string_from_tool_versions(&tool_version_file)
    {
        spec
    } else if let Some(parent) = dir.parent() {
        find_version_string_from_file(parent, quiet)
    } else {
        exit_with_err(
            "No .java_version file found in this directory or any parent!",
            quiet,
        );
    }
}

fn exit_with_err(msg: &str, quiet: bool) -> ! {
    if quiet {
        exit(0)
    } else {
        eprintln!("{}", msg);
        exit(1)
    }
}

fn display_zsh_init() {
    let binr = env::current_exe().unwrap();
    let bin = binr.display();
    println!(
        r#"
jdk() {{
    if [[ -n "$1" ]]; then
        local located="$({bin} $1)"
        if [[ -n "$located" ]]; then
            export JAVA_HOME="$located"
        fi
    else
        {bin}
    fi
}}
autoload -U add-zsh-hook
_jvmvj_cd_hook() {{
    local located="$({bin} auto --quiet)"
    if [[ -n "$located" ]]; then
        export JAVA_HOME="$located"
    fi
}}
add-zsh-hook chpwd _jvmvj_cd_hook
"#
    );
}

fn main() {
    let java_home_in = Command::new("/usr/libexec/java_home")
        .arg("-X")
        .output()
        .expect("Failed to run java_home. Is this a MacOS system?")
        .stdout;

    let jvms: Vec<Jvm> = plist::from_bytes(&java_home_in).expect(
        "Failed to parse the list of JVMs. This should probably be raised as a bug!",
    );

    let args: Vec<String> = env::args().collect();

    match args.get(1) {
        None => list_all(&jvms),
        Some(cmd) if cmd == "init" => display_zsh_init(),
        Some(cmd) if cmd == "auto" => {
            let quiet = args.iter().any(|arg| arg == "-q" || arg == "--quiet");
            let here = Path::new(".")
                .canonicalize()
                .expect("?? Couldn't find the path to this directory? What?");
            let spec = find_version_string_from_file(&here, quiet);
            switch_to(&spec, &jvms, quiet)
        }
        Some(spec) => switch_to(spec, &jvms, false),
    }
}

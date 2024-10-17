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
            exit_with_err(&format!(
                "Version number {} of jvm {} should contain at least one period!",
                self.version, self.home_path
            ))
        });

        let major_version = match major_version {
            "1" => rest.split_once('.').unwrap_or_else(|| {
                exit_with_err(&format!(
                    "Version number {} of jvm {} should contain at least two periods when 1-prefixed!",
                    self.version, self.home_path
                ))
            }).0,
            otherwise => otherwise,
        };

        major_version.parse::<u16>().unwrap_or_else(|_| {
            exit_with_err(&format!(
                "Major version number {} of JVM {} should be numeric!",
                major_version, self.home_path
            ))
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

fn get_version_from_input(spec: &str) -> Option<u16> {
    match spec.split_once('.') {
        Some(("1", ver)) => ver.parse::<u16>().ok(),
        Some((ver, _)) => ver.parse::<u16>().ok(),
        _ => spec.parse::<u16>().ok(),
    }
}

fn switch_to(spec: &str, jvms: &[Jvm]) {
    if let Some(v) = get_version_from_input(spec) {
        let selection = jvms
            .iter()
            .find(|jvm| jvm.major_version() == v)
            .unwrap_or_else(|| {
                panic!(
                    "You requested a JVM of version {}, but no such JVM is installed!",
                    v
                )
            });

        println!("{}", selection.home_path);
        eprintln!("Activating Java {}", selection.name);
    } else {
        panic!("Did not understand version spec {}", spec);
    }
}

fn find_from_file(dir: &Path) -> String {
    let jv_file = dir.join(".java-version");
    if fs::exists(jv_file.clone()).unwrap() {
        let contents = fs::read_to_string(jv_file).unwrap();
        contents.trim().to_string()
    } else if let Some(parent) = dir.parent() {
        find_from_file(parent)
    } else {
        exit_with_err(
            "No .java_version file found in this directory or any parent!",
        );
    }
}

fn exit_with_err(msg: &str) -> ! {
    eprintln!("{}", msg);
    exit(1)
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
        Some(spec) if spec == "use" => {
            // TODO be quiet if "switching" to same jdk
            let here = Path::new(".")
                .canonicalize()
                .expect("?? Couldn't find the path to this directory? What?");
            let spec = find_from_file(&here);
            switch_to(&spec, &jvms)
        }
        Some(spec) => switch_to(spec, &jvms),
    }
}

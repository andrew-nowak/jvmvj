use std::env;
use std::process::Command;

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
        let (major_version, rest) = version.split_once(".").expect(&format!(
            "Version number {} of jvm {} should contain at least one period!",
            self.version, self.home_path
        ));

        let major_version = match major_version {
            "1" => rest.split_once(".").expect(&format!("Version number {} of jvm {} should contain at least two periods when 1-prefixed!", self.version, self.home_path)).0,
            otherwise => otherwise
        };

        let major_version = major_version.parse::<u16>().expect(&format!(
            "Major version number {} of JVM {} should be numeric!",
            major_version, self.home_path
        ));

        major_version
    }

    fn to_display(&self, i: usize) -> DisplayJvm {
        DisplayJvm {
            i,
            arch: self.arch.clone(),
            name: self.name.clone(),
            version: self.version.clone(),
        }
    }
}

#[derive(Tabled)]
struct DisplayJvm {
    i: usize,
    version: String,
    name: String,
    arch: String,
}

fn list_all(jvms: &[Jvm]) {
    let table = jvms
        .iter()
        .enumerate()
        .map(|(i, jvm)| jvm.to_display(i + 1))
        .collect::<Vec<DisplayJvm>>();

    let table = Table::new(table).with(Style::rounded()).to_string();

    println!("{}", table);
}

fn get_version_from_input(spec: &str) -> Option<u16> {
    match spec.split_once(".") {
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
            .expect(&format!(
                "You requested a JVM of version {}, but no such JVM is installed!",
                v
            ));

        println!("{}", selection.home_path);
        eprintln!("Activating Java {}", selection.name);
    } else {
        panic!("Did not understand version spec {}", spec);
    }
}

fn main() {
    let java_home_in = Command::new("/usr/libexec/java_home")
        .arg("-X")
        .output()
        .expect("Failed to run java_home. Is this a MacOS system?")
        .stdout;

    let jvms: Vec<Jvm> = plist::from_bytes(&java_home_in)
        .expect("Failed to parse the list of JVMs. This should probably be raised as a bug!");

    let args: Vec<String> = env::args().collect();

    match args.get(1) {
        None => list_all(&jvms),
        Some(spec) => switch_to(spec, &jvms),
    }
}

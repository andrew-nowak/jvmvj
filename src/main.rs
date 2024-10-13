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


fn main() {
    let java_home_in = Command::new("/usr/libexec/java_home")
        .arg("-X")
        .output()
        .expect("Failed to run java_home. Is this a MacOS system?")
        .stdout;

    let jvms: Vec<Jvm> = plist::from_bytes(&java_home_in)
        .expect("Failed to parse the list of JVMs. This should probably be raised as a bug!");

    list_all(&jvms);
}

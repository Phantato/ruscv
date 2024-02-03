use std::fs::{read_dir, File};
use std::io::{Error, Write};

static TARGET_PATH: &str = "target/riscv64gc-unknown-none-elf/release/";
fn main() {
    println!("cargo:rerun-if-changed=src/kernel.ld");
    println!("cargo:rerun-if-changed=../user/src/");
    println!("cargo:rerun-if-changed=../{}", TARGET_PATH);
    insert_app_data().unwrap();
}

fn insert_app_data() -> Result<(), Error> {
    let mut f = File::create("src/link_app.s").unwrap();
    let mut apps: Vec<_> = read_dir("../user/src/bin")
        .unwrap()
        .into_iter()
        .map(|dir_entry| {
            let mut name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
            name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
            name_with_ext
        })
        .collect();
    apps.sort();

    writeln!(
        f,
        r#"
    .align 3
    .section .rodata
    .global _num_app
_num_app:
    .quad {}"#,
        apps.len()
    )?;

    for i in 0..apps.len() {
        writeln!(f, r#"    .quad app_{}_start"#, i)?;
    }
    writeln!(
        f,
        r#"    .quad app_{}_end
    .align 1"#,
        apps.len() - 1
    )?;

    for (_, app) in apps.iter().enumerate() {
        let str = app.split_at(2);
        writeln!(f, r#"    .string "{}""#, str.1)?
    }
    writeln!(
        f,
        r#"
    .align 3"#
    )?;
    for (idx, app) in apps.iter().enumerate() {
        println!("app_{}: {}", idx, app);
        writeln!(
            f,
            r#"
    .section .rodata
    .global app_{0}_start
    .global app_{0}_end
    .align 3
app_{0}_start:
    .incbin "{2}{1}"
app_{0}_end:"#,
            idx, app, TARGET_PATH
        )?;
    }
    Ok(())
}

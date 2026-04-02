use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
enum ServerFormat {
    Paper,
    Vanilla,
}

/// Vanilla server root files.
const VANILLA_FILES: &[&str] = &[
    "server.properties",
    "whitelist.json",
    "banned-players.json",
    "banned-ips.json",
    "ops.json",
    "eula.txt",
    "server-icon.png",
    "usercache.json",
];

/// Paper-specific root files (not part of a vanilla server).
const PAPER_FILES: &[&str] = &[
    "bukkit.yml",
    "spigot.yml",
    "paper.yml",
    "commands.yml",
    "permissions.yml",
    "help.yml",
];

/// Paper-specific directories.
const PAPER_DIRS: &[&str] = &["config", "plugins"];

fn detect_world_name(server_dir: &Path) -> String {
    let props = server_dir.join("server.properties");
    if let Ok(content) = fs::read_to_string(&props) {
        for line in content.lines() {
            let line = line.trim();
            if let Some(value) = line.strip_prefix("level-name=") {
                let name = value.trim();
                if !name.is_empty() {
                    return name.to_string();
                }
            }
        }
    }
    "world".to_string()
}

fn detect_format(source: &Path, world_name: &str) -> ServerFormat {
    let has_paper_nether = source.join(format!("{world_name}_nether")).is_dir();
    let has_paper_end = source.join(format!("{world_name}_the_end")).is_dir();
    let has_vanilla_nether = source.join(world_name).join("DIM-1").is_dir();
    let has_vanilla_end = source.join(world_name).join("DIM1").is_dir();

    // Paper uses separate top-level dirs for nether/end
    if has_paper_nether || has_paper_end {
        ServerFormat::Paper
    } else if has_vanilla_nether || has_vanilla_end {
        ServerFormat::Vanilla
    } else {
        // No nether/end found — check for Paper-specific files
        if source.join("paper.yml").is_file()
            || source.join("config").join("paper-global.yml").is_file()
        {
            ServerFormat::Paper
        } else {
            ServerFormat::Vanilla
        }
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn copy_dir_if_exists(src: &Path, dst: &Path, label: &str) -> std::io::Result<bool> {
    if src.is_dir() {
        println!("  Copying {label}...");
        copy_dir_recursive(src, dst)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn copy_file_if_exists(src: &Path, dst: &Path, label: &str) -> std::io::Result<bool> {
    if src.is_file() {
        println!("  Copying {label}...");
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn copy_dir_excluding(src: &Path, dst: &Path, exclude: &[&str]) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if exclude.iter().any(|e| *e == name_str.as_ref()) {
            continue;
        }
        let src_path = entry.path();
        let dst_path = dst.join(&name);
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Copy dimension data from a Paper separate folder into the vanilla world dir.
/// Handles both `<world>_nether/DIM-1/` layout and flat layout.
fn copy_paper_dimension(paper_dim_dir: &Path, dst_dim: &Path, dim_subdir: &str) -> std::io::Result<()> {
    let dim_path = paper_dim_dir.join(dim_subdir);
    if dim_path.is_dir() {
        copy_dir_recursive(&dim_path, dst_dim)?;
    } else {
        // Flat layout — copy everything except world-level metadata files
        fs::create_dir_all(dst_dim)?;
        for entry in fs::read_dir(paper_dim_dir)? {
            let entry = entry?;
            let name_str = entry.file_name();
            let name = name_str.to_string_lossy();
            if matches!(
                name.as_ref(),
                "level.dat" | "level.dat_old" | "uid.dat" | "session.lock" | "paper-world.yml"
            ) {
                continue;
            }
            let src_path = entry.path();
            let dst_path = dst_dim.join(&*name_str);
            if src_path.is_dir() {
                copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path)?;
            }
        }
    }
    Ok(())
}

fn paper_to_vanilla(source: &Path, dest: &Path, world_name: &str) -> std::io::Result<()> {
    let src_world = source.join(world_name);
    let dst_world = dest.join(world_name);

    if src_world.is_dir() {
        println!("  Copying overworld...");
        copy_dir_recursive(&src_world, &dst_world)?;
    }

    let paper_nether = source.join(format!("{world_name}_nether"));
    if paper_nether.is_dir() {
        println!("  Copying nether...");
        copy_paper_dimension(&paper_nether, &dst_world.join("DIM-1"), "DIM-1")?;
    }

    let paper_end = source.join(format!("{world_name}_the_end"));
    if paper_end.is_dir() {
        println!("  Copying the end...");
        copy_paper_dimension(&paper_end, &dst_world.join("DIM1"), "DIM1")?;
    }

    Ok(())
}

fn vanilla_to_paper(source: &Path, dest: &Path, world_name: &str) -> std::io::Result<()> {
    let src_world = source.join(world_name);
    let dst_world = dest.join(world_name);

    if src_world.is_dir() {
        println!("  Copying overworld (excluding DIM-1, DIM1)...");
        copy_dir_excluding(&src_world, &dst_world, &["DIM-1", "DIM1"])?;
    }

    let vanilla_nether = src_world.join("DIM-1");
    if vanilla_nether.is_dir() {
        let paper_nether = dest.join(format!("{world_name}_nether"));
        println!("  Copying nether -> {world_name}_nether/DIM-1/...");
        fs::create_dir_all(&paper_nether)?;
        copy_dir_recursive(&vanilla_nether, &paper_nether.join("DIM-1"))?;
        // Paper needs level.dat, uid.dat, session.lock in dimension folders
        for f in &["level.dat", "uid.dat", "session.lock"] {
            let src_f = src_world.join(f);
            if src_f.is_file() {
                fs::copy(&src_f, paper_nether.join(f))?;
            }
        }
    }

    let vanilla_end = src_world.join("DIM1");
    if vanilla_end.is_dir() {
        let paper_end = dest.join(format!("{world_name}_the_end"));
        println!("  Copying the end -> {world_name}_the_end/DIM1/...");
        fs::create_dir_all(&paper_end)?;
        copy_dir_recursive(&vanilla_end, &paper_end.join("DIM1"))?;
        for f in &["level.dat", "uid.dat", "session.lock"] {
            let src_f = src_world.join(f);
            if src_f.is_file() {
                fs::copy(&src_f, paper_end.join(f))?;
            }
        }
    }

    Ok(())
}

fn copy_files(source: &Path, dest: &Path, files: &[&str]) -> std::io::Result<()> {
    for file in files {
        copy_file_if_exists(&source.join(file), &dest.join(file), file)?;
    }
    Ok(())
}

fn convert(source: &Path, dest: &Path) -> std::io::Result<()> {
    if !source.is_dir() {
        eprintln!("Error: source directory '{}' does not exist", source.display());
        std::process::exit(1);
    }
    fs::create_dir_all(dest)?;

    let world_name = detect_world_name(source);
    let format = detect_format(source, &world_name);

    let (from, to) = match format {
        ServerFormat::Paper => ("Paper MC", "Vanilla"),
        ServerFormat::Vanilla => ("Vanilla", "Paper MC"),
    };

    println!("Detected: {from} server");
    println!("Converting {from} -> {to}");
    println!("  Source: {}", source.display());
    println!("  Dest:   {}", dest.display());
    println!("  World:  {world_name}");
    println!();

    match format {
        ServerFormat::Paper => paper_to_vanilla(source, dest, &world_name)?,
        ServerFormat::Vanilla => vanilla_to_paper(source, dest, &world_name)?,
    }

    println!();
    copy_files(source, dest, VANILLA_FILES)?;

    match format {
        ServerFormat::Paper => {
            let old = dest.join("papercfg.old");
            fs::create_dir_all(&old)?;
            println!("  Saving Paper-specific files to papercfg.old/...");
            copy_files(source, &old, PAPER_FILES)?;
            for dir in PAPER_DIRS {
                copy_dir_if_exists(&source.join(dir), &old.join(dir), dir)?;
            }
        }
        ServerFormat::Vanilla => {
            copy_files(source, dest, PAPER_FILES)?;
            for dir in PAPER_DIRS {
                copy_dir_if_exists(&source.join(dir), &dest.join(dir), dir)?;
            }
        }
    }

    println!();
    println!("Done! {to} server written to: {}", dest.display());
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <source> <dest>", args[0]);
        eprintln!("Auto-detect and convert between Paper MC and Vanilla MC server formats");
        std::process::exit(1);
    }
    let source = PathBuf::from(&args[1]);
    let dest = PathBuf::from(&args[2]);
    if let Err(e) = convert(&source, &dest) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

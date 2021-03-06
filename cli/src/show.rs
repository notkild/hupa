use DEFAULT_FSO;
use clap::ArgMatches;
use colored::*;
use common::resolve_names;
use humansize::*;
use libhupa::*;

/// List subcommand
pub fn list_subcommand(hupas: Vec<Hupa>, sub_m: &ArgMatches) {
    let size_enabled = sub_m.is_present("size");
    let mut categories = hupas.into_categories();
    if let Some(val) = sub_m.values_of("category") {
        let vals = val.into_iter()
            .map(|s| s.to_owned())
            .collect::<Vec<String>>();
        categories = categories
            .into_iter()
            .filter(|c| vals.contains(c.get_name()))
            .collect();
    }
    categories.sort();
    for category in &categories {
        print_category(category, size_enabled);
    }
}

pub fn show_subcommand(mut hupas: Vec<Hupa>, sub_m: &ArgMatches) {
    let size_enabled = sub_m.is_present("size");
    if let Some(val) = sub_m.values_of("hupa") {
        hupas = resolve_names(
            &val.into_iter()
                .map(|s| s.to_owned())
                .collect::<Vec<String>>(),
            &hupas,
        );
    }
    for hupa in hupas {
        let size = hupa.get_backup_size().unwrap_or(0);
        print_hupa(
            &hupa,
            &size.file_size(DEFAULT_FSO).unwrap_or(String::new()),
            size_enabled,
            "-",
        );
    }
}

/// Print category
fn print_category(category: &Category, size_enabled: bool) {
    let (sizes, total_str) = compute_size(category, size_enabled);
    println!(
        "{}: {} item(s){}",
        category.get_name().bold(),
        category.len(),
        total_str
    );
    for (i, hupa) in category.iter().enumerate() {
        print_hupa(hupa, &sizes[i], size_enabled, " --");
    }
}

/// Print hupa
fn print_hupa(hupa: &Hupa, size: &str, size_enabled: bool, base: &str) {
    println!("{} {}:", base, hupa.get_name().yellow().bold());
    println!("  {} origin: {}", base, hupa.get_origin().display());
    if size_enabled {
        println!("  {} backup size: {}", base, size);
    }
    let autobackup = if hupa.is_autobackup_enabled() {
        format!("{}", "enabled".green())
    } else {
        format!("{}", "disabled".red())
    };
    println!("  {} autobackup is {}", base, autobackup);
    println!("  {} description: {}", base, hupa.get_desc());
    let needed_vars = hupa.get_needed_vars();
    if needed_vars.len() > 0 {
        println!(
            "  {} needed vars: {}",
            base,
            needed_vars
                .iter()
                .map(|s| format!("{} ", s))
                .collect::<String>()
        );
    }
}

/// Compute size
fn compute_size(category: &Category, size_enabled: bool) -> (Vec<String>, String) {
    let mut sizes = Vec::new();
    let mut total = 0;
    let mut total_str = String::new();
    if size_enabled {
        for hupa in category {
            let size = hupa.get_backup_size().unwrap_or(0);
            total += size;
            sizes.push(size.file_size(DEFAULT_FSO).unwrap());
        }
        total_str = format!(", total {}", total.file_size(DEFAULT_FSO).unwrap());
    } else {
        for _ in 0..category.len() {
            sizes.push(String::new());
        }
    }
    (sizes, total_str)
}

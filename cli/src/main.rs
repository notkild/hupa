//! Tool to backup and restore data

extern crate app_dirs;
#[macro_use]
extern crate clap;
extern crate colored;
extern crate humansize;
extern crate libhupa;

#[macro_use]
mod macros;
mod hupa;
mod io;

use hupa::*;
use io::*;

use clap::AppSettings;
use colored::*;
use humansize::{FileSize, file_size_opts};
use humansize::file_size_opts::FileSizeOpts;
use libhupa::*;

const DEFAULT_FSO: FileSizeOpts = FileSizeOpts {
    space: false,
    ..file_size_opts::DECIMAL
};

fn main() {
    // TODO add ability to modify config
    // TODO add ability to modify hupa
    // TODO seperate each subcommand in module
    let matches = clap_app!(hupa =>
        (version: crate_version!())
        (author: "Bastien Badzioch <notkild@gmail.com>")
        (about: "Hupa is a tool to backup and restore data")
        (setting: AppSettings::SubcommandRequiredElseHelp)
        (@arg config: -c --config +global +takes_value "Set config path")
        (@arg metadata: --metadata +global +takes_value "Set metadata path")
        (@subcommand add =>
            (about: "Add a new hupa")
            (@arg count: -n --count +takes_value "Set the number of hupa to add"))
        (@subcommand remove =>
            (about: "Remove one or multiple hupas")
            (aliases: &["rm", "del"])
            (@arg hupa: +takes_value +multiple "Hupa(s) to remove"))
        (@subcommand modify =>
            (about: "Modify parameter of an hupa")
            (@arg hupa: +takes_value +multiple "Hupa(s) to modify"))
        (@subcommand backup =>
            (about: "Backup hupa(s)")
            (@arg all: -a --all conflicts_with[hupa] "Backup all hupas")
            (@arg hupa: +takes_value +multiple "Hupa(s) to backup"))
        (@subcommand restore =>
            (about: "Restore hupa(s)")
            (@arg all: -a --all conflicts_with[hupa] "Restore all hupas")
            (@arg hupa: +takes_value +multiple "Hupa(s) to restore")
            (@arg ignore_root: -i --ignore-root "Ignore hupas that need root access, only for unix"))
        (@subcommand generate =>
            (about: "Generate an archive of all hupas")
            (@arg format: -f --format +takes_value possible_value[tar zip] "File format to use for achive")
            (@arg output: -o --output +takes_value "Output directory/file of the created archive"))
        (@subcommand unpack =>
            (about: "Unpack an hupa archive")
            (@arg archive: +required +takes_value "Path to the archive"))
        (@subcommand print =>
            (about: "Print list of all hupas")
            (@arg size: -s --size "Show files sizes"))
        (@subcommand clean =>
            (about: "Clean hupa(s)")
            (@arg all: -a -all "Clean all hupas")
            (@arg hupa: +takes_value +multiple "Hupa(s) to clean"))).get_matches();

    let config = match matches.value_of("config") {
            Some(s) => Config::read_config_from_path(s),
            None => Config::read_config(),
        }
        .unwrap_or(Config::default());
    let mut hupas = read_metadata_from_config(&config).unwrap_or(Vec::new());
    if let Some(p) = matches.value_of("metadata") {
        let mut f = ::std::fs::File::open(p).expect(&format!("Can't open {}", p));
        hupas = read_metadata(&mut f, None).unwrap_or(hupas);
    }


    match matches.subcommand() {
        ("add", Some(sub_m)) => {
            let count = sub_m
                .value_of("count")
                .unwrap_or("1")
                .parse::<usize>()
                .unwrap_or(1);
            // TODO check if hupa is already used
            for _ in 0..count {
                let name = read_line("Name: ");
                let desc = read_line("Description: ");
                let categories = read_line("Categories (ex: os/linux): ");
                let origin = read_line("Origin path: ");
                #[cfg(unix)]
                let origin = origin.replace('~', env!("HOME"));
                let autobackup = read_line_bool("Enable autobackup (y/n)? ",
                                                "The answer is yes or no");
                println!("{} is now added.", name.yellow());
                let hupa = Hupa::new(name,
                                     desc,
                                     categories.split('/').map(|s| s.to_string()).collect(),
                                     Hupa::get_default_backup_parent().expect("Can't get default backup parent"),
                                     origin,
                                     autobackup);
                hupas.push(hupa);
            }
            save_hupas(&config, &hupas);
        }
        ("remove", Some(sub_m)) => {
            // TODO show to the user which one is remove
            // TODO add security
            let hupas_to_remove = if let Some(hupas_names) = sub_m.values_of("hupa") {
                let hupas_names: Vec<String> = hupas_names.map(|s| s.to_string()).collect();
                resolve_names(&hupas_names, &hupas)
            } else {
                select_hupas(&hupas, "Select hupas to remove")
            };
            let hupas = hupas
                .into_iter()
                .filter(|h| !hupas_to_remove.contains(h))
                .collect::<Vec<Hupa>>();
            for h in &hupas_to_remove {
                println!("{} is now removed.", h.get_name().yellow().bold());
            }
            save_hupas(&config, &hupas);
        }
        ("modify", Some(sub_m)) => {
            let mut hupas_to_modify = if let Some(hupas_names) = sub_m.value_of("hupa") {
                let hupas_names: Vec<String> = hupas_names.iter().map(|s| s.to_string()).collect();
                resolve_names(&hupas_names, &hupas)
            } else {
                select_hupas(&hupas, "Select hupas to modify")
            };
            for hupa in &mut hupas_to_modify {
                println!("Hupa {}:", hupa.get_name());
                println!("[1] Set name");
                println!("[2] Set desc");
                println!("[3] Set categories");
                println!("[4] Set backup parent");
                println!("[5] Set origin path");
                println!("[6] Set autobackup");
                println!("[7] Cancel");
                let readed = read_line_usize("Select action [1-7]: ", "", 7);
            }
            // TODO
        }
        ("print", Some(sub_m)) => {
            for hupa in &hupas {
                let mut size_b = ColoredString::default();
                let mut size_o = ColoredString::default();
                if sub_m.is_present("size") {
                    size_b = format!(" ({})",
                                     hupa.get_backup_size()
                                         .unwrap_or(0)
                                         .file_size(DEFAULT_FSO)
                                         .expect("Error when showing size"))
                            .bold();
                    size_o = format!(" ({})",
                                     hupa.get_origin_size()
                                         .unwrap_or(0)
                                         .file_size(DEFAULT_FSO)
                                         .expect("Error when showing size"))
                            .bold();
                }
                let autobackup = if hupa.is_autobackup_enabled() {
                    format!("autobackup: {}", "enabled".green())
                } else {
                    format!("autobackup: {}", "disabled".red())
                };
                println!("{}/{}{} {} {}{}:\n{}\ndescription: {}\n",
                         hupa.get_categories_str().bold(),
                         hupa.get_name().yellow().bold(),
                         size_b,
                         "<->".bold(),
                         hupa.get_origin().display().to_string().bold(),
                         size_o,
                         autobackup,
                         hupa.get_desc().dimmed());
                hupa.needs_root();
            }
        }
        ("backup", Some(sub_m)) => {
            if sub_m.is_present("all") {
                backup(&hupas);
            } else if let Some(hupas_names) = sub_m.values_of("hupa") {
                let hupas_names: Vec<String> = hupas_names.map(|s| s.to_string()).collect();
                backup(&resolve_names(&hupas_names, &hupas));
            } else {
                let hupas = select_hupas(&hupas, "Select hupas to backup");
                backup(&hupas);
            }
        }
        ("restore", Some(sub_m)) => {
            let hupas = if sub_m.is_present("all") {
                hupas
            } else if let Some(hupas_names) = sub_m.values_of("hupa") {
                let hupas_names: Vec<String> = hupas_names.map(|s| s.to_string()).collect();
                resolve_names(&hupas_names, &hupas)
            } else {
                select_hupas(&hupas, "Select hupas to restore")
            };
            #[cfg(not(unix))]
            restore(&hupas);
            #[cfg(unix)]
            restore(&hupas, sub_m.is_present("ignore_root"));
        }
        ("clean", Some(sub_m)) => {
            if sub_m.is_present("all") {
                clean(&hupas);
            } else if let Some(hupas_names) = sub_m.values_of("hupa") {
                let hupas_names: Vec<String> = hupas_names.map(|s| s.to_string()).collect();
                clean(&resolve_names(&hupas_names, &hupas));
            } else {
                let hupas = select_hupas(&hupas, "Select hupas to clean");
                clean(&hupas);
            }
        }
        (s, _) => println!("`{}` is not supported yet", s),
    }
}

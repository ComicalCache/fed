use std::path::PathBuf;

#[derive(Default)]
pub struct Cli {
    pub path: Option<PathBuf>,

    pub doc_mode: Option<String>,
    pub view_mode: Option<String>,
}

impl Cli {
    pub fn parse() -> Self {
        let mut cli = Cli::default();
        let mut args = std::env::args().skip(1);

        while let Some(arg) = args.next() {
            if arg.starts_with("--") {
                if arg == "--help" {
                    Self::print_help();
                    std::process::exit(0);
                }
                if arg == "--version" {
                    Self::print_version();
                    std::process::exit(0);
                }

                let (key, value) = if let Some(sep) = arg.find('=') {
                    (&arg[2..sep], arg[sep + 1..].to_string())
                } else {
                    Self::print_help();
                    std::process::exit(1);
                };

                match key {
                    "dmode" => {
                        cli.doc_mode = Some(value);
                    }
                    "vmode" => {
                        cli.view_mode = Some(value);
                    }
                    _ => {
                        Self::print_help();
                        std::process::exit(1);
                    }
                }
            } else if arg.starts_with('-') {
                if arg == "-h" {
                    Self::print_help();
                    std::process::exit(0);
                }
                if arg == "-v" {
                    Self::print_version();
                    std::process::exit(0);
                }

                Self::print_help();
                std::process::exit(1);
            } else {
                cli.path = Some(PathBuf::from(arg));
                break;
            }
        }

        cli
    }

    fn print_help() {
        Self::print_version();
        println!();
        println!("Usage:");
        println!("  fed [options] [path]");
        println!();
        println!("Arguments:");
        println!("  path              Optional file or directory path");
        println!();
        println!("Options:");
        println!("  --dmode=<MODE>    Sets the initial document's mode");
        println!("  --vmode=<MODE>    Sets the initial view's mode");
        println!("  -h, --help        Print help information");
        println!("  -v, --version     Print version information");
    }

    fn print_version() {
        println!("fed - file editor (v{})", env!("CARGO_PKG_VERSION"));
    }
}

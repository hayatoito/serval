use failure;
use loggerv;
use serval;

use std::fs;
use std::io::prelude::*;
use structopt::StructOpt;

type Result<T> = std::result::Result<T, failure::Error>;

#[derive(StructOpt, Debug)]
struct Opt {
    #[structopt(short = "v", parse(from_occurrences))]
    verbose: u64,
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt, Debug)]
enum Command {
    #[structopt(name = "parse-html")]
    ParseHtml { html: String },
    #[structopt(name = "layout")]
    Layout { html: String, stylesheet: String },
    #[structopt(name = "paint")]
    Paint {
        html: String,
        stylesheet: String,
        output_file: String,
        #[structopt(name = "format")]
        format: String,
    },
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    loggerv::init_with_verbosity(opt.verbose).unwrap();
    match opt.cmd {
        Command::ParseHtml { html } => {
            let mut f = fs::File::open(html)?;
            let mut s = String::new();
            f.read_to_string(&mut s)?;
            println!("{:#}", serval::parse_html(&s)?);
        }
        Command::Layout { html, stylesheet } => {
            let mut f = fs::File::open(html)?;
            let mut html = String::new();
            f.read_to_string(&mut html)?;

            let mut f = fs::File::open(stylesheet)?;
            let mut stylesheet = String::new();
            f.read_to_string(&mut stylesheet)?;
            println!("{}", serval::dump_layout(&html, &stylesheet)?);
        }
        Command::Paint {
            html,
            stylesheet,
            output_file,
            format,
        } => {
            let mut f = fs::File::open(html)?;
            let mut html = String::new();
            f.read_to_string(&mut html)?;
            let mut f = fs::File::open(stylesheet)?;
            let mut stylesheet = String::new();
            f.read_to_string(&mut stylesheet)?;
            serval::paint_and_save(&html, &stylesheet, output_file, &format)?;
        }
    }
    Ok(())
}

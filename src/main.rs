use std::process::ExitCode;

use clap::Parser;

use crate::args::FabricateOptions;

mod args;
mod cache;
mod commands;
mod config;
mod dependencies;
mod ninja;

fn main() -> ExitCode {
    let opts = FabricateOptions::parse();

    env_logger::init();

    if let Err(err) = commands::run(opts) {
        eprintln!("{err}");
        if err.chain().len() > 1 {
            eprintln!("Caused by:");
            for (i, sub_error) in err.chain().skip(1).enumerate() {
                for (j, line) in sub_error.to_string().lines().enumerate() {
                    if j == 0 {
                        eprintln!("  {i}: {line}");
                    } else {
                        eprintln!("     {line}");
                    }
                }
            }
        }

        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

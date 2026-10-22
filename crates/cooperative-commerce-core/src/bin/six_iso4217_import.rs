//! Normalize a retained SIX ISO 4217 List One XML file to deterministic JSON.
//! Does not download, authenticate, or activate the code list.

use cooperative_commerce_core::code_list::six_xml::parse_six_list_one_xml;
use std::error::Error;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let path = args.next().map(PathBuf::from).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "usage: six-iso4217-import <retained-list-one.xml>")
    })?;
    if args.next().is_some() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected exactly one input path").into());
    }
    let bytes = fs::read(path)?;
    let xml = String::from_utf8(bytes).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, format!("input is not valid UTF-8: {e}"))
    })?;
    print!("{}", parse_six_list_one_xml(&xml)?.to_json());
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("six-iso4217-import: {error}");
            ExitCode::FAILURE
        }
    }
}

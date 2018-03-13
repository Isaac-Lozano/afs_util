extern crate afs_util;
extern crate getopts;

use std::io::{self, BufReader, BufWriter};
use std::env;
use std::error::Error;
use std::fs::{File, DirBuilder};
use std::path::PathBuf;
use std::process;

use afs_util::{AfsReader, AfsWriter};

use getopts::Options;

trait UnwrapOrBarfExt<T> {
    fn unwrap_or_barf(self, err_str: &str) -> T;
}

impl<T, E> UnwrapOrBarfExt<T> for Result<T, E>
    where E: Error
{
    fn unwrap_or_barf(self, err_desc: &str) -> T {
        self.unwrap_or_else(|err| {
            let err_string = format!("{}: {}", err_desc, err);
            barf(&err_string);
        })
    }
}

impl<T> UnwrapOrBarfExt<T> for Option<T> {
    fn unwrap_or_barf(self, err_desc: &str) -> T {
        self.unwrap_or_else(|| {
            let err_string = format!("{}", err_desc);
            barf(&err_string);
        })
    }
}

struct FileGetter {
    path: PathBuf,
    total_files: usize,
    idx: usize,
}

impl FileGetter {
    fn new<P>(into_path: P) -> FileGetter
        where P: Into<PathBuf>
    {
        let path = into_path.into();

        let mut file_count = 0;
        loop {
            let filename = format!("{}.adx", file_count);
            if !path.join(filename).exists() {
                break;
            }
            file_count += 1;
        }

        FileGetter {
            path: path,
            total_files: file_count,
            idx: 0,
        }
    }
}

impl Iterator for FileGetter {
    type Item = BufReader<File>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx == self.total_files {
            None
        }
        else {
            let filename = format!("{}.adx", self.idx);
            self.idx += 1;
            File::open(self.path.join(filename))
                .ok()
                .map(|f| BufReader::new(f))
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.total_files, Some(self.total_files))
    }
}

impl ExactSizeIterator for FileGetter {}

fn main() {
    let mut args = env::args();
    let prog_name = args.next().unwrap();
    let options: Vec<_> = args.collect();

    // Create options
    let mut opts = Options::new();
    opts.optflag("e", "extract", "Extract afs file");
    opts.optflag("p", "pack", "Pack afs file");
    opts.optflag("h", "help", "Print this help menu");

    let matches = opts.parse(&options).unwrap_or_barf("Could not parse options");

    if matches.opt_present("h") {
        help(&prog_name, opts);
    }

    let mut free_iter = matches.free.iter();

    if matches.opt_present("e") && matches.opt_present("p") {
        barf("Cannot use both extract and pack flags");
    }

    if !(matches.opt_present("e") || matches.opt_present("p")) {
        barf("Must specify either pack or extract flag");
    }

    if matches.opt_present("e") {
        let afs_filename = free_iter.next().unwrap_or_barf("No afs filename specified");
        let folder_name = free_iter.next().unwrap_or_barf("No extract foldername specified");
        extract(afs_filename, folder_name);
        println!("Extracted all files successfully.")
    }

    if matches.opt_present("p") {
        let folder_name = free_iter.next().unwrap_or_barf("No extract foldername specified");
        let afs_filename = free_iter.next().unwrap_or_barf("No afs filename specified");
        pack(folder_name, afs_filename);
        println!("Packed all files successfully.")
    }
}

fn barf(message: &str) -> ! {
    println!("Error: {}", message);
    process::exit(1);
}

fn help(prog_name: &str, opts: Options) -> ! {
    let brief = format!("Usage: {} (-e|-p) INPUT OUTPUT", prog_name);
	println!("afs_util {}", env!("CARGO_PKG_VERSION"));
    print!("{}", opts.usage(&brief));
    process::exit(0);
}

fn extract(afs_filename: &str, folder_name: &str) {
    let file = BufReader::new(File::open(afs_filename).unwrap_or_barf("Afs file not found"));
    let mut afs = AfsReader::new(file).unwrap_or_barf("Could not parse afs file");

    DirBuilder::new()
        .recursive(true)
        .create(folder_name)
        .unwrap_or_barf("Could not create extraction folder");

    let len = afs.len();
    for idx in 0..len {
        let mut test = afs.open(idx).unwrap().unwrap_or_barf("Could not extract file from afs archive");
        let mut out = BufWriter::new(File::create(format!("{}/{}.adx", folder_name, idx)).unwrap_or_barf("Could not extract file from afs archive"));
        io::copy(&mut test, &mut out).unwrap_or_barf("Could not extract file from afs archive");
    }
}

fn pack(folder_name: &str, afs_filename: &str) {
    let output_file = BufWriter::new(File::create(afs_filename).unwrap_or_barf("Could not open afs file"));

    let file_getter = FileGetter::new(folder_name);
    let afs_writer = AfsWriter::new(output_file, file_getter);
    afs_writer.write().unwrap_or_barf("Error writing afs file");
}
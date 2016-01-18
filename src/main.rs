mod wave;
mod flac;

extern crate byteorder;
extern crate docopt;
extern crate rustc_serialize;
extern crate num;

use std::fs::File;
use std::io::prelude::*;
use byteorder::{LittleEndian, ByteOrder, ReadBytesExt, WriteBytesExt};
use docopt::Docopt;
use wave::Wave;
use flac::Flac;

fn main() {
    const USAGE: &'static str = "
    Usage: surfboard -r <source>
       surfboard -w <source> <dest>
       surfboard -h

    Options:
        -r, --read   Parse file.
        -w, --write  Write data to file.
        -h, --help   Show this message.
    ";

    #[derive(RustcDecodable, Debug)]
    struct Args {
        arg_source: String,
        arg_dest: Option<String>,
        flag_read: bool,
        flag_write: bool,
        flag_help: bool,
    }

    let args: Args = Docopt::new(USAGE).unwrap().decode().unwrap_or_else(|e| e.exit());

    if args.flag_write && args.arg_dest.is_some() {
        let mut data_file = File::open(&args.arg_source).unwrap();

        let mut data = Vec::new();
        data_file.read_to_end(&mut data).unwrap();

        let mut wav_file = File::create(&args.arg_dest.unwrap()).unwrap();

        Wave::write(&mut wav_file, 1, 44100, 8, data);
    }

    if args.flag_read || args.flag_write {
        let mut read_test = File::open(&args.arg_source).unwrap();

        let file_id = read_test.read_u32::<LittleEndian>().unwrap();

        let mut t = vec![];

        t.write_u32::<LittleEndian>(file_id).unwrap();
        let file_id = String::from_utf8(t).unwrap();

        println!("file id: {}", file_id);

        match &*file_id {
            "RIFF" => { Wave::parse(&mut read_test); },
            "fLaC" => { Flac::parse(&mut read_test); },
            _ => panic!("Unrecognized file type"),
        }

    }
}

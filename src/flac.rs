#![allow(dead_code)]

use std::io::prelude::*;
use std::io;

pub struct Flac {
    data: Vec<u8>,
}

impl Flac {
    pub fn parse<R: io::Read + io::Seek>(r: &mut R) -> Flac {
        let mut data = Vec::new();
        r.read_to_end(&mut data).unwrap();

        Flac {
            data: data,
        }
    }
}
